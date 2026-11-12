//! Rendering for the flat model event batch.
//!
//! The shell reads the model's published event records and maps their numeric
//! fields to the command's output schema. It does not detect an event,
//! aggregate a timeframe, or compute an indicator.

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use quantrule_model::events::EventRecord;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::TimeframeModel;
use serde_json::{Value, json};

use quantrule_app::artifact::labels;

/// Writes the full event batch, grouped by timeframe and availability order.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn write_event_batch(run: &ModelRun, writer: &mut impl Write) -> io::Result<()> {
    for timeframe in run.timeframes() {
        for event in timeframe.all_events() {
            write_line(&event_value(timeframe, event), writer)?;
        }
    }
    Ok(())
}

fn write_line(value: &Value, writer: &mut impl Write) -> io::Result<()> {
    let text = serde_json::to_string(value).map_err(io::Error::other)?;
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

fn event_value(timeframe: &TimeframeModel, event: &EventRecord) -> Value {
    let occurred_time = timeframe
        .candle(event.occurred_bar_index)
        .map(|candle| format_utc_timestamp(candle.timestamp()));
    let available_time = timeframe
        .candle(event.available_bar_index)
        .map(|candle| format_utc_timestamp(candle.timestamp()));
    json!({
        "timeframe": labels::timeframe_label(timeframe.timeframe_secs()),
        "type": event.event_type.code(),
        "type_label": labels::event_type_label(event.event_type.code()),
        "subtype": event.event_subtype_code,
        "subtype_label": labels::event_subtype_label(event.event_type.code(), event.event_subtype_code),
        "source": event.source_code,
        "source_label": labels::source_label(event.source_code),
        "occurred_bar": event.occurred_bar_index,
        "occurred_time": occurred_time,
        "available_bar": event.available_bar_index,
        "available_time": available_time,
        "direction": event.direction_code,
        "reference_code": event.reference_code,
        "primary_value": event.primary_value,
        "reference_value": event.reference_value,
    })
}
