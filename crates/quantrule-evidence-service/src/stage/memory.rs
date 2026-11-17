//! The in-memory stage store.
//!
//! The memory store is the test double and the local-development backend. It
//! implements the same state machine as the Postgres and SQLite stores so a
//! test cannot accept a transition production rejects: a queued stage starts on
//! a claim, a redelivered uncomputed stage is reclaimed up to
//! [`MAX_ATTEMPTS`], and only a running stage can succeed or fail.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use quantrule_api::{Digest, ProfileId, StageArtifact, StageKind, StageRunSummary, StageState};
use quantrule_runtime::adapters::BoxFuture;

use super::store::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageCreateOutcome,
    StageDeletion, StageExpiry, StageRecord, StageStore, StageStoreError, validate_artifact,
    validate_new_stage,
};

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<String, StageRecord>,
    submissions: BTreeSet<(String, String)>,
}

/// An in-memory stage store, for tests and for a local instance.
#[derive(Debug, Clone, Default)]
pub struct MemoryStages {
    inner: Arc<Mutex<MemoryState>>,
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
                Some(record) => Ok(StageCreateOutcome::Existing(record)),
                None => {
                    let record = StageRecord::queued(&run);
                    state.runs.insert(key, record.clone());
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
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Ok(StageClaimOutcome::Missing);
            };
            if record.state == StageState::Queued {
                record.state = StageState::Running;
                record.attempts = 1;
                return Ok(StageClaimOutcome::Claimed(record.clone()));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record.clone()));
            }
            // A live worker still holds the delivery; only a broker
            // redelivery proves the holder is gone. A stage has no
            // "computed but not finalized" state: completion is atomic.
            if !redelivered {
                return Ok(StageClaimOutcome::Running(record.clone()));
            }
            if record.attempts >= MAX_ATTEMPTS {
                return Ok(StageClaimOutcome::Exhausted(record.clone()));
            }
            record.attempts += 1;
            Ok(StageClaimOutcome::Claimed(record.clone()))
        })
    }

    fn complete(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            validate_artifact(&artifact)?;
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
            };
            // Only a running stage can finish. The SQL stores enforce this
            // with a `status = 'running'` predicate; the memory store must
            // agree or a test could accept a transition production rejects.
            if record.state != StageState::Running {
                return Ok(record.clone());
            }
            record.state = StageState::Succeeded;
            record.artifact = Some(artifact);
            record.error = None;
            Ok(record.clone())
        })
    }

    fn fail(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let Some(record) = state.runs.get_mut(&stage_run_id) else {
                return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
            };
            if record.state != StageState::Running {
                return Ok(record.clone());
            }
            record.state = StageState::Failed;
            record.error = Some(error);
            Ok(record.clone())
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
            Ok(StageCancelOutcome::Cancelled(record.clone()))
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
                stage_run_ids.push(record.stage_run_id.clone());
                artifact_hashes.extend(artifact_hashes_of(record));
                state.runs.remove(&record.stage_run_id.to_string());
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
mod tests {
    use super::*;
    use quantrule_api::{ArtifactRef, Digest, StageInput, StageKind, StageProvenance};

    fn profile(name: &str) -> ProfileId {
        ProfileId::parse(name).expect("profile")
    }

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn provenance(seed: &[u8]) -> StageProvenance {
        StageProvenance {
            data_version: digest(seed),
            config_hash: digest(b"config"),
            model_version: "M1.3".to_owned(),
            image_digest: digest(b"image"),
        }
    }

    fn run(seed: &[u8]) -> NewStageRun {
        NewStageRun::new(
            StageKind::Model,
            "cfg-1".to_owned(),
            StageInput::DataVersion {
                data_version: digest(seed),
            },
            provenance(seed),
            "operator".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
    }

    fn artifact(name_a: &str, name_b: &str) -> StageArtifact {
        StageArtifact::from_files(vec![
            ArtifactRef {
                name: name_b.to_owned(),
                bytes: 2,
                sha256: digest(b"b"),
            },
            ArtifactRef {
                name: name_a.to_owned(),
                bytes: 1,
                sha256: digest(b"a"),
            },
        ])
        .expect("the artifact")
    }

    #[tokio::test]
    async fn a_submission_is_created_once() {
        let store = MemoryStages::default();
        let first = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("first");
        assert!(first.created());
        assert_eq!(first.record().state, StageState::Queued);
        let second = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("second");
        assert!(!second.created());
        assert_eq!(first.record().stage_run_id, second.record().stage_run_id);
        assert_eq!(
            store
                .list(profile("default"), None, None, 50)
                .await
                .expect("list")
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn two_profiles_share_one_stage_and_a_third_cannot_see_it() {
        let store = MemoryStages::default();
        store
            .create(profile("alpha"), run(b"shared"))
            .await
            .expect("alpha");
        store
            .create(profile("beta"), run(b"shared"))
            .await
            .expect("beta");
        let id = run(b"shared").stage_run_id.to_string();
        assert!(
            store
                .get(profile("alpha"), id.clone())
                .await
                .expect("alpha get")
                .is_some()
        );
        assert!(
            store
                .get(profile("beta"), id.clone())
                .await
                .expect("beta get")
                .is_some()
        );
        assert_eq!(
            store
                .get(profile("gamma"), id.clone())
                .await
                .expect("gamma"),
            None
        );
        assert_eq!(
            store
                .list(profile("gamma"), None, None, 50)
                .await
                .expect("gamma list")
                .len(),
            0
        );
        assert!(store.get_any(id).await.expect("worker read").is_some());
    }

    #[tokio::test]
    async fn a_claim_starts_once_and_a_redelivery_reclaims_until_the_budget_is_spent() {
        let store = MemoryStages::default();
        let submitted = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("submit");
        let id = submitted.record().stage_run_id.to_string();
        let StageClaimOutcome::Claimed(first) =
            store.claim(id.clone(), false).await.expect("first")
        else {
            panic!("the queued stage was not claimed");
        };
        assert_eq!(first.state, StageState::Running);
        assert_eq!(first.attempts, 1);
        assert!(matches!(
            store.claim(id.clone(), false).await.expect("live"),
            StageClaimOutcome::Running(_)
        ));
        let StageClaimOutcome::Claimed(second) =
            store.claim(id.clone(), true).await.expect("second")
        else {
            panic!("the redelivery was not reclaimed");
        };
        assert_eq!(second.attempts, 2);
        let StageClaimOutcome::Claimed(third) = store.claim(id.clone(), true).await.expect("third")
        else {
            panic!("the second redelivery was not reclaimed");
        };
        assert_eq!(third.attempts, 3);
        let StageClaimOutcome::Exhausted(spent) = store.claim(id, true).await.expect("fourth")
        else {
            panic!("the fourth delivery was not exhausted");
        };
        assert_eq!(spent.attempts, 3);
    }

    #[tokio::test]
    async fn only_a_running_stage_can_complete_or_fail() {
        let store = MemoryStages::default();
        let submitted = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("submit");
        let id = submitted.record().stage_run_id.to_string();
        let artifact = artifact("a.csv", "b.csv");
        let record = store
            .complete(id.clone(), artifact.clone())
            .await
            .expect("too early");
        assert_eq!(record.state, StageState::Queued);
        assert_eq!(record.artifact, None);
        let record = store
            .fail(id.clone(), "too early".to_owned())
            .await
            .expect("too early fail");
        assert_eq!(record.state, StageState::Queued);
        assert_eq!(record.error, None);

        store.claim(id.clone(), false).await.expect("claim");
        let record = store
            .complete(id.clone(), artifact.clone())
            .await
            .expect("complete");
        assert_eq!(record.state, StageState::Succeeded);
        assert_eq!(record.artifact, Some(artifact.clone()));
        assert_eq!(
            record
                .artifact
                .as_ref()
                .expect("artifact")
                .files
                .iter()
                .map(|file| file.name.as_str())
                .collect::<Vec<_>>(),
            vec!["a.csv", "b.csv"]
        );
        let record = store
            .fail(id, "after success".to_owned())
            .await
            .expect("late fail");
        assert_eq!(record.state, StageState::Succeeded);
        assert_eq!(record.error, None);
    }

    #[tokio::test]
    async fn a_failed_stage_records_its_error() {
        let store = MemoryStages::default();
        let submitted = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("submit");
        let id = submitted.record().stage_run_id.to_string();
        store.claim(id.clone(), false).await.expect("claim");
        let record = store
            .fail(id, "the model failed".to_owned())
            .await
            .expect("fail");
        assert_eq!(record.state, StageState::Failed);
        assert_eq!(record.error.as_deref(), Some("the model failed"));
    }

    #[tokio::test]
    async fn a_cancelled_stage_is_terminal_and_profile_scoped() {
        let store = MemoryStages::default();
        let submitted = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("submit");
        let id = submitted.record().stage_run_id.to_string();
        assert!(matches!(
            store
                .cancel(profile("other"), id.clone())
                .await
                .expect("other cancel"),
            StageCancelOutcome::Missing
        ));
        assert!(matches!(
            store
                .cancel(profile("default"), id.clone())
                .await
                .expect("cancel"),
            StageCancelOutcome::Cancelled(_)
        ));
        assert!(matches!(
            store
                .cancel(profile("default"), id.clone())
                .await
                .expect("second cancel"),
            StageCancelOutcome::Conflict(_)
        ));
        assert!(matches!(
            store.claim(id, false).await.expect("claim"),
            StageClaimOutcome::Terminal(_)
        ));
    }

    #[tokio::test]
    async fn a_forged_artifact_is_refused() {
        let store = MemoryStages::default();
        let submitted = store
            .create(profile("default"), run(b"one"))
            .await
            .expect("submit");
        let id = submitted.record().stage_run_id.to_string();
        store.claim(id.clone(), false).await.expect("claim");
        let mut forged = artifact("a.csv", "b.csv");
        forged.artifact_id = digest(b"forged");
        assert!(store.complete(id, forged).await.is_err());
    }
}
