//! The evidence leader lease and the leader-only finalizer.
//!
//! Two evidence replicas serve reads; one holds a database lease and runs the
//! `run.computed` consumer. The lease row is shared state, the term increments
//! on acquisition, and every finalisation carries the term. A standby waits for
//! the lease to expire, acquires a higher term, and resumes the same queue; the
//! old leader's term no longer matches, so its write is refused.
//!
//! The outbox and the queue remain authoritative. Losing the leader stops the
//! consumer but loses no intent: the run row still names a `run.computed`
//! notification, and the new leader's startup reconciliation re-arms it.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, SystemTime};

use quantrule_runtime::adapters::{BoxFuture, Message, MessageHandler, QueueConsumer, QueueError};

use crate::evidence::{EvidenceError, EvidenceServices, finalize};

/// How long an evidence leader lease stays valid without a renewal.
pub const LEADER_LEASE_SECONDS: u64 = 30;
/// How often the leader renews its lease.
pub const LEADER_RENEW_INTERVAL: Duration = Duration::from_secs(10);
/// How often a standby retries acquisition or a leader retries a failed renewal.
pub const LEADER_RETRY_INTERVAL: Duration = Duration::from_secs(5);
/// How long a fenced delivery waits for the election loop to publish the loss.
const FENCE_RELEASE_WAIT: Duration = Duration::from_secs(5);

/// Why a leader lease could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderLeaseError(pub String);

impl std::fmt::Display for LeaderLeaseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for LeaderLeaseError {}

/// What one replica believes about its leadership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Leadership {
    /// Another replica holds the lease, or this one has not acquired it yet.
    Standby,
    /// This replica holds the lease for the named term.
    Leader(u64),
}

/// The shared leader row, for operator reads and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderStatus {
    /// The replica that holds the row, live or expired.
    pub holder: String,
    /// The fencing term of that holder.
    pub term: u64,
    /// Whether the lease is live at the database's read time.
    pub live: bool,
}

/// A database-backed evidence leader lease.
///
/// Acquisition is a compare-and-swap on expiry. Renewal requires the same
/// holder and term. Release expires the row immediately so a standby can take
/// over without waiting for the whole window.
pub trait LeaderLease: Send + Sync {
    /// Claim an expired lease and return the new term, or `None` while another
    /// replica's lease is live.
    fn acquire(&self, holder: String) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>>;

    /// Renew a lease this holder still owns, or `None` when the term is stale.
    fn renew(&self, holder: String, term: u64) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>>;

    /// Expire this holder's own lease; another holder's lease is untouched.
    fn release(&self, holder: String, term: u64) -> BoxFuture<Result<(), LeaderLeaseError>>;

    /// Read the row as the database sees it.
    fn current(&self) -> BoxFuture<Result<Option<LeaderStatus>, LeaderLeaseError>>;
}

/// The shared memory leader row.
#[derive(Debug, Clone)]
pub(crate) struct MemoryLeaderRow {
    holder: String,
    term: u64,
    expires_at: SystemTime,
}

impl MemoryLeaderRow {
    /// The live term at a given instant, or `None` when expired.
    pub(crate) fn live_term(&self, now: SystemTime) -> Option<u64> {
        (self.expires_at > now).then_some(self.term)
    }
}

/// A leader lease for tests and for a local instance.
#[derive(Debug, Clone)]
pub struct MemoryLeaderLease {
    pub(crate) inner: Arc<Mutex<MemoryLeaderRow>>,
    ttl: Duration,
}

impl Default for MemoryLeaderLease {
    fn default() -> Self {
        Self::with_lease_seconds(LEADER_LEASE_SECONDS)
    }
}

impl MemoryLeaderLease {
    /// Build a lease with an explicit window.
    ///
    /// A test passes zero to observe expiry without sleeping.
    #[must_use]
    pub fn with_lease_seconds(seconds: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryLeaderRow {
                holder: String::new(),
                term: 0,
                expires_at: SystemTime::UNIX_EPOCH,
            })),
            ttl: Duration::from_secs(seconds),
        }
    }

    /// Expire the row without waiting for its window.
    ///
    /// This is the fast fake's equivalent of a leader process disappearing.
    pub fn expire(&self) {
        lock(&self.inner).expires_at = SystemTime::UNIX_EPOCH;
    }

    /// Lock the row for a finalisation fence.
    ///
    /// The run store takes this lock before its own state lock so an acquisition
    /// and a finalisation cannot interleave: the same order the Postgres row
    /// lock enforces.
    pub(crate) fn lock_row(&self) -> MutexGuard<'_, MemoryLeaderRow> {
        lock(&self.inner)
    }
}

impl LeaderLease for MemoryLeaderLease {
    fn acquire(&self, holder: String) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>> {
        let inner = Arc::clone(&self.inner);
        let ttl = self.ttl;
        Box::pin(async move {
            let mut row = lock(&inner);
            let now = SystemTime::now();
            if row.live_term(now).is_some() {
                return Ok(None);
            }
            row.holder = holder;
            row.term = row.term.saturating_add(1);
            row.expires_at = now + ttl;
            Ok(Some(row.term))
        })
    }

    fn renew(&self, holder: String, term: u64) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>> {
        let inner = Arc::clone(&self.inner);
        let ttl = self.ttl;
        Box::pin(async move {
            let mut row = lock(&inner);
            if row.holder != holder || row.term != term {
                return Ok(None);
            }
            row.expires_at = SystemTime::now() + ttl;
            Ok(Some(row.term))
        })
    }

    fn release(&self, holder: String, term: u64) -> BoxFuture<Result<(), LeaderLeaseError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut row = lock(&inner);
            if row.holder == holder && row.term == term {
                row.expires_at = SystemTime::UNIX_EPOCH;
            }
            Ok(())
        })
    }

    fn current(&self) -> BoxFuture<Result<Option<LeaderStatus>, LeaderLeaseError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let row = lock(&inner);
            Ok(Some(LeaderStatus {
                holder: row.holder.clone(),
                term: row.term,
                live: row.live_term(SystemTime::now()).is_some(),
            }))
        })
    }
}

/// Run the acquire-or-renew loop and publish leadership changes.
///
/// The first successful acquisition increments the term. A renewal keeps it.
/// A failed renewal while the database is unreachable keeps the term and
/// retries: the write fence still refuses a finalisation when the lease is not
/// live, so keeping the belief cannot create a second finalizer. A renewal that
/// returns `None` proves another term owns the row, so the replica becomes a
/// standby.
pub fn spawn_leader_election(
    lease: Arc<dyn LeaderLease>,
    holder: String,
    sender: tokio::sync::watch::Sender<Leadership>,
    shutdown: quantrule_runtime::shutdown::Shutdown,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut term: Option<u64> = None;
        loop {
            if shutdown.is_requested() {
                release_term(lease.as_ref(), &holder, term).await;
                return;
            }
            let (next, wait) = match term {
                Some(current) => match lease.renew(holder.clone(), current).await {
                    Ok(Some(renewed)) => {
                        publish(&sender, Leadership::Leader(renewed));
                        (Some(renewed), LEADER_RENEW_INTERVAL)
                    }
                    Ok(None) => {
                        tracing::warn!(term = current, "the evidence leader lease was lost");
                        publish(&sender, Leadership::Standby);
                        (None, LEADER_RETRY_INTERVAL)
                    }
                    Err(error) => {
                        tracing::warn!(%error, term = current, "the evidence leader lease renewal will retry");
                        (Some(current), LEADER_RETRY_INTERVAL)
                    }
                },
                None => match lease.acquire(holder.clone()).await {
                    Ok(Some(acquired)) => {
                        tracing::info!(term = acquired, "this evidence replica is the leader");
                        publish(&sender, Leadership::Leader(acquired));
                        (Some(acquired), LEADER_RENEW_INTERVAL)
                    }
                    Ok(None) => {
                        publish(&sender, Leadership::Standby);
                        (None, LEADER_RETRY_INTERVAL)
                    }
                    Err(error) => {
                        tracing::warn!(%error, "the evidence leader lease acquisition will retry");
                        (None, LEADER_RETRY_INTERVAL)
                    }
                },
            };
            term = next;
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => {
                    release_term(lease.as_ref(), &holder, term).await;
                    return;
                }
                () = tokio::time::sleep(wait) => {}
            }
        }
    })
}

/// Release a live term during a drain, and log the outcome.
async fn release_term(lease: &dyn LeaderLease, holder: &str, term: Option<u64>) {
    let Some(term) = term else {
        return;
    };
    match lease.release(holder.to_owned(), term).await {
        Ok(()) => tracing::info!(term, "the draining evidence leader released its lease"),
        Err(error) => tracing::warn!(%error, term, "the draining leader lease was not released"),
    }
}

/// Release the live lease this replica owns so a standby takes over at once.
///
/// The caller stops the election task first; reading the watch afterward is the
/// last published term. A release by a stale holder or term is a no-op in the
/// lease implementation, so a race cannot clear the new leader's row.
///
/// # Errors
///
/// Returns [`LeaderLeaseError`] when the database refuses the release.
pub async fn release_leadership(
    lease: &dyn LeaderLease,
    holder: &str,
    leadership: &tokio::sync::watch::Receiver<Leadership>,
) -> Result<(), LeaderLeaseError> {
    if let Leadership::Leader(term) = *leadership.borrow() {
        lease.release(holder.to_owned(), term).await?;
        tracing::info!(term, "the evidence leader released its lease");
    }
    Ok(())
}

/// The leader-only `run.computed` consumer.
///
/// It waits for leadership, consumes while the term stays current, and drops
/// the consumer future on loss. Dropping the future closes the consumer channel;
/// an unacknowledged delivery returns to the queue, so no work is lost.
pub struct LeaderConsumer {
    consumer: Arc<dyn QueueConsumer>,
    queue: String,
    services: Arc<EvidenceServices>,
    metrics: Option<quantrule_runtime::metrics::Metrics>,
}

impl LeaderConsumer {
    /// Build the leader-only consumer.
    #[must_use]
    pub fn new(
        consumer: Arc<dyn QueueConsumer>,
        queue: String,
        services: Arc<EvidenceServices>,
    ) -> Self {
        Self {
            consumer,
            queue,
            services,
            metrics: None,
        }
    }

    /// Attach the process metrics for reconnect counts.
    #[must_use]
    pub fn with_metrics(mut self, metrics: quantrule_runtime::metrics::Metrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Consume while this replica is the leader, until shutdown or an exhausted
    /// reconnect budget.
    ///
    /// A leadership loss drops the consumer without a reconnect: the replica is
    /// a standby, and the new leader re-arms the outbox. A consumer failure
    /// inside a live term reconnects with the shared backoff.
    ///
    /// # Errors
    ///
    /// Returns [`quantrule_runtime::reconnect::ReconnectError`] when the
    /// consumer fails past its reconnect budget, so the container restarts.
    pub async fn run(
        self,
        mut leadership: tokio::sync::watch::Receiver<Leadership>,
        shutdown: quantrule_runtime::shutdown::Shutdown,
    ) -> Result<(), quantrule_runtime::reconnect::ReconnectError> {
        loop {
            let Some(term) = next_leader_term(&mut leadership, &shutdown).await else {
                return Ok(());
            };
            tracing::info!(term, queue = %self.queue, "the evidence leader is consuming run.computed");
            let services = Arc::clone(&self.services);
            let handler_leadership = leadership.clone();
            let handler: MessageHandler = Arc::new(move |message: Message| {
                let services = Arc::clone(&services);
                let handler_leadership = handler_leadership.clone();
                Box::pin(
                    async move { handle_computed(message, services, handler_leadership).await },
                )
            });
            let consuming = quantrule_runtime::reconnect::consume(
                Arc::clone(&self.consumer),
                self.queue.clone(),
                handler,
                shutdown.clone(),
                "evidence",
                "evidence",
                self.metrics.clone(),
            );
            let mut drain = shutdown.clone();
            tokio::select! {
                biased;
                () = drain.wait() => {
                    tracing::info!(term, "the evidence leader stopped the run.computed consumer for drain");
                    return Ok(());
                }
                () = wait_until_term_ends(leadership.clone(), term) => {
                    tracing::warn!(term, "the evidence leader stopped the run.computed consumer");
                }
                result = consuming => {
                    return result;
                }
            }
        }
    }
}

/// Finalize one delivery under the current leader term.
async fn handle_computed(
    message: Message,
    services: Arc<EvidenceServices>,
    leadership: tokio::sync::watch::Receiver<Leadership>,
) -> Result<(), QueueError> {
    let Some(run_id) = serde_json::from_slice::<serde_json::Value>(&message.payload)
        .ok()
        .and_then(|value| {
            value
                .get("run_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
    else {
        tracing::error!("run.computed carries no run_id; acknowledging");
        return Ok(());
    };
    let term = match *leadership.borrow() {
        Leadership::Leader(term) => term,
        Leadership::Standby => {
            return Err(QueueError(
                "this evidence replica is not the leader".to_owned(),
            ));
        }
    };
    match finalize(run_id.clone(), &services, term).await {
        Ok(outcome) => {
            tracing::info!(run_id = %run_id, outcome = ?outcome, term, "run finalized");
            Ok(())
        }
        Err(EvidenceError::LeaderFenced) => {
            // Wait for the election loop to publish the loss so the consumer
            // task drops the in-flight delivery instead of counting a rejection
            // toward the quorum delivery limit.
            tracing::warn!(run_id = %run_id, term, "the evidence leader term is stale");
            let _ =
                tokio::time::timeout(FENCE_RELEASE_WAIT, wait_until_term_ends(leadership, term))
                    .await;
            Err(QueueError("the evidence leader term is stale".to_owned()))
        }
        Err(error) => {
            tracing::error!(run_id = %run_id, error = %error, "run finalization failed");
            Err(QueueError(error.to_string()))
        }
    }
}

/// Wait for a live leader term, or `None` when the watch closes or the process
/// drains.
async fn next_leader_term(
    leadership: &mut tokio::sync::watch::Receiver<Leadership>,
    shutdown: &quantrule_runtime::shutdown::Shutdown,
) -> Option<u64> {
    loop {
        let current = *leadership.borrow_and_update();
        if let Leadership::Leader(term) = current {
            return Some(term);
        }
        let mut drain = shutdown.clone();
        tokio::select! {
            biased;
            () = drain.wait() => return None,
            changed = leadership.changed() => {
                if changed.is_err() {
                    return None;
                }
            }
        }
    }
}

/// Wait until the named term is no longer the current one.
async fn wait_until_term_ends(mut leadership: tokio::sync::watch::Receiver<Leadership>, term: u64) {
    loop {
        let current = *leadership.borrow_and_update();
        if !matches!(current, Leadership::Leader(current) if current == term) {
            return;
        }
        if leadership.changed().await.is_err() {
            return;
        }
    }
}

fn publish(sender: &tokio::sync::watch::Sender<Leadership>, leadership: Leadership) {
    sender.send_if_modified(|current| {
        if *current == leadership {
            false
        } else {
            *current = leadership;
            true
        }
    });
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
