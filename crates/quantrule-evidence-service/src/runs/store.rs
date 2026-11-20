//! Run storage behind one trait.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{ArtifactRef, Digest, ProfileId, RunId, RunState, RunSummary, StageProgress};

use quantrule_runtime::adapters::BoxFuture;

pub use quantrule_service_api::{MAX_ATTEMPTS, NewRun, RunRecord};
pub use quantrule_service_api::{
    RunCancelOutcome as CancelOutcome, RunClaimOutcome as ClaimOutcome,
};

/// Why a run write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStoreError(pub String);

impl std::fmt::Display for RunStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RunStoreError {}

/// What a submission did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubmitOutcome {
    /// The run was created and queued.
    Created(RunRecord),
    /// The identical run already existed; this profile now points at it.
    Existing(RunRecord),
}

impl SubmitOutcome {
    /// The record either way.
    #[must_use]
    pub fn record(&self) -> &RunRecord {
        match self {
            Self::Created(record) | Self::Existing(record) => record,
        }
    }

    /// Whether the run was created.
    #[must_use]
    pub fn created(&self) -> bool {
        matches!(self, Self::Created(_))
    }
}

/// One profile's reference to a run's dataset, for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunDatasetReference {
    /// The profile that sees the run.
    pub profile: ProfileId,
    /// The run identity.
    pub run_id: RunId,
    /// The dataset the run consumes.
    pub data_version: Digest,
}

/// What an archive did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunArchiveOutcome {
    /// The profile's view is archived.
    Archived(RunSummary),
    /// The profile does not see the run.
    Missing,
    /// The profile already archived it.
    AlreadyArchived,
}

/// The receipt for one erased run submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEraseReceipt {
    /// The run identity.
    pub run_id: RunId,
    /// The dataset reference to release.
    pub data_version: Digest,
    /// Artifact bytes whose blobs became unreferenced.
    pub bytes: u64,
    /// Blob hashes with no remaining run reference.
    pub blobs: Vec<Digest>,
    /// Whether the shared run row itself was removed.
    pub removed_run: bool,
}

/// What an erase did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEraseOutcome {
    /// The submission and possibly the run row were removed.
    Erased(RunEraseReceipt),
    /// The profile does not see the run.
    Missing,
    /// The profile has not archived the run.
    NotArchived,
    /// The run is evidence or still active.
    Protected(RunState),
}

/// Where runs live.
pub trait RunStore: Send + Sync {
    /// Submit one run for a profile.
    fn submit(
        &self,
        profile: ProfileId,
        run: NewRun,
    ) -> BoxFuture<Result<SubmitOutcome, RunStoreError>>;

    /// List a profile's runs, newest first.
    fn list(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>>;

    /// List a profile's runs including archived submissions, newest first.
    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>>;

    /// Read one run within a profile.
    fn get(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>>;

    /// Archive one profile's view of a run.
    fn archive(
        &self,
        profile: ProfileId,
        run_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<RunArchiveOutcome, RunStoreError>>;

    /// Erase one archived submission and, when it is the last one, the run.
    fn erase(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunEraseOutcome, RunStoreError>>;

    /// Every run a profile sees, for dataset-reference reconciliation.
    fn submitted_datasets(&self) -> BoxFuture<Result<Vec<RunDatasetReference>, RunStoreError>>;

    /// Cancel a run within a profile.
    fn cancel(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<CancelOutcome, RunStoreError>>;

    /// Read one run without a profile, for the worker.
    fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>>;

    /// Claim a delivery. A queued run starts; a redelivered, uncomputed
    /// `running` run is reclaimed; a computed run is left for evidence.
    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<ClaimOutcome, RunStoreError>>;

    /// Record the computed digest and artifacts without finishing the run.
    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;

    /// Mark a run succeeded with its artifacts and final digest.
    fn complete(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;

    /// Mark a run failed with its stages.
    fn fail(
        &self,
        run_id: String,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>>;
}

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<String, RunRecord>,
    submissions: BTreeMap<(String, String), Option<String>>,
}

/// An in-memory store, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryRuns {
    inner: Arc<Mutex<MemoryState>>,
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
            match existing {
                Some(record) => Ok(SubmitOutcome::Existing(record)),
                None => {
                    let record = RunRecord::queued(&run);
                    state.runs.insert(key, record.clone());
                    Ok(SubmitOutcome::Created(record))
                }
            }
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
            state.runs.remove(&run_id);
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
            Ok(CancelOutcome::Cancelled(record.clone()))
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
        redelivered: bool,
    ) -> BoxFuture<Result<ClaimOutcome, RunStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = inner
                .lock()
                .map_err(|_| RunStoreError("the run lock is poisoned".to_owned()))?;
            let Some(record) = state.runs.get_mut(&run_id) else {
                return Ok(ClaimOutcome::Missing);
            };
            if record.status == RunState::Queued {
                record.status = RunState::Running;
                record.attempts = 1;
                return Ok(ClaimOutcome::Claimed(record.clone()));
            }
            if record.status != RunState::Running {
                return Ok(ClaimOutcome::Terminal(record.clone()));
            }
            // A computed run waits for evidence; a live worker still holds the
            // delivery. Only a broker redelivery proves the holder is gone.
            if !redelivered || record.digest.is_some() {
                return Ok(ClaimOutcome::Running(record.clone()));
            }
            if record.attempts >= MAX_ATTEMPTS {
                return Ok(ClaimOutcome::Exhausted(record.clone()));
            }
            record.attempts += 1;
            Ok(ClaimOutcome::Claimed(record.clone()))
        })
    }

    fn computed(
        &self,
        run_id: String,
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
            if record.status == RunState::Running {
                record.digest = Some(digest);
                record.stages = stages;
                record.artifacts = artifacts;
            }
            Ok(record.clone())
        })
    }

    fn complete(
        &self,
        run_id: String,
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
            // Only a running run can finish. The Postgres store enforces this
            // with the `status = 'running'` predicate; the memory store must
            // agree or a test could accept a transition production rejects.
            if record.status != RunState::Running {
                return Ok(record.clone());
            }
            record.status = RunState::Succeeded;
            record.digest = Some(digest);
            record.stages = stages;
            record.artifacts = artifacts;
            record.error = None;
            Ok(record.clone())
        })
    }

    fn fail(
        &self,
        run_id: String,
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
            if record.status != RunState::Running {
                return Ok(record.clone());
            }
            record.status = RunState::Failed;
            record.error = Some(error);
            record.stages = stages;
            Ok(record.clone())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::RunId;

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
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
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    use quantrule_api::{RunId, StageName, StageStatus};

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
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
            store.claim(id.clone(), false).await.expect("claim"),
            ClaimOutcome::Claimed(_)
        ));
        assert!(matches!(
            store.claim(id.clone(), false).await.expect("claim"),
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

        let ClaimOutcome::Claimed(first) = store.claim(id.clone(), false).await.expect("first")
        else {
            panic!("the queued run was not claimed");
        };
        assert_eq!(first.attempts, 1);
        assert!(matches!(
            store.claim(id.clone(), false).await.expect("live"),
            ClaimOutcome::Running(_)
        ));
        let ClaimOutcome::Claimed(second) = store.claim(id.clone(), true).await.expect("second")
        else {
            panic!("the redelivery was not reclaimed");
        };
        assert_eq!(second.attempts, 2);
        let ClaimOutcome::Claimed(third) = store.claim(id.clone(), true).await.expect("third")
        else {
            panic!("the second redelivery was not reclaimed");
        };
        assert_eq!(third.attempts, 3);
        let ClaimOutcome::Exhausted(spent) = store.claim(id, true).await.expect("fourth") else {
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
        store.claim(id.clone(), false).await.expect("claim");
        store
            .computed(
                id.clone(),
                Digest::sha256(b"canonical"),
                Vec::new(),
                done_stages(),
            )
            .await
            .expect("computed");
        assert!(matches!(
            store.claim(id, true).await.expect("redelivery"),
            ClaimOutcome::Running(_)
        ));
    }

    #[tokio::test]
    async fn only_a_running_run_can_be_completed_or_failed() {
        let store = MemoryRuns::default();
        let run = store
            .submit(profile("default"), new_run())
            .await
            .expect("submit");
        let id = run.record().run_id.to_string();

        let record = store
            .complete(
                id.clone(),
                Digest::sha256(b"too-early"),
                Vec::new(),
                done_stages(),
            )
            .await
            .expect("complete");
        assert_eq!(record.status, RunState::Queued);
        assert_eq!(record.digest, None);

        let record = store
            .fail(id.clone(), "too early".to_owned(), done_stages())
            .await
            .expect("fail");
        assert_eq!(record.status, RunState::Queued);
        assert_eq!(record.error, None);

        store.claim(id.clone(), false).await.expect("claim");
        store
            .complete(
                id.clone(),
                Digest::sha256(b"canonical"),
                Vec::new(),
                done_stages(),
            )
            .await
            .expect("complete");
        let record = store
            .fail(id, "after success".to_owned(), done_stages())
            .await
            .expect("fail");
        assert_eq!(record.status, RunState::Succeeded);
        assert_eq!(record.error, None);
    }

    #[tokio::test]
    async fn a_completed_run_carries_its_artifacts_and_digest() {
        let store = MemoryRuns::default();
        let run = store
            .submit(profile("default"), new_run())
            .await
            .expect("submit");
        let id = run.record().run_id.to_string();
        store.claim(id.clone(), false).await.expect("claim");
        let digest = Digest::sha256(b"canonical");
        let artifact = ArtifactRef {
            name: "results.json".to_owned(),
            bytes: 32,
            sha256: Digest::sha256(b"results"),
        };
        let record = store
            .complete(
                id.clone(),
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
            store.claim(id, false).await.expect("claim"),
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
        store.claim(id.clone(), false).await.expect("claim");
        let record = store
            .fail(id, "model failed".to_owned(), done_stages())
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
            store.claim(id, false).await.expect("claim"),
            ClaimOutcome::Terminal(_)
        ));
    }
}
