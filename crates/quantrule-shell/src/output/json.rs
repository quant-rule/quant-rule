//! Canonical JSON lines and value formatting.
//!
//! The generic writers keep their byte behavior: one canonical line per value,
//! and an array or an events/marks envelope unwrapped into one line per record.

use std::io::Write;

use serde_json::Value;

use crate::error::RunError;

/// Print one JSON value as a single canonical line.
///
/// # Errors
///
/// Returns [`RunError::Output`] when the writer fails.
pub(crate) fn write_json_line(writer: &mut dyn Write, value: &Value) -> Result<(), RunError> {
    let text = serde_json::to_string(value).map_err(|error| RunError::Decode {
        path: String::from("the local JSON view"),
        message: error.to_string(),
    })?;
    writeln!(writer, "{text}").map_err(RunError::Output)
}

/// Print an array as one JSON line per element, or one object as one line.
///
/// A response that wraps its records in an `events` or `marks` array is
/// unwrapped so a caller reads the records, not the envelope.
///
/// # Errors
///
/// Returns the same failures as [`write_json_line`].
pub(crate) fn write_records(writer: &mut dyn Write, value: &Value) -> Result<(), RunError> {
    match value {
        Value::Array(items) => {
            for item in items {
                write_json_line(writer, item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            let records = map
                .get("events")
                .or_else(|| map.get("marks"))
                .and_then(Value::as_array);
            if let Some(items) = records {
                for item in items {
                    write_json_line(writer, item)?;
                }
                Ok(())
            } else {
                write_json_line(writer, value)
            }
        }
        _ => write_json_line(writer, value),
    }
}
pub(super) fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::from("none"),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
