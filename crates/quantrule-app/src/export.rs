//! Deterministic export of the application-service run summary.
//!
//! The export is a plain JSON document with a versioned schema. It carries the
//! validated configuration identity and composition, not readings, decisions,
//! or prices. The same summary always produces the same bytes: keys are sorted
//! by the serializer's ordered map and every list keeps its ladder order.

use std::fs;
use std::io::Write;
use std::path::Path;

use serde_json::{Value, json};

use crate::error::AppError;
use crate::service::{ModelRunSummary, RunSummary};

/// The export schema version.
pub const RUN_SUMMARY_SCHEMA: &str = "quantrule.run-summary/v1";

/// Builds the ordered JSON value for one run summary.
#[must_use]
pub fn run_summary_json(summary: &RunSummary) -> Value {
    json!({
        "schema": RUN_SUMMARY_SCHEMA,
        "name": summary.name,
        "output_root": summary.output_root.to_string_lossy(),
        "config_hash": summary.config_hash,
        "model_id": summary.model_id,
        "execution_id": summary.execution_id,
        "portfolio_id": summary.portfolio_id,
        "instrument": summary.instrument,
        "data_version": summary.data_version,
        "data_source": summary.data_source.to_string_lossy(),
        "data_timeframes": summary.data_timeframes,
        "model_timeframes": summary.model_timeframes,
        "execution_fill_timeframe": summary.execution_fill_timeframe,
        "initial_cash": summary.initial_cash,
        "leverage": summary.leverage,
        "max_open_positions": summary.max_open_positions,
    })
}

/// Builds the deterministic model-count object stored with a run.
#[must_use]
pub fn model_summary_json(summary: &ModelRunSummary) -> Value {
    json!({
        "timeframes": summary
            .timeframes
            .iter()
            .map(|timeframe| json!({
                "timeframe_secs": timeframe.timeframe_secs,
                "bars": timeframe.bars,
                "events": timeframe.events,
                "levels": timeframe.levels,
            }))
            .collect::<Vec<_>>(),
        "events": summary.events,
    })
}

/// Writes one summary as pretty JSON followed by a newline.
///
/// # Errors
///
/// Returns the writer's error when the document cannot be written.
pub fn write_run_summary(summary: &RunSummary, writer: &mut impl Write) -> std::io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, &run_summary_json(summary))?;
    writer.write_all(b"\n")
}

/// Writes one summary to a file, replacing an existing file.
///
/// The parent directory is created when it is missing, so the default export
/// path from a run with a fresh output root is usable.
///
/// # Errors
///
/// Returns [`AppError::Io`] when the destination cannot be created or written.
pub fn write_run_summary_file(summary: &RunSummary, path: &Path) -> Result<(), AppError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| AppError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut file = fs::File::create(path).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    write_run_summary(summary, &mut file).map_err(|source| AppError::Io {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::service::{AppService, RunSummary};

    use super::{RUN_SUMMARY_SCHEMA, run_summary_json, write_run_summary, write_run_summary_file};

    fn fixture_run() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-shell/tests/fixtures/config-run/run.json")
    }

    fn summary() -> RunSummary {
        AppService::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .expect("the repository is a workspace")
            .check_run(fixture_run())
            .expect("the fixture run is valid")
    }

    #[test]
    fn the_export_carries_the_versioned_schema() {
        let value = run_summary_json(&summary());
        assert_eq!(value["schema"], RUN_SUMMARY_SCHEMA);
        assert_eq!(value["name"], "fixture");
        assert_eq!(value["instrument"], "EURUSD");
    }

    #[test]
    fn the_export_is_byte_deterministic() {
        let summary = summary();
        let mut first = Vec::new();
        let mut second = Vec::new();
        write_run_summary(&summary, &mut first).expect("the first export is written");
        write_run_summary(&summary, &mut second).expect("the second export is written");
        assert_eq!(first, second);
    }

    #[test]
    fn the_export_creates_a_missing_parent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/app-service-tests/export-parent");
        let _ = std::fs::remove_dir_all(&root);
        let path = root.join("nested/deeper/run-summary.json");
        write_run_summary_file(&summary(), &path).expect("the export is written");
        assert!(path.is_file());
        let text = std::fs::read_to_string(&path).expect("the export is readable");
        assert!(text.contains(RUN_SUMMARY_SCHEMA));
    }
}
