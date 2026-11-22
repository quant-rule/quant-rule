//! Run lineage: the exact research artifacts a run cites.
//!
//! A run v5 names a model artifact, a strategy pack, a signal artifact, a
//! portfolio policy and an execution model by id and hash. Evidence stores that
//! set in its own schema; it records what the run cited, not a copy of the pack.
//! The set is immutable: a repeated record with the same set is idempotent, and
//! a different set for the same run is refused. Deleting a run cascades to its
//! lineage; the catalog rows and object bytes are the data service's records.
//!
//! This store does not call the data service. Registering a signal artifact's
//! consumer reference is the signal writer's job when that writer lands; the
//! generic reference and retention seam already exists in the catalog.

use quantrule_api::{ProfileId, RunArtifactReference, RunId, RunLineage};
use quantrule_runtime::adapters::BoxFuture;

/// Why a lineage read or write failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineageError {
    /// The run does not exist for this profile.
    Missing,
    /// The run already recorded a different reference set.
    Conflict,
    /// The reference set is malformed.
    Invalid(String),
    /// The store refused the operation.
    Store(String),
}

impl std::fmt::Display for LineageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("the run is unknown"),
            Self::Conflict => {
                formatter.write_str("the run already records a different reference set")
            }
            Self::Invalid(message) => formatter.write_str(message),
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for LineageError {}

/// Where a run's research artifact lineage lives.
pub trait RunLineageStore: Send + Sync {
    /// Record a run's reference set, or return the identical existing set.
    fn record(
        &self,
        profile: ProfileId,
        run_id: RunId,
        references: Vec<RunArtifactReference>,
        recorded_at: String,
    ) -> BoxFuture<Result<RunLineage, LineageError>>;

    /// Read one run's lineage.
    fn get(
        &self,
        profile: ProfileId,
        run_id: RunId,
    ) -> BoxFuture<Result<Option<RunLineage>, LineageError>>;
}

/// Validate and order one reference set.
///
/// One kind appears at most once, each id and hash agree, and a pack carries
/// provenance exactly when its kind names a pack. The order is the fixed
/// reference order, so the record is stable across callers.
///
/// # Errors
///
/// Returns [`LineageError::Invalid`] when a reference is inconsistent, repeated
/// or carries the wrong provenance.
pub fn normalize(
    mut references: Vec<RunArtifactReference>,
) -> Result<Vec<RunArtifactReference>, LineageError> {
    references.sort_by_key(|reference| reference.kind);
    for (index, reference) in references.iter().enumerate() {
        if !reference.is_consistent() {
            return Err(LineageError::Invalid(format!(
                "{} names two different content addresses",
                reference.kind
            )));
        }
        if !reference.has_expected_provenance() {
            return Err(LineageError::Invalid(format!(
                "{} carries the wrong pack provenance",
                reference.kind
            )));
        }
        if index > 0 && references[index - 1].kind == reference.kind {
            return Err(LineageError::Invalid(format!(
                "{} is named more than once",
                reference.kind
            )));
        }
        if let Some(pack) = &reference.pack
            && (pack.name.trim().is_empty()
                || pack.version.trim().is_empty()
                || pack.owner.trim().is_empty())
        {
            return Err(LineageError::Invalid(
                "pack provenance needs a name, version and owner".to_owned(),
            ));
        }
    }
    Ok(references)
}

pub mod memory;
pub mod postgres;

pub use memory::MemoryLineage;
pub use postgres::PgLineage;
