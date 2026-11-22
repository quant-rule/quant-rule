//! Dataset-reference release tombstones.
//!
//! A consumer registers a reference in the data service before it writes its
//! row, and releases it before it erases that row. The two databases cannot join
//! or share a transaction, so the release is recorded in the evidence database
//! and retried by the reconciler. The row survives the consumer, which makes the
//! retry idempotent and keeps a crash from leaving a reference that blocks a
//! dataset erase forever.
//!
//! This is not a general saga: the record names one reference, one dataset, and
//! one action. There is no compensation step and no cross-database write.

use quantrule_runtime::adapters::BoxFuture;

use crate::runs::DatasetReferenceRelease;

/// Why a release tombstone could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseLogError(pub String);

impl std::fmt::Display for ReleaseLogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ReleaseLogError {}

/// Where dataset-reference release tombstones live.
///
/// The record is written in the same transaction as the consumer delete and
/// kept after it. `recorded` marks the release confirmed, or records the error
/// and leaves it pending for the next pass.
pub trait ReferenceReleaseLog: Send + Sync {
    /// Record a release intent. A repeated record is idempotent.
    fn record(&self, release: DatasetReferenceRelease) -> BoxFuture<Result<(), ReleaseLogError>>;

    /// The releases the reconciler has not confirmed.
    fn pending_releases(&self) -> BoxFuture<Result<Vec<DatasetReferenceRelease>, ReleaseLogError>>;

    /// Record one release attempt. `None` means the data service confirmed it.
    fn recorded(
        &self,
        release: &DatasetReferenceRelease,
        error: Option<&str>,
    ) -> BoxFuture<Result<(), ReleaseLogError>>;
}
