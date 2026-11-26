//! Evidence leader lease, fencing, and leader-only finalisation.
//!
//! The memory cases run in the fast gate. The Postgres cases are opt-in because
//! they need a live server; `tools/postgres_composed.sh` gives this target a
//! disposable database.

mod common;

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::{
    ArtifactRef, Digest, PROFILE_HEADER, ProfileId, RunState, RunSummary, StageName, StageProgress,
    StageStatus,
};
use quantrule_evidence_service::evidence::{
    EvidenceError, EvidenceServices as FinalizerServices, FinalizeOutcome, RunManifest, finalize,
};
use quantrule_evidence_service::leader::{
    LeaderConsumer, LeaderLease, Leadership, release_leadership, spawn_leader_election,
};
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::postgres::PgRunStore;
use quantrule_evidence_service::postgres_leader::PgLeaderLease;
use quantrule_evidence_service::runs::{ClaimOutcome, MemoryRuns, NewRun, RunStore, derive};
use quantrule_evidence_service::{EvidenceServices as StateServices, EvidenceState, router};
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::{
    Backend, BoxFuture, Message, MessageHandler, ObjectStore, QueueConsumer, QueueError,
};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn new_run(seed: &[u8]) -> NewRun {
    let data_version = Digest::sha256(b"data");
    let config_hash = Digest::sha256(b"config");
    let image_digest = Digest::sha256(seed);
    NewRun {
        run_id: derive(&data_version, &config_hash, "M1.0", &image_digest),
        lineage: Vec::new(),
        config_id: "cfg_leader".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest,
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

fn computed_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: if name == StageName::Evidence {
            StageStatus::Pending
        } else {
            StageStatus::Done
        },
        elapsed_ms: None,
    })
    .collect()
}

/// Submit and claim the seed run, then record a manifested computed set.
async fn computed_run(runs: &MemoryRuns, objects: &MemoryObjects, seed: &[u8]) -> (String, u64) {
    let submitted = runs
        .submit(profile("default"), new_run(seed))
        .await
        .expect("submit");
    let run_id = submitted.record().run_id.to_string();
    let claimed = runs
        .claim(run_id.clone(), "leader-test-worker".to_owned(), None)
        .await
        .expect("claim");
    let ClaimOutcome::Claimed(record) = claimed else {
        panic!("the run was not claimable");
    };
    let bytes = b"{\"profit\":1}";
    let artifact = ArtifactRef {
        name: "results.json".to_owned(),
        bytes: bytes.len() as u64,
        sha256: Digest::sha256(bytes),
    };
    objects
        .put(&RunManifest::blob_key(&artifact.sha256), bytes.to_vec())
        .await
        .expect("artifact");
    let manifest = RunManifest::from_artifacts(&run_id, std::slice::from_ref(&artifact));
    objects
        .put(
            &RunManifest::key(&run_id),
            manifest.to_bytes().expect("manifest bytes"),
        )
        .await
        .expect("manifest");
    runs.computed(
        run_id.clone(),
        record.lease_term,
        manifest.digest,
        vec![artifact],
        computed_stages(),
    )
    .await
    .expect("computed");
    (run_id, record.lease_term)
}

fn finalizer(runs: &Arc<MemoryRuns>, objects: &Arc<MemoryObjects>) -> FinalizerServices {
    FinalizerServices {
        runs: Arc::clone(runs) as Arc<dyn RunStore>,
        objects: Arc::clone(objects) as Arc<dyn ObjectStore>,
    }
}

fn message(run_id: &str) -> Message {
    Message {
        routing_key: "run.computed".to_owned(),
        payload: serde_json::json!({ "run_id": run_id })
            .to_string()
            .into_bytes(),
        redelivered: false,
    }
}

#[derive(Clone, Default)]
struct ScriptedConsumer {
    messages: Arc<Mutex<VecDeque<Message>>>,
    starts: Arc<AtomicUsize>,
    handled: Arc<AtomicUsize>,
}

impl ScriptedConsumer {
    fn with_message(message: Message) -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::from([message]))),
            starts: Arc::new(AtomicUsize::new(0)),
            handled: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl QueueConsumer for ScriptedConsumer {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn consume(&self, _queue: &str, handler: MessageHandler) -> BoxFuture<Result<(), QueueError>> {
        self.starts.fetch_add(1, Ordering::SeqCst);
        let messages = Arc::clone(&self.messages);
        let handled = Arc::clone(&self.handled);
        Box::pin(async move {
            loop {
                let next = messages
                    .lock()
                    .map_err(|_| QueueError("the message lock is poisoned".to_owned()))?
                    .pop_front();
                match next {
                    Some(message) => {
                        handler(message).await?;
                        handled.fetch_add(1, Ordering::SeqCst);
                    }
                    None => std::future::pending::<()>().await,
                }
            }
        })
    }
}

async fn wait_for(counter: &AtomicUsize, expected: usize) {
    for _ in 0..300 {
        if counter.load(Ordering::SeqCst) >= expected {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the counter did not reach {expected}");
}

#[tokio::test]
async fn only_the_leader_finalizes_and_a_duplicate_delivery_is_idempotent() {
    let runs = Arc::new(MemoryRuns::default());
    let objects = Arc::new(MemoryObjects::default());
    let lease = runs.leader_lease();
    let term_a = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire a")
        .expect("leader a");
    let (run_id, _run_term) = computed_run(&runs, &objects, b"leader-one").await;
    let services = Arc::new(finalizer(&runs, &objects));

    let consumer_a = Arc::new(ScriptedConsumer::with_message(message(&run_id)));
    let consumer_b = Arc::new(ScriptedConsumer::with_message(message(&run_id)));
    let (sender_a, receiver_a) = tokio::sync::watch::channel(Leadership::Leader(term_a));
    let (sender_b, receiver_b) = tokio::sync::watch::channel(Leadership::Standby);
    let task_a = tokio::spawn(
        LeaderConsumer::new(
            Arc::clone(&consumer_a) as Arc<dyn QueueConsumer>,
            "runs.evidence.g1".to_owned(),
            Arc::clone(&services),
        )
        .run(receiver_a, quantrule_runtime::Shutdown::channel().1),
    );
    let task_b = tokio::spawn(
        LeaderConsumer::new(
            Arc::clone(&consumer_b) as Arc<dyn QueueConsumer>,
            "runs.evidence.g1".to_owned(),
            Arc::clone(&services),
        )
        .run(receiver_b, quantrule_runtime::Shutdown::channel().1),
    );

    wait_for(&consumer_a.handled, 1).await;
    assert_eq!(
        runs.get_any(run_id.clone())
            .await
            .expect("get")
            .expect("record")
            .status,
        RunState::Succeeded
    );
    assert_eq!(
        consumer_b.starts.load(Ordering::SeqCst),
        0,
        "a standby must not consume the finalisation queue"
    );

    // The first replica disappears. The second acquires term 2 and receives a
    // duplicate delivery of the same message.
    lease.expire();
    let term_b = lease
        .acquire("replica-b".to_owned())
        .await
        .expect("acquire b")
        .expect("leader b");
    assert!(term_b > term_a);
    sender_a.send(Leadership::Standby).expect("stop a");
    sender_b.send(Leadership::Leader(term_b)).expect("start b");
    wait_for(&consumer_b.handled, 1).await;
    assert_eq!(
        consumer_a.starts.load(Ordering::SeqCst),
        1,
        "the old leader did not restart after its lease ended"
    );
    let record = runs
        .get_any(run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(
        runs.list(profile("default"), None, 10)
            .await
            .expect("list")
            .len(),
        1,
        "a duplicate delivery must not create a second run record"
    );
    task_a.abort();
    task_b.abort();
}

#[tokio::test]
async fn a_stale_leader_cannot_finalize_and_the_new_term_can() {
    let runs = Arc::new(MemoryRuns::default());
    let objects = Arc::new(MemoryObjects::default());
    let lease = runs.leader_lease();
    let stale_term = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire a")
        .expect("leader a");
    let (run_id, _run_term) = computed_run(&runs, &objects, b"leader-stale").await;
    lease.expire();
    let current_term = lease
        .acquire("replica-b".to_owned())
        .await
        .expect("acquire b")
        .expect("leader b");
    let services = finalizer(&runs, &objects);

    assert_eq!(
        finalize(run_id.clone(), &services, stale_term)
            .await
            .expect_err("the stale leader must be refused"),
        EvidenceError::LeaderFenced
    );
    assert_eq!(
        runs.get_any(run_id.clone())
            .await
            .expect("get")
            .expect("record")
            .status,
        RunState::Running,
        "a stale leader must not finish the run"
    );
    assert_eq!(
        finalize(run_id.clone(), &services, current_term)
            .await
            .expect("current finalizer"),
        FinalizeOutcome::Succeeded
    );
    assert_eq!(
        finalize(run_id.clone(), &services, current_term)
            .await
            .expect("duplicate delivery"),
        FinalizeOutcome::Skipped
    );
    assert_eq!(
        runs.get_any(run_id)
            .await
            .expect("get")
            .expect("record")
            .status,
        RunState::Succeeded
    );
}

#[tokio::test]
async fn a_draining_leader_releases_its_lease_for_a_standby() {
    let runs = MemoryRuns::default();
    let lease = runs.leader_lease();
    let (sender, receiver) = tokio::sync::watch::channel(Leadership::Standby);
    let (_, shutdown) = quantrule_runtime::Shutdown::channel();
    let task = spawn_leader_election(
        Arc::new(lease.clone()) as Arc<dyn LeaderLease>,
        "replica-a".to_owned(),
        sender,
        shutdown,
    );
    for _ in 0..100 {
        if matches!(*receiver.borrow(), Leadership::Leader(_)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(matches!(*receiver.borrow(), Leadership::Leader(_)));

    release_leadership(&lease, "replica-a", &receiver)
        .await
        .expect("release");
    let status = lease.current().await.expect("current").expect("row");
    assert!(!status.live, "the released lease is still live");
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("acquire b"),
        Some(2),
        "a standby cannot take over immediately"
    );
    task.abort();
}

#[tokio::test]
async fn a_shutdown_releases_the_election_lease_for_a_standby() {
    let runs = MemoryRuns::default();
    let lease = runs.leader_lease();
    let (sender, receiver) = tokio::sync::watch::channel(Leadership::Standby);
    let (trigger, shutdown) = quantrule_runtime::Shutdown::channel();
    let task = spawn_leader_election(
        Arc::new(lease.clone()) as Arc<dyn LeaderLease>,
        "replica-a".to_owned(),
        sender,
        shutdown,
    );
    for _ in 0..100 {
        if matches!(*receiver.borrow(), Leadership::Leader(_)) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(matches!(*receiver.borrow(), Leadership::Leader(_)));
    trigger.trigger();
    task.await.expect("election task");
    assert!(!lease.current().await.expect("current").expect("row").live);
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("acquire b"),
        Some(2)
    );
}

#[tokio::test]
async fn a_standby_still_serves_reads() {
    let runs = Arc::new(MemoryRuns::default());
    let submitted = runs
        .submit(profile("default"), new_run(b"standby-read"))
        .await
        .expect("submit");
    let lease = runs.leader_lease();
    lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire a")
        .expect("leader a");
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("acquire b"),
        None,
        "the second replica is a standby"
    );

    let state = Arc::new(EvidenceState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        StateServices {
            runs: Some(Arc::clone(&runs) as Arc<dyn RunStore>),
            ..StateServices::default()
        },
    ));
    let response = router(state)
        .oneshot(
            Request::get("/runs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let runs: Vec<RunSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("run list");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_id, submitted.record().run_id);
}

const PG_MIGRATION_ERROR: &str =
    "QR_TEST_DATABASE_URL must name a disposable Postgres database; this test is ignored";

async fn postgres() -> sqlx::PgPool {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(PG_MIGRATION_ERROR);
    let database = quantrule_runtime::adapters::postgres::PgDatabase::connect(&url)
        .await
        .expect("the Postgres pool");
    quantrule_runtime::adapters::postgres::apply_migrations_in(
        database.pool(),
        "runs",
        &migrations::available(),
    )
    .await
    .expect("the evidence migrations");
    reset_postgres(database.pool()).await;
    database.pool().clone()
}

async fn reset_postgres(pool: &sqlx::PgPool) {
    sqlx::query(
        "TRUNCATE runs.outbox, runs.artifacts, runs.stages, runs.submissions,
                  runs.dataset_reference_releases, runs.retention_events,
                  runs.runs CASCADE",
    )
    .execute(pool)
    .await
    .expect("truncate the evidence tables");
    sqlx::query(
        "UPDATE runs.leader_lease
            SET holder = '', term = 0, expires_at = '-infinity'",
    )
    .execute(pool)
    .await
    .expect("reset the leader lease");
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn postgres_acquisition_renewal_and_takeover() {
    let pool = postgres().await;
    let lease = PgLeaderLease::with_lease_seconds(pool.clone(), 60);
    let first = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire a")
        .expect("leader a");
    assert_eq!(first, 1);
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("acquire b"),
        None
    );
    assert_eq!(
        lease
            .renew("replica-a".to_owned(), first)
            .await
            .expect("renew"),
        Some(first)
    );
    sqlx::query("UPDATE runs.leader_lease SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .expect("expire the lease");
    let second = lease
        .acquire("replica-b".to_owned())
        .await
        .expect("takeover")
        .expect("leader b");
    assert_eq!(second, first + 1);
    assert_eq!(
        lease
            .renew("replica-a".to_owned(), first)
            .await
            .expect("stale"),
        None
    );
    lease
        .release("replica-b".to_owned(), second)
        .await
        .expect("release");
    assert_eq!(
        lease
            .acquire("replica-a".to_owned())
            .await
            .expect("reacquire")
            .expect("leader a again"),
        second + 1
    );
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn postgres_a_stale_leader_cannot_finish_a_run() {
    let pool = postgres().await;
    let store = PgRunStore::new(pool.clone());
    let lease = PgLeaderLease::with_lease_seconds(pool.clone(), 60);
    let submitted = store
        .submit(profile("pg-leader"), new_run(b"pg-leader"))
        .await
        .expect("submit");
    let run_id = submitted.record().run_id.to_string();
    let claimed = store
        .claim(run_id.clone(), "pg-worker".to_owned(), None)
        .await
        .expect("claim");
    let ClaimOutcome::Claimed(record) = claimed else {
        panic!("the run was not claimable");
    };
    let stale = lease
        .acquire("replica-a".to_owned())
        .await
        .expect("acquire a")
        .expect("leader a");
    sqlx::query("UPDATE runs.leader_lease SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .expect("expire the lease");
    let current = lease
        .acquire("replica-b".to_owned())
        .await
        .expect("acquire b")
        .expect("leader b");
    assert!(current > stale);

    let refused = store
        .complete(
            run_id.clone(),
            record.lease_term,
            stale,
            Digest::sha256(b"stale-digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("stale complete");
    assert_eq!(refused.status, RunState::Running);
    assert_eq!(refused.digest, None);
    let refused = store
        .fail_finalization(
            run_id.clone(),
            record.lease_term,
            stale,
            "stale failure".to_owned(),
            Vec::new(),
        )
        .await
        .expect("stale fail");
    assert_eq!(refused.status, RunState::Running);

    let digest = Digest::sha256(b"current-digest");
    let finished = store
        .complete(
            run_id.clone(),
            record.lease_term,
            current,
            digest.clone(),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("current complete");
    assert_eq!(finished.status, RunState::Succeeded);
    assert_eq!(finished.digest, Some(digest));
    let duplicate = store
        .complete(
            run_id.clone(),
            record.lease_term,
            current,
            Digest::sha256(b"other-digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("duplicate complete");
    assert_eq!(duplicate.status, RunState::Succeeded);
    assert_eq!(duplicate.digest, finished.digest);
    assert_eq!(
        store
            .list(profile("pg-leader"), None, 10)
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn postgres_a_read_works_while_the_other_replica_is_the_leader() {
    let pool = postgres().await;
    let store = PgRunStore::new(pool.clone());
    let lease = PgLeaderLease::with_lease_seconds(pool.clone(), 60);
    let submitted = store
        .submit(profile("pg-read"), new_run(b"pg-read"))
        .await
        .expect("submit");
    assert!(
        lease
            .acquire("replica-a".to_owned())
            .await
            .expect("acquire a")
            .is_some()
    );
    assert_eq!(
        lease
            .acquire("replica-b".to_owned())
            .await
            .expect("acquire b"),
        None
    );
    let listed = store
        .list(profile("pg-read"), None, 10)
        .await
        .expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].run_id, submitted.record().run_id);
}
