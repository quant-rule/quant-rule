//! The diagnostic-stage request outbox.
//!
//! A stage row and its `stage.requested` intent commit in one transaction. The
//! publisher delivers each intent at least once; a confirmed publish marks the
//! row, and a crash before the mark only repeats a message the worker treats as
//! a lookup. The reconciler compares the unfinished stage rows with the outbox
//! and re-arms them at startup and on evidence-leader takeover, so a rebuilt or
//! emptied queue does not lose a stage.
//!
//! The outbox carries no payload beyond the stage run id. Terminal rows are
//! never wanted, so a re-arm cannot publish a stage that already succeeded,
//! failed, or was cancelled. The stage row is authoritative.

use std::sync::Arc;
use std::time::Duration;

use quantrule_api::StageRunId;
use quantrule_runtime::adapters::{BoxFuture, Queue};
use quantrule_runtime::queue::ROUTING_STAGE_REQUESTED;

use crate::leader::Leadership;
use crate::outbox::{DrainReport, EnsureOutcome, OutboxError, ReconcileMode, ReconcileReport};

/// How many pending stage notifications one drain pass publishes.
pub const DRAIN_LIMIT: usize = 100;
/// How often the stage publisher reconciles and drains while the service runs.
pub const PUBLISH_INTERVAL: Duration = Duration::from_secs(5);

/// One pending stage request the publisher may deliver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageOutboxNotification {
    /// The durable row identity, stable across attempts.
    pub event_id: i64,
    /// The stage the message names.
    pub stage_run_id: StageRunId,
    /// How many publishes were attempted.
    pub attempts: u32,
}

/// Where stage request notifications live.
///
/// The store implements it over the same state as [`super::StageStore`], so a
/// submission and its notification are one commit. `wanted` reads the
/// unfinished stage rows; the remaining methods read and write the outbox.
pub trait StageOutbox: Send + Sync {
    /// The request each unfinished stage still needs.
    fn wanted(&self) -> BoxFuture<Result<Vec<StageRunId>, OutboxError>>;

    /// Insert a missing intent, and re-arm a confirmed one according to the
    /// pass: startup re-arms every unfinished stage; a periodic pass re-arms
    /// only an expired-lease request whose publish is older than the lease
    /// window.
    fn ensure(
        &self,
        wanted: &[StageRunId],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>>;

    /// The pending notifications, oldest first.
    fn pending(&self, limit: usize)
    -> BoxFuture<Result<Vec<StageOutboxNotification>, OutboxError>>;

    /// Mark one notification published.
    fn published(&self, event_id: i64) -> BoxFuture<Result<(), OutboxError>>;

    /// Record one failed publish; the row stays pending.
    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>>;

    /// How many notifications are not published.
    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>>;
}

/// Reconcile the unfinished stage rows with the outbox.
///
/// # Errors
///
/// Returns [`OutboxError`] when the stage or outbox store cannot be read.
pub async fn reconcile(
    outbox: &dyn StageOutbox,
    mode: ReconcileMode,
) -> Result<ReconcileReport, OutboxError> {
    let wanted = outbox.wanted().await?;
    let outcome = outbox.ensure(&wanted, mode).await?;
    Ok(ReconcileReport {
        wanted: wanted.len(),
        inserted: outcome.inserted,
        rearmed: outcome.rearmed,
    })
}

/// Publish every pending stage request up to `limit`.
///
/// A failed publish stops the pass: the broker is the shared dependency, and
/// the row stays pending for the next tick. The stage row is untouched either
/// way, so this function cannot lose or duplicate a stage.
///
/// # Errors
///
/// Returns [`OutboxError`] when the outbox cannot be read or updated. A publish
/// failure is a [`DrainReport::error`], not an error, because it is expected
/// while the broker is down.
pub async fn drain(
    outbox: &dyn StageOutbox,
    queue: &dyn Queue,
    limit: usize,
) -> Result<DrainReport, OutboxError> {
    let pending = outbox.pending(limit).await?;
    let mut report = DrainReport::default();
    for notification in pending {
        match queue
            .publish(ROUTING_STAGE_REQUESTED, payload(&notification.stage_run_id))
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

/// The wire payload of every stage request: the stage run id and nothing else.
#[must_use]
pub fn payload(stage_run_id: &StageRunId) -> Vec<u8> {
    serde_json::json!({ "stage_run_id": stage_run_id.to_string() })
        .to_string()
        .into_bytes()
}

/// Run one reconcile-then-drain pass now, and repeat it on the interval.
///
/// The first pass is a startup pass, so a process that starts against an empty
/// or rebuilt queue republishes every unfinished stage. Later passes insert a
/// missing intent and drain; they never repeat a confirmed publish.
pub fn spawn_publisher(
    outbox: Arc<dyn StageOutbox>,
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
                        "stage notifications reconciled"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%error, "the stage-notification reconciliation will retry");
                }
            }
            match drain(outbox.as_ref(), queue.as_ref(), DRAIN_LIMIT).await {
                Ok(report) if report.published > 0 => {
                    tracing::info!(
                        published = report.published,
                        "stage notifications published"
                    );
                }
                Ok(report) => {
                    if let Some(error) = report.error {
                        tracing::warn!(%error, "the stage notification will retry");
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, "the stage-notification drain will retry");
                }
            }
            if let Some(metrics) = &metrics {
                match outbox.outstanding().await {
                    Ok(pending) => metrics.set_stage_outbox_pending(pending),
                    Err(error) => {
                        tracing::warn!(%error, "the stage outbox depth could not be read");
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

/// Re-arm every unfinished stage whenever this replica becomes the leader.
///
/// A lease takeover is the same recovery point as a process restart: the old
/// leader may have consumed a notification that was then lost or dead-lettered.
/// The startup pass re-arms every unfinished stage, and the queue stays the
/// delivery path. A standby performs no work here.
pub fn spawn_leader_reconcile(
    mut leadership: tokio::sync::watch::Receiver<Leadership>,
    outbox: Arc<dyn StageOutbox>,
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
                            "the new evidence leader re-armed stage notifications"
                        );
                    }
                    Ok(_) => {}
                    Err(error) => {
                        tracing::warn!(%error, "the leader stage re-arm will retry on the next election");
                    }
                }
                match drain(outbox.as_ref(), queue.as_ref(), DRAIN_LIMIT).await {
                    Ok(report) => {
                        if let Some(error) = report.error {
                            tracing::warn!(%error, "the leader stage re-arm drain will retry");
                        }
                    }
                    Err(error) => {
                        tracing::warn!(%error, "the leader stage re-arm drain will retry");
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
