//! The stored-row shapes the catalog repositories decode.
//!
//! Postgres and SQLite select the same columns in the same order, so the tuple
//! and its decoder live here once. The position rule is the whole point: a
//! swap makes a catalog read parse a timestamp as a quality report, and the
//! regression test in `tests/d1.rs` fails loudly rather than silently. A
//! repository that changes its select order changes this file, so the two
//! dialects cannot drift apart without touching one definition.

use quantrule_api::{ConfigOrigin, Digest};

use crate::catalog::CatalogError;
use crate::configs::{ConfigRecord, ConfigStoreError};

/// The columns of one catalog row, in select order.
///
/// `quality` is nullable, so it must be the last position and typed as an
/// option. The timestamp is non-null and precedes it.
pub(crate) type DatasetRow = (
    String,
    String,
    i64,
    i64,
    i64,
    i64,
    String,
    String,
    String,
    Option<String>,
);

/// The columns of one config row, in select order.
pub(crate) type ConfigRow = (
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
);

/// Convert a stored count to an unsigned one.
pub(crate) fn dataset_count(value: i64, field: &str) -> Result<u64, CatalogError> {
    u64::try_from(value)
        .map_err(|_| CatalogError::Store(format!("the catalog holds a negative {field}")))
}

/// Decode one catalog row.
pub(crate) fn dataset_record(
    row: DatasetRow,
) -> Result<quantrule_api::DatasetRecord, CatalogError> {
    let summary = quantrule_api::DatasetSummary {
        data_version: Digest::parse(&row.0).map_err(|error| {
            CatalogError::Store(format!("a stored version is malformed: {error}"))
        })?,
        name: row.1,
        candles: dataset_count(row.2, "candle count")?,
        first_bar: dataset_count(row.3, "first bar")?,
        last_bar: dataset_count(row.4, "last bar")?,
        bytes: dataset_count(row.5, "byte count")?,
    };
    Ok(quantrule_api::DatasetRecord {
        summary,
        object_key: row.6,
        sha256: Digest::parse(&row.7)
            .map_err(|error| CatalogError::Store(format!("a stored hash is malformed: {error}")))?,
        quality: row
            .9
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|error| {
                CatalogError::Store(format!("a stored quality report is malformed: {error}"))
            })?,
        created_at: row.8,
    })
}

/// Decode the stored origin.
pub(crate) fn config_origin(value: &str) -> Result<ConfigOrigin, ConfigStoreError> {
    match value {
        "repository" => Ok(ConfigOrigin::Repository),
        "upload" => Ok(ConfigOrigin::Upload),
        other => Err(ConfigStoreError(format!(
            "a stored config has an unknown origin: {other}"
        ))),
    }
}

/// Decode one config row.
pub(crate) fn config_record(row: ConfigRow) -> Result<ConfigRecord, ConfigStoreError> {
    let config_hash = row
        .5
        .map(|value| Digest::parse(&value))
        .transpose()
        .map_err(|error| ConfigStoreError(format!("a stored config hash is malformed: {error}")))?;
    Ok(ConfigRecord {
        config_id: row.0,
        name: row.1,
        origin: config_origin(&row.2)?,
        valid: row.3,
        message: row.4,
        config_hash,
        body: row.6,
        files: serde_json::from_str(&row.7).map_err(|error| {
            ConfigStoreError(format!("stored config files are malformed: {error}"))
        })?,
        created_at: row.8,
    })
}
