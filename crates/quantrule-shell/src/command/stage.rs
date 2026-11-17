//! Model-stage artifact rendering.
//!
//! A model stage publishes plain artifacts: `events.csv` and `report.json`. The
//! shell reads those published bytes and filters or formats them; it computes no
//! indicator, event, or coverage value. The parser is deterministic and refuses
//! a column shape it does not recognise rather than guessing.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::error::RunError;
use crate::output::StageEventsView;

/// Parse `events.csv` and `report.json` into one deterministic view.
///
/// # Errors
///
/// Returns [`RunError::Decode`] when the CSV has no event header, a row has the
/// wrong field count, or `report.json` is not the model report.
pub(crate) fn stage_events(
    stage_run_id: &str,
    timeframe: &str,
    events_csv: &[u8],
    report_json: Option<&[u8]>,
) -> Result<StageEventsView, RunError> {
    let seconds = timeframe_secs(timeframe)?;
    let text = std::str::from_utf8(events_csv).map_err(|error| RunError::Decode {
        path: String::from("events.csv"),
        message: error.to_string(),
    })?;
    let mut lines = text.lines().filter(|line| !line.is_empty());
    let header = lines.next().ok_or_else(|| RunError::Decode {
        path: String::from("events.csv"),
        message: String::from("the event artifact carries no header"),
    })?;
    let columns: Vec<String> = header
        .trim_end_matches('\r')
        .split(',')
        .map(str::to_owned)
        .collect();
    let timeframe_column = columns
        .iter()
        .position(|column| column == "timeframe_secs")
        .ok_or_else(|| RunError::Decode {
            path: String::from("events.csv"),
            message: String::from("the event artifact has no timeframe_secs column"),
        })?;
    let mut events = Vec::new();
    for (line_number, line) in lines.enumerate() {
        let values: Vec<&str> = line.trim_end_matches('\r').split(',').collect();
        if values.len() != columns.len() {
            return Err(RunError::Decode {
                path: String::from("events.csv"),
                message: format!(
                    "event row {} has {} fields, not {}",
                    line_number + 2,
                    values.len(),
                    columns.len()
                ),
            });
        }
        if values[timeframe_column] != seconds.to_string() {
            continue;
        }
        events.push(
            columns
                .iter()
                .cloned()
                .zip(values.into_iter().map(str::to_owned))
                .collect::<BTreeMap<_, _>>(),
        );
    }
    let coverage = match report_json {
        Some(bytes) => report_coverage(bytes)?,
        None => Vec::new(),
    };
    Ok(StageEventsView {
        stage_run_id: stage_run_id.to_owned(),
        timeframe: timeframe.to_owned(),
        columns,
        events,
        coverage,
    })
}

/// Extract the four coverage fields from the model report.
fn report_coverage(bytes: &[u8]) -> Result<Vec<Value>, RunError> {
    let report: Value = serde_json::from_slice(bytes).map_err(|error| RunError::Decode {
        path: String::from("report.json"),
        message: error.to_string(),
    })?;
    let timeframes = report
        .get("flat_report")
        .and_then(|report| report.get("timeframes"))
        .and_then(Value::as_array)
        .ok_or_else(|| RunError::Decode {
            path: String::from("report.json"),
            message: String::from("the model report carries no flat_report.timeframes array"),
        })?;
    let mut coverage = Vec::with_capacity(timeframes.len());
    for timeframe in timeframes {
        let object = timeframe.as_object().ok_or_else(|| RunError::Decode {
            path: String::from("report.json"),
            message: String::from("a model report timeframe is not an object"),
        })?;
        let seconds = object
            .get("timeframe_secs")
            .and_then(Value::as_u64)
            .ok_or_else(|| RunError::Decode {
                path: String::from("report.json"),
                message: String::from("a model report timeframe has no timeframe_secs"),
            })?;
        let bars = object.get("bars").cloned().unwrap_or(Value::Null);
        let expected_bars = object.get("expected_bars").cloned().unwrap_or(Value::Null);
        let coverage_share = object.get("coverage_share").cloned().unwrap_or(Value::Null);
        coverage.push(serde_json::json!({
            "timeframe_secs": seconds,
            "bars": bars,
            "expected_bars": expected_bars,
            "coverage_share": coverage_share,
        }));
    }
    coverage.sort_by_key(|value| value["timeframe_secs"].as_u64().unwrap_or(0));
    Ok(coverage)
}

/// Convert a timeframe short code to seconds.
///
/// The accepted forms are the ladder codes (`1m`, `5m`, `15m`, `1h`) and a bare
/// non-negative integer count of seconds. The shell does not build a ladder; it
/// only maps the caller's filter to the artifact's `timeframe_secs` column.
pub(crate) fn timeframe_secs(value: &str) -> Result<u64, RunError> {
    let invalid = || {
        RunError::Usage(format!(
            "--timeframe expects a ladder code such as 1m, 5m, 15m, or 1h, or seconds; got '{value}'"
        ))
    };
    if value.is_empty() {
        return Err(invalid());
    }
    if let Some(number) = value.strip_suffix('s') {
        return number
            .parse::<u64>()
            .map_err(|_| invalid())
            .and_then(|seconds| {
                if seconds == 0 {
                    Err(invalid())
                } else {
                    Ok(seconds)
                }
            });
    }
    if let Some(number) = value.strip_suffix('m') {
        return number
            .parse::<u64>()
            .map_err(|_| invalid())
            .and_then(|minutes| {
                let seconds = minutes.saturating_mul(60);
                if seconds == 0 {
                    Err(invalid())
                } else {
                    Ok(seconds)
                }
            });
    }
    if let Some(number) = value.strip_suffix('h') {
        return number
            .parse::<u64>()
            .map_err(|_| invalid())
            .and_then(|hours| {
                let seconds = hours.saturating_mul(3_600);
                if seconds == 0 {
                    Err(invalid())
                } else {
                    Ok(seconds)
                }
            });
    }
    if let Some(number) = value.strip_suffix('d') {
        return number
            .parse::<u64>()
            .map_err(|_| invalid())
            .and_then(|days| {
                let seconds = days.saturating_mul(86_400);
                if seconds == 0 {
                    Err(invalid())
                } else {
                    Ok(seconds)
                }
            });
    }
    value
        .parse::<u64>()
        .map_err(|_| invalid())
        .and_then(|seconds| {
            if seconds == 0 {
                Err(invalid())
            } else {
                Ok(seconds)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENTS: &[u8] = b"timeframe_secs,event_index,event_code\n300,0,1\n900,0,2\n300,1,3\n";
    const REPORT: &[u8] = br#"{"model_version":"2.0.0","view_contract":"quantrule.model-view/v4","data_version":"fixture","requested_period":{"from":null,"to":null},"flat_report":{"timeframes":[{"timeframe_secs":900,"bars":0,"expected_bars":1,"coverage_share":0.0},{"timeframe_secs":300,"bars":3,"expected_bars":3,"coverage_share":1.0}]}}"#;

    #[test]
    fn a_timeframe_filter_keeps_only_its_rows() {
        let view = stage_events("stage", "5m", EVENTS, Some(REPORT)).expect("events");
        assert_eq!(view.events.len(), 2);
        assert!(view.events.iter().all(|row| row["timeframe_secs"] == "300"));
        assert_eq!(view.coverage.len(), 2);
        assert_eq!(view.coverage[0]["timeframe_secs"], 300);
    }

    #[test]
    fn the_real_report_envelope_is_read_through_flat_report() {
        let view = stage_events("stage", "5m", EVENTS, Some(REPORT)).expect("envelope");
        assert_eq!(view.coverage.len(), 2);
        assert_eq!(view.coverage[0]["timeframe_secs"], 300);
        assert_eq!(view.coverage[0]["bars"], 3);
    }

    #[test]
    fn a_report_without_the_flat_report_timeframes_is_refused() {
        assert!(stage_events("stage", "5m", EVENTS, Some(b"{}")).is_err());
        assert!(
            stage_events("stage", "5m", EVENTS, Some(br#"{"timeframes":[]}"#)).is_err(),
            "a top-level timeframes array is not the model report envelope"
        );
    }

    #[test]
    fn a_malformed_event_row_is_refused() {
        let malformed = b"timeframe_secs,event_index\n300\n";
        assert!(stage_events("stage", "5m", malformed, None).is_err());
    }

    #[test]
    fn the_short_codes_map_to_seconds() {
        assert_eq!(timeframe_secs("1m").expect("1m"), 60);
        assert_eq!(timeframe_secs("5m").expect("5m"), 300);
        assert_eq!(timeframe_secs("1h").expect("1h"), 3_600);
        assert_eq!(timeframe_secs("300").expect("seconds"), 300);
        for bad in ["", "0", "0m", "abc", "5x"] {
            assert!(timeframe_secs(bad).is_err(), "{bad}");
        }
    }
}
