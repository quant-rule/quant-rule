//! The inspect command handlers.

use std::io::Write;

use quantrule_api::{DatasetSummary, TimeframeInspection};
use serde_json::Value;

use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn inspect_candles(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/inspect/candles?data_version={}", encode(data_version));
    let response = client.get(&path)?;
    let summary: DatasetSummary = decode_json!(DatasetSummary, response, &path);
    output::emit(globals.json, json_view!(&summary), stdout, |writer| {
        output::dataset_summary(writer, &summary)
    })
}

pub(super) fn inspect_timeframes(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/inspect/timeframes?data_version={}", encode(data_version));
    let response = client.get(&path)?;
    let inspection: TimeframeInspection = decode_json!(TimeframeInspection, response, &path);
    output::emit(globals.json, json_view!(&inspection), stdout, |writer| {
        output::timeframes(writer, &inspection)
    })
}

pub(super) fn inspect_events(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
    timeframe: &str,
) -> Result<(), RunError> {
    let path = format!(
        "/inspect/events?data_version={}&timeframe={}",
        encode(data_version),
        encode(timeframe)
    );
    let response = client.get(&path)?;
    let events: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, events.clone(), stdout, |writer| {
        output::write_records(writer, &events)
    })
}
