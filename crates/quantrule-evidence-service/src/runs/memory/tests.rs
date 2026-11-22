use super::*;
use crate::leader::LeaderLease;
use quantrule_api::{RunId, StageName, StageStatus};

const HOLDER: &str = "test-worker";

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

async fn claimed(store: &MemoryRuns, id: &str) -> RunRecord {
    let outcome = store
        .claim(id.to_owned(), HOLDER.to_owned(), None)
        .await
        .expect("claim");
    let ClaimOutcome::Claimed(record) = outcome else {
        panic!("the run was not claimed");
    };
    record
}

async fn leader(store: &MemoryRuns) -> u64 {
    store
        .leader_lease()
        .acquire("test-evidence".to_owned())
        .await
        .expect("leader lease")
        .expect("this process is the leader")
}

fn run(seed: &[u8]) -> NewRun {
    NewRun {
        run_id: RunId(Digest::sha256(seed)),
        config_id: "cfg_abc".to_owned(),
        data_version: Digest::sha256(b"data"),
        config_hash: Digest::sha256(b"config"),
        model_version: "M1.0".to_owned(),
        image_digest: Digest::sha256(b"image"),
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-13T00:00:00Z".to_owned(),
    }
}

#[tokio::test]
async fn a_submission_is_created_once() {
    let store = MemoryRuns::default();
    let first = store
        .submit(profile("default"), run(b"one"))
        .await
        .expect("first");
    assert!(first.created());
    assert_eq!(first.record().status, RunState::Queued);
    let second = store
        .submit(profile("default"), run(b"one"))
        .await
        .expect("second");
    assert!(!second.created());
    assert_eq!(first.record().run_id, second.record().run_id);
    assert_eq!(
        store
            .list(profile("default"), None, 50)
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
async fn two_profiles_share_one_run() {
    let store = MemoryRuns::default();
    store
        .submit(profile("alpha"), run(b"shared"))
        .await
        .expect("alpha");
    store
        .submit(profile("beta"), run(b"shared"))
        .await
        .expect("beta");
    assert_eq!(
        store
            .list(profile("alpha"), None, 50)
            .await
            .expect("alpha list")
            .len(),
        1
    );
    assert_eq!(
        store
            .list(profile("beta"), None, 50)
            .await
            .expect("beta list")
            .len(),
        1
    );
    assert_eq!(
        store
            .list(profile("gamma"), None, 50)
            .await
            .expect("gamma list")
            .len(),
        0
    );
}

#[tokio::test]
async fn a_cancelled_run_is_not_cancellable_again() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"one"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    assert!(matches!(
        store
            .cancel(profile("default"), id.clone())
            .await
            .expect("cancel"),
        CancelOutcome::Cancelled(_)
    ));
    assert!(matches!(
        store.cancel(profile("default"), id).await.expect("cancel"),
        CancelOutcome::Conflict(_)
    ));
}

#[tokio::test]
async fn the_status_filter_and_limit_apply() {
    let store = MemoryRuns::default();
    store
        .submit(profile("default"), run(b"one"))
        .await
        .expect("one");
    store
        .submit(profile("default"), run(b"two"))
        .await
        .expect("two");
    assert_eq!(
        store
            .list(profile("default"), Some(RunState::Queued), 1)
            .await
            .expect("list")
            .len(),
        1
    );
    assert_eq!(
        store
            .list(profile("default"), Some(RunState::Succeeded), 50)
            .await
            .expect("list")
            .len(),
        0
    );
}

fn new_run() -> NewRun {
    NewRun {
        run_id: RunId(Digest::sha256(b"worker")),
        config_id: "cfg_abc".to_owned(),
        data_version: Digest::sha256(b"data"),
        config_hash: Digest::sha256(b"config"),
        model_version: "M1.0".to_owned(),
        image_digest: Digest::sha256(b"image"),
        model_artifact_id: None,
        owner: "default".to_owned(),
        submitted_at: "2026-11-13T00:00:00Z".to_owned(),
    }
}

fn done_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: StageStatus::Done,
        elapsed_ms: None,
    })
    .collect()
}

#[tokio::test]
async fn a_claim_is_atomic_and_one_shot() {
    let store = MemoryRuns::default();
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();
    assert!(matches!(
        store
            .claim(id.clone(), HOLDER.to_owned(), None)
            .await
            .expect("claim"),
        ClaimOutcome::Claimed(_)
    ));
    assert!(matches!(
        store
            .claim(id.clone(), HOLDER.to_owned(), None)
            .await
            .expect("claim"),
        ClaimOutcome::Running(_)
    ));
    assert_eq!(
        store
            .get_any(id)
            .await
            .expect("get")
            .expect("record")
            .status,
        RunState::Running
    );
}

#[tokio::test]
async fn a_redelivery_reclaims_until_the_budget_is_spent() {
    let store = MemoryRuns::default();
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();

    let ClaimOutcome::Claimed(first) = store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("first")
    else {
        panic!("the queued run was not claimed");
    };
    assert_eq!(first.attempts, 1);
    assert!(matches!(
        store
            .claim(id.clone(), HOLDER.to_owned(), None)
            .await
            .expect("live"),
        ClaimOutcome::Running(_)
    ));
    let ClaimOutcome::Claimed(second) = store
        .claim(id.clone(), HOLDER.to_owned(), Some(first.lease_term))
        .await
        .expect("second")
    else {
        panic!("the redelivery was not reclaimed");
    };
    assert_eq!(second.attempts, 2);
    let ClaimOutcome::Claimed(third) = store
        .claim(id.clone(), HOLDER.to_owned(), Some(second.lease_term))
        .await
        .expect("third")
    else {
        panic!("the second redelivery was not reclaimed");
    };
    assert_eq!(third.attempts, 3);
    let ClaimOutcome::Exhausted(spent) = store
        .claim(id, HOLDER.to_owned(), Some(third.lease_term))
        .await
        .expect("fourth")
    else {
        panic!("the fourth delivery was not exhausted");
    };
    assert_eq!(spent.attempts, 3);
}

#[tokio::test]
async fn a_computed_run_is_not_reclaimed() {
    let store = MemoryRuns::default();
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();
    let ClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the run was not claimed");
    };
    store
        .computed(
            id.clone(),
            claimed.lease_term,
            Digest::sha256(b"canonical"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("computed");
    assert!(matches!(
        store
            .claim(id, HOLDER.to_owned(), None)
            .await
            .expect("redelivery"),
        ClaimOutcome::Running(_)
    ));
}

#[tokio::test]
async fn only_a_running_run_can_be_completed_or_failed() {
    let store = MemoryRuns::default();
    let leader = leader(&store).await;
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();

    let record = store
        .complete(
            id.clone(),
            0,
            leader,
            Digest::sha256(b"too-early"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("complete");
    assert_eq!(record.status, RunState::Queued);
    assert_eq!(record.digest, None);

    let record = store
        .fail(id.clone(), 0, "too early".to_owned(), done_stages())
        .await
        .expect("fail");
    assert_eq!(record.status, RunState::Queued);
    assert_eq!(record.error, None);

    let ClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the run was not claimed");
    };
    store
        .complete(
            id.clone(),
            claimed.lease_term,
            leader,
            Digest::sha256(b"canonical"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("complete");
    let record = store
        .fail(
            id,
            claimed.lease_term,
            "after success".to_owned(),
            done_stages(),
        )
        .await
        .expect("fail");
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(record.error, None);
}

#[tokio::test]
async fn a_completed_run_carries_its_artifacts_and_digest() {
    let store = MemoryRuns::default();
    let leader = leader(&store).await;
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();
    let ClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the run was not claimed");
    };
    let digest = Digest::sha256(b"canonical");
    let artifact = ArtifactRef {
        name: "results.json".to_owned(),
        bytes: 32,
        sha256: Digest::sha256(b"results"),
    };
    let record = store
        .complete(
            id.clone(),
            claimed.lease_term,
            leader,
            digest.clone(),
            vec![artifact.clone()],
            done_stages(),
        )
        .await
        .expect("complete");
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(record.digest, Some(digest));
    assert_eq!(record.detail().artifacts, vec![artifact]);
    assert!(
        record
            .stages
            .iter()
            .all(|stage| stage.state == StageStatus::Done)
    );
    assert!(matches!(
        store
            .claim(id, HOLDER.to_owned(), None)
            .await
            .expect("claim"),
        ClaimOutcome::Terminal(_)
    ));
}

#[tokio::test]
async fn a_failed_run_records_the_error() {
    let store = MemoryRuns::default();
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();
    let ClaimOutcome::Claimed(claimed) = store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim")
    else {
        panic!("the run was not claimed");
    };
    let record = store
        .fail(
            id,
            claimed.lease_term,
            "model failed".to_owned(),
            done_stages(),
        )
        .await
        .expect("fail");
    assert_eq!(record.status, RunState::Failed);
    assert_eq!(record.error.as_deref(), Some("model failed"));
}

#[tokio::test]
async fn a_cancelled_run_is_terminal_for_the_worker() {
    let store = MemoryRuns::default();
    let run = store
        .submit(profile("default"), new_run())
        .await
        .expect("submit");
    let id = run.record().run_id.to_string();
    store
        .cancel(profile("default"), id.clone())
        .await
        .expect("cancel");
    assert!(matches!(
        store
            .claim(id, HOLDER.to_owned(), None)
            .await
            .expect("claim"),
        ClaimOutcome::Terminal(_)
    ));
}

#[tokio::test]
async fn a_committed_submission_records_one_notification() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"outbox-one"))
        .await
        .expect("submit");
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].run_id, submitted.record().run_id);
    assert_eq!(wanted[0].kind, NotificationKind::Requested);
    assert_eq!(store.outstanding().await.expect("outstanding"), 1);
}

#[tokio::test]
async fn a_published_notification_is_not_pending_again() {
    let store = MemoryRuns::default();
    store
        .submit(profile("default"), run(b"outbox-publish"))
        .await
        .expect("submit");
    let pending = store.pending(10).await.expect("pending");
    assert_eq!(pending.len(), 1);
    store
        .published(pending[0].event_id)
        .await
        .expect("published");
    assert!(store.pending(10).await.expect("pending").is_empty());
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
}

#[tokio::test]
async fn a_restart_rearms_an_unfinished_run() {
    let store = MemoryRuns::default();
    store
        .submit(profile("default"), run(b"outbox-restart"))
        .await
        .expect("submit");
    let pending = store.pending(10).await.expect("pending");
    store
        .published(pending[0].event_id)
        .await
        .expect("published");
    let wanted = store.wanted_notifications().await.expect("wanted");
    let outcome = store
        .ensure(&wanted, ReconcileMode::Startup)
        .await
        .expect("ensure");
    assert_eq!(outcome.inserted, 0);
    assert_eq!(outcome.rearmed, 1);
    assert_eq!(store.pending(10).await.expect("pending").len(), 1);

    // A periodic pass does not rearm an already published row.
    let outcome = store
        .ensure(&wanted, ReconcileMode::Periodic)
        .await
        .expect("ensure");
    assert_eq!(outcome.inserted, 0);
    assert_eq!(outcome.rearmed, 0);
}

#[tokio::test]
async fn a_computed_run_is_notified_and_a_finished_run_is_silent() {
    let store = MemoryRuns::default();
    let leader = leader(&store).await;
    let submitted = store
        .submit(profile("default"), run(b"outbox-computed"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let claimed = claimed(&store, &id).await;
    store
        .computed(
            id.clone(),
            claimed.lease_term,
            Digest::sha256(b"canonical"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("computed");
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].kind, NotificationKind::Computed);

    store
        .complete(
            id.clone(),
            claimed.lease_term,
            leader,
            Digest::sha256(b"canonical"),
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
            .is_empty(),
        "a succeeded run still wants a notification"
    );
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
}

#[tokio::test]
async fn a_failed_run_clears_its_notification() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"outbox-failed"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let claimed = claimed(&store, &id).await;
    store
        .fail(
            id,
            claimed.lease_term,
            "the run failed".to_owned(),
            Vec::new(),
        )
        .await
        .expect("fail");
    assert_eq!(store.outstanding().await.expect("outstanding"), 0);
}

#[tokio::test]
async fn an_erase_records_a_release_tombstone() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"outbox-release"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let claimed = claimed(&store, &id).await;
    store
        .fail(
            id.clone(),
            claimed.lease_term,
            "the run failed".to_owned(),
            Vec::new(),
        )
        .await
        .expect("fail");
    store
        .archive(
            profile("default"),
            id.clone(),
            "2026-11-22T00:00:00Z".to_owned(),
        )
        .await
        .expect("archive");
    store
        .erase(profile("default"), id.clone())
        .await
        .expect("erase");
    let pending = store.pending_releases().await.expect("releases");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].profile, profile("default"));
    assert_eq!(pending[0].reference_id, format!("run:{id}"));
    assert_eq!(pending[0].kind, DatasetReferenceKind::Run);
    assert_eq!(pending[0].data_version, Digest::sha256(b"data"));

    store
        .recorded(&pending[0], None)
        .await
        .expect("release recorded");
    assert!(store.pending_releases().await.expect("releases").is_empty());
}

#[tokio::test]
async fn recording_the_same_release_twice_keeps_one_tombstone() {
    let store = MemoryRuns::default();
    let release = DatasetReferenceRelease {
        profile: profile("default"),
        data_version: Digest::sha256(b"data"),
        reference_id: "run:twice".to_owned(),
        kind: DatasetReferenceKind::Run,
    };
    store.record(release.clone()).await.expect("first");
    store.record(release).await.expect("second");
    assert_eq!(store.pending_releases().await.expect("releases").len(), 1);
}

#[tokio::test]
async fn a_run_written_before_the_outbox_gets_its_intent_on_reconcile() {
    let store = MemoryRuns::default();
    store
        .submit(profile("default"), run(b"outbox-legacy"))
        .await
        .expect("submit");
    // A run row written before migration 0018 has no outbox row.
    store
        .inner
        .lock()
        .expect("the run lock")
        .notifications
        .clear();
    assert!(store.pending(10).await.expect("pending").is_empty());

    let report = crate::outbox::reconcile(&store, crate::outbox::ReconcileMode::Periodic)
        .await
        .expect("reconcile");
    assert_eq!(report.wanted, 1);
    assert_eq!(report.inserted, 1);
    assert_eq!(store.pending(10).await.expect("pending").len(), 1);
}

#[tokio::test]
async fn a_live_lease_is_not_wanted_and_an_expired_one_is() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"outbox-running-wanted"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim");
    let record = store
        .get_any(id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.digest, None);
    assert!(
        store
            .wanted_notifications()
            .await
            .expect("wanted")
            .is_empty(),
        "a live lease was wanted"
    );

    store.expire_lease(&id);
    let wanted = store.wanted_notifications().await.expect("wanted");
    assert_eq!(wanted.len(), 1);
    assert_eq!(wanted[0].kind, NotificationKind::Requested);
}

#[tokio::test]
async fn a_running_run_without_a_digest_gets_a_missing_intent_on_the_periodic_pass() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"outbox-running-legacy"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    store
        .claim(id.clone(), HOLDER.to_owned(), None)
        .await
        .expect("claim");
    store.expire_lease(&id);
    // A run row written before migration 0018 has no outbox row.
    store
        .inner
        .lock()
        .expect("the run lock")
        .notifications
        .clear();
    assert!(store.pending(10).await.expect("pending").is_empty());

    let report = crate::outbox::reconcile(&store, crate::outbox::ReconcileMode::Periodic)
        .await
        .expect("reconcile");
    assert_eq!(report.wanted, 1);
    assert_eq!(report.inserted, 1);
    let pending = store.pending(10).await.expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, NotificationKind::Requested);
}

#[tokio::test]
async fn a_live_lease_is_not_stolen_and_the_holder_can_continue() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"lease-live"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let first = claimed(&store, &id).await;
    assert_eq!(first.lease_term, 1);
    assert_eq!(first.attempts, 1);

    let stolen = store
        .claim(id.clone(), "other-worker".to_owned(), None)
        .await
        .expect("other claim");
    assert!(matches!(stolen, ClaimOutcome::Running(_)));
    assert_eq!(
        store
            .get_any(id.clone())
            .await
            .expect("get")
            .expect("record")
            .attempts,
        1,
        "a live lease was stolen"
    );

    let ClaimOutcome::Claimed(continued) = store
        .claim(id.clone(), HOLDER.to_owned(), Some(first.lease_term))
        .await
        .expect("continuation")
    else {
        panic!("the same holder could not continue its retry");
    };
    assert_eq!(continued.lease_term, 2);
    assert_eq!(continued.attempts, 2);
}

#[tokio::test]
async fn an_expired_lease_is_reclaimed() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"lease-expired"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let first = claimed(&store, &id).await;
    store.expire_lease(&id);

    let ClaimOutcome::Claimed(second) = store
        .claim(id.clone(), "other-worker".to_owned(), None)
        .await
        .expect("reclaim")
    else {
        panic!("the expired lease was not reclaimed");
    };
    assert_eq!(second.lease_term, first.lease_term + 1);
    assert_eq!(second.attempts, 2);
    assert_eq!(second.lease_holder.as_deref(), Some("other-worker"));
}

#[tokio::test]
async fn a_stale_completion_or_failure_is_rejected() {
    let store = MemoryRuns::default();
    let leader = leader(&store).await;
    let submitted = store
        .submit(profile("default"), run(b"lease-stale"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let first = claimed(&store, &id).await;
    store.expire_lease(&id);
    let ClaimOutcome::Claimed(second) = store
        .claim(id.clone(), "other-worker".to_owned(), None)
        .await
        .expect("reclaim")
    else {
        panic!("the expired lease was not reclaimed");
    };

    let stale = store
        .complete(
            id.clone(),
            first.lease_term,
            leader,
            Digest::sha256(b"stale"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("stale complete");
    assert_eq!(stale.status, RunState::Running);
    assert_eq!(stale.digest, None);
    let stale = store
        .fail(
            id.clone(),
            first.lease_term,
            "stale failure".to_owned(),
            done_stages(),
        )
        .await
        .expect("stale fail");
    assert_eq!(stale.status, RunState::Running);
    assert_eq!(stale.error, None);

    let current = store
        .complete(
            id.clone(),
            second.lease_term,
            leader,
            Digest::sha256(b"current"),
            Vec::new(),
            done_stages(),
        )
        .await
        .expect("current complete");
    assert_eq!(current.status, RunState::Succeeded);
    assert_eq!(current.lease_holder, None);
    assert_eq!(current.lease_expires_at, None);
    assert!(matches!(
        store
            .claim(id, "third-worker".to_owned(), None)
            .await
            .expect("terminal claim"),
        ClaimOutcome::Terminal(_)
    ));
}

#[tokio::test]
async fn a_heartbeat_renews_only_the_matching_live_term() {
    let store = MemoryRuns::default();
    let submitted = store
        .submit(profile("default"), run(b"lease-heartbeat"))
        .await
        .expect("submit");
    let id = submitted.record().run_id.to_string();
    let claimed = claimed(&store, &id).await;

    let renewed = store
        .heartbeat(id.clone(), HOLDER.to_owned(), claimed.lease_term)
        .await
        .expect("heartbeat");
    assert!(matches!(renewed, HeartbeatOutcome::Renewed(_)));
    assert!(matches!(
        store
            .heartbeat(id.clone(), "other-worker".to_owned(), claimed.lease_term)
            .await
            .expect("other heartbeat"),
        HeartbeatOutcome::Stale
    ));
    assert!(matches!(
        store
            .heartbeat(id.clone(), HOLDER.to_owned(), claimed.lease_term + 1)
            .await
            .expect("future term"),
        HeartbeatOutcome::Stale
    ));

    store.expire_lease(&id);
    assert!(matches!(
        store
            .heartbeat(id, HOLDER.to_owned(), claimed.lease_term)
            .await
            .expect("expired heartbeat"),
        HeartbeatOutcome::Stale
    ));
}
