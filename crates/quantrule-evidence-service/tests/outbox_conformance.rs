//! The outbox and release-tombstone conformance suite.
//!
//! The shared body runs against the memory store in the fast gate. The Postgres
//! case is opt-in because it needs a live server; `tools/postgres_composed.sh`
//! gives it a disposable database.
//!
//! The suite covers the failure shapes the protocol exists for: a crash after
//! the run commit (the intent is pending), a repeated publish (one run record),
//! a restart (the intent is re-armed), a computed run (the notification commits
//! with the digest), a terminal run (no intent remains), and an erase (the
//! release tombstone survives the delete and is confirmed by the reconciler).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use quantrule_api::{
    DatasetReferenceCount, DatasetReferenceKind, Digest, ProfileId, RunState, StageProgress,
};
use quantrule_evidence_service::leader::LeaderLease;
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::outbox::{
    DRAIN_LIMIT, DrainReport, ReconcileMode, RunOutbox, drain, reconcile,
};
use quantrule_evidence_service::postgres::PgRunStore;
use quantrule_evidence_service::postgres_leader::PgLeaderLease;
use quantrule_evidence_service::reconcile::release_all;
use quantrule_evidence_service::releases::ReferenceReleaseLog;
use quantrule_evidence_service::runs::{MemoryRuns, NewRun, RunStore};
use quantrule_runtime::adapters::{Backend, BoxFuture, Queue, QueueError};
use quantrule_runtime::client::{ClientError, DataClient, Service};
use quantrule_service_api::{ConfigBundle, DatasetObject};

type Published = Arc<Mutex<Vec<(String, Vec<u8>)>>>;

#[derive(Clone, Default)]
struct RecordingQueue {
    published: Published,
    failing: Arc<AtomicBool>,
}

impl RecordingQueue {
    fn messages(&self) -> Vec<(String, Vec<u8>)> {
        self.published.lock().expect("the queue lock").clone()
    }

    fn set_failing(&self, failing: bool) {
        self.failing.store(failing, Ordering::SeqCst);
    }
}

impl Queue for RecordingQueue {
    fn backend(&self) -> Backend {
        Backend::InProcess
    }

    fn publish(&self, routing_key: &str, payload: Vec<u8>) -> BoxFuture<Result<(), QueueError>> {
        if self.failing.load(Ordering::SeqCst) {
            return Box::pin(async { Err(QueueError("the broker is down".to_owned())) });
        }
        let published = Arc::clone(&self.published);
        let routing_key = routing_key.to_owned();
        Box::pin(async move {
            published
                .lock()
                .map_err(|_| QueueError("the queue lock is poisoned".to_owned()))?
                .push((routing_key, payload));
            Ok(())
        })
    }
}

/// A data client that records releases and can refuse them.
#[derive(Clone, Default)]
struct RecordingData {
    releases: Arc<Mutex<Vec<String>>>,
    failing: Arc<AtomicBool>,
}

impl DataClient for RecordingData {
    fn config_bundle(
        &self,
        _profile: ProfileId,
        _config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>> {
        Box::pin(async { Ok(None) })
    }

    fn dataset_object(
        &self,
        _profile: ProfileId,
        _data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        Box::pin(async { Ok(None) })
    }

    fn dataset_count(&self, _profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        Box::pin(async { Ok(0) })
    }

    fn register_dataset_reference(
        &self,
        _profile: ProfileId,
        _data_version: Digest,
        _reference_id: String,
        _kind: DatasetReferenceKind,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        Box::pin(async { Ok(DatasetReferenceCount { references: 1 }) })
    }

    fn release_dataset_reference(
        &self,
        _profile: ProfileId,
        _data_version: Digest,
        reference_id: String,
    ) -> BoxFuture<Result<DatasetReferenceCount, ClientError>> {
        let failing = Arc::clone(&self.failing);
        let releases = Arc::clone(&self.releases);
        Box::pin(async move {
            if failing.load(Ordering::SeqCst) {
                return Err(ClientError::Status {
                    service: Service::Data,
                    status: 503,
                    message: "the data service is down".to_owned(),
                });
            }
            releases
                .lock()
                .map_err(|_| ClientError::Unreachable {
                    service: Service::Data,
                    message: "the release lock is poisoned".to_owned(),
                })?
                .push(reference_id);
            Ok(DatasetReferenceCount { references: 0 })
        })
    }
}

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn run(seed: &[u8]) -> NewRun {
    NewRun {
        run_id: quantrule_api::RunId(Digest::sha256(seed)),
        lineage: Vec::new(),
        config_id: "cfg_conformance".to_owned(),
        data_version: Digest::sha256(b"conformance-data"),
        config_hash: Digest::sha256(b"conformance-config"),
        model_version: "M1.0".to_owned(),
        image_digest: Digest::sha256(b"conformance-image"),
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

fn done_stages() -> Vec<StageProgress> {
    Vec::new()
}

/// Clear the evidence tables before a Postgres case.
///
/// The composed script gives each test target one disposable database, and the
/// ignored cases in a target share it. Each Postgres case starts from an empty
/// run/outbox state so a global `pending` or `outstanding` count means what the
/// assertion says. The script runs these cases with one test thread.
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

async fn claim<S>(store: &S, run_id: &str) -> u64
where
    S: RunStore + RunOutbox + ReferenceReleaseLog,
{
    let claimed = store
        .claim(run_id.to_owned(), "conformance-worker".to_owned(), None)
        .await
        .expect("claim");
    let quantrule_evidence_service::runs::ClaimOutcome::Claimed(record) = claimed else {
        panic!("the run was not claimable");
    };
    record.lease_term
}

async fn conformance<S>(store: &S, leader_term: u64)
where
    S: RunStore + RunOutbox + ReferenceReleaseLog,
{
    let owner = profile("conformance");
    let queue = RecordingQueue::default();
    let emitted = store
        .submit(owner.clone(), run(b"conformance-run"))
        .await
        .expect("submit");
    let run_id = emitted.record().run_id.to_string();

    // Crash after commit: the run and its intent committed, nothing published.
    let pending = store.pending(DRAIN_LIMIT).await.expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind.routing_key(), "run.requested");
    assert_eq!(store.outstanding().await.expect("outstanding"), 1);

    // A failed publish keeps the intent.
    queue.set_failing(true);
    let report: DrainReport = drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(report.published, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(store.outstanding().await.expect("outstanding"), 1);
    queue.set_failing(false);
    let report = drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(report.published, 1);
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
    assert_eq!(queue.messages().len(), 1);

    // A duplicate submission finds one run row and republishes the queued run
    // so a lost publish can be recovered; it never creates a second run.
    let again = store
        .submit(owner.clone(), run(b"conformance-run"))
        .await
        .expect("resubmit");
    assert!(!again.created());
    drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 2);
    assert_eq!(
        store
            .list(owner.clone(), None, 10)
            .await
            .expect("list")
            .len(),
        1
    );

    // A restart re-arms the unfinished run and republishes it.
    let report = reconcile(store, ReconcileMode::Startup)
        .await
        .expect("startup");
    assert_eq!(report.rearmed, 1);
    drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 3);

    // Claim, compute, and finalise: the computed intent commits with the digest
    // and the terminal state removes it.
    let term = claim(store, &run_id).await;
    store
        .computed(
            run_id.clone(),
            term,
            Digest::sha256(b"conformance-digest"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("computed");
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].kind.routing_key(), "run.computed");
    let report = drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(report.published, 1);
    assert_eq!(
        queue.messages().last().map(|(key, _)| key.as_str()),
        Some("run.computed")
    );
    store
        .complete(
            run_id.clone(),
            term,
            leader_term,
            Digest::sha256(b"conformance-digest"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("complete");
    assert!(
        store
            .wanted_notifications()
            .await
            .expect("wanted")
            .is_empty()
    );
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);

    // A running run without a digest is unfinished: a restart re-arms its
    // request exactly once, and a periodic pass inserts but never repeats.
    let published = queue.messages().len();
    let running = store
        .submit(owner.clone(), run(b"conformance-running"))
        .await
        .expect("running submit");
    let running_id = running.record().run_id.to_string();
    drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), published + 1);
    let running_term = claim(store, &running_id).await;
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].kind.routing_key(), "run.requested");
    let report = reconcile(store, ReconcileMode::Startup)
        .await
        .expect("startup");
    assert_eq!(report.wanted, 1);
    assert_eq!(report.rearmed, 1);
    drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(
        queue.messages().len(),
        published + 2,
        "a restart did not republish the running run"
    );

    // A periodic pass re-arms the expired lease after its window; this test
    // store uses a zero window, so the pass observes it immediately.
    let report = reconcile(store, ReconcileMode::Periodic)
        .await
        .expect("periodic");
    assert_eq!(report.inserted, 0);
    assert_eq!(report.rearmed, 1);
    drain(store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), published + 3);

    store
        .fail(
            running_id.clone(),
            running_term,
            "the recovered run failed".to_owned(),
            Vec::new(),
        )
        .await
        .expect("fail");
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
    assert!(
        store
            .wanted_notifications()
            .await
            .expect("wanted")
            .is_empty()
    );
    assert_eq!(
        store
            .list(owner.clone(), None, 10)
            .await
            .expect("list")
            .len(),
        2,
        "the recovery created a second run record"
    );

    // An erase writes a release tombstone that survives the row.
    let erase_run = store
        .submit(owner.clone(), run(b"conformance-erase"))
        .await
        .expect("erase submit");
    let erase_id = erase_run.record().run_id.to_string();
    let erase_term = claim(store, &erase_id).await;
    store
        .fail(
            erase_id.clone(),
            erase_term,
            "failed for the erase test".to_owned(),
            Vec::new(),
        )
        .await
        .expect("fail");
    store
        .archive(
            owner.clone(),
            erase_id.clone(),
            "2026-11-22T00:00:00Z".to_owned(),
        )
        .await
        .expect("archive");
    store
        .erase(owner.clone(), erase_id.clone())
        .await
        .expect("erase");
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
    let pending = store.pending_releases().await.expect("releases");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].reference_id, format!("run:{erase_id}"));

    let data = RecordingData::default();
    data.failing.store(true, Ordering::SeqCst);
    let error = release_all(store, &data)
        .await
        .expect_err("a failed release");
    assert!(error.contains("not released"), "{error}");
    assert_eq!(
        store.pending_releases().await.expect("releases").len(),
        1,
        "a failed release dropped its tombstone"
    );

    data.failing.store(false, Ordering::SeqCst);
    assert_eq!(release_all(store, &data).await.expect("release"), 1);
    assert!(store.pending_releases().await.expect("releases").is_empty());
    assert_eq!(
        data.releases.lock().expect("releases").as_slice(),
        [format!("run:{erase_id}")]
    );
}

#[tokio::test]
async fn the_memory_outbox_conforms() {
    let store = MemoryRuns::with_lease_seconds(0);
    let leader_term = store
        .leader_lease()
        .acquire("conformance-leader".to_owned())
        .await
        .expect("leader lease")
        .expect("this process is the leader");
    conformance(&store, leader_term).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_outbox_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
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
    let store = PgRunStore::with_lease_seconds(database.pool().clone(), 0);
    let leader_term = PgLeaderLease::with_lease_seconds(database.pool().clone(), 3_600)
        .acquire("conformance-leader".to_owned())
        .await
        .expect("leader lease")
        .expect("this process is the leader");
    conformance(&store, leader_term).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_periodic_pass_inserts_a_missing_running_intent() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
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
    let store = PgRunStore::with_lease_seconds(database.pool().clone(), 0);
    let owner = profile("pg-missing-intent");
    let submitted = store
        .submit(owner, run(b"pg-missing-intent"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let _term = claim(&store, &id).await;
    // A run row written before the outbox existed has no intent row.
    sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
        .bind(&id)
        .execute(database.pool())
        .await
        .expect("delete the intent");
    assert!(store.pending(10).await.expect("pending").is_empty());

    let report = reconcile(&store, ReconcileMode::Periodic)
        .await
        .expect("periodic");
    assert_eq!(report.wanted, 1);
    assert_eq!(report.inserted, 1);
    let pending = store.pending(10).await.expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind.routing_key(), "run.requested");
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_lease_fences_a_stale_worker() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
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
    let store = PgRunStore::new(database.pool().clone());
    let leader_term = PgLeaderLease::with_lease_seconds(database.pool().clone(), 3_600)
        .acquire("pg-lease-leader".to_owned())
        .await
        .expect("leader lease")
        .expect("this process is the leader");
    let owner = profile("pg-lease");
    let submitted = store.submit(owner, run(b"pg-lease")).await.expect("submit");
    let run_id = submitted.record().run_id.to_string();
    let first = claim(&store, &run_id).await;
    assert_eq!(first, 1);

    let stolen = store
        .claim(run_id.clone(), "other-worker".to_owned(), None)
        .await
        .expect("other claim");
    assert!(
        matches!(
            stolen,
            quantrule_evidence_service::runs::ClaimOutcome::Running(_)
        ),
        "a live lease was stolen"
    );
    let stale = store
        .computed(
            run_id.clone(),
            0,
            Digest::sha256(b"stale-digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("stale computed");
    assert_eq!(stale.digest, None);
    let stale = store
        .fail(run_id.clone(), 0, "stale failure".to_owned(), Vec::new())
        .await
        .expect("stale failed");
    assert_eq!(stale.status, RunState::Running);
    assert_eq!(stale.error, None);
    assert!(matches!(
        store
            .heartbeat(run_id.clone(), "other-worker".to_owned(), first)
            .await
            .expect("other heartbeat"),
        quantrule_evidence_service::runs::HeartbeatOutcome::Stale
    ));

    let continued = store
        .claim(run_id.clone(), "conformance-worker".to_owned(), Some(first))
        .await
        .expect("continuation");
    let quantrule_evidence_service::runs::ClaimOutcome::Claimed(continued) = continued else {
        panic!("the same holder could not continue its retry");
    };
    assert_eq!(continued.lease_term, first + 1);
    assert_eq!(continued.attempts, 2);

    let stale = store
        .complete(
            run_id.clone(),
            first,
            leader_term,
            Digest::sha256(b"stale-digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("stale complete");
    assert_eq!(stale.status, RunState::Running);
    assert_eq!(stale.digest, None);

    let current = store
        .complete(
            run_id.clone(),
            continued.lease_term,
            leader_term,
            Digest::sha256(b"current-digest"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("current complete");
    assert_eq!(current.status, RunState::Succeeded);
    assert_eq!(current.lease_holder, None);
    assert_eq!(current.lease_expires_at, None);
    assert!(matches!(
        store
            .claim(run_id, "third-worker".to_owned(), None)
            .await
            .expect("terminal claim"),
        quantrule_evidence_service::runs::ClaimOutcome::Terminal(_)
    ));

    // A live lease is not wanted; an expired one is. This is the selection
    // the outbox uses, not a re-arm of a live run.
    let wanted_run = store
        .submit(profile("pg-wanted"), run(b"pg-wanted"))
        .await
        .expect("wanted submit");
    let wanted_id = wanted_run.record().run_id.to_string();
    claim(&store, &wanted_id).await;
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert!(
        !wanted.iter().any(|notification| {
            notification.run_id.to_string() == wanted_id
                && notification.kind.routing_key() == "run.requested"
        }),
        "a live lease was wanted"
    );
    sqlx::query(
        "UPDATE runs.runs SET lease_expires_at = now() - interval '1 second'
          WHERE run_id = $1",
    )
    .bind(&wanted_id)
    .execute(database.pool())
    .await
    .expect("expire the lease");
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert!(
        wanted.iter().any(|notification| {
            notification.run_id.to_string() == wanted_id
                && notification.kind.routing_key() == "run.requested"
        }),
        "an expired lease was not wanted"
    );
}
