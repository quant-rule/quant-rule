//! Stable printed output.
//!
//! The shell prints numbers and codes, never a label the gateway did not send.
//! Lists are sorted where the gateway's order is not part of the contract, so
//! the same response prints the same bytes. `--json` prints the gateway value
//! unchanged (canonical JSON) for the parity tests and for agents.

use std::io::{self, Write};

use serde_json::Value;

use crate::error::RunError;

/// One filtered model-stage event batch and the report coverage.
///
/// The type is presentation, not measurement: it carries the published CSV
/// columns and values and the four report coverage fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageEventsView {
    /// The stage run the artifact belongs to.
    pub(crate) stage_run_id: String,
    /// The timeframe short code the caller requested.
    pub(crate) timeframe: String,
    /// The CSV header, in artifact order.
    pub(crate) columns: Vec<String>,
    /// The event rows for the requested timeframe, in artifact order.
    pub(crate) events: Vec<std::collections::BTreeMap<String, String>>,
    /// The per-timeframe coverage from `report.json`.
    pub(crate) coverage: Vec<Value>,
}

impl StageEventsView {
    /// Build the canonical JSON value for `--json` and the parity fixture.
    pub(crate) fn to_value(&self) -> Value {
        serde_json::json!({
            "stage_run_id": self.stage_run_id,
            "timeframe": self.timeframe,
            "columns": self.columns,
            "events": self.events,
            "coverage": self.coverage,
        })
    }
}

/// Print one command result as text, or as JSON when `--json` is set.
///
/// # Errors
///
/// Returns [`RunError::Output`] when the writer fails, and
/// [`RunError::Decode`] when the local JSON view cannot be encoded.
pub(crate) fn emit(
    json: bool,
    value: Value,
    writer: &mut dyn Write,
    text: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> Result<(), RunError> {
    if json {
        write_json_line(writer, &value)
    } else {
        text(writer).map_err(RunError::Output)?;
        writer.flush().map_err(RunError::Output)
    }
}

/// Print one command result whose text renderer already returns [`RunError`].
///
/// # Errors
///
/// Returns the text renderer's failure, or [`RunError::Output`] when the writer
/// fails.
pub(crate) fn emit_raw(
    json: bool,
    value: Value,
    writer: &mut dyn Write,
    text: impl FnOnce(&mut dyn Write) -> Result<(), RunError>,
) -> Result<(), RunError> {
    if json {
        write_json_line(writer, &value)
    } else {
        text(writer)?;
        writer.flush().map_err(RunError::Output)
    }
}

mod config;
mod dataset;
mod health;
mod history;
mod inspect;
mod json;
mod profile;
mod research;
mod run;
mod simulation;
mod stage;
mod verify;

pub(crate) use self::config::{config_list, config_record};
pub(crate) use self::dataset::{dataset_list, dataset_record, dataset_summary};
pub(crate) use self::health::health;
pub(crate) use self::history::{archived, dry_run, erased, history};
pub(crate) use self::inspect::timeframes;
pub(crate) use self::json::{write_json_line, write_records};
pub(crate) use self::profile::{active_profile, profile, profiles};
pub(crate) use self::research::{
    baseline_detail, baseline_list, improvement_comparison, improvement_detail, improvement_list,
    invalidation_list,
};
pub(crate) use self::run::{fetched, run_detail, run_list, run_submit};
pub(crate) use self::simulation::{simulation_check, simulation_list};
pub(crate) use self::stage::{
    stage_deleted, stage_detail, stage_events, stage_explain, stage_fetched, stage_list,
    stage_submitted,
};
pub(crate) use self::verify::digest;
