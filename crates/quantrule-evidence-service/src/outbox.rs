//! The evidence run-notification outbox.
//!
//! A run row and the message it needs commit in one transaction. The publisher
//! reads the outbox and delivers each message at least once; a confirmed publish
//! marks the row, and a crash before the mark only repeats a message the
//! consumer already treats as a lookup. The reconciler compares the live run
//! rows with the outbox, inserts a missing intent, and re-arms every unfinished
//! run once at startup so a rebuilt or emptied queue does not lose work.
//!
//! The outbox is not a saga. It carries no payload beyond the run id, no
//! cross-database step, and no decision: the run row is authoritative.

use std::sync::Arc;
use std::time::Duration;

use quantrule_api::RunId;
use quantrule_runtime::adapters::{BoxFuture, Queue};

use crate::leader::Leadership;
use crate::runs::{NotificationKind, RunNotification};

/// How many pending notifications one drain pass publishes.
pub const DRAIN_LIMIT: usize = 100;
/// How often the publisher reconciles and drains while the service runs.
pub const PUBLISH_INTERVAL: Duration = Duration::from_secs(5);

/// Why the outbox could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxError(pub String);

impl std::fmt::Display for OutboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for OutboxError {}

/// One pending notification the publisher may deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxNotification {
    /// The durable row identity, stable across attempts.
    pub event_id: i64,
    /// The run the message names.
    pub run_id: RunId,
    /// Which message the run needs.
    pub kind: NotificationKind,
    /// How many publishes were attempted.
    pub attempts: u32,
}

/// What one ensure pass wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnsureOutcome {
    /// Notifications inserted for a run that had none.
    pub inserted: u64,
    /// Published notifications re-armed for a retry.
    pub rearmed: u64,
}

/// Why a reconciliation pass ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileMode {
    /// The service is starting: re-arm every unfinished run, even if its last
    /// publish was confirmed. This is what rebuilds the queue from run rows.
    Startup,
    /// A periodic pass: insert an intent that is missing, but never repeat a
    /// confirmed publish.
    Periodic,
}

/// What one reconciliation pass found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    /// How many live runs want a notification.
    pub wanted: usize,
    /// How many intents were inserted.
    pub inserted: u64,
    /// How many confirmed publishes were re-armed.
    pub rearmed: u64,
}

/// What one drain pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DrainReport {
    /// How many notifications were published and marked.
    pub published: u32,
    /// How many failed; the last failure is in `error`.
    pub failed: u32,
    /// The first publish failure, when any.
    pub error: Option<String>,
}

/// Where run notifications live.
///
/// A store implements it over the same state as [`crate::runs::RunStore`], so a
/// submission and its notification are one commit. `wanted_notifications` reads
/// the live run rows; the remaining methods read and write the outbox.
pub trait RunOutbox: Send + Sync {
    /// The notification every live run still needs.
    fn wanted_notifications(&self) -> BoxFuture<Result<Vec<RunNotification>, OutboxError>>;

    /// Insert a missing intent, and re-arm a confirmed one according to the
    /// pass: startup re-arms every unfinished run; a periodic pass re-arms
    /// only an expired-lease request whose publish is older than the lease
    /// window.
    fn ensure(
        &self,
        wanted: &[RunNotification],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>>;

    /// The pending notifications, oldest first.
    fn pending(&self, limit: usize) -> BoxFuture<Result<Vec<OutboxNotification>, OutboxError>>;

    /// Mark one notification published.
    fn published(&self, event_id: i64) -> BoxFuture<Result<(), OutboxError>>;

    /// Record one failed publish; the row stays pending.
    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>>;

    /// How many notifications are not published.
    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>>;
}

/// Reconcile the live runs with the outbox.
///
/// # Errors
///
/// Returns [`OutboxError`] when the run or outbox store cannot be read.
pub async fn reconcile(
    outbox: &dyn RunOutbox,
    mode: ReconcileMode,
) -> Result<ReconcileReport, OutboxError> {
    let wanted = outbox.wanted_notifications().await?;
    let outcome = outbox.ensure(&wanted, mode).await?;
    Ok(ReconcileReport {
        wanted: wanted.len(),
        inserted: outcome.inserted,
        rearmed: outcome.rearmed,
    })
}

/// Publish every pending notification up to `limit`.
///
/// A failed publish stops the pass: the broker is the shared dependency, and
/// the row stays pending for the next tick. The run row is untouched either
/// way, so this function cannot lose or duplicate a run record.
///
/// # Errors
///
/// Returns [`OutboxError`] when the outbox cannot be read or updated. A publish
/// failure is a [`DrainReport::error`], not an error, because it is expected
/// while the broker is down.
pub async fn drain(
    outbox: &dyn RunOutbox,
    queue: &dyn Queue,
    limit: usize,
) -> Result<DrainReport, OutboxError> {
    let pending = outbox.pending(limit).await?;
    let mut report = DrainReport::default();
    for notification in pending {
        match queue
            .publish(
                notification.kind.routing_key(),
                payload(&notification.run_id),
            )
            .await
        {
            Ok(()) => {
                outbox.published(notification.event_id).await?;
                report.published = report.published.saturating_add(1);
            }
            Err(error) => {
                let message = error.to_string();
                outbox
                    .failed(notification.event_id, message.clone())
                    .await?;
                report.failed = report.failed.saturating_add(1);
                report.error = Some(message);
                break;
            }
        }
    }
    Ok(report)
}

/// The wire payload of every run message: the run id and nothing else.
#[must_use]
pub fn payload(run_id: &RunId) -> Vec<u8> {
    serde_json::json!({ "run_id": run_id.to_string() })
        .to_string()
        .into_bytes()
}

/// Run one reconcile-then-drain pass now, and repeat it on the interval.
///
/// The first pass is a startup pass, so a process that starts against an empty
/// or rebuilt queue republishes every unfinished run. Later passes insert a
/// missing intent and drain; they never repeat a confirmed publish.
pub fn spawn_publisher(
    outbox: Arc<dyn RunOutbox>,
    queue: Arc<dyn Queue>,
    interval: Duration,
    mut shutdown: quantrule_runtime::shutdown::Shutdown,
    metrics: Option<quantrule_runtime::metrics::Metrics>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut startup = true;
        loop {
            if shutdown.is_requested() {
                return;
            }
            let mode = if startup {
                ReconcileMode::Startup
            } else {
                ReconcileMode::Periodic
            };
            startup = false;
            match reconcile(outbox.as_ref(), mode).await {
                Ok(report) if report.inserted > 0 || report.rearmed > 0 => {
                    tracing::info!(
                        wanted = report.wanted,
                        inserted = report.inserted,
                        rearmed = report.rearmed,
                        "run notifications reconciled"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "the run-notification reconciliation will retry");
                }
            }
            match drain(outbox.as_ref(), queue.as_ref(), DRAIN_LIMIT).await {
                Ok(report) if report.published > 0 => {
                    tracing::info!(published = report.published, "run notifications published");
                }
                Ok(report) => {
                    if let Some(error) = report.error {
                        tracing::warn!(%error, "the run notification will retry");
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "the run-notification drain will retry");
                }
            }
            if let Some(metrics) = &metrics {
                match outbox.outstanding().await {
                    Ok(pending) => metrics.set_outbox_pending(pending),
                    Err(error) => {
                        tracing::warn!(%error, "the outbox depth could not be read");
                    }
                }
            }
            tokio::select! {
                biased;
                () = shutdown.wait() => return,
                () = tokio::time::sleep(interval) => {}
            }
        }
    })
}

/// Re-arm every unfinished run whenever this replica becomes the leader.
///
/// A lease takeover is the same recovery point as a process restart: the old
/// leader may have consumed a notification that was then lost or dead-lettered.
/// The startup pass re-arms every live run, and the queue stays the delivery
/// path. A standby performs no work here.
pub fn spawn_leader_reconcile(
    mut leadership: tokio::sync::watch::Receiver<Leadership>,
    outbox: Arc<dyn RunOutbox>,
    queue: Arc<dyn Queue>,
    shutdown: quantrule_runtime::shutdown::Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if shutdown.is_requested() {
                return;
            }
            let is_leader = matches!(*leadership.borrow_and_update(), Leadership::Leader(_));
            if is_leader {
                match reconcile(outbox.as_ref(), ReconcileMode::Startup).await {
                    Ok(report) if report.inserted > 0 || report.rearmed > 0 => {
                        tracing::info!(
                            wanted = report.wanted,
                            inserted = report.inserted,
                            rearmed = report.rearmed,
                            "the new evidence leader re-armed run notifications"
                        );
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(%error, "the leader re-arm will retry on the next election");
                    }
                }
                match drain(outbox.as_ref(), queue.as_ref(), DRAIN_LIMIT).await {
                    Ok(report) => {
                        if let Some(error) = report.error {
                            tracing::warn!(%error, "the leader re-arm drain will retry");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "the leader re-arm drain will retry");
                    }
                }
            }
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => return,
                changed = leadership.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::Digest;
    use quantrule_runtime::adapters::{Backend, QueueError};

    use crate::runs::{ClaimOutcome, MemoryRuns, NewRun, RunStore};

    type Published = Arc<std::sync::Mutex<Vec<(String, Vec<u8>)>>>;

    #[derive(Clone, Default)]
    struct RecordingQueue {
        published: Published,
        failing: Arc<std::sync::atomic::AtomicBool>,
    }

    impl RecordingQueue {
        fn messages(&self) -> Vec<(String, Vec<u8>)> {
            self.published.lock().expect("the queue lock").clone()
        }

        fn set_failing(&self, failing: bool) {
            self.failing
                .store(failing, std::sync::atomic::Ordering::SeqCst);
        }
    }

    impl Queue for RecordingQueue {
        fn backend(&self) -> Backend {
            Backend::InProcess
        }

        fn publish(
            &self,
            routing_key: &str,
            payload: Vec<u8>,
        ) -> BoxFuture<Result<(), QueueError>> {
            if self.failing.load(std::sync::atomic::Ordering::SeqCst) {
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

    fn profile(name: &str) -> quantrule_api::ProfileId {
        quantrule_api::ProfileId::parse(name).expect("profile")
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
            submitted_at: "2026-11-22T00:00:00Z".to_owned(),
        }
    }

    #[tokio::test]
    async fn a_committed_run_is_published_once_and_the_second_pass_is_quiet() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        let submitted = store
            .submit(profile("default"), run(b"outbox-drain"))
            .await
            .expect("submit");
        let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(report.published, 1);
        assert_eq!(report.failed, 0);
        let messages = queue.messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, "run.requested");
        let body: serde_json::Value = serde_json::from_slice(&messages[0].1).expect("payload");
        assert_eq!(
            body["run_id"].as_str(),
            Some(submitted.record().run_id.to_string().as_str())
        );
        assert_eq!(body.as_object().map(serde_json::Map::len), Some(1));

        let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(report.published, 0);
        assert_eq!(queue.messages().len(), 1);
    }

    #[tokio::test]
    async fn a_failed_publish_keeps_the_notification_and_the_next_pass_sends_it() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        store
            .submit(profile("default"), run(b"outbox-retry"))
            .await
            .expect("submit");
        queue.set_failing(true);
        let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(report.published, 0);
        assert_eq!(report.failed, 1);
        assert!(report.error.is_some());
        assert_eq!(store.outstanding().await.expect("outstanding"), 1);

        queue.set_failing(false);
        let report = drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(report.published, 1);
        assert_eq!(store.outstanding().await.expect("outstanding"), 0);
        assert_eq!(queue.messages().len(), 1);
    }

    #[tokio::test]
    async fn a_startup_pass_rearms_an_unfinished_run_and_a_periodic_pass_does_not() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        store
            .submit(profile("default"), run(b"outbox-restart"))
            .await
            .expect("submit");
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 1);

        let report = reconcile(&store, ReconcileMode::Periodic)
            .await
            .expect("periodic");
        assert_eq!(report.rearmed, 0);
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 1);

        let report = reconcile(&store, ReconcileMode::Startup)
            .await
            .expect("startup");
        assert_eq!(report.rearmed, 1);
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 2, "a restarted process republishes");
    }

    #[tokio::test]
    async fn a_running_run_without_a_digest_is_rearmed_once_on_restart() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        let submitted = store
            .submit(profile("default"), run(b"outbox-running"))
            .await
            .expect("submit");
        let id = submitted.record().run_id.to_string();
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 1);

        let ClaimOutcome::Claimed(claimed) = store
            .claim(id.clone(), "test-worker".to_owned(), None)
            .await
            .expect("claim")
        else {
            panic!("the run was not claimed");
        };
        store.expire_lease(&id);
        let report = reconcile(&store, ReconcileMode::Startup)
            .await
            .expect("startup");
        assert_eq!(report.wanted, 1);
        assert_eq!(report.rearmed, 1);
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(
            queue.messages().len(),
            2,
            "a restart did not republish the unfinished run"
        );

        // A periodic pass never repeats a confirmed publish.
        let report = reconcile(&store, ReconcileMode::Periodic)
            .await
            .expect("periodic");
        assert_eq!(report.inserted, 0);
        assert_eq!(report.rearmed, 0);
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 2);

        // A terminal transition removes the intent and the run stops being
        // wanted; a repeat cannot create a second run.
        store
            .fail(
                id.clone(),
                claimed.lease_term,
                "the recovered run failed".to_owned(),
                Vec::new(),
            )
            .await
            .expect("fail");
        assert_eq!(store.outstanding().await.expect("outstanding"), 0);
        let report = reconcile(&store, ReconcileMode::Periodic)
            .await
            .expect("periodic");
        assert_eq!(report.wanted, 0);
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(queue.messages().len(), 2);
        assert_eq!(
            store
                .list(profile("default"), None, 10)
                .await
                .expect("list")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn a_new_leader_rearms_a_published_computed_notification() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        let submitted = store
            .submit(profile("default"), run(b"leader-rearm"))
            .await
            .expect("submit");
        let id = submitted.record().run_id.to_string();
        let ClaimOutcome::Claimed(claimed) = store
            .claim(id.clone(), "test-worker".to_owned(), None)
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
                Vec::new(),
            )
            .await
            .expect("computed");
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        let published = queue.messages().len();
        assert_eq!(published, 2, "requested and computed intents published");

        let (sender, receiver) = tokio::sync::watch::channel(Leadership::Standby);
        let outbox: Arc<dyn RunOutbox> = Arc::new(store.clone());
        let queue_handle: Arc<dyn Queue> = Arc::new(queue.clone());
        let (_, shutdown) = quantrule_runtime::shutdown::Shutdown::channel();
        let task = spawn_leader_reconcile(receiver, outbox, queue_handle, shutdown);
        sender
            .send(Leadership::Leader(1))
            .expect("become the leader");
        for _ in 0..100 {
            if queue.messages().len() > published {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            queue.messages().len(),
            published + 1,
            "a new leader must re-arm the published computed notification"
        );
        assert_eq!(store.outstanding().await.expect("outstanding"), 0);
        task.abort();
    }

    #[tokio::test]
    async fn a_draining_publisher_stops_before_it_publishes() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        store
            .submit(profile("default"), run(b"outbox-drain-stop"))
            .await
            .expect("submit");
        let (trigger, shutdown) = quantrule_runtime::shutdown::Shutdown::channel();
        trigger.trigger();
        let task = spawn_publisher(
            Arc::new(store) as Arc<dyn RunOutbox>,
            Arc::new(queue.clone()) as Arc<dyn Queue>,
            Duration::from_millis(1),
            shutdown,
            None,
        );
        task.await.expect("publisher task");
        assert!(
            queue.messages().is_empty(),
            "a draining publisher published"
        );
    }

    #[tokio::test]
    async fn a_repeated_submission_republishes_but_does_not_duplicate_the_run() {
        let store = MemoryRuns::default();
        let queue = RecordingQueue::default();
        store
            .submit(profile("default"), run(b"outbox-duplicate"))
            .await
            .expect("submit");
        reconcile(&store, ReconcileMode::Startup)
            .await
            .expect("startup");
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        store
            .submit(profile("default"), run(b"outbox-duplicate"))
            .await
            .expect("resubmit");
        drain(&store, &queue, DRAIN_LIMIT).await.expect("drain");
        assert_eq!(
            queue.messages().len(),
            2,
            "a resubmission republishes the queued run"
        );
        assert_eq!(
            store
                .list(profile("default"), None, 10)
                .await
                .expect("list")
                .len(),
            1
        );
    }
}
