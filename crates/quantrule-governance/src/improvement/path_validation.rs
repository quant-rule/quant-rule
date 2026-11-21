//! Path validation against matched baselines and seeded controls.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::store;

mod model;
mod output;
use crate::research::eval;
use model::{apply_sensitivity, evaluate, grade, load_bundle};
use output::{markdown, row_json};

const TIMEFRAME_CODES: [(i64, &str); 4] = [(60, "1m"), (300, "5m"), (900, "15m"), (3600, "1h")];
const TIMEFRAME_ORDER: [(i64, usize); 3] = [(300, 0), (900, 1), (3600, 2)];
const PRIMARY_CELLS: usize = 12;

#[derive(Clone)]
pub(super) struct Options {
    horizon: i64,
    target_atr: f64,
    stop_atr: f64,
    random_draws: usize,
    shuffle_draws: usize,
    back_lag: i64,
    seed: u64,
}

#[derive(Clone)]
pub(super) struct Reading {
    bar: i64,
    high: f64,
    low: f64,
    close: f64,
    atr: Option<f64>,
    atr_ratio: Option<f64>,
    rsi: Option<f64>,
    adx_regime: Option<f64>,
    adx_slope: Option<f64>,
    di_state: Option<f64>,
    ema_stack: Option<f64>,
    macd_state: Option<f64>,
    ema_fast: Option<f64>,
    ema_mid: Option<f64>,
    ema_slow: Option<f64>,
}

#[derive(Clone)]
pub(super) struct Event {
    index: i64,
    event_type: i64,
    subtype: i64,
    source: i64,
    available: i64,
    direction: i64,
    reference: Option<f64>,
    primary: Option<f64>,
}

#[derive(Clone)]
pub(super) struct Selected {
    bar: i64,
    direction: i64,
    hit: bool,
    state: Option<(String, i64, Option<i64>)>,
    state_applicable: bool,
}

pub(super) type StateKey = (String, i64, Option<i64>);

pub(super) struct Bundle {
    instrument: String,
    config: String,
    bases: BTreeMap<i64, BTreeMap<i64, (usize, usize)>>,
    hits: BTreeMap<i64, BTreeMap<i64, Vec<Option<bool>>>>,
    valid: BTreeMap<i64, BTreeMap<i64, Vec<usize>>>,
    states: BTreeMap<i64, BTreeMap<StateKey, (usize, usize)>>,
    selected: BTreeMap<(String, i64), Vec<Selected>>,
    raw: BTreeMap<(String, i64), usize>,
    prepared: BTreeMap<(String, i64), usize>,
    signature: BTreeMap<(String, i64), String>,
}

pub(super) struct InstrumentStat {
    n: usize,
    hits: usize,
    hit_rate: Option<f64>,
    base: Option<f64>,
}

pub(super) struct CellRow {
    family: String,
    label: String,
    timeframe: String,
    timeframe_secs: i64,
    config: String,
    raw: usize,
    prepared: usize,
    n: usize,
    clusters: usize,
    singleton_bundles: usize,
    hits: usize,
    hit_rate: Option<f64>,
    base_rate: Option<f64>,
    lift: Option<f64>,
    lift_lcb: Option<f64>,
    state_applicable: bool,
    state_rate: Option<f64>,
    state_missing: usize,
    incremental_lift: Option<f64>,
    incremental_lcb: Option<f64>,
    cluster_se: Option<f64>,
    random_mean: Option<f64>,
    random_p95: Option<f64>,
    random_p: Option<f64>,
    shuffle_mean: Option<f64>,
    shuffle_p95: Option<f64>,
    shuffle_p: Option<f64>,
    back_lag_rate: Option<f64>,
    back_lag_n: usize,
    back_lag_lift: Option<f64>,
    forward_lag_rate: Option<f64>,
    forward_lag_n: usize,
    forward_lag_lift: Option<f64>,
    replication: usize,
    instruments: BTreeMap<String, InstrumentStat>,
    event_signature: String,
    grade: String,
    reasons: Vec<String>,
    sensitivity_ok: Option<bool>,
    sensitivity_reason: Option<String>,
}

/// Run the path-validation harness.
///
/// # Errors
///
/// Returns a message for invalid input or a write failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut matrix_path = None;
    let mut families_path = None;
    let mut out = None;
    let mut config = None;
    let mut timeframes = "5m,15m,1h".to_owned();
    let mut options = Options {
        horizon: 20,
        target_atr: 1.0,
        stop_atr: 1.0,
        random_draws: 200,
        shuffle_draws: 200,
        back_lag: 20,
        seed: 0,
    };
    let mut primary_cells = PRIMARY_CELLS;
    let mut skip_sensitivity = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--matrix" => matrix_path = Some(value(args, &mut index, "--matrix")?),
            "--families" => families_path = Some(value(args, &mut index, "--families")?),
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--config" => config = Some(value(args, &mut index, "--config")?),
            "--timeframes" => timeframes = value(args, &mut index, "--timeframes")?,
            "--horizon" => options.horizon = parse(args, &mut index, "--horizon")?,
            "--target-atr" => options.target_atr = parse(args, &mut index, "--target-atr")?,
            "--stop-atr" => options.stop_atr = parse(args, &mut index, "--stop-atr")?,
            "--random-draws" => options.random_draws = parse(args, &mut index, "--random-draws")?,
            "--shuffle-draws" => {
                options.shuffle_draws = parse(args, &mut index, "--shuffle-draws")?
            }
            "--back-lag" => options.back_lag = parse(args, &mut index, "--back-lag")?,
            "--seed" => options.seed = parse(args, &mut index, "--seed")?,
            "--primary-cells" => primary_cells = parse(args, &mut index, "--primary-cells")?,
            "--skip-sensitivity" => skip_sensitivity = true,
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let matrix = store::read_json(Path::new(
        &matrix_path.ok_or_else(|| "--matrix is required".to_owned())?,
    ))?;
    let families = store::read_json(Path::new(
        &families_path.ok_or_else(|| "--families is required".to_owned())?,
    ))?;
    let Some(families) = families.as_array() else {
        return Err("families must be a JSON list".to_owned());
    };
    let cells = expand_cells(&matrix, config.as_deref());
    if cells.is_empty() {
        return Err("no cells".to_owned());
    }
    let mut timeframe_set = BTreeSet::new();
    for code in timeframes.split(',') {
        let code = code.trim();
        if let Some((seconds, _)) = TIMEFRAME_CODES.iter().find(|(_, name)| *name == code) {
            timeframe_set.insert(*seconds);
        }
    }
    if timeframe_set.is_empty() {
        return Err("no supported timeframes".to_owned());
    }
    let bundles: Vec<Bundle> = cells
        .iter()
        .map(|cell| load_bundle(cell, families, &options, &timeframe_set))
        .collect::<Result<_, _>>()?;
    let mut configs: Vec<String> = Vec::new();
    for cell in &cells {
        if let Some(config) = cell.get("config").and_then(Value::as_str)
            && !configs.iter().any(|value| value == config)
        {
            configs.push(config.to_owned());
        }
    }
    let primary_config = matrix
        .get("primary_config")
        .and_then(Value::as_str)
        .map_or_else(
            || configs.first().cloned().unwrap_or_default(),
            str::to_owned,
        );
    let sensitivity_config = matrix
        .get("sensitivity_config")
        .and_then(Value::as_str)
        .map_or_else(
            || configs.last().cloned().unwrap_or_default(),
            str::to_owned,
        );
    let z = eval::primary_z(primary_cells);
    let mut rows = Vec::new();
    for config in &configs {
        let subset: Vec<&Bundle> = bundles
            .iter()
            .filter(|bundle| bundle.config == *config)
            .collect();
        rows.extend(evaluate(&subset, families, &timeframe_set, &options, z));
    }
    if !skip_sensitivity {
        apply_sensitivity(&mut rows, &primary_config, &sensitivity_config);
    }
    for row in &mut rows {
        let (grade, reasons) = grade(row, row.config == primary_config);
        row.grade = grade;
        row.reasons = reasons;
        if !skip_sensitivity && row.config == primary_config && row.sensitivity_ok != Some(true) {
            row.grade = "C".to_owned();
            row.reasons.push("sensitivity".to_owned());
        }
    }
    let mut passing: Vec<&CellRow> = rows
        .iter()
        .filter(|row| row.config == primary_config && matches!(row.grade.as_str(), "A" | "B"))
        .collect();
    passing.sort_by(|left, right| {
        let left_lcb = left.incremental_lcb.unwrap_or(f64::NEG_INFINITY);
        let right_lcb = right.incremental_lcb.unwrap_or(f64::NEG_INFINITY);
        right_lcb
            .total_cmp(&left_lcb)
            .then_with(|| order(left.timeframe_secs).cmp(&order(right.timeframe_secs)))
            .then_with(|| left.family.cmp(&right.family))
    });
    let selection = passing.first().copied();
    let decision = selection.map_or_else(
        || "No primary cell passes; the development partition records **no edge**. The validation partition is not opened.".to_owned(),
        |row| format!(
            "Frozen candidate: `{}` on `{}` under `{}` (grade {}, incremental LCB {}). The validation partition may now be opened once.",
            row.family, row.timeframe, primary_config, row.grade, fmt_num(row.incremental_lcb)
        ),
    );
    let mut sorted_rows: Vec<&CellRow> = rows.iter().collect();
    sorted_rows.sort_by(|left, right| {
        order(left.timeframe_secs)
            .cmp(&order(right.timeframe_secs))
            .then_with(|| left.family.cmp(&right.family))
            .then_with(|| left.config.cmp(&right.config))
    });
    let document = json!({
        "schema": "quantrule.path-validation/v2",
        "horizon_bars": options.horizon,
        "target_atr": options.target_atr,
        "stop_atr": options.stop_atr,
        "seed": options.seed,
        "random_draws": options.random_draws,
        "shuffle_draws": options.shuffle_draws,
        "back_lag": options.back_lag,
        "primary_config": primary_config,
        "sensitivity_config": sensitivity_config,
        "primary_cells": primary_cells,
        "cells": sorted_rows.iter().copied().map(row_json).collect::<Vec<_>>(),
        "selection": selection.map_or(Value::Null, row_json),
        "decision": decision,
    });
    let out = PathBuf::from(out.ok_or_else(|| "--out is required".to_owned())?);
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let text = serde_json::to_string_pretty(&document)
        .map_err(|error| format!("cannot encode path_validation.json: {error}"))?;
    fs::write(out.join("path_validation.json"), format!("{text}\n"))
        .map_err(|error| format!("cannot write path_validation.json: {error}"))?;
    fs::write(out.join("path_validation.md"), markdown(&document))
        .map_err(|error| format!("cannot write path_validation.md: {error}"))?;
    println!("ok path validation -> {}", out.display());
    Ok(())
}

fn expand_cells(matrix: &Value, config: Option<&str>) -> Vec<Value> {
    if let Some(cells) = matrix.as_array() {
        return cells.clone();
    }
    let Some(cells) = matrix.get("cells").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut expanded = Vec::new();
    for cell in cells {
        let Some(runs) = cell.get("runs").and_then(Value::as_object) else {
            expanded.push(cell.clone());
            continue;
        };
        for (name, run) in runs {
            if config.is_some_and(|config| config != name) {
                continue;
            }
            expanded.push(json!({
                "instrument": cell.get("instrument"),
                "fold": cell.get("fold"),
                "partition": cell.get("partition"),
                "config": name,
                "run": run,
            }));
        }
    }
    expanded
}

fn fmt_num(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:.4}"))
}

fn order(timeframe: i64) -> usize {
    TIMEFRAME_ORDER
        .iter()
        .find(|(code, _)| *code == timeframe)
        .map_or(99, |(_, order)| *order)
}
fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}
fn parse<T: std::str::FromStr>(
    args: &[String],
    index: &mut usize,
    name: &str,
) -> Result<T, String> {
    value(args, index, name)?
        .parse()
        .map_err(|_| format!("{name} has an invalid value"))
}

#[cfg(test)]
mod tests;
