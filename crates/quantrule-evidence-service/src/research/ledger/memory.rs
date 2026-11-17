//! The in-memory research ledger, for tests and a local process.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, ImprovementTier, InvalidationRecord, ProfileId,
    RunId,
};
use quantrule_runtime::adapters::BoxFuture;

use super::store::{CreateOutcome, ImprovementStore, ImprovementStoreError};

/// An in-memory research ledger.
#[derive(Debug, Clone, Default)]
pub struct MemoryImprovementStore {
    inner: Arc<Mutex<MemoryState>>,
}

#[derive(Debug, Default)]
struct MemoryState {
    runs: BTreeMap<(String, String), ImprovementRunDetail>,
    baselines: BTreeMap<(String, String), BaselineRecord>,
    invalidations: BTreeMap<(String, String), InvalidationRecord>,
}

impl MemoryImprovementStore {
    /// Add one historical baseline to the in-memory fixture.
    ///
    /// The trait has no append method: the committed registry is the human-owned
    /// source, and a store write is not exposed to a client.
    pub fn add_baseline(&self, profile: ProfileId, record: BaselineRecord) {
        if let Ok(mut state) = self.inner.lock() {
            state
                .baselines
                .entry((profile.as_str().to_owned(), record.baseline_id.clone()))
                .or_insert(record);
        }
    }

    /// Add one historical invalidation tombstone to the in-memory fixture.
    pub fn add_invalidation(&self, profile: ProfileId, record: InvalidationRecord) {
        if let Ok(mut state) = self.inner.lock() {
            state
                .invalidations
                .entry((profile.as_str().to_owned(), record.invalidation_id.clone()))
                .or_insert(record);
        }
    }
}

fn lock(
    inner: &Arc<Mutex<MemoryState>>,
) -> Result<std::sync::MutexGuard<'_, MemoryState>, ImprovementStoreError> {
    inner
        .lock()
        .map_err(|_| ImprovementStoreError("the research lock is poisoned".to_owned()))
}

impl ImprovementStore for MemoryImprovementStore {
    fn create(
        &self,
        profile: ProfileId,
        manifest: ImprovementManifest,
        created_at: String,
    ) -> BoxFuture<Result<CreateOutcome, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            manifest
                .validate()
                .map_err(|error| ImprovementStoreError(error.to_string()))?;
            let mut state = lock(&inner)?;
            let key = (profile.as_str().to_owned(), manifest.improvement_id.clone());
            if let Some(existing) = state.runs.get(&key) {
                if existing.manifest == manifest {
                    return Ok(CreateOutcome::Existing(existing.clone()));
                }
                return Err(ImprovementStoreError(
                    "the improvement id already names another manifest".to_owned(),
                ));
            }
            let detail = detail(&manifest, created_at);
            state.runs.insert(key, detail.clone());
            Ok(CreateOutcome::Created(detail))
        })
    }

    fn link_pipeline_run(
        &self,
        profile: ProfileId,
        improvement_id: String,
        pipeline_run_id: RunId,
        canonical_digest: Digest,
        submitted_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let mut state = lock(&inner)?;
            let key = (profile.as_str().to_owned(), improvement_id);
            let record = state.runs.get_mut(&key).ok_or_else(|| {
                ImprovementStoreError("the improvement record is unknown".to_owned())
            })?;
            if record.state != ImprovementRunState::Created {
                return Err(ImprovementStoreError(
                    "the pipeline run is already linked or decided".to_owned(),
                ));
            }
            record.pipeline_run_id = Some(pipeline_run_id);
            record.canonical_digest = Some(canonical_digest);
            record.state = ImprovementRunState::Submitted;
            record.submitted_at = Some(submitted_at);
            Ok(record.clone())
        })
    }

    fn decide(
        &self,
        profile: ProfileId,
        improvement_id: String,
        decision: ImprovementDecision,
        note: String,
        decided_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            if note.trim().is_empty() {
                return Err(ImprovementStoreError(
                    "a decision needs a non-empty note".to_owned(),
                ));
            }
            let mut state = lock(&inner)?;
            let key = (profile.as_str().to_owned(), improvement_id);
            let record = state.runs.get_mut(&key).ok_or_else(|| {
                ImprovementStoreError("the improvement record is unknown".to_owned())
            })?;
            if record.state != ImprovementRunState::Submitted {
                return Err(ImprovementStoreError(
                    "a decision needs a submitted pipeline run and can be recorded once".to_owned(),
                ));
            }
            record.state = ImprovementRunState::Decided;
            record.decision = Some(decision);
            record.decision_note = Some(note);
            record.decided_at = Some(decided_at);
            Ok(record.clone())
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        limit: usize,
    ) -> BoxFuture<Result<Vec<ImprovementRunSummary>, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            let mut rows: Vec<&ImprovementRunDetail> = state
                .runs
                .iter()
                .filter(|((owner, _), _)| owner == profile.as_str())
                .map(|(_, record)| record)
                .collect();
            rows.sort_by(|left, right| {
                right
                    .created_at
                    .cmp(&left.created_at)
                    .then_with(|| left.improvement_id.cmp(&right.improvement_id))
            });
            Ok(rows
                .into_iter()
                .take(limit)
                .map(ImprovementRunDetail::summary)
                .collect())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<Option<ImprovementRunDetail>, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            Ok(lock(&inner)?
                .runs
                .get(&(profile.as_str().to_owned(), improvement_id))
                .cloned())
        })
    }

    fn baselines(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<BaselineRecord>, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            let mut rows: Vec<BaselineRecord> = state
                .baselines
                .iter()
                .filter(|((owner, _), _)| owner == profile.as_str())
                .map(|(_, record)| record.clone())
                .collect();
            rows.sort_by(|left, right| {
                right
                    .accepted_on
                    .cmp(&left.accepted_on)
                    .then_with(|| left.baseline_id.cmp(&right.baseline_id))
            });
            Ok(rows)
        })
    }

    fn invalidations(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<InvalidationRecord>, ImprovementStoreError>> {
        let inner = Arc::clone(&self.inner);
        Box::pin(async move {
            let state = lock(&inner)?;
            let mut rows: Vec<InvalidationRecord> = state
                .invalidations
                .iter()
                .filter(|((owner, _), _)| owner == profile.as_str())
                .map(|(_, record)| record.clone())
                .collect();
            rows.sort_by(|left, right| {
                right
                    .date
                    .cmp(&left.date)
                    .then_with(|| left.invalidation_id.cmp(&right.invalidation_id))
            });
            Ok(rows)
        })
    }
}

fn detail(manifest: &ImprovementManifest, created_at: String) -> ImprovementRunDetail {
    ImprovementRunDetail {
        improvement_id: manifest.improvement_id.clone(),
        tier: manifest.tier,
        state: ImprovementRunState::Created,
        decision: None,
        decision_note: None,
        manifest: manifest.clone(),
        pipeline_run_id: None,
        canonical_digest: None,
        created_at,
        submitted_at: None,
        decided_at: None,
    }
}

/// The three accepted historical model baseline scopes.
#[must_use]
pub fn seeded_baselines() -> Vec<BaselineRecord> {
    [
        (
            "MODEL-2026-11-04-04..05",
            "1m, 4 ATR over 20 bars, BTCUSD/ETHUSD/XAUUSD; dev and validation pass",
        ),
        (
            "MODEL-2026-11-04-06..07",
            "1m, 5-bar compression range, next-bar boundary touch; dev and validation pass; second use of model_validation disclosed",
        ),
        (
            "MODEL-2026-11-04-10..11",
            "5m fresh-onset range; dev and validation pass; third use of model_validation disclosed",
        ),
    ]
    .into_iter()
    .map(|(id, scope)| BaselineRecord {
        baseline_id: id.to_owned(),
        tier: ImprovementTier::Model,
        run_id: Some(id.to_owned()),
        improvement_id: None,
        accepted_by: "Human".to_owned(),
        accepted_on: "2026-11-04".to_owned(),
        scope: scope.to_owned(),
        supersedes: None,
        created_at: "2026-11-04T00:00:00Z".to_owned(),
    })
    .collect()
}
