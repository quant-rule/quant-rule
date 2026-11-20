//! The evidence service's operational status.
//!
//! Each retention sweep writes one snapshot here and the public
//! `/operations` route reads it. It holds no run, stage, or research row and is
//! shared by every profile: a cleanup is process metadata, not a profile
//! record. A poisoned lock returns the last written value rather than failing a
//! read that the Operations screen uses to diagnose the service.

use std::sync::{Arc, Mutex, MutexGuard};

use quantrule_api::{CleanupReport, OperationsReport};

/// The configured retention window and the last cleanup.
pub struct OperationsStatus {
    stage_retention_days: u64,
    stage_store: bool,
    last_cleanup: Mutex<Option<CleanupReport>>,
}

impl OperationsStatus {
    /// Build the status from the configuration and the selected stage store.
    #[must_use]
    pub fn new(stage_retention_days: u64, stage_store: bool) -> Arc<Self> {
        Arc::new(Self {
            stage_retention_days,
            stage_store,
            last_cleanup: Mutex::new(None),
        })
    }

    /// Record one cleanup sweep, successful or failed.
    pub fn record(&self, report: CleanupReport) {
        *lock(&self.last_cleanup) = Some(report);
    }

    /// Read the snapshot the route returns.
    #[must_use]
    pub fn report(&self) -> OperationsReport {
        OperationsReport {
            stage_retention_days: self.stage_retention_days,
            stage_store: self.stage_store,
            last_cleanup: lock(&self.last_cleanup).clone(),
        }
    }
}

fn lock<T>(value: &Mutex<T>) -> MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn before_the_first_cleanup_the_report_is_null() {
        let status = OperationsStatus::new(30, true);
        let report = status.report();
        assert_eq!(report.stage_retention_days, 30);
        assert!(report.stage_store);
        assert!(report.last_cleanup.is_none());
    }

    #[test]
    fn a_successful_cleanup_records_its_counts() {
        let status = OperationsStatus::new(7, true);
        status.record(CleanupReport {
            completed_utc: "2026-11-17T08:00:00Z".to_owned(),
            stages_removed: 2,
            objects_removed: 5,
            error: None,
        });
        let cleanup = status.report().last_cleanup.expect("a cleanup");
        assert_eq!(cleanup.completed_utc, "2026-11-17T08:00:00Z");
        assert_eq!(cleanup.stages_removed, 2);
        assert_eq!(cleanup.objects_removed, 5);
        assert_eq!(cleanup.error, None);
    }

    #[test]
    fn a_failed_cleanup_records_the_error() {
        let status = OperationsStatus::new(30, true);
        status.record(CleanupReport {
            completed_utc: "2026-11-17T08:00:00Z".to_owned(),
            stages_removed: 0,
            objects_removed: 0,
            error: Some("the stage store refused a statement".to_owned()),
        });
        let cleanup = status.report().last_cleanup.expect("a cleanup");
        assert_eq!(
            cleanup.error.as_deref(),
            Some("the stage store refused a statement")
        );
    }

    #[test]
    fn no_stage_store_reports_no_cleanup() {
        let status = OperationsStatus::new(30, false);
        let report = status.report();
        assert!(!report.stage_store);
        assert!(report.last_cleanup.is_none());
    }
}
