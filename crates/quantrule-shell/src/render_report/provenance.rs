//! The shared provenance record for a report bundle.
//!
//! Provenance names the configuration and the data version that produced the
//! bundle, so a later run can be shown to have used the same inputs. It records
//! the config file names as written, never a resolved absolute path.

use std::io;

use serde_json::{Value, json};

/// The report contract version carried by every new bundle.
pub(crate) const REPORT_VERSION: &str = "2026-10-28";

/// One report's provenance values.
pub(crate) struct Provenance<'a> {
    /// The report name, such as `model`.
    pub report: &'a str,
    /// The data version supplied by the run configuration.
    pub data_version: &'a str,
    /// The stable fingerprint of the config files.
    pub config_hash: &'a str,
    /// The requested lower date, as written.
    pub from: Option<&'a str>,
    /// The requested upper date, as written.
    pub to: Option<&'a str>,
    /// The first actual chart bar's open time.
    pub first_open_time: Option<i64>,
    /// The last actual chart bar's open time.
    pub last_open_time: Option<i64>,
    /// The config file names, as written.
    pub config_files: &'a [String],
    /// The resolved engine configuration values.
    pub config: Value,
    /// The bundle's artifact names, in the contract's order.
    pub artifacts: &'a [String],
}

/// Renders `provenance.json` as compact JSON with a trailing newline.
///
/// # Errors
///
/// Returns the serializer's I/O error when the value cannot be encoded.
pub(crate) fn bytes(provenance: &Provenance<'_>) -> io::Result<Vec<u8>> {
    let value = json!({
        "schema": "quantrule.provenance/v1",
        "report": provenance.report,
        "report_version": REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": provenance.data_version,
        "config_hash": provenance.config_hash,
        "requested_period": {
            "from": provenance.from,
            "to": provenance.to,
        },
        "actual_period": {
            "first_open_time": provenance.first_open_time,
            "last_open_time": provenance.last_open_time,
        },
        "config_files": provenance.config_files,
        "config": provenance.config,
        "artifacts": provenance.artifacts,
    });
    let text = serde_json::to_string(&value).map_err(io::Error::other)?;
    Ok(format!("{text}\n").into_bytes())
}
