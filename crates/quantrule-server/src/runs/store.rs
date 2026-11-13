//! Run storage behind one trait.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use quantrule_api::{
    Digest, ProfileId, Provenance, RunDetail, RunId, RunState, RunSummary, StageName,
    StageProgress, StageStatus,
};

use crate::runtime::adapters::BoxFuture;

/// A new submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRun {
    /// The content-derived id.
    pub run_id: RunId,
    /// The configuration it applies.
    pub config_id: String,
    /// The dataset it consumes.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model identity.
    pub model_version: String,
    /// The image that will execute it.
    pub image_digest: Digest,
    /// The first submitter.
    pub owner: String,
    /// The submission time, RFC 3339 UTC.
    pub submitted_at: String,
}

/// One stored run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    /// The run id.
    pub run_id: RunId,
    /// Its state.
    pub status: RunState,
    /// The first submitter.
    pub owner: String,
    /// The configuration id.
    pub config_id: String,
    /// The dataset consumed.
    pub data_version: Digest,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model identity.
    pub model_version: String,
    /// The image digest.
    pub image_digest: Digest,
    /// The submission time.
    pub submitted_at: String,
    /// Per-stage progress.
    pub stages: Vec<StageProgress>,
    /// The canonical digest once complete.
    pub digest: Option<Digest>,
    /// The failure, when failed.
    pub error: Option<String>,
}

impl RunRecord {
    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> RunSummary {
        RunSummary {
            run_id: self.run_id.clone(),
            status: self.status,
            config_id: self.config_id.clone(),
            data_version: self.data_version.clone(),
            submitted_at: self.submitted_at.clone(),
            elapsed_ms: None,
        }
    }

    /// The detail projection.
    #[must_use]
    pub fn detail(&self) -> RunDetail {
        RunDetail {
            run_id: self.run_id.clone(),
            status: self.status,
            owner: self.owner.clone(),
            stages: self.stages.clone(),
            provenance: Provenance {
                data_version: self.data_version.clone(),
                config_hash: self.config_hash.clone(),
                model_version: self.model_version.clone(),
                image_digest: self.image_digest.clone(),
            },
            artifacts: Vec::new(),
            digest: self.digest.clone(),
            error: self.error.clone(),
        }
    }

    fn queued(run: &NewRun) -> Self {
        Self {
            run_id: run.run_id.clone(),
            status: RunState::Queued,
            owner: run.owner.clone(),
            config_id: run.config_id.clone(),
            data_version: run.data_version.clone(),
            config_hash: run.config_hash.clone(),
            model_version: run.model_version.clone(),
            image_digest: run.image_digest.clone(),
            submitted_at: run.submitted_at.clone(),
            stages: [
                StageName::Data,
                StageName::Model,
                StageName::Backtest,
                StageName::Evidence,
            ]
            .into_iter()
            .map(|name| StageProgress {
                name,
                state: StageStatus::Pending,
                elapsed_ms: None,
            })
            .collect(),
            digest: None,
            error: None,
        }
    }
}

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

/// What a cancellation did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelOutcome {
    /// The run moved to `cancelled`.
    Cancelled(RunRecord),
    /// The run was already terminal.
    Conflict(RunRecord),
    /// This profile does not see the run.
    Missing,
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

    /// Read one run within a profile.
    fn get(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>>;

    /// Cancel a run within a profile.
    fn cancel(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<CancelOutcome, RunStoreError>>;
}

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<String, RunRecord>,
    submissions: BTreeSet<(String, String)>,
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
            state.submissions.insert((profile, key.clone()));
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
                        .contains(&(profile.to_owned(), (*key).clone()))
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
                .contains(&(profile.as_str().to_owned(), run_id.clone()))
            {
                return Ok(None);
            }
            Ok(state.runs.get(&run_id).cloned())
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
                .contains(&(profile.as_str().to_owned(), run_id.clone()))
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
