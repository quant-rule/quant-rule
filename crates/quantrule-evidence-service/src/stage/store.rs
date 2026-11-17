//! The diagnostic stage store and its shared vocabulary.
//!
//! The trait is the boundary the evidence routes (#851) call. The memory
//! implementation is the test double; the Postgres and SQLite implementations
//! own the two production backends. Every implementation enforces the same
//! rules: the id is content-derived, the input belongs to the kind, a stage is
//! profile-scoped through a submission row, and only a running stage can finish.

use quantrule_api::{
    Digest, ProfileId, StageArtifact, StageInput, StageKind, StageRunId, StageState,
};
use quantrule_runtime::adapters::BoxFuture;

pub use quantrule_service_api::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageRecord,
};

/// Why a stage write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageStoreError(pub String);

impl std::fmt::Display for StageStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for StageStoreError {}

/// What a stage submission did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageCreateOutcome {
    /// The stage was created and queued.
    Created(StageRecord),
    /// The identical stage already existed; this profile now points at it.
    Existing(StageRecord),
}

impl StageCreateOutcome {
    /// The record either way.
    #[must_use]
    pub fn record(&self) -> &StageRecord {
        match self {
            Self::Created(record) | Self::Existing(record) => record,
        }
    }

    /// Whether the stage was created.
    #[must_use]
    pub fn created(&self) -> bool {
        matches!(self, Self::Created(_))
    }
}

/// What an explicit stage deletion removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDeletion {
    /// The stage that was deleted.
    pub stage_run_id: StageRunId,
    /// The artifact hashes its rows referenced.
    pub artifact_hashes: Vec<Digest>,
}

/// What a retention pass removed from the stage tables.
///
/// The ids name the manifests to delete; the hashes are candidates whose blobs
/// are removed only when no remaining stage artifact references them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StageExpiry {
    /// The expired stage runs, in id order.
    pub stage_run_ids: Vec<StageRunId>,
    /// The artifact hashes the expired rows referenced.
    pub artifact_hashes: Vec<Digest>,
}

impl StageExpiry {
    /// Whether the pass removed nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stage_run_ids.is_empty()
    }
}

/// Where stage runs live.
///
/// A stage row is never visible through [`crate::runs::RunStore`]: the two
/// stores read different schemas and return different record types. A caller
/// cannot turn a stage into evidence by asking the run store for its id.
pub trait StageStore: Send + Sync {
    /// Submit one stage for a profile.
    fn create(
        &self,
        profile: ProfileId,
        run: NewStageRun,
    ) -> BoxFuture<Result<StageCreateOutcome, StageStoreError>>;

    /// List a profile's stages, newest first.
    fn list(
        &self,
        profile: ProfileId,
        kind: Option<StageKind>,
        state: Option<StageState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<quantrule_api::StageRunSummary>, StageStoreError>>;

    /// Read one stage within a profile.
    fn get(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>>;

    /// Read one stage without a profile, for the worker.
    fn get_any(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>>;

    /// Claim a delivery. A queued stage starts; a redelivered, uncomputed
    /// running stage is reclaimed; a succeeded stage is terminal.
    fn claim(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>>;

    /// Record the content-addressed artifact and mark the stage succeeded.
    fn complete(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>>;

    /// Mark a running stage failed with its error.
    fn fail(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>>;

    /// Cancel a queued or running stage within a profile.
    fn cancel(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<StageCancelOutcome, StageStoreError>>;

    /// Delete one stage within a profile and return its object references.
    ///
    /// The row, its submission, and its artifact rows are removed in one
    /// transaction. A run row is never named: the stage schema is separate.
    fn delete(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageDeletion>, StageStoreError>>;

    /// Delete every stage whose submission is older than `cutoff`.
    ///
    /// `cutoff` is an RFC 3339 UTC instant; the stored instants have one fixed
    /// spelling, so string order is time order. The stage tables are the only
    /// relations named.
    fn purge_expired(&self, cutoff: &str) -> BoxFuture<Result<StageExpiry, StageStoreError>>;

    /// Whether any remaining stage artifact references a hash.
    fn artifact_referenced(&self, sha256: Digest) -> BoxFuture<Result<bool, StageStoreError>>;
}

/// Reject a submission whose identity or input contradicts the contract.
pub(crate) fn validate_new_stage(run: &NewStageRun) -> Result<(), StageStoreError> {
    if !run.input.matches(run.kind) {
        return Err(StageStoreError(format!(
            "a {} stage cannot consume a {} input",
            run.kind.as_str(),
            run.input.label()
        )));
    }
    if !run.has_derived_id() {
        return Err(StageStoreError(
            "the stage run id is not the one its identity derives".to_owned(),
        ));
    }
    if run.kind == StageKind::Model
        && let StageInput::DataVersion { data_version } = &run.input
        && data_version != &run.provenance.data_version
    {
        return Err(StageStoreError(
            "a model stage must record the dataset it consumes".to_owned(),
        ));
    }
    Ok(())
}

/// Reject an artifact whose identity does not match its files.
pub(crate) fn validate_artifact(artifact: &StageArtifact) -> Result<(), StageStoreError> {
    if artifact.is_consistent() {
        Ok(())
    } else {
        Err(StageStoreError(
            "the stage artifact id does not match its files".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{Digest, StageProvenance};

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn provenance() -> StageProvenance {
        StageProvenance {
            data_version: digest(b"data"),
            config_hash: digest(b"config"),
            model_version: "M1.3".to_owned(),
            image_digest: digest(b"image"),
        }
    }

    fn run() -> NewStageRun {
        NewStageRun::new(
            StageKind::Model,
            "cfg-1".to_owned(),
            StageInput::DataVersion {
                data_version: digest(b"data"),
            },
            provenance(),
            "operator".to_owned(),
            "2026-11-17T00:00:00Z".to_owned(),
        )
    }

    #[test]
    fn a_consistent_submission_validates() {
        assert_eq!(validate_new_stage(&run()), Ok(()));
    }

    #[test]
    fn a_mismatched_input_is_refused() {
        let mut run = run();
        run.kind = StageKind::Strategy;
        assert!(validate_new_stage(&run).is_err());
    }

    #[test]
    fn a_forged_id_is_refused() {
        let mut run = run();
        run.stage_run_id = quantrule_api::StageRunId(digest(b"forged"));
        assert!(validate_new_stage(&run).is_err());
    }

    #[test]
    fn a_model_stage_that_changes_its_dataset_is_refused() {
        let mut run = run();
        run.provenance.data_version = digest(b"other");
        assert!(validate_new_stage(&run).is_err());
    }

    #[test]
    fn a_forged_artifact_is_refused() {
        let mut artifact = StageArtifact::from_files(vec![quantrule_api::ArtifactRef {
            name: "model.json".to_owned(),
            bytes: 2,
            sha256: digest(b"{}"),
        }])
        .expect("the artifact");
        artifact.artifact_id = digest(b"forged");
        assert!(validate_artifact(&artifact).is_err());
    }

    #[test]
    fn a_create_outcome_projects_its_record() {
        let record = StageRecord::queued(&run());
        let created = StageCreateOutcome::Created(record.clone());
        let existing = StageCreateOutcome::Existing(record);
        assert!(created.created());
        assert!(!existing.created());
        assert_eq!(created.record(), existing.record());
    }
}
