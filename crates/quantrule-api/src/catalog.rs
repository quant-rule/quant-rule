//! The dataset and configuration catalog.
//!
//! A catalog entry is metadata over content-addressed bytes. Two profiles may
//! point at the same object; neither duplicates it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// Where a configuration came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigOrigin {
    /// A file under the read-only repository mount.
    Repository,
    /// A file uploaded through the API.
    Upload,
}

/// A dataset in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetSummary {
    /// Content identity of the normalized candles.
    pub data_version: Digest,
    /// A human label, usually the source file name.
    pub name: String,
    /// Number of M1 candles.
    pub candles: u64,
    /// First open time, Unix seconds UTC.
    pub first_bar: u64,
    /// Last open time, Unix seconds UTC.
    pub last_bar: u64,
    /// Stored size in bytes.
    pub bytes: u64,
}

/// One row skipped by a tolerant ingest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedRow {
    /// One-based line number.
    pub line: u64,
    /// A stable reason code: `field_count`, `invalid`, `misaligned`,
    /// `duplicate`, or `out_of_order`.
    pub reason: String,
}

/// The quality report of a tolerant ingest.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct QualityReport {
    /// Data rows read, excluding the header and blank lines.
    pub rows_read: u64,
    /// Candles accepted.
    pub accepted: u64,
    /// Rows rejected.
    pub rejected: u64,
    /// Rows repeating the previous timestamp.
    pub duplicates: u64,
    /// Rows earlier than the previous timestamp.
    pub out_of_order: u64,
    /// Rows with a field-count, parse, or alignment failure.
    pub invalid: u64,
    /// The skipped rows, in file order.
    pub skipped: Vec<SkippedRow>,
}

/// A dataset record, including where its bytes live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetRecord {
    /// The list projection.
    pub summary: DatasetSummary,
    /// The object key inside the content-addressed store.
    pub object_key: String,
    /// The hash of the stored bytes.
    pub sha256: Digest,
    /// The tolerant quality report; absent for a strict upload.
    pub quality: Option<QualityReport>,
    /// Server-assigned creation time, RFC 3339 UTC.
    pub created_at: String,
}

/// A configuration in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSummary {
    /// Stable configuration id.
    pub config_id: String,
    /// A human label.
    pub name: String,
    /// Repository or upload.
    pub origin: ConfigOrigin,
    /// Whether the configuration passes schema and cross-config validation.
    pub valid: bool,
    /// The parser message when invalid.
    pub message: Option<String>,
    /// The canonical hash, present when valid.
    pub config_hash: Option<Digest>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> Digest {
        Digest::parse(&"b".repeat(64)).expect("fixture")
    }

    #[test]
    fn origins_serialize_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&ConfigOrigin::Repository).expect("encode"),
            "\"repository\""
        );
        assert_eq!(
            serde_json::to_string(&ConfigOrigin::Upload).expect("encode"),
            "\"upload\""
        );
    }

    #[test]
    fn a_record_round_trips() {
        let record = DatasetRecord {
            summary: DatasetSummary {
                data_version: digest(),
                name: "candles.csv".to_owned(),
                candles: 1_000,
                first_bar: 1_546_300_800,
                last_bar: 1_549_065_600,
                bytes: 65_536,
            },
            object_key: "candles/abc".to_owned(),
            sha256: digest(),
            quality: None,
            created_at: "2026-11-12T00:00:00Z".to_owned(),
        };
        let text = serde_json::to_string(&record).expect("encode");
        let back: DatasetRecord = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, record);
    }

    #[test]
    fn an_invalid_config_carries_a_message_and_no_hash() {
        let summary = ConfigSummary {
            config_id: "cfg-1".to_owned(),
            name: "run".to_owned(),
            origin: ConfigOrigin::Repository,
            valid: false,
            message: Some("portfolio.initial_cash must be positive".to_owned()),
            config_hash: None,
        };
        assert!(summary.message.is_some());
        assert!(summary.config_hash.is_none());
    }
}

/// One timeframe's bars and coverage in an inspection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeframeBars {
    /// Short code, such as `5m`.
    pub timeframe: String,
    /// Complete bars.
    pub bars: u64,
    /// Trailing buckets missing at least one M1 candle.
    pub partial_buckets: u64,
    /// First complete bar, Unix seconds UTC.
    pub first_bar: Option<u64>,
    /// Last complete bar, Unix seconds UTC.
    pub last_bar: Option<u64>,
}

/// A dataset's timeframe ladder, counted from its stored bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeframeInspection {
    /// Content identity.
    pub data_version: Digest,
    /// One entry per timeframe, in ascending order.
    pub timeframes: Vec<TimeframeBars>,
}

/// A run-config bundle uploaded as one body.
///
/// The keys are relative paths inside the bundle; `run.json` is required and
/// every referenced file must be present. No key may escape the bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UploadConfigRequest {
    /// A human label for the catalog row.
    pub name: String,
    /// Relative path to file body.
    pub files: BTreeMap<String, String>,
}
