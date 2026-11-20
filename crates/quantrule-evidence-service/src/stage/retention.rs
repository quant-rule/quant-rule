//! Stage retention and explicit cleanup.
//!
//! A diagnostic stage is temporary. This module deletes only the `stage`
//! schema rows and the stage object namespace; it never names a run, a research
//! record, or a run artifact key. The cutoff is injected as an RFC 3339 UTC
//! instant, so a test expires a stage without sleeping and a service reads the
//! clock once at its boundary.
//!
//! The manifest is written last and is the visible edge of a stage bundle. A
//! blob is removed only when no remaining stage artifact references its hash,
//! and a run blob lives under a different prefix, so a shared digest is never a
//! reason to delete a run artifact.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use quantrule_api::CleanupReport;
use quantrule_runtime::adapters::{ObjectStore, StoreError};
use quantrule_runtime::clock;
use quantrule_service_api::StageManifest;

use crate::operations::OperationsStatus;
use crate::stage::{StageDeletion, StageExpiry, StageStore, StageStoreError};

/// Why stage retention could not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionError {
    /// A clock reading was before the Unix epoch.
    Clock(String),
    /// The stage store refused a statement.
    Store(String),
    /// The object store refused a delete.
    Object(String),
}

impl std::fmt::Display for RetentionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clock(message) | Self::Store(message) | Self::Object(message) => {
                formatter.write_str(message)
            }
        }
    }
}

impl std::error::Error for RetentionError {}

/// The interval between stage-retention sweeps.
pub const SWEEP_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// What one cleanup pass removed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetentionReport {
    /// How many stage rows were deleted.
    pub stages: usize,
    /// How many object keys were deleted.
    pub objects: usize,
}

/// The RFC 3339 UTC cutoff for a retention window.
///
/// The stored submission instant and the cutoff use one fixed spelling, so the
/// store compares them as text. A window of zero days is refused by the
/// configuration reader, so the subtraction is always meaningful.
///
/// # Errors
///
/// Returns [`RetentionError::Clock`] when `now` precedes the Unix epoch.
pub fn cutoff(now: SystemTime, days: u64) -> Result<String, RetentionError> {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_err(|error| RetentionError::Clock(error.to_string()))?
        .as_secs();
    let retained = days.saturating_mul(86_400);
    Ok(clock::rfc3339(seconds.saturating_sub(retained)))
}

/// Delete every stage older than `cutoff` and its unreferenced blobs.
///
/// # Errors
///
/// Returns [`RetentionError`] when the stage store or the object store fails.
pub async fn purge_expired(
    stages: &dyn StageStore,
    objects: &dyn ObjectStore,
    cutoff: &str,
) -> Result<RetentionReport, RetentionError> {
    let expiry = stages.purge_expired(cutoff).await.map_err(store_error)?;
    let objects = remove_expiry(stages, objects, &expiry).await?;
    Ok(RetentionReport {
        stages: expiry.stage_run_ids.len(),
        objects,
    })
}

/// Sweep expired stages now, then on every `period` until the task is dropped.
///
/// The first tick is immediate, so a restart still performs the startup pass.
/// The period is clamped to at least 1 ms so a zero value cannot panic the
/// interval constructor.
pub async fn sweep(
    stages: Arc<dyn StageStore>,
    objects: Arc<dyn ObjectStore>,
    operations: Arc<OperationsStatus>,
    days: u64,
    period: Duration,
) {
    let mut interval = tokio::time::interval(period.max(Duration::from_millis(1)));
    loop {
        interval.tick().await;
        let report = run_cleanup(stages.as_ref(), objects.as_ref(), SystemTime::now(), days).await;
        if let Some(error) = report.error.as_deref() {
            tracing::warn!(
                %error,
                completed_utc = %report.completed_utc,
                "stage retention did not finish"
            );
        } else {
            tracing::info!(
                stages = report.stages_removed,
                objects = report.objects_removed,
                days,
                completed_utc = %report.completed_utc,
                "expired stage runs removed"
            );
        }
        operations.record(report);
    }
}

/// Run one cleanup pass and describe it for the operations route.
///
/// The cleanup instant is read once from the injected clock. A failed cutoff or
/// purge still returns a report with the error, so the caller records a visible
/// failure instead of leaving the previous status in place.
pub async fn run_cleanup(
    stages: &dyn StageStore,
    objects: &dyn ObjectStore,
    now: SystemTime,
    days: u64,
) -> CleanupReport {
    let completed_utc = now.duration_since(UNIX_EPOCH).map_or_else(
        |_| clock::rfc3339(0),
        |duration| clock::rfc3339(duration.as_secs()),
    );
    let result = match cutoff(now, days) {
        Ok(cutoff) => purge_expired(stages, objects, &cutoff).await,
        Err(error) => Err(error),
    };
    match result {
        Ok(report) => CleanupReport {
            completed_utc,
            stages_removed: report.stages,
            objects_removed: report.objects,
            error: None,
        },
        Err(error) => CleanupReport {
            completed_utc,
            stages_removed: 0,
            objects_removed: 0,
            error: Some(error.to_string()),
        },
    }
}

/// Delete the objects of a stage row that was already deleted explicitly.
///
/// # Errors
///
/// Returns [`RetentionError`] when the stage store or the object store fails.
pub async fn remove_deleted(
    stages: &dyn StageStore,
    objects: &dyn ObjectStore,
    deletion: &StageDeletion,
) -> Result<usize, RetentionError> {
    let expiry = StageExpiry {
        stage_run_ids: vec![deletion.stage_run_id.clone()],
        artifact_hashes: deletion.artifact_hashes.clone(),
    };
    remove_expiry(stages, objects, &expiry).await
}

async fn remove_expiry(
    stages: &dyn StageStore,
    objects: &dyn ObjectStore,
    expiry: &StageExpiry,
) -> Result<usize, RetentionError> {
    let mut removed = 0;
    for stage_run_id in &expiry.stage_run_ids {
        objects
            .delete(&StageManifest::key(stage_run_id))
            .await
            .map_err(object_error)?;
        removed += 1;
    }
    for sha256 in &expiry.artifact_hashes {
        if !stages
            .artifact_referenced(sha256.clone())
            .await
            .map_err(store_error)?
        {
            objects
                .delete(&StageManifest::blob_key(sha256))
                .await
                .map_err(object_error)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn store_error(error: StageStoreError) -> RetentionError {
    RetentionError::Store(error.to_string())
}

fn object_error(error: StoreError) -> RetentionError {
    RetentionError::Object(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn the_cutoff_is_the_retention_window_before_now() {
        let now = UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        assert_eq!(
            cutoff(now, 30).expect("cutoff"),
            clock::rfc3339(1_800_000_000 - 30 * 86_400)
        );
        assert_eq!(
            cutoff(UNIX_EPOCH + Duration::from_secs(10), 30).expect("past cutoff"),
            "1970-01-01T00:00:00Z"
        );
    }

    #[test]
    fn a_pre_epoch_clock_is_refused() {
        let before = UNIX_EPOCH - Duration::from_secs(1);
        assert!(matches!(cutoff(before, 1), Err(RetentionError::Clock(_))));
    }
}
