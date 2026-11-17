//! The operations surface: stage retention and the last startup cleanup.
//!
//! This is operational metadata. It is not a model measurement, not an
//! artifact, and not an evidence record. The evidence service reports what its
//! startup cleanup did and how long a diagnostic stage is kept; the values are
//! shared by every profile and never enter a run or a canonical digest.

use serde::{Deserialize, Serialize};

/// One startup cleanup, successful or failed.
///
/// A failed cleanup still carries the instant the pass ran and the error that
/// stopped it, so the operations screen can show the failure rather than an
/// absent status. The removed counts are zero when the purge did not finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CleanupReport {
    /// When the cleanup pass ran, as an RFC 3339 UTC instant.
    pub completed_utc: String,
    /// How many diagnostic stage rows were removed.
    pub stages_removed: usize,
    /// How many stage object keys were removed.
    pub objects_removed: usize,
    /// The failure that stopped the cleanup, or `null` when it succeeded.
    pub error: Option<String>,
}

/// The evidence service's operational snapshot.
///
/// `last_cleanup` is `null` before the first startup cleanup and when no stage
/// store is configured. `stage_store` distinguishes the two cases without
/// inventing a cleanup result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationsReport {
    /// The configured diagnostic stage retention window in days.
    pub stage_retention_days: u64,
    /// Whether the service has a stage store to clean.
    pub stage_store: bool,
    /// The last startup cleanup, or `null`.
    pub last_cleanup: Option<CleanupReport>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_round_trips_with_a_cleanup() {
        let report = OperationsReport {
            stage_retention_days: 30,
            stage_store: true,
            last_cleanup: Some(CleanupReport {
                completed_utc: "2026-11-17T08:00:00Z".to_owned(),
                stages_removed: 3,
                objects_removed: 4,
                error: None,
            }),
        };
        let text = serde_json::to_string(&report).expect("encode");
        let back: OperationsReport = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, report);
    }

    #[test]
    fn no_cleanup_yet_is_null_not_a_zero_report() {
        let report = OperationsReport {
            stage_retention_days: 30,
            stage_store: true,
            last_cleanup: None,
        };
        let text = serde_json::to_string(&report).expect("encode");
        assert!(text.contains("\"last_cleanup\":null"), "{text}");
    }

    #[test]
    fn a_failed_cleanup_keeps_the_error_and_zero_counts() {
        let report = CleanupReport {
            completed_utc: "2026-11-17T08:00:00Z".to_owned(),
            stages_removed: 0,
            objects_removed: 0,
            error: Some("the object store refused a delete".to_owned()),
        };
        let text = serde_json::to_string(&report).expect("encode");
        let back: CleanupReport = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, report);
    }
}
