//! The in-memory run store.
//!
//! It carries the same rules as the Postgres store: one content-addressed run
//! row per identity, one submission row per profile, the terminal-state
//! predicate, and the outbox and release records that make a retry safe. It is
//! the fake the fast gate uses, so every rule a production test relies on must
//! hold here too.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use quantrule_api::{
    ArtifactRef, DatasetReferenceKind, Digest, ProfileId, RunId, RunState, RunSummary,
    StageProgress,
};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_service_api::RUN_LEASE_TTL_SECONDS;

use crate::leader::MemoryLeaderLease;
use crate::outbox::{EnsureOutcome, OutboxError, OutboxNotification, ReconcileMode, RunOutbox};
use crate::releases::{ReferenceReleaseLog, ReleaseLogError};

use super::store::{
    CancelOutcome, ClaimOutcome, DatasetReferenceRelease, HeartbeatOutcome, NewRun,
    NotificationKind, RunArchiveOutcome, RunDatasetReference, RunEraseOutcome, RunEraseReceipt,
    RunNotification, RunRecord, RunStore, RunStoreError, SubmitOutcome,
};

#[derive(Debug, Default)]
pub(crate) struct MemoryState {
    runs: BTreeMap<String, RunRecord>,
    submissions: BTreeMap<(String, String), Option<String>>,
    notifications: BTreeMap<(String, String), MemoryNotification>,
    next_event_id: i64,
    releases: BTreeMap<(String, String, String), MemoryRelease>,
    leases: BTreeMap<String, MemoryLease>,
}

#[derive(Debug, Clone)]
struct MemoryLease {
    holder: String,
    term: u64,
    expires_at: SystemTime,
}

#[derive(Debug, Clone)]
struct MemoryNotification {
    event_id: i64,
    kind: NotificationKind,
    attempts: u32,
    last_error: Option<String>,
    published: bool,
    published_at: Option<Instant>,
}

#[derive(Debug, Clone)]
struct MemoryRelease {
    profile: ProfileId,
    data_version: Digest,
    reference_id: String,
    kind: DatasetReferenceKind,
    released: bool,
    attempts: u32,
    last_error: Option<String>,
}

impl MemoryState {
    fn ensure_notification(&mut self, run_id: &str, kind: NotificationKind) -> bool {
        let key = (run_id.to_owned(), kind.routing_key().to_owned());
        if self.notifications.contains_key(&key) {
            return false;
        }
        let event_id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);
        self.notifications.insert(
            key,
            MemoryNotification {
                event_id,
                kind,
                attempts: 0,
                last_error: None,
                published: false,
                published_at: None,
            },
        );
        true
    }

    fn rearm_notification(&mut self, run_id: &str, kind: NotificationKind) -> bool {
        let key = (run_id.to_owned(), kind.routing_key().to_owned());
        let Some(row) = self.notifications.get_mut(&key) else {
            return false;
        };
        if !row.published {
            return false;
        }
        row.published = false;
        row.last_error = None;
        true
    }

    /// Re-arm an expired-lease request at most once per lease window.
    ///
    /// A periodic pass must not publish the same request every tick; the row's
    /// last confirmed publish is the throttle. A live lease never matches.
    fn periodic_rearm(&mut self, run_id: &str, ttl: Duration) -> bool {
        let Some(record) = self.runs.get(run_id) else {
            return false;
        };
        if record.status != RunState::Running || record.digest.is_some() {
            return false;
        }
        if self.live_lease(run_id).is_some() {
            return false;
        }
        let key = (
            run_id.to_owned(),
            NotificationKind::Requested.routing_key().to_owned(),
        );
        let Some(row) = self.notifications.get_mut(&key) else {
            return false;
        };
        if !row.published || !row.published_at.is_some_and(|at| at.elapsed() >= ttl) {
            return false;
        }
        row.published = false;
        row.published_at = None;
        row.last_error = None;
        true
    }

    fn clear_notifications(&mut self, run_id: &str) {
        self.notifications
            .retain(|(candidate, _), _| candidate != run_id);
    }

    fn live_lease(&self, run_id: &str) -> Option<&MemoryLease> {
        self.leases
            .get(run_id)
            .filter(|lease| lease.expires_at > SystemTime::now())
    }

    fn start_lease(&mut self, run_id: &str, holder: &str, ttl: Duration) -> bool {
        let Some(record) = self.runs.get_mut(run_id) else {
            return false;
        };
        if record.attempts >= super::store::MAX_ATTEMPTS {
            return false;
        }
        record.status = RunState::Running;
        record.attempts = record.attempts.saturating_add(1);
        record.lease_term = record.lease_term.saturating_add(1);
        record.lease_holder = Some(holder.to_owned());
        let expires_at = SystemTime::now() + ttl;
        record.lease_expires_at = quantrule_runtime::clock::rfc3339_at(expires_at).ok();
        self.leases.insert(
            run_id.to_owned(),
            MemoryLease {
                holder: holder.to_owned(),
                term: record.lease_term,
                expires_at,
            },
        );
        true
    }

    fn clear_lease(&mut self, run_id: &str) {
        self.leases.remove(run_id);
        if let Some(record) = self.runs.get_mut(run_id) {
            record.lease_holder = None;
            record.lease_expires_at = None;
        }
    }

    fn ensure_release(&mut self, release: &DatasetReferenceRelease) -> bool {
        let key = (
            release.profile.as_str().to_owned(),
            release.data_version.to_string(),
            release.reference_id.clone(),
        );
        if self.releases.contains_key(&key) {
            return false;
        }
        self.releases.insert(
            key,
            MemoryRelease {
                profile: release.profile.clone(),
                data_version: release.data_version.clone(),
                reference_id: release.reference_id.clone(),
                kind: release.kind,
                released: false,
                attempts: 0,
                last_error: None,
            },
        );
        true
    }

    fn notification_by_event(&mut self, event_id: i64) -> Option<&mut MemoryNotification> {
        self.notifications
            .values_mut()
            .find(|row| row.event_id == event_id)
    }
}

/// An in-memory store, for tests and for a local instance.
#[derive(Debug, Clone)]
pub struct MemoryRuns {
    inner: Arc<Mutex<MemoryState>>,
    lease_ttl: Duration,
    leader: MemoryLeaderLease,
}

impl Default for MemoryRuns {
    fn default() -> Self {
        Self::with_lease_seconds(RUN_LEASE_TTL_SECONDS)
    }
}

impl MemoryRuns {
    /// Build a store with an explicit lease TTL.
    ///
    /// A test that must observe an expired lease passes zero instead of
    /// sleeping for the production window.
    #[must_use]
    pub fn with_lease_seconds(seconds: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryState::default())),
            lease_ttl: Duration::from_secs(seconds),
            leader: MemoryLeaderLease::default(),
        }
    }

    /// The leader lease that fences this store's finalisation.
    ///
    /// The handle shares the row with the store, so a test can acquire a term
    /// and then complete a run under it.
    #[must_use]
    pub fn leader_lease(&self) -> MemoryLeaderLease {
        self.leader.clone()
    }

    /// Expire a run's lease without waiting for its window.
    ///
    /// This is the fast fake's equivalent of a worker dying: the term stays on
    /// the record, and the next claim records a new one.
    pub fn expire_lease(&self, run_id: &str) {
        let mut state = self.inner.lock().expect("the run lock");
        state.leases.remove(run_id);
        if let Some(record) = state.runs.get_mut(run_id) {
            record.lease_expires_at = None;
        }
    }
}

impl RunStore for MemoryRuns {
    fn submit(
        &self,
        profile: ProfileId,
        run: NewRun,
    ) -> BoxFuture<Result<SubmitOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let key = run.run_id.to_string();
            let profile = profile.as_str().to_owned();
            let existing = state.runs.get(&key).cloned();
            state
                .submissions
                .entry((profile, key.clone()))
                .or_insert(None);
            let (record, created) = match existing {
                Some(record) => (record, false),
                None => {
                    let record = RunRecord::queued(&run);
                    state.runs.insert(key.clone(), record.clone());
                    (record, true)
                }
            };
            if record.status == RunState::Queued {
                state.ensure_notification(&key, NotificationKind::Requested);
                // A repeat submission is the client's recovery signal: a queued
                // run whose publish was lost is republished. The worker treats
                // the repeat as a lookup, so the run still executes once.
                state.rearm_notification(&key, NotificationKind::Requested);
            }
            Ok(if created {
                SubmitOutcome::Created(record)
            } else {
                SubmitOutcome::Existing(record)
            })
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let profile = profile.as_str();
            let mut runs: Vec<&RunRecord> = state
                .runs
                .iter()
                .filter(|(key, record)| {
                    state
                        .submissions
                        .get(&(profile.to_owned(), (*key).clone()))
                        .is_some_and(Option::is_none)
                        && status.is_none_or(|wanted| wanted == record.status)
                })
                .map(|(_, record)| record)
                .collect();
            runs.sort_by(|left, right| {
                right
                    .submitted_at
                    .cmp(&left.submitted_at)
                    .then_with(|| left.run_id.cmp(&right.run_id))
            });
            Ok(runs
                .into_iter()
                .take(limit)
                .map(RunRecord::summary)
                .collect())
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let profile = profile.as_str();
            let mut runs: Vec<((String, String), &RunRecord)> = state
                .runs
                .iter()
                .filter(|(key, record)| {
                    state
                        .submissions
                        .contains_key(&(profile.to_owned(), (*key).clone()))
                        && status.is_none_or(|wanted| wanted == record.status)
                })
                .map(|(key, record)| ((profile.to_owned(), key.clone()), record))
                .collect();
            runs.sort_by(|(_, left), (_, right)| {
                right
                    .submitted_at
                    .cmp(&left.submitted_at)
                    .then_with(|| left.run_id.cmp(&right.run_id))
            });
            Ok(runs
                .into_iter()
                .take(limit)
                .map(|(key, record)| {
                    let mut summary = record.summary();
                    summary.archived_at = state
                        .submissions
                        .get(&(profile.to_owned(), key.0))
                        .cloned()
                        .flatten();
                    summary
                })
                .collect())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            if !state
                .submissions
                .contains_key(&(profile.as_str().to_owned(), run_id.clone()))
            {
                return Ok(None);
            }
            Ok(state.runs.get(&run_id).cloned())
        })
    }

    fn archive(
        &self,
        profile: ProfileId,
        run_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<RunArchiveOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), run_id.clone());
            let Some(entry) = state.submissions.get_mut(&key) else {
                return Ok(RunArchiveOutcome::Missing);
            };
            if entry.is_some() {
                return Ok(RunArchiveOutcome::AlreadyArchived);
            }
            *entry = Some(archived_at.clone());
            let Some(record) = state.runs.get(&run_id) else {
                return Ok(RunArchiveOutcome::Missing);
            };
            let mut summary = record.summary();
            summary.archived_at = Some(archived_at);
            Ok(RunArchiveOutcome::Archived(summary))
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunEraseOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let key = (profile.as_str().to_owned(), run_id.clone());
            let Some(archived_at) = state.submissions.get(&key) else {
                return Ok(RunEraseOutcome::Missing);
            };
            if archived_at.is_none() {
                return Ok(RunEraseOutcome::NotArchived);
            }
            let Some(record) = state.runs.get(&run_id).cloned() else {
                return Ok(RunEraseOutcome::Missing);
            };
            if !matches!(record.status, RunState::Failed | RunState::Cancelled) {
                return Ok(RunEraseOutcome::Protected(record.status));
            }
            // The release is recorded before the submission is removed and
            // survives it, so a failed data-service call is retried instead of
            // leaving an orphan reference that blocks the dataset forever.
            state.ensure_release(&DatasetReferenceRelease {
                profile: profile.clone(),
                data_version: record.data_version.clone(),
                reference_id: format!("run:{}", record.run_id),
                kind: DatasetReferenceKind::Run,
            });
            state.submissions.remove(&key);
            let remaining = state.submissions.keys().any(|(_, id)| id == &run_id);
            if remaining {
                return Ok(RunEraseOutcome::Erased(RunEraseReceipt {
                    run_id: record.run_id,
                    data_version: record.data_version,
                    bytes: 0,
                    blobs: Vec::new(),
                    removed_run: false,
                }));
            }
            state.clear_lease(&run_id);
            state.runs.remove(&run_id);
            state.clear_notifications(&run_id);
            let mut blobs = Vec::new();
            let mut bytes = 0u64;
            let mut seen = std::collections::BTreeSet::new();
            for artifact in &record.artifacts {
                if !seen.insert(artifact.sha256.clone()) {
                    continue;
                }
                let shared = state.runs.values().any(|other| {
                    other
                        .artifacts
                        .iter()
                        .any(|candidate| candidate.sha256 == artifact.sha256)
                });
                if !shared {
                    bytes = bytes.saturating_add(artifact.bytes);
                    blobs.push(artifact.sha256.clone());
                }
            }
            Ok(RunEraseOutcome::Erased(RunEraseReceipt {
                run_id: record.run_id,
                data_version: record.data_version,
                bytes,
                blobs,
                removed_run: true,
            }))
        })
    }

    fn submitted_datasets(&self) -> BoxFuture<Result<Vec<RunDatasetReference>, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let mut rows = Vec::new();
            for (profile, run_id) in state.submissions.keys() {
                let Some(record) = state.runs.get(run_id) else {
                    continue;
                };
                rows.push(RunDatasetReference {
                    profile: ProfileId::parse(profile).map_err(|error| {
                        RunStoreError(format!("a stored profile is malformed: {error}"))
                    })?,
                    run_id: record.run_id.clone(),
                    data_version: record.data_version.clone(),
                });
            }
            Ok(rows)
        })
    }

    fn cancel(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<CancelOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            if !state
                .submissions
                .contains_key(&(profile.as_str().to_owned(), run_id.clone()))
            {
                return Ok(CancelOutcome::Missing);
            }
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Ok(CancelOutcome::Missing);
            };
            if !record.status.is_cancellable() {
                return Ok(CancelOutcome::Conflict(record.clone()));
            }
            record.status = RunState::Cancelled;
            // The cancellation is the fence: a worker holding the old term can
            // no longer complete or fail the run.
            record.lease_term = record.lease_term.saturating_add(1);
            record.lease_holder = None;
            record.lease_expires_at = None;
            let record = record.clone();
            state.leases.remove(&run_id);
            state.clear_notifications(&run_id);
            Ok(CancelOutcome::Cancelled(record))
        })
    }

    fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            Ok(state.runs.get(&run_id).cloned())
        })
    }

    fn claim(
        &self,
        run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<ClaimOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        let lease_ttl = self.lease_ttl;
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let Some(record) = state.runs.get(&run_id).cloned() else {
                return Ok(ClaimOutcome::Missing);
            };
            match record.status {
                RunState::Queued => {}
                RunState::Running => {
                    // A computed run waits for evidence; no worker may claim it.
                    if record.digest.is_some() {
                        return Ok(ClaimOutcome::Running(record));
                    }
                    let live = state.live_lease(&run_id).cloned();
                    let same_attempt = live.as_ref().is_some_and(|lease| {
                        continuation == Some(lease.term) && lease.holder == holder
                    });
                    if live.is_some() && !same_attempt {
                        return Ok(ClaimOutcome::Running(record));
                    }
                    if record.attempts >= super::store::MAX_ATTEMPTS {
                        return Ok(ClaimOutcome::Exhausted(record));
                    }
                }
                _ => return Ok(ClaimOutcome::Terminal(record)),
            }
            if !state.start_lease(&run_id, &holder, lease_ttl) {
                return Ok(ClaimOutcome::Exhausted(record));
            }
            let Some(claimed) = state.runs.get(&run_id).cloned() else {
                return Ok(ClaimOutcome::Missing);
            };
            Ok(ClaimOutcome::Claimed(claimed))
        })
    }

    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<HeartbeatOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        let ttl = self.lease_ttl;
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let Some(record) = state.runs.get(&run_id).cloned() else {
                return Ok(HeartbeatOutcome::Missing);
            };
            if record.status != RunState::Running {
                return Ok(HeartbeatOutcome::Stale);
            }
            let Some(lease) = state.live_lease(&run_id).cloned() else {
                return Ok(HeartbeatOutcome::Stale);
            };
            if lease.holder != holder || lease.term != term {
                return Ok(HeartbeatOutcome::Stale);
            }
            let expires_at = SystemTime::now() + ttl;
            if let Some(lease) = state.leases.get_mut(&run_id) {
                lease.expires_at = expires_at;
            }
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Ok(HeartbeatOutcome::Missing);
            };
            record.lease_expires_at = quantrule_runtime::clock::rfc3339_at(expires_at).ok();
            Ok(HeartbeatOutcome::Renewed(Box::new(record.clone())))
        })
    }

    fn computed(
        &self,
        run_id: String,
        term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Err(RunStoreError(format!("run {run_id} is unknown")));
            };
            if record.status == RunState::Running && record.lease_term == term {
                record.digest = Some(digest);
                record.stages = stages;
                record.artifacts = artifacts;
                let record = record.clone();
                state.ensure_notification(&run_id, NotificationKind::Computed);
                return Ok(record);
            }
            Ok(record.clone())
        })
    }

    fn complete(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let result = self.finalize_succeeded(run_id, term, leader_term, digest, artifacts, stages);
        Box::pin(async move { result })
    }

    fn fail_finalization(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let result = self.finalize_failed(run_id, term, leader_term, error, stages);
        Box::pin(async move { result })
    }

    fn fail(
        &self,
        run_id: String,
        term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Err(RunStoreError(format!("run {run_id} is unknown")));
            };
            if record.status != RunState::Running || record.lease_term != term {
                return Ok(record.clone());
            }
            record.status = RunState::Failed;
            record.error = Some(error);
            record.stages = stages;
            record.lease_holder = None;
            record.lease_expires_at = None;
            let record = record.clone();
            state.leases.remove(&run_id);
            state.clear_notifications(&run_id);
            Ok(record)
        })
    }
}

impl RunOutbox for MemoryRuns {
    fn wanted_notifications(&self) -> BoxFuture<Result<Vec<RunNotification>, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            let mut wanted = Vec::new();
            for (run_id, record) in &state.runs {
                let kind = match record.status {
                    RunState::Queued => Some(NotificationKind::Requested),
                    // A running run without a digest may have lost its worker
                    // or its delivery. It is wanted only when the lease is
                    // absent or expired; a live lease must not spin messages.
                    RunState::Running
                        if record.digest.is_none() && state.live_lease(run_id).is_none() =>
                    {
                        Some(NotificationKind::Requested)
                    }
                    RunState::Running if record.digest.is_none() => None,
                    RunState::Running => Some(NotificationKind::Computed),
                    _ => None,
                };
                if let Some(kind) = kind {
                    wanted.push(RunNotification {
                        run_id: RunId(Digest::parse(run_id).map_err(|error| {
                            OutboxError(format!("a stored run id is malformed: {error}"))
                        })?),
                        kind,
                    });
                }
            }
            Ok(wanted)
        })
    }

    fn ensure(
        &self,
        wanted: &[RunNotification],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        let lease_ttl = self.lease_ttl;
        let wanted = wanted.to_vec();
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            let mut outcome = EnsureOutcome::default();
            for notification in &wanted {
                let run_id = notification.run_id.to_string();
                if state.ensure_notification(&run_id, notification.kind) {
                    outcome.inserted = outcome.inserted.saturating_add(1);
                    continue;
                }
                let rearmed = match mode {
                    ReconcileMode::Startup => state.rearm_notification(&run_id, notification.kind),
                    ReconcileMode::Periodic => state.periodic_rearm(&run_id, lease_ttl),
                };
                if rearmed {
                    outcome.rearmed = outcome.rearmed.saturating_add(1);
                }
            }
            Ok(outcome)
        })
    }

    fn pending(&self, limit: usize) -> BoxFuture<Result<Vec<OutboxNotification>, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            let mut rows: Vec<(&(String, String), &MemoryNotification)> = state
                .notifications
                .iter()
                .filter(|(_, row)| !row.published)
                .collect();
            rows.sort_by_key(|(_, row)| row.event_id);
            rows.into_iter()
                .take(limit)
                .map(|((run_id, _), row)| {
                    Ok(OutboxNotification {
                        event_id: row.event_id,
                        run_id: RunId(Digest::parse(run_id).map_err(|error| {
                            OutboxError(format!("a stored run id is malformed: {error}"))
                        })?),
                        kind: row.kind,
                        attempts: row.attempts,
                    })
                })
                .collect()
        })
    }

    fn published(&self, event_id: i64) -> BoxFuture<Result<(), OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            let Some(row) = state.notification_by_event(event_id) else {
                return Err(OutboxError(format!("outbox event {event_id} is unknown")));
            };
            row.published = true;
            row.published_at = Some(Instant::now());
            row.attempts = row.attempts.saturating_add(1);
            row.last_error = None;
            Ok(())
        })
    }

    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            let Some(row) = state.notification_by_event(event_id) else {
                return Err(OutboxError(format!("outbox event {event_id} is unknown")));
            };
            row.attempts = row.attempts.saturating_add(1);
            row.last_error = Some(error);
            Ok(())
        })
    }

    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the run lock is poisoned".to_owned()))?;
            Ok(state
                .notifications
                .values()
                .filter(|row| !row.published)
                .count() as u64)
        })
    }
}

impl ReferenceReleaseLog for MemoryRuns {
    fn record(&self, release: DatasetReferenceRelease) -> BoxFuture<Result<(), ReleaseLogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ReleaseLogError("the run lock is poisoned".to_owned()))?;
            state.ensure_release(&release);
            Ok(())
        })
    }

    fn pending_releases(&self) -> BoxFuture<Result<Vec<DatasetReferenceRelease>, ReleaseLogError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| ReleaseLogError("the run lock is poisoned".to_owned()))?;
            let mut rows: Vec<&MemoryRelease> = state
                .releases
                .values()
                .filter(|row| !row.released)
                .collect();
            rows.sort_by(|left, right| {
                left.profile
                    .as_str()
                    .cmp(right.profile.as_str())
                    .then_with(|| left.data_version.cmp(&right.data_version))
                    .then_with(|| left.reference_id.cmp(&right.reference_id))
            });
            Ok(rows
                .into_iter()
                .map(|row| DatasetReferenceRelease {
                    profile: row.profile.clone(),
                    data_version: row.data_version.clone(),
                    reference_id: row.reference_id.clone(),
                    kind: row.kind,
                })
                .collect())
        })
    }

    fn recorded(
        &self,
        release: &DatasetReferenceRelease,
        error: Option<&str>,
    ) -> BoxFuture<Result<(), ReleaseLogError>> {
        let inner = Arc::clone(&self.inner);
        let release = release.clone();
        let error = error.map(str::to_owned);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| ReleaseLogError("the run lock is poisoned".to_owned()))?;
            let key = (
                release.profile.as_str().to_owned(),
                release.data_version.to_string(),
                release.reference_id.clone(),
            );
            let Some(row) = state.releases.get_mut(&key) else {
                return Err(ReleaseLogError(format!(
                    "the release tombstone {} is unknown",
                    release.reference_id
                )));
            };
            row.attempts = row.attempts.saturating_add(1);
            match error {
                Some(message) => row.last_error = Some(message),
                None => {
                    row.released = true;
                    row.last_error = None;
                }
            }
            Ok(())
        })
    }
}

mod finalize;

#[cfg(test)]
mod tests;
