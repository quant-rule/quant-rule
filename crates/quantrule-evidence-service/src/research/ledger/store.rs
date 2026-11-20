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

/// One decision write, including the optional registry row.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionWrite {
    /// The human decision.
    pub decision: ImprovementDecision,
    /// The non-empty note.
    pub note: String,
    /// The UTC decision time.
    pub decided_at: String,
    /// The accepted baseline row, for an accepted decision.
    pub baseline: Option<BaselineRecord>,
    /// The invalidation tombstone, for an invalidated decision.
    pub invalidation: Option<InvalidationRecord>,
}

/// One profile's research record and the dataset it references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchDatasetReference {
    /// The profile that owns the record.
    pub profile: ProfileId,
    /// The pre-registration identity.
    pub improvement_id: String,
    /// The dataset the manifest names.
    pub data_version: Digest,
}

/// What a research archive did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchArchiveOutcome {
    /// The record is archived.
    Archived(ImprovementRunSummary),
    /// The profile holds no such record.
    Missing,
    /// The record was already archived.
    AlreadyArchived,
}

/// The receipt for an erased research record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchEraseReceipt {
    /// The pre-registration identity.
    pub improvement_id: String,
    /// The dataset reference to release.
    pub data_version: Digest,
}

/// What a research erase did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchEraseOutcome {
    /// The record was removed.
    Erased(ResearchEraseReceipt),
    /// The profile holds no such record.
    Missing,
    /// The record has not been archived.
    NotArchived,
    /// The record is decided and is evidence.
    Protected,
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

    /// Link one pipeline run once and move the record to `submitted`.
    ///
    /// The caller must pass a pipeline run id, not a stage artifact id. The
    /// digest is not known when a queued run is submitted, so it is linked
    /// separately by [`ImprovementStore::link_digest`].
    fn link_run(
        &self,
        profile: ProfileId,
        improvement_id: String,
        pipeline_run_id: RunId,
        submitted_at: String,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>>;

    /// Record the canonical digest once, after a pipeline run is linked.
    ///
    /// A digest may be linked while the record is `submitted` or `decided`, but
    /// it can never be changed once it is present.
    fn link_digest(
        &self,
        profile: ProfileId,
        improvement_id: String,
        canonical_digest: Digest,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>>;

    /// Record the human decision and its registry row in one transaction.
    ///
    /// An accepted decision requires a baseline row; an invalidated decision
    /// requires an invalidation row. The run update and the registry insert
    /// commit together, so a failure cannot leave a decision without its row.
    fn decide_with_registry(
        &self,
        profile: ProfileId,
        improvement_id: String,
        write: DecisionWrite,
    ) -> BoxFuture<Result<ImprovementRunDetail, ImprovementStoreError>>;

    /// Record the human decision once, after both the run and digest are linked.
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

    /// List active and archived records, newest first.
    fn list_including_archived(
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

    /// Archive one record. A decided record may be archived but not erased.
    fn archive(
        &self,
        profile: ProfileId,
        improvement_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<ResearchArchiveOutcome, ImprovementStoreError>>;

    /// Erase one archived, undecided record.
    fn erase(
        &self,
        profile: ProfileId,
        improvement_id: String,
    ) -> BoxFuture<Result<ResearchEraseOutcome, ImprovementStoreError>>;

    /// Every record and the dataset its manifest names, for reconciliation.
    fn submitted_datasets(
        &self,
    ) -> BoxFuture<Result<Vec<ResearchDatasetReference>, ImprovementStoreError>>;

    /// Append one accepted baseline and return the stored row.
    ///
    /// The row is inserted once. A duplicate id is a conflict; an existing row
    /// is never updated or deleted.
    fn add_baseline(
        &self,
        profile: ProfileId,
        record: BaselineRecord,
    ) -> BoxFuture<Result<BaselineRecord, ImprovementStoreError>>;

    /// Append one invalidation tombstone and return the stored row.
    ///
    /// The row is inserted once. A duplicate id is a conflict; an existing row
    /// is never updated or deleted.
    fn add_invalidation(
        &self,
        profile: ProfileId,
        record: InvalidationRecord,
    ) -> BoxFuture<Result<InvalidationRecord, ImprovementStoreError>>;

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

pub(super) fn validate_baseline(record: &BaselineRecord) -> Result<(), ImprovementStoreError> {
    for (field, value) in [
        ("run_id", &record.run_id),
        ("improvement_id", &record.improvement_id),
    ] {
        if let Some(value) = value
            && value.trim().is_empty()
        {
            return Err(ImprovementStoreError(format!(
                "a baseline {field} is empty"
            )));
        }
    }
    if record.run_id.is_none() && record.improvement_id.is_none() {
        return Err(ImprovementStoreError(
            "a baseline needs a run id or an improvement id".to_owned(),
        ));
    }
    if record.accepted_by.trim().is_empty() {
        return Err(ImprovementStoreError(
            "a baseline needs a non-empty accepted_by".to_owned(),
        ));
    }
    if record.scope.trim().is_empty() {
        return Err(ImprovementStoreError(
            "a baseline needs a non-empty scope".to_owned(),
        ));
    }
    Ok(())
}

pub(super) fn validate_invalidation(
    record: &InvalidationRecord,
) -> Result<(), ImprovementStoreError> {
    for (field, value) in [
        ("date", &record.date),
        ("commit", &record.commit),
        ("removed", &record.removed),
        ("reason", &record.reason),
    ] {
        if value.trim().is_empty() {
            return Err(ImprovementStoreError(format!(
                "an invalidation {field} is empty"
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_decision_registry(
    decision: ImprovementDecision,
    baseline: Option<&BaselineRecord>,
    invalidation: Option<&InvalidationRecord>,
) -> Result<(), ImprovementStoreError> {
    match decision {
        ImprovementDecision::Accepted => {
            let Some(baseline) = baseline else {
                return Err(ImprovementStoreError(
                    "an accepted decision needs a baseline row".to_owned(),
                ));
            };
            if invalidation.is_some() {
                return Err(ImprovementStoreError(
                    "an accepted decision cannot carry an invalidation row".to_owned(),
                ));
            }
            validate_baseline(baseline)
        }
        ImprovementDecision::Invalidated => {
            let Some(invalidation) = invalidation else {
                return Err(ImprovementStoreError(
                    "an invalidated decision needs an invalidation row".to_owned(),
                ));
            };
            if baseline.is_some() {
                return Err(ImprovementStoreError(
                    "an invalidated decision cannot carry a baseline row".to_owned(),
                ));
            }
            validate_invalidation(invalidation)
        }
        _ => {
            if baseline.is_some() || invalidation.is_some() {
                return Err(ImprovementStoreError(
                    "only an accepted or invalidated decision carries a registry row".to_owned(),
                ));
            }
            Ok(())
        }
    }
}
