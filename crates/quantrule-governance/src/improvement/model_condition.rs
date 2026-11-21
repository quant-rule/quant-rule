//! Prepare a per-bar condition from published model artifacts.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::store;

const SCHEMA: &str = "quantrule.model-condition/v1";
const GATE_SCHEMA: &str = "quantrule.model-state-gate/v1";
const CONDITION_COLUMN: &str = "condition";
const DIRECTIONS: [&str; 3] = ["any", "same", "opposite"];
const CLOSE_SIDES: [&str; 3] = ["any", "breakout", "original"];
const PIVOT_ALIASES: [(&str, &str); 2] = [
    ("pivot_high", "last_pivot_high"),
    ("pivot_low", "last_pivot_low"),
];

struct Event {
    event_id: String,
    occurred: i64,
    direction: i64,
    reference_code: String,
    reference_value: Option<f64>,
}

/// Run the model-condition command.
///
/// # Errors
///
/// Returns a message for an invalid spec, a missing input, or a write failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut bundle = None;
    let mut condition = None;
    let mut registry = PathBuf::from("docs/registry/indicators.json");
    let mut out = None;
    let mut events = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--bundle" => bundle = Some(value(args, &mut index, "--bundle")?),
            "--condition" => condition = Some(value(args, &mut index, "--condition")?),
            "--registry" => registry = PathBuf::from(value(args, &mut index, "--registry")?),
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--events" => events = Some(value(args, &mut index, "--events")?),
            "--levels" => {
                let _ = value(args, &mut index, "--levels")?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let bundle = PathBuf::from(bundle.ok_or_else(|| "--bundle is required".to_owned())?);
    let condition_path =
        PathBuf::from(condition.ok_or_else(|| "--condition is required".to_owned())?);
    let out = PathBuf::from(out.ok_or_else(|| "--out is required".to_owned())?);
    let spec = load_condition(&condition_path)?;
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let readings_path = bundle.join("readings.csv");
    let events_path = events.map_or_else(|| bundle.join("events.csv"), PathBuf::from);
    let (header, rows) = read_csv(&readings_path)?;
    if header.iter().any(|name| name == CONDITION_COLUMN) {
        return Err(format!(
            "{} already has a {CONDITION_COLUMN} column",
            readings_path.display()
        ));
    }
    let bar_index = column(&header, "bar").ok_or_else(|| {
        format!(
            "{} is missing a bar/timeframe column",
            readings_path.display()
        )
    })?;
    let timeframe_index = column(&header, "timeframe_secs").ok_or_else(|| {
        format!(
            "{} is missing a bar/timeframe column",
            readings_path.display()
        )
    })?;
    let timeframes: BTreeSet<i64> = rows
        .iter()
        .filter_map(|row| row.get(timeframe_index)?.parse().ok())
        .collect();
    if timeframes.len() != 1 {
        return Err(format!(
            "{} spans multiple timeframes",
            readings_path.display()
        ));
    }
    let timeframe = *timeframes.iter().next().unwrap_or(&0);
    if let Some(declared) = spec.get("timeframe_secs").and_then(Value::as_i64)
        && declared != timeframe
    {
        return Err(format!(
            "condition timeframe {declared} does not match bundle {timeframe}"
        ));
    }
    let mut readings: BTreeMap<i64, BTreeMap<String, String>> = BTreeMap::new();
    for row in &rows {
        let bar: i64 = row
            .get(bar_index)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        readings.insert(
            bar,
            header.iter().cloned().zip(row.iter().cloned()).collect(),
        );
    }
    let kind = spec.get("kind").and_then(Value::as_str).unwrap_or("");
    let (condition, matches) = match kind {
        "sequence" => {
            if !events_path.is_file() {
                return Err(format!("events file not found: {}", events_path.display()));
            }
            let codes = load_event_codes(&registry)?;
            sequence_condition(&spec, &events_path, &readings, &codes, timeframe)?
        }
        "confluence" => confluence_condition(&spec, &readings),
        _ => column_condition(&spec, &readings)?,
    };
    let mut output_rows = Vec::new();
    for row in &rows {
        let bar: i64 = row
            .get(bar_index)
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let mut output = row.clone();
        output.push(condition.get(&bar).copied().unwrap_or(0).to_string());
        output_rows.push(output);
    }
    let mut output_header = header.clone();
    output_header.push(CONDITION_COLUMN.to_owned());
    write_csv(&out.join("readings.csv"), &output_header, &output_rows)?;
    for name in ["prediction.csv", "events.csv", "levels.csv", "panel.csv"] {
        let source = bundle.join(name);
        if source.is_file() {
            fs::copy(&source, out.join(name))
                .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
        }
    }
    let gate = json!({
        "schema": GATE_SCHEMA,
        "all": [{"column": CONDITION_COLUMN, "values": [1]}],
    });
    store::write_json(&out.join("state.json"), &gate)?;
    let document = json!({
        "schema": SCHEMA,
        "kind": kind,
        "spec": spec,
        "timeframe_secs": timeframe,
        "bars": rows.len(),
        "condition_bars": condition.len(),
        "matches": matches,
        "inputs": {
            "readings_sha256": sha256(&readings_path)?,
            "events_sha256": if events_path.is_file() { Some(sha256(&events_path)?) } else { None },
            "condition_sha256": sha256(&condition_path)?,
        },
    });
    store::write_json(&out.join("condition.json"), &document)?;
    println!(
        "ok condition kind={kind} bars={} matched={} -> {}",
        rows.len(),
        condition.len(),
        out.display()
    );
    Ok(())
}

fn load_condition(path: &Path) -> Result<Value, String> {
    let document = store::read_json(path)?;
    let Some(document) = document.as_object() else {
        return Err(format!("{} must contain an object", path.display()));
    };
    if document.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("{} schema must be {SCHEMA}", path.display()));
    }
    let kind = document.get("kind").and_then(Value::as_str).unwrap_or("");
    let allowed: BTreeSet<&str> = match kind {
        "sequence" => [
            "schema",
            "kind",
            "description",
            "timeframe_secs",
            "first_event",
            "second_event",
            "first_events",
            "second_events",
            "within_bars",
            "min_gap_bars",
            "same_level",
            "level_tolerance_atr",
            "direction",
            "close_side",
            "level_touches_min",
            "level_age_max",
        ]
        .into_iter()
        .collect(),
        "confluence" => [
            "schema",
            "kind",
            "description",
            "timeframe_secs",
            "references",
            "min_count",
            "within_atr",
        ]
        .into_iter()
        .collect(),
        "column" => [
            "schema",
            "kind",
            "description",
            "timeframe_secs",
            "column",
            "values",
            "min",
            "max",
            "level_age_max",
            "level_touches_min",
        ]
        .into_iter()
        .collect(),
        _ => return Err(format!("{0} has unknown kind {kind:?}", path.display())),
    };
    let unknown: Vec<&str> = document
        .keys()
        .map(String::as_str)
        .filter(|key| !allowed.contains(key))
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "{} has unknown fields: {unknown:?}",
            path.display()
        ));
    }
    match kind {
        "sequence" => {
            event_id_list(document, path, "first_event", "first_events")?;
            event_id_list(document, path, "second_event", "second_events")?;
            non_negative_int(document, path, "within_bars", true)?;
            non_negative_int(document, path, "min_gap_bars", false)?;
            if let (Some(within), Some(gap)) = (
                document.get("within_bars").and_then(Value::as_i64),
                document.get("min_gap_bars").and_then(Value::as_i64),
            ) && gap > within
            {
                return Err(format!(
                    "{} min_gap_bars must not exceed within_bars",
                    path.display()
                ));
            }
            positive_int(document, path, "level_touches_min")?;
            non_negative_int(document, path, "level_age_max", false)?;
            member(document, path, "direction", &DIRECTIONS)?;
            member(document, path, "close_side", &CLOSE_SIDES)?;
        }
        "confluence" => {
            let references = document.get("references").and_then(Value::as_array);
            if !references.is_some_and(|values| {
                !values.is_empty()
                    && values
                        .iter()
                        .all(|value| value.as_str().is_some_and(|text| !text.is_empty()))
            }) {
                return Err(format!(
                    "{} references must be a non-empty string list",
                    path.display()
                ));
            }
            let min_count = document.get("min_count").and_then(Value::as_i64);
            if !min_count.is_some_and(|value| value >= 1) {
                return Err(format!(
                    "{} min_count must be a positive integer",
                    path.display()
                ));
            }
            let within = document.get("within_atr").and_then(Value::as_f64);
            if !within.is_some_and(|value| value > 0.0) {
                return Err(format!("{} within_atr must be positive", path.display()));
            }
        }
        _ => {
            if !document
                .get("column")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.is_empty())
            {
                return Err(format!(
                    "{} column must be a non-empty string",
                    path.display()
                ));
            }
            if !["values", "min", "max", "level_age_max", "level_touches_min"]
                .iter()
                .any(|key| document.get(*key).is_some_and(|value| !value.is_null()))
            {
                return Err(format!(
                    "{} needs values, min, max, level_age_max, or level_touches_min",
                    path.display()
                ));
            }
        }
    }
    Ok(Value::Object(document.clone()))
}

fn event_id_list(
    document: &Map<String, Value>,
    path: &Path,
    singular: &str,
    plural: &str,
) -> Result<Vec<String>, String> {
    let single = document.get(singular);
    let multiple = document.get(plural);
    if single.is_some() && multiple.is_some() {
        return Err(format!(
            "{} must not set both {singular} and {plural}",
            path.display()
        ));
    }
    if let Some(multiple) = multiple {
        let values = multiple.as_array().filter(|values| {
            !values.is_empty()
                && values
                    .iter()
                    .all(|value| value.as_str().is_some_and(|text| !text.is_empty()))
        });
        let Some(values) = values else {
            return Err(format!(
                "{} {plural} must be a non-empty string list",
                path.display()
            ));
        };
        let mut seen = BTreeSet::new();
        let mut output = Vec::new();
        for value in values {
            let value = value.as_str().unwrap_or("").to_owned();
            if !seen.insert(value.clone()) {
                return Err(format!(
                    "{} {plural} must not contain duplicates",
                    path.display()
                ));
            }
            output.push(value);
        }
        return Ok(output);
    }
    if let Some(single) = single
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    {
        return Ok(vec![single.to_owned()]);
    }
    Err(format!(
        "{} {singular} or {plural} must name events",
        path.display()
    ))
}

fn non_negative_int(
    document: &Map<String, Value>,
    path: &Path,
    key: &str,
    required: bool,
) -> Result<(), String> {
    match document.get(key) {
        None if !required => Ok(()),
        Some(value) if value.as_i64().is_some_and(|value| value >= 0) => Ok(()),
        _ => Err(format!(
            "{0} {key} must be a non-negative integer",
            path.display()
        )),
    }
}

fn positive_int(document: &Map<String, Value>, path: &Path, key: &str) -> Result<(), String> {
    match document.get(key) {
        None => Ok(()),
        Some(value) if value.as_i64().is_some_and(|value| value >= 1) => Ok(()),
        _ => Err(format!(
            "{} {key} must be a positive integer",
            path.display()
        )),
    }
}

fn member(
    document: &Map<String, Value>,
    path: &Path,
    key: &str,
    allowed: &[&str],
) -> Result<(), String> {
    match document.get(key) {
        None => Ok(()),
        Some(value) if value.as_str().is_some_and(|value| allowed.contains(&value)) => Ok(()),
        _ => Err(format!(
            "{} {key} must be one of {allowed:?}",
            path.display()
        )),
    }
}

fn sequence_condition(
    spec: &Value,
    events_path: &Path,
    readings: &BTreeMap<i64, BTreeMap<String, String>>,
    codes: &BTreeMap<String, i64>,
    timeframe: i64,
) -> Result<(BTreeMap<i64, i64>, usize), String> {
    let empty = Map::new();
    let spec = spec.as_object().unwrap_or(&empty);
    let first_ids = event_id_list(spec, Path::new("condition"), "first_event", "first_events")?;
    let second_ids = event_id_list(
        spec,
        Path::new("condition"),
        "second_event",
        "second_events",
    )?;
    let first = event_rows(events_path, codes, &first_ids, timeframe)?;
    let second = event_rows(events_path, codes, &second_ids, timeframe)?;
    let within = spec.get("within_bars").and_then(Value::as_i64).unwrap_or(0);
    let min_gap = spec
        .get("min_gap_bars")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let same = spec
        .get("same_level")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tolerance = spec
        .get("level_tolerance_atr")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let direction = spec
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("any");
    let close_side = spec
        .get("close_side")
        .and_then(Value::as_str)
        .unwrap_or("any");
    let touches_min = spec.get("level_touches_min").and_then(Value::as_i64);
    let age_max = spec.get("level_age_max").and_then(Value::as_i64);
    let mut condition = BTreeMap::new();
    let mut matches = 0;
    for (bar, seconds) in &second {
        let Some(row) = readings.get(bar) else {
            continue;
        };
        if let Some(minimum) = touches_min {
            let touches = optional_int(row.get("level_touches"));
            if !touches.is_some_and(|value| value >= minimum) {
                continue;
            }
        }
        if let Some(maximum) = age_max {
            let age = optional_int(row.get("level_age"));
            if !age.is_some_and(|value| value <= maximum) {
                continue;
            }
        }
        let close = optional_number(row.get("close"));
        let atr = optional_number(row.get("atr"));
        for second_event in seconds {
            let mut matched = false;
            let start = (bar - within).max(0);
            let end = bar - min_gap;
            for first_bar in start..=end {
                for first_event in first.get(&first_bar).into_iter().flatten() {
                    if direction == "same" && second_event.direction != first_event.direction {
                        continue;
                    }
                    if direction == "opposite" && second_event.direction == first_event.direction {
                        continue;
                    }
                    if same && !same_level(first_event, second_event, atr, tolerance) {
                        continue;
                    }
                    let Some(close) = close else {
                        continue;
                    };
                    if !close_side_matches(
                        close,
                        first_event.reference_value,
                        first_event.direction,
                        close_side,
                    ) {
                        continue;
                    }
                    matched = true;
                    break;
                }
                if matched {
                    break;
                }
            }
            if matched {
                condition.insert(*bar, 1);
                matches += 1;
                break;
            }
        }
    }
    Ok((condition, matches))
}

fn confluence_condition(
    spec: &Value,
    readings: &BTreeMap<i64, BTreeMap<String, String>>,
) -> (BTreeMap<i64, i64>, usize) {
    let spec = spec.as_object().cloned().unwrap_or_default();
    let references: Vec<String> = spec
        .get("references")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let min_count = spec.get("min_count").and_then(Value::as_i64).unwrap_or(0);
    let within_atr = spec
        .get("within_atr")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let mut condition = BTreeMap::new();
    let mut matches = 0;
    for (bar, row) in readings {
        let Some(close) = optional_number(row.get("close")) else {
            continue;
        };
        let Some(atr) = optional_number(row.get("atr")) else {
            continue;
        };
        if atr <= 0.0 {
            continue;
        }
        let mut count = 0;
        for reference in &references {
            let column = PIVOT_ALIASES
                .iter()
                .find(|(alias, _)| alias == reference)
                .map_or(reference.as_str(), |(_, canonical)| *canonical);
            let Some(value) = optional_number(row.get(column)) else {
                continue;
            };
            if (close - value).abs() <= within_atr * atr {
                count += 1;
            }
        }
        if count >= min_count {
            condition.insert(*bar, 1);
            matches += 1;
        }
    }
    (condition, matches)
}

fn column_condition(
    spec: &Value,
    readings: &BTreeMap<i64, BTreeMap<String, String>>,
) -> Result<(BTreeMap<i64, i64>, usize), String> {
    let spec = spec.as_object().cloned().unwrap_or_default();
    let column = spec.get("column").and_then(Value::as_str).unwrap_or("");
    let values: Option<Vec<i64>> = spec
        .get("values")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_i64).collect());
    if let Some(values) = &values
        && values.is_empty()
    {
        return Err("column values must be a non-empty integer list".to_owned());
    }
    let minimum = spec.get("min").and_then(Value::as_f64);
    let maximum = spec.get("max").and_then(Value::as_f64);
    let age_max = spec.get("level_age_max").and_then(Value::as_i64);
    let touches_min = spec.get("level_touches_min").and_then(Value::as_i64);
    let mut condition = BTreeMap::new();
    let mut matches = 0;
    for (bar, row) in readings {
        if let Some(values) = &values {
            let value = optional_int(row.get(column));
            if !value.is_some_and(|value| values.contains(&value)) {
                continue;
            }
        } else {
            let Some(value) = optional_number(row.get(column)) else {
                continue;
            };
            if minimum.is_some_and(|minimum| value < minimum) {
                continue;
            }
            if maximum.is_some_and(|maximum| value > maximum) {
                continue;
            }
        }
        if let Some(maximum) = age_max {
            let age = optional_int(row.get("level_age"));
            if !age.is_some_and(|value| value <= maximum) {
                continue;
            }
        }
        if let Some(minimum) = touches_min {
            let touches = optional_int(row.get("level_touches"));
            if !touches.is_some_and(|value| value >= minimum) {
                continue;
            }
        }
        condition.insert(*bar, 1);
        matches += 1;
    }
    Ok((condition, matches))
}

fn event_rows(
    path: &Path,
    codes: &BTreeMap<String, i64>,
    event_ids: &[String],
    timeframe: i64,
) -> Result<BTreeMap<i64, Vec<Event>>, String> {
    let mut requested = BTreeMap::new();
    for event_id in event_ids {
        let Some(code) = codes.get(event_id) else {
            return Err(format!("event {event_id:?} is not registered"));
        };
        requested.insert(*code, event_id.clone());
    }
    let (header, rows) = read_csv(path)?;
    let code_index = column(&header, "event_code")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let available_index = column(&header, "available_bar_index")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let occurred_index = column(&header, "occurred_bar_index")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let direction_index = column(&header, "direction_code")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let reference_code_index = column(&header, "reference_code")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let reference_value_index = column(&header, "reference_value")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let timeframe_index = column(&header, "timeframe_secs")
        .ok_or_else(|| format!("{} is missing a sequence column", path.display()))?;
    let mut by_bar: BTreeMap<i64, Vec<Event>> = BTreeMap::new();
    for row in rows {
        let Ok(timeframe_value) = row
            .get(timeframe_index)
            .map(String::as_str)
            .unwrap_or("")
            .parse::<i64>()
        else {
            continue;
        };
        if timeframe_value != timeframe {
            continue;
        }
        let Some(code) = row
            .get(code_index)
            .and_then(|value| value.parse::<i64>().ok())
        else {
            continue;
        };
        let Some(event_id) = requested.get(&code) else {
            continue;
        };
        let (Ok(available), Ok(occurred), Ok(direction)) = (
            row.get(available_index)
                .map(String::as_str)
                .unwrap_or("")
                .parse::<i64>(),
            row.get(occurred_index)
                .map(String::as_str)
                .unwrap_or("")
                .parse::<i64>(),
            row.get(direction_index)
                .map(String::as_str)
                .unwrap_or("")
                .parse::<i64>(),
        ) else {
            continue;
        };
        by_bar.entry(available).or_default().push(Event {
            event_id: event_id.clone(),
            occurred,
            direction,
            reference_code: row
                .get(reference_code_index)
                .map(|value| value.trim().to_owned())
                .unwrap_or_default(),
            reference_value: optional_number(row.get(reference_value_index)),
        });
    }
    for values in by_bar.values_mut() {
        values.sort_by(|left, right| {
            left.occurred
                .cmp(&right.occurred)
                .then_with(|| left.event_id.cmp(&right.event_id))
                .then_with(|| left.reference_code.cmp(&right.reference_code))
                .then_with(|| {
                    left.reference_value
                        .unwrap_or(0.0)
                        .total_cmp(&right.reference_value.unwrap_or(0.0))
                })
                .then_with(|| left.direction.cmp(&right.direction))
        });
    }
    Ok(by_bar)
}

fn same_level(first: &Event, second: &Event, atr: Option<f64>, tolerance_atr: f64) -> bool {
    if !first.reference_code.is_empty() && !second.reference_code.is_empty() {
        return first.reference_code == second.reference_code;
    }
    let (Some(first), Some(second)) = (first.reference_value, second.reference_value) else {
        return false;
    };
    let tolerance = tolerance_atr.max(0.0) * atr.filter(|atr| *atr > 0.0).unwrap_or(0.0);
    (first - second).abs() <= tolerance
}

fn close_side_matches(
    close: f64,
    level: Option<f64>,
    first_direction: i64,
    close_side: &str,
) -> bool {
    if close_side == "any" {
        return true;
    }
    let Some(level) = level else {
        return false;
    };
    if close_side == "breakout" {
        if first_direction > 0 {
            close > level
        } else {
            close < level
        }
    } else if first_direction > 0 {
        close < level
    } else {
        close > level
    }
}

fn load_event_codes(path: &Path) -> Result<BTreeMap<String, i64>, String> {
    let registry = store::read_json(path)?;
    let Some(events) = registry.get("events").and_then(Value::as_array) else {
        return Err(format!("{} has no events array", path.display()));
    };
    let mut codes = BTreeMap::new();
    for event in events {
        let (Some(ident), Some(code)) = (
            event.get("id").and_then(Value::as_str),
            event.get("code").and_then(Value::as_i64),
        ) else {
            continue;
        };
        codes.insert(ident.to_owned(), code);
    }
    Ok(codes)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<String> = lines
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect();
    let rows = lines.map(parse_csv_line).collect();
    Ok((header, rows))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    line.split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect()
}

fn write_csv(path: &Path, header: &[String], rows: &[Vec<String>]) -> Result<(), String> {
    let mut text = String::new();
    text.push_str(&csv_line(header));
    for row in rows {
        text.push_str(&csv_line(row));
    }
    fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn csv_line(values: &[String]) -> String {
    let fields: Vec<String> = values
        .iter()
        .map(|value| {
            if value.contains(',')
                || value.contains('"')
                || value.contains('\n')
                || value.contains('\r')
            {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value.clone()
            }
        })
        .collect();
    format!("{}\r\n", fields.join(","))
}

fn column(header: &[String], name: &str) -> Option<usize> {
    header.iter().position(|field| field == name)
}

fn optional_number(value: Option<&String>) -> Option<f64> {
    value?.parse::<f64>().ok().filter(|value| !value.is_nan())
}

fn optional_int(value: Option<&String>) -> Option<i64> {
    let number = optional_number(value)?;
    number.is_finite().then_some(number as i64)
}

fn sha256(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}

fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}

#[cfg(test)]
mod tests;
