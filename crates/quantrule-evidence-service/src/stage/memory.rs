//! The in-memory stage store.
//!
//! The memory store is the test double and the local-development backend. It
//! implements the same state machine as the Postgres store so a test cannot
//! accept a transition production rejects: a queued stage starts on a claim, a
//! live lease is never stolen, an expired running stage is reclaimed up to
//! [`MAX_ATTEMPTS`], and only a running stage under the current term can
//! succeed or fail.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use quantrule_api::{
    Digest, ProfileId, StageArtifact, StageKind, StageRunId, StageRunSummary, StageState,
};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_service_api::STAGE_LEASE_TTL_SECONDS;

use super::outbox::{StageOutbox, StageOutboxNotification};
use super::store::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageCreateOutcome,
    StageDeletion, StageExpiry, StageHeartbeatOutcome, StageRecord, StageStore, StageStoreError,
    validate_artifact, validate_new_stage,
};
use crate::outbox::{EnsureOutcome, OutboxError, ReconcileMode};

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<String, StageRecord>,
    submissions: BTreeSet<(String, String)>,
    leases: BTreeMap<String, MemoryLease>,
    notifications: BTreeMap<String, MemoryNotification>,
    next_event_id: i64,
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
    attempts: u32,
    last_error: Option<String>,
    published: bool,
    published_at: Option<Instant>,
}

/// An in-memory stage store, for tests and for a local instance.
#[derive(Debug, Clone)]
pub struct MemoryStages {
    inner: Arc<Mutex<MemoryState>>,
    lease_ttl: Duration,
}

impl Default for MemoryStages {
    fn default() -> Self {
        Self::with_lease_seconds(STAGE_LEASE_TTL_SECONDS)
    }
}

impl MemoryStages {
    /// Build a store with an explicit lease TTL.
    ///
    /// A test that must observe an expired lease passes zero instead of
    /// sleeping for the production window.
    #[must_use]
    pub fn with_lease_seconds(seconds: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(MemoryState::default())),
            lease_ttl: Duration::from_secs(seconds),
        }
    }

    /// Expire a stage's lease without waiting for its window.
    ///
    /// This is the fast fake's equivalent of a worker dying: the term stays on
    /// the record, and the next claim records a new one.
    pub fn expire_lease(&self, stage_run_id: &str) {
        let mut state = self.inner.lock().expect("the stage lock");
        state.leases.remove(stage_run_id);
    }
}

impl MemoryState {
    fn lease_live(&self, stage_run_id: &str) -> bool {
        self.leases
            .get(stage_run_id)
            .is_some_and(|lease| lease.expires_at > SystemTime::now())
    }

    fn take_lease(&mut self, stage_run_id: &str, holder: &str, ttl: Duration) {
        let record = self.runs.get_mut(stage_run_id);
        let Some(record) = record else {
            return;
        };
        record.lease_term = record.lease_term.saturating_add(1);
        let term = record.lease_term;
        self.leases.insert(
            stage_run_id.to_owned(),
            MemoryLease {
                holder: holder.to_owned(),
                term,
                expires_at: SystemTime::now() + ttl,
            },
        );
    }

    fn clear_lease(&mut self, stage_run_id: &str) {
        self.leases.remove(stage_run_id);
    }

    fn ensure_notification(&mut self, stage_run_id: &str) -> bool {
        if self.notifications.contains_key(stage_run_id) {
            return false;
        }
        let event_id = self.next_event_id;
        self.next_event_id = self.next_event_id.saturating_add(1);
        self.notifications.insert(
            stage_run_id.to_owned(),
            MemoryNotification {
                event_id,
                attempts: 0,
                last_error: None,
                published: false,
                published_at: None,
            },
        );
        true
    }
}

fn lock(
    inner: &Arc<Mutex<MemoryState>>,
) -> Result<std::sync::MutexGuard<'_, MemoryState>, StageStoreError> {
    inner
        .lock()
        .map_err(|_| StageStoreError("the stage lock is poisoned".to_owned()))
}

impl StageStore for MemoryStages {
    fn create(
        &self,
        profile: ProfileId,
        run: NewStageRun,
    ) -> BoxFuture<Result<StageCreateOutcome, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            validate_new_stage(&run)?;
            let mut state = lock(&inner)?;
            let key = run.stage_run_id.to_string();
            let profile = profile.as_str().to_owned();
            let existing = state.runs.get(&key).cloned();
            state.submissions.insert((profile, key.clone()));
            match existing {
                Some(record) => {
                    // A repeated submission re-arms an unfinished stage so a
                    // lost publish is recovered; a terminal stage is never
                    // re-armed.
                    if record.state == StageState::Queued || record.state == StageState::Running {
                        state.ensure_notification(&key);
                        if let Some(notification) = state.notifications.get_mut(&key)
                            && notification.published
                        {
                            notification.published = false;
                            notification.published_at = None;
                        }
                    }
                    Ok(StageCreateOutcome::Existing(record))
                }
                None => {
                    let record = StageRecord::queued(&run);
                    state.runs.insert(key.clone(), record.clone());
                    state.ensure_notification(&key);
                    Ok(StageCreateOutcome::Created(record))
                }
            }
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        kind: Option<StageKind>,
        status: Option<StageState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageRunSummary>, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            let profile = profile.as_str();
            let mut runs: Vec<&StageRecord> = state
                .runs
                .iter()
                .filter(|(key, record)| {
                    state
                        .submissions
                        .contains(&(profile.to_owned(), (*key).clone()))
                        && kind.is_none_or(|wanted| wanted == record.kind)
                        && status.is_none_or(|wanted| wanted == record.state)
                })
                .map(|(_, record)| record)
                .collect();
            runs.sort_by(|left, right| {
                right
                    .submitted_at
                    .cmp(&left.submitted_at)
                    .then_with(|| left.stage_run_id.cmp(&right.stage_run_id))
            });
            Ok(runs
                .into_iter()
                .take(limit)
                .map(StageRecord::summary)
                .collect())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            if !state
                .submissions
                .contains(&(profile.as_str().to_owned(), stage_run_id.clone()))
            {
                return Ok(None);
            }
            Ok(state.runs.get(&stage_run_id).cloned())
        })
    }

    fn get_any(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            Ok(state.runs.get(&stage_run_id).cloned())
        })
    }

    fn find_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            Ok(state
                .runs
                .values()
                .find(|record| {
                    record.state == StageState::Succeeded
                        && record.kind == kind
                        && record
                            .artifact
                            .as_ref()
                            .is_some_and(|artifact| artifact.artifact_id == artifact_id)
                        && state.submissions.contains(&(
                            profile.as_str().to_owned(),
                            record.stage_run_id.to_string(),
                        ))
                })
                .cloned())
        })
    }

    fn claim(
        &self,
        stage_run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        let lease_ttl = self.lease_ttl;
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get(&stage_run_id) else {
                return Ok(StageClaimOutcome::Missing);
            };
            if record.state == StageState::Queued {
                let record = state
                    .runs
                    .get_mut(&stage_run_id)
                    .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
                record.state = StageState::Running;
                record.attempts = 1;
                state.take_lease(&stage_run_id, &holder, lease_ttl);
                let record = state
                    .runs
                    .get(&stage_run_id)
                    .cloned()
                    .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
                return Ok(StageClaimOutcome::Claimed(record));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record.clone()));
            }
            let live = state.lease_live(&stage_run_id);
            let own_continuation = continuation.is_some_and(|term| {
                state
                    .leases
                    .get(&stage_run_id)
                    .is_some_and(|lease| lease.term == term && lease.holder == holder)
            });
            if live && (!own_continuation || record.attempts >= MAX_ATTEMPTS) {
                return Ok(StageClaimOutcome::Running(record.clone()));
            }
            if !live && record.attempts >= MAX_ATTEMPTS {
                return Ok(StageClaimOutcome::Exhausted(record.clone()));
            }
            // The conditional update is what makes two claims race safely:
            // only one wins the live lease, the other sees it and skips.
            let record = state
                .runs
                .get_mut(&stage_run_id)
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            record.attempts = record.attempts.saturating_add(1);
            state.take_lease(&stage_run_id, &holder, lease_ttl);
            let record = state
                .runs
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            Ok(StageClaimOutcome::Claimed(record))
        })
    }

    fn renew(
        &self,
        stage_run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<StageHeartbeatOutcome, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        let lease_ttl = self.lease_ttl;
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get(&stage_run_id) else {
                return Ok(StageHeartbeatOutcome::Missing);
            };
            let renewed = record.state == StageState::Running
                && state.leases.get(&stage_run_id).is_some_and(|lease| {
                    lease.holder == holder
                        && lease.term == term
                        && lease.expires_at > SystemTime::now()
                });
            if !renewed {
                return Ok(StageHeartbeatOutcome::Stale);
            }
            if let Some(lease) = state.leases.get_mut(&stage_run_id) {
                lease.expires_at = SystemTime::now() + lease_ttl;
            }
            let record = state
                .runs
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            Ok(StageHeartbeatOutcome::Renewed(Box::new(record)))
        })
    }

    fn complete(
        &self,
        stage_run_id: String,
        term: u64,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            validate_artifact(&artifact)?;
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
            };
            // Only the current lease term can finish. The SQL stores enforce
            // the same predicate; the memory store must agree or a test could
            // accept a stale write production refuses.
            if record.state != StageState::Running || record.lease_term != term {
                return Ok(record.clone());
            }
            record.state = StageState::Succeeded;
            record.artifact = Some(artifact);
            record.error = None;
            state.clear_lease(&stage_run_id);
            state.notifications.remove(&stage_run_id);
            let record = state
                .runs
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            Ok(record)
        })
    }

    fn fail(
        &self,
        stage_run_id: String,
        term: u64,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
            };
            if record.state == StageState::Running && record.lease_term == term {
                record.state = StageState::Failed;
                record.error = Some(error);
                state.clear_lease(&stage_run_id);
                state.notifications.remove(&stage_run_id);
            }
            let record = state
                .runs
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            Ok(record)
        })
    }

    fn cancel(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<StageCancelOutcome, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            if !state
                .submissions
                .contains(&(profile.as_str().to_owned(), stage_run_id.clone()))
            {
                return Ok(StageCancelOutcome::Missing);
            }
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Ok(StageCancelOutcome::Missing);
            };
            if !record.state.is_cancellable() {
                return Ok(StageCancelOutcome::Conflict(record.clone()));
            }
            record.state = StageState::Cancelled;
            state.clear_lease(&stage_run_id);
            state.notifications.remove(&stage_run_id);
            let record = state
                .runs
                .get(&stage_run_id)
                .cloned()
                .ok_or_else(|| StageStoreError("the stage vanished".to_owned()))?;
            Ok(StageCancelOutcome::Cancelled(record))
        })
    }

    fn delete(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageDeletion>, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            if !state
                .submissions
                .contains(&(profile.as_str().to_owned(), stage_run_id.clone()))
            {
                return Ok(None);
            }
            let Some(record) = state.runs.remove(&stage_run_id) else {
                return Ok(None);
            };
            state.leases.remove(&stage_run_id);
            state.notifications.remove(&stage_run_id);
            state
                .submissions
                .retain(|(_, stage_run_id)| stage_run_id != &record.stage_run_id.to_string());
            Ok(Some(StageDeletion {
                artifact_hashes: artifact_hashes(&record),
                stage_run_id: record.stage_run_id,
            }))
        })
    }

    fn purge_expired(&self, cutoff: &str) -> BoxFuture<Result<StageExpiry, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        let cutoff = cutoff.to_owned();
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let expired: Vec<StageRecord> = state
                .runs
                .values()
                .filter(|record| record.submitted_at < cutoff)
                .cloned()
                .collect();
            let mut stage_run_ids = Vec::with_capacity(expired.len());
            let mut artifact_hashes = Vec::new();
            for record in &expired {
                let key = record.stage_run_id.to_string();
                stage_run_ids.push(record.stage_run_id.clone());
                artifact_hashes.extend(artifact_hashes_of(record));
                state.runs.remove(&key);
                state.leases.remove(&key);
                state.notifications.remove(&key);
            }
            stage_run_ids.sort();
            stage_run_ids.dedup();
            let expired_ids: std::collections::BTreeSet<String> =
                stage_run_ids.iter().map(ToString::to_string).collect();
            state
                .submissions
                .retain(|(_, stage_run_id)| !expired_ids.contains(stage_run_id));
            artifact_hashes.sort();
            artifact_hashes.dedup();
            Ok(StageExpiry {
                stage_run_ids,
                artifact_hashes,
            })
        })
    }

    fn artifact_referenced(&self, sha256: Digest) -> BoxFuture<Result<bool, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            Ok(state.runs.values().any(|record| {
                record
                    .artifact
                    .as_ref()
                    .is_some_and(|artifact| artifact.files.iter().any(|file| file.sha256 == sha256))
            }))
        })
    }
}

impl StageOutbox for MemoryStages {
    fn wanted(&self) -> BoxFuture<Result<Vec<StageRunId>, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            let mut wanted = Vec::new();
            for (key, record) in &state.runs {
                let unfinished = match record.state {
                    StageState::Queued => true,
                    StageState::Running => !state.lease_live(key),
                    _ => false,
                };
                if unfinished {
                    wanted.push(record.stage_run_id.clone());
                }
            }
            wanted.sort();
            Ok(wanted)
        })
    }

    fn ensure(
        &self,
        wanted: &[StageRunId],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        let lease_ttl = self.lease_ttl;
        let wanted = wanted.to_vec();
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            let mut outcome = EnsureOutcome::default();
            for stage in &wanted {
                let key = stage.to_string();
                if state.ensure_notification(&key) {
                    outcome.inserted = outcome.inserted.saturating_add(1);
                    continue;
                }
                let Some(notification) = state.notifications.get(&key) else {
                    continue;
                };
                if !notification.published {
                    continue;
                }
                let rearm = match mode {
                    ReconcileMode::Startup => true,
                    ReconcileMode::Periodic => {
                        let running = state
                            .runs
                            .get(&key)
                            .is_some_and(|record| record.state == StageState::Running);
                        running
                            && !state.lease_live(&key)
                            && notification
                                .published_at
                                .is_some_and(|at| at.elapsed() >= lease_ttl)
                    }
                };
                if rearm && let Some(notification) = state.notifications.get_mut(&key) {
                    notification.published = false;
                    notification.published_at = None;
                    notification.last_error = None;
                    outcome.rearmed = outcome.rearmed.saturating_add(1);
                }
            }
            Ok(outcome)
        })
    }

    fn pending(
        &self,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageOutboxNotification>, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            let mut rows: Vec<(String, i64, u32)> = state
                .notifications
                .iter()
                .filter(|(_, notification)| !notification.published)
                .map(|(key, notification)| {
                    (key.clone(), notification.event_id, notification.attempts)
                })
                .collect();
            rows.sort_by_key(|(_, event_id, _)| *event_id);
            rows.into_iter()
                .take(limit)
                .map(|(key, event_id, attempts)| {
                    Ok(StageOutboxNotification {
                        event_id,
                        stage_run_id: StageRunId(Digest::parse(&key).map_err(|error| {
                            OutboxError(format!("a stored stage run id is malformed: {error}"))
                        })?),
                        attempts,
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
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            let Some(notification) = state
                .notifications
                .values_mut()
                .find(|notification| notification.event_id == event_id)
            else {
                return Err(OutboxError(format!(
                    "stage outbox event {event_id} is unknown"
                )));
            };
            notification.published = true;
            notification.published_at = Some(Instant::now());
            notification.attempts = notification.attempts.saturating_add(1);
            notification.last_error = None;
            Ok(())
        })
    }

    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            let Some(notification) = state
                .notifications
                .values_mut()
                .find(|notification| notification.event_id == event_id)
            else {
                return Err(OutboxError(format!(
                    "stage outbox event {event_id} is unknown"
                )));
            };
            notification.attempts = notification.attempts.saturating_add(1);
            notification.last_error = Some(error);
            Ok(())
        })
    }

    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = inner
                .lock()
                .map_err(|_| OutboxError("the stage lock is poisoned".to_owned()))?;
            Ok(u64::try_from(
                state
                    .notifications
                    .values()
                    .filter(|notification| !notification.published)
                    .count(),
            )
            .unwrap_or(u64::MAX))
        })
    }
}

fn artifact_hashes(record: &StageRecord) -> Vec<Digest> {
    let mut hashes = artifact_hashes_of(record);
    hashes.sort();
    hashes.dedup();
    hashes
}

fn artifact_hashes_of(record: &StageRecord) -> Vec<Digest> {
    record
        .artifact
        .as_ref()
        .map(|artifact| {
            artifact
                .files
                .iter()
                .map(|file| file.sha256.clone())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
