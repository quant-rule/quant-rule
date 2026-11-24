//! The stage lease, request outbox, and rebuild conformance suite.
//!
//! The suite covers the failure shapes the stage control plane exists for: a
//! live lease is never stolen, an expired lease is reclaimable, a stale term
//! cannot overwrite a later attempt, a terminal row is never claimed or
//! re-armed, a rebuild publishes the exact unfinished set, and a duplicate
//! delivery does not create a second execution or artifact.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageProvenance,
    StageState,
};
use quantrule_evidence_service::leader::Leadership;
use quantrule_evidence_service::outbox::ReconcileMode;
use quantrule_evidence_service::stage::outbox::{
    DRAIN_LIMIT, StageOutbox, drain, reconcile, spawn_leader_reconcile,
};
use quantrule_evidence_service::stage::{
    MemoryStages, NewStageRun, StageClaimOutcome, StageHeartbeatOutcome, StageStore,
};
use quantrule_runtime::adapters::{Backend, BoxFuture, Queue, QueueError};

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

    fn stage_ids(&self) -> Vec<String> {
        self.messages()
            .into_iter()
            .map(|(_, payload)| {
                let body: serde_json::Value =
                    serde_json::from_slice(&payload).expect("the stage payload");
                body["stage_run_id"]
                    .as_str()
                    .expect("the payload carries a stage run id")
                    .to_owned()
            })
            .collect()
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

fn profile() -> ProfileId {
    ProfileId::parse("default").expect("profile")
}

fn run(seed: &[u8]) -> NewStageRun {
    let data_version = Digest::sha256(seed);
    NewStageRun::new(
        StageKind::Model,
        "cfg-1".to_owned(),
        StageInput::DataVersion { data_version },
        StageProvenance {
            data_version: Digest::sha256(seed),
            config_hash: Digest::sha256(b"config"),
            model_version: "M1.0".to_owned(),
            image_digest: Digest::sha256(b"image"),
        },
        "default".to_owned(),
        "2026-11-24T00:00:00Z".to_owned(),
    )
}

fn artifact(seed: &[u8]) -> StageArtifact {
    StageArtifact::from_files(vec![ArtifactRef {
        name: "summary.txt".to_owned(),
        bytes: seed.len() as u64,
        sha256: Digest::sha256(seed),
    }])
    .expect("the artifact")
}

#[tokio::test]
async fn a_rebuild_rearms_the_exact_unfinished_set() {
    let store = MemoryStages::default();
    let queue = RecordingQueue::default();

    let queued = store
        .create(profile(), run(b"queued"))
        .await
        .expect("the queued stage");
    let queued_id = queued.record().stage_run_id.to_string();

    let running = store
        .create(profile(), run(b"running"))
        .await
        .expect("the running stage");
    let running_id = running.record().stage_run_id.to_string();
    store
        .claim(running_id.clone(), "dead-worker".to_owned(), None)
        .await
        .expect("claim");
    store.expire_lease(&running_id);

    let succeeded = store
        .create(profile(), run(b"succeeded"))
        .await
        .expect("the succeeded stage");
    let succeeded_id = succeeded.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(record) = store
        .claim(succeeded_id.clone(), "worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the stage was not claimed");
    };
    store
        .complete(succeeded_id.clone(), record.lease_term, artifact(b"ok"))
        .await
        .expect("complete");

    let cancelled = store
        .create(profile(), run(b"cancelled"))
        .await
        .expect("the cancelled stage");
    let cancelled_id = cancelled.record().stage_run_id.to_string();
    store
        .cancel(profile(), cancelled_id.clone())
        .await
        .expect("cancel");

    let report = reconcile(&store, ReconcileMode::Startup)
        .await
        .expect("startup");
    assert_eq!(report.wanted, 2);
    assert_eq!(
        report.inserted, 0,
        "both intents committed with their stage rows"
    );
    let drained = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(drained.published, 2);
    let mut published = queue.stage_ids();
    published.sort();
    let mut expected = vec![queued_id.clone(), running_id.clone()];
    expected.sort();
    assert_eq!(
        published, expected,
        "the rebuild did not publish the exact unfinished set"
    );
    assert!(
        !published.contains(&succeeded_id) && !published.contains(&cancelled_id),
        "a terminal stage was re-armed"
    );
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);

    // A second startup pass re-arms the same set; it never creates a second
    // row or a second artifact set.
    let report = reconcile(&store, ReconcileMode::Startup)
        .await
        .expect("second startup");
    assert_eq!(report.rearmed, 2);
    drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 4);
    assert_eq!(
        store
            .get_any(succeeded_id)
            .await
            .expect("succeeded")
            .expect("row")
            .artifact,
        Some(artifact(b"ok"))
    );
    assert_eq!(
        store
            .get_any(cancelled_id)
            .await
            .expect("cancelled")
            .expect("row")
            .state,
        StageState::Cancelled
    );
}

#[tokio::test]
async fn a_live_lease_is_never_rearmed_and_a_terminal_row_is_never_wanted() {
    let store = MemoryStages::default();
    let queue = RecordingQueue::default();
    let submitted = store.create(profile(), run(b"live")).await.expect("create");
    let id = submitted.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(record) = store
        .claim(id.clone(), "live-worker".to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the queued stage was not claimed");
    };
    reconcile(&store, ReconcileMode::Startup)
        .await
        .expect("startup");
    drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 1);

    // The first startup publish is confirmed; a periodic pass while the lease
    // is live must not re-arm it and a duplicate claim must not steal it.
    let report = reconcile(&store, ReconcileMode::Periodic)
        .await
        .expect("periodic");
    assert_eq!(report.rearmed, 0);
    assert!(matches!(
        store
            .claim(id.clone(), "other-worker".to_owned(), None)
            .await
            .expect("live claim"),
        StageClaimOutcome::Running(_)
    ));
    drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 1);

    store
        .complete(id.clone(), record.lease_term, artifact(b"ok"))
        .await
        .expect("complete");
    let report = reconcile(&store, ReconcileMode::Startup)
        .await
        .expect("startup after terminal");
    assert_eq!(report.wanted, 0);
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
}

#[tokio::test]
async fn an_expired_running_stage_is_reclaimable_and_a_stale_term_is_refused() {
    let store = MemoryStages::with_lease_seconds(0);
    let submitted = store
        .create(profile(), run(b"expired"))
        .await
        .expect("create");
    let id = submitted.record().stage_run_id.to_string();
    let StageClaimOutcome::Claimed(first) = store
        .claim(id.clone(), "first-worker".to_owned(), None)
        .await
        .expect("first claim")
    else {
        panic!("the queued stage was not claimed");
    };
    assert_eq!(first.lease_term, 1);

    // The zero-window fake lease is already expired, so another worker may
    // reclaim it. The first worker's term is now stale.
    let StageClaimOutcome::Claimed(second) = store
        .claim(id.clone(), "second-worker".to_owned(), None)
        .await
        .expect("second claim")
    else {
        panic!("the expired stage was not reclaimed");
    };
    assert_eq!(second.lease_term, 2);
    assert_eq!(second.attempts, 2);

    let unchanged = store
        .complete(id.clone(), first.lease_term, artifact(b"stale"))
        .await
        .expect("stale complete");
    assert_eq!(unchanged.state, StageState::Running);
    assert_eq!(unchanged.artifact, None);
    let unchanged = store
        .fail(id.clone(), first.lease_term, "stale".to_owned())
        .await
        .expect("stale fail");
    assert_eq!(unchanged.state, StageState::Running);
    assert_eq!(unchanged.error, None);
    let StageHeartbeatOutcome::Stale = store
        .renew(id.clone(), "first-worker".to_owned(), first.lease_term)
        .await
        .expect("stale renew")
    else {
        panic!("a stale heartbeat was renewed");
    };

    let done = store
        .complete(id, second.lease_term, artifact(b"fresh"))
        .await
        .expect("fresh complete");
    assert_eq!(done.state, StageState::Succeeded);
    assert_eq!(done.artifact, Some(artifact(b"fresh")));
}

#[tokio::test]
async fn an_expired_spent_stage_is_exhausted_not_reclaimed() {
    let store = MemoryStages::with_lease_seconds(0);
    let submitted = store
        .create(profile(), run(b"spent"))
        .await
        .expect("create");
    let id = submitted.record().stage_run_id.to_string();
    for _ in 0..3 {
        let StageClaimOutcome::Claimed(_) = store
            .claim(id.clone(), "worker".to_owned(), None)
            .await
            .expect("claim")
        else {
            panic!("the expired stage was not claimed");
        };
    }
    let StageClaimOutcome::Exhausted(record) = store
        .claim(id, "worker".to_owned(), None)
        .await
        .expect("exhausted")
    else {
        panic!("the spent retry budget was reclaimed");
    };
    assert_eq!(record.attempts, 3);
    assert_eq!(record.state, StageState::Running);
}

#[tokio::test]
async fn a_failed_publish_keeps_the_intent_and_a_duplicate_delivery_is_one_stage() {
    let store = MemoryStages::default();
    let queue = RecordingQueue::default();
    let submitted = store
        .create(profile(), run(b"retry"))
        .await
        .expect("create");
    let id = submitted.record().stage_run_id.to_string();
    queue.set_failing(true);
    let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(report.published, 0);
    assert_eq!(report.failed, 1);
    assert_eq!(store.outstanding().await.expect("outstanding"), 1);
    queue.set_failing(false);
    let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(report.published, 1);
    let report = drain(&store, &queue, DRAIN_LIMIT)
        .await
        .expect("drain again");
    assert_eq!(report.published, 0);
    assert_eq!(queue.stage_ids(), vec![id.clone()]);
    assert_eq!(
        store
            .list(profile(), None, None, 10)
            .await
            .expect("list")
            .len(),
        1,
        "a duplicate delivery created a second stage"
    );
}

#[tokio::test]
async fn a_new_evidence_leader_rearms_the_unfinished_stages() {
    let store = MemoryStages::default();
    let queue = RecordingQueue::default();
    let submitted = store
        .create(profile(), run(b"leader"))
        .await
        .expect("create");
    let id = submitted.record().stage_run_id.to_string();
    reconcile(&store, ReconcileMode::Startup)
        .await
        .expect("startup");
    drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
    assert_eq!(queue.messages().len(), 1);

    let (sender, receiver) = tokio::sync::watch::channel(Leadership::Standby);
    let outbox: Arc<dyn StageOutbox> = Arc::new(store.clone());
    let queue_handle: Arc<dyn Queue> = Arc::new(queue.clone());
    let (_, shutdown) = quantrule_runtime::shutdown::Shutdown::channel();
    let task = spawn_leader_reconcile(receiver, outbox, queue_handle, shutdown);
    sender
        .send(Leadership::Leader(1))
        .expect("become the leader");
    for _ in 0..100 {
        if queue.messages().len() > 1 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(
        queue.stage_ids(),
        vec![id.clone(), id],
        "the new leader did not re-arm the unfinished stage"
    );
    task.abort();
}
