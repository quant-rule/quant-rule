//! The research-ledger trait and shared errors.

use quantrule_api::{
    BaselineRecord, Digest, ImprovementDecision, ImprovementManifest, ImprovementRunDetail,
    ImprovementRunSummary, InvalidationRecord, ProfileId, RunId,
};
use quantrule_runtime::adapters::BoxFuture;

/// A research-ledger failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImprovementStoreError(pub String);

impl std::fmt::Display for ImprovementStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ImprovementStoreError {}

/// What a create did.
#[derive(Debug, Clone, PartialEq)]
pub enum CreateOutcome {
    /// A new pre-registration was written.
    Created(ImprovementRunDetail),
    /// The identical pre-registration already existed.
    Existing(ImprovementRunDetail),
}

impl CreateOutcome {
    /// The record either way.
    #[must_use]
    pub fn record(&self) -> &ImprovementRunDetail {
        match self {
            Self::Created(record) | Self::Existing(record) => record,
        }
    }

    /// Whether the record was created.
    #[must_use]
    pub fn created(&self) -> bool {
        matches!(self, Self::Created(_))
    }
}

/// Where research-ledger records live.
pub trait ImprovementStore: Send + Sync {
    /// Create one v3 pre-registration for a profile.
    fn create(
        &self,
        profile: ProfileId,
        manifest: ImprovementManifest,
        created_at: String,
    ) -> BoxFuture<Result<CreateOutcome, ImprovementStoreError>>;

    /// Link one pipeline run and its canonical digest once.
    ///
    /// The caller must pass a pipeline run id, not a stage artifact id. The
    /// evidence service owns that validation boundary.
    fn link_pipeline_run(
        &self,
        profile: ProfileId,
        improvement_id: String,
        pipeline_run_id: RunId,
        canonical_digest: Digest,
        submitted_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>>;

    /// Record the human decision once, with a non-empty note.
    fn decide(
        &self,
        profile: ProfileId,
        improvement_id: String,
        decision: ImprovementDecision,
        note: String,
        decided_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>>;

    /// List records newest first.
    fn list(
        &self,
        profile: ProfileId,
        limit: usize,
    ) -> BoxFuture<Result<Vec<ImprovementRunSummary>, ImprovementStoreError>>;

    /// Read one record.
    fn get(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<Option<ImprovementRunDetail>, ImprovementStoreError>>;

    /// List the accepted baselines for a profile.
    fn baselines(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<BaselineRecord>, ImprovementStoreError>>;

    /// List the append-only invalidation tombstones for a profile.
    fn invalidations(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<InvalidationRecord>, ImprovementStoreError>>;
}
