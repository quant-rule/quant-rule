//! Forward/barrier model-prediction evaluation with matched controls.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::store;
use crate::research::eval;
use crate::research::random::Random;
use crate::research::stats::inverse_normal_cdf;

const SCHEMA: &str = "quantrule.model-prediction/v1";
const STATE_SCHEMA: &str = "quantrule.model-state-gate/v1";
const STATE_COLUMNS: [&str; 9] = [
    "context_code",
    "adx_regime",
    "adx_slope",
    "ema_stack",
    "di_state",
    "rsi_zone",
    "macd_state",
    "level_touches",
    "condition",
];
const PARTITIONS: [&str; 6] = [
    "model_dev",
    "model_validation",
    "strategy_dev",
    "strategy_validation",
    "portfolio_dev",
    "portfolio_holdout",
];

mod output;

#[derive(Clone)]
pub(super) struct Args {
    bundles: Vec<(String, PathBuf, PathBuf, Option<PathBuf>)>,
    instrument: String,
    spec_id: Option<String>,
    family: Option<String>,
    partition: String,
    timeframe: Option<i64>,
    split_start: Option<String>,
    split_end: Option<String>,
    horizon: i64,
    outcome: String,
    target_atr: f64,
    stop_atr: f64,
    cost_bps: f64,
    random_draws: usize,
    shuffle_draws: usize,
    back_lag: Option<i64>,
    seed: u64,
    primary_cells: usize,
    alpha: f64,
    min_events: usize,
    min_clusters: usize,
    min_lift: f64,
    min_instrument_events: usize,
    min_instruments: usize,
    state_file: Option<String>,
    min_state_increment: f64,
}

#[derive(Clone)]
struct StateClause {
    column: String,
    values: Option<Vec<i64>>,
    min_value: Option<i64>,
    max_value: Option<i64>,
}

#[derive(Clone)]
struct StateGate {
    clauses: Vec<StateClause>,
    raw: Value,
}

#[derive(Clone)]
struct Outcome {
    hit: bool,
    exit_index: usize,
}

#[derive(Clone)]
struct Prediction {
    timeframe: i64,
    bar: i64,
    timestamp: Option<i64>,
    prediction: i64,
}

#[derive(Clone)]
struct Row {
    timeframe: i64,
    bar: i64,
    timestamp: Option<i64>,
    close: f64,
    high: f64,
    low: f64,
    atr: Option<f64>,
    states: BTreeMap<String, String>,
}

#[derive(Clone)]
struct BaseStats {
    rate: Option<f64>,
    valid_indices: Vec<usize>,
    by_year: BTreeMap<i64, (usize, usize)>,
}

#[derive(Clone)]
struct Entry {
    bundle: usize,
    bar: i64,
    timestamp: Option<i64>,
    direction: i64,
    outcome: Outcome,
    base_rate: Option<f64>,
    fold_base_rate: Option<f64>,
    hit_by_direction: BTreeMap<i64, bool>,
    state_ok: bool,
    state_base_rate: Option<f64>,
    state_fold_base_rate: Option<f64>,
}

struct BundleData {
    name: String,
    rows: Vec<Row>,
    index_by_bar: BTreeMap<i64, usize>,
    entries: Vec<Entry>,
    base: BTreeMap<i64, BaseStats>,
    raw_predictions: usize,
    purged: usize,
    state_base: Option<BTreeMap<i64, BaseStats>>,
    state_gate: Option<StateGate>,
}

struct Metrics {
    scope: &'static str,
    n: usize,
    predictions: usize,
    coverage: f64,
    purged: usize,
    clusters: usize,
    hits: usize,
    hit_rate: Option<f64>,
    base_rate: Option<f64>,
    lift: Option<f64>,
    lift_lcb: Option<f64>,
    lcb_method: &'static str,
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
    replication: usize,
    instruments: BTreeMap<String, Value>,
    folds: BTreeMap<String, Value>,
}

/// Run the model-prediction evaluator.
///
/// # Errors
///
/// Returns a message for invalid input or a write failure.
pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = parse_args(args)?;
    let document = evaluate(&parsed)?;
    let out = PathBuf::from(
        args.iter()
            .position(|arg| arg == "--out")
            .and_then(|index| args.get(index + 1))
            .ok_or_else(|| "--out is required".to_owned())?,
    );
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    store::write_json(&out.join("model_prediction.json"), &document)?;
    fs::write(out.join("model_prediction.md"), output::markdown(&document))
        .map_err(|error| format!("cannot write model_prediction.md: {error}"))?;
    let spec = document["spec_id"].as_str().unwrap_or("model");
    println!(
        "ok {spec} {} grade={} verdict={} -> {}",
        document["instrument"].as_str().unwrap_or(""),
        document["grade"].as_str().unwrap_or(""),
        document["verdict"].as_str().unwrap_or(""),
        out.display()
    );
    Ok(())
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args {
        bundles: Vec::new(),
        instrument: "unknown".to_owned(),
        spec_id: None,
        family: None,
        partition: "model_dev".to_owned(),
        timeframe: None,
        split_start: None,
        split_end: None,
        horizon: 20,
        outcome: "barrier".to_owned(),
        target_atr: 2.0,
        stop_atr: 1.0,
        cost_bps: 6.0,
        random_draws: 200,
        shuffle_draws: 200,
        back_lag: None,
        seed: 0,
        primary_cells: 5,
        alpha: 0.05,
        min_events: 200,
        min_clusters: 20,
        min_lift: 0.02,
        min_instrument_events: 30,
        min_instruments: 2,
        state_file: None,
        min_state_increment: 0.01,
    };
    let mut index = 0;
    while index < args.len() {
        let name = args[index].as_str();
        let value = |index: &mut usize| -> Result<String, String> {
            *index += 1;
            args.get(*index)
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match name {
            "--bundle" => {
                let raw = value(&mut index)?;
                let (label, path) = raw
                    .split_once('=')
                    .ok_or_else(|| "--bundle must be NAME=DIR".to_owned())?;
                if label.trim().is_empty() || path.trim().is_empty() {
                    return Err("--bundle must be NAME=DIR".to_owned());
                }
                let root = PathBuf::from(path.trim());
                let prediction = root.join("prediction.csv");
                let readings = root.join("readings.csv");
                if !prediction.is_file() {
                    return Err(format!("bundle file not found: {}", prediction.display()));
                }
                if !readings.is_file() {
                    return Err(format!("bundle file not found: {}", readings.display()));
                }
                let provenance = root.join("provenance.json");
                parsed.bundles.push((
                    label.trim().to_owned(),
                    prediction,
                    readings,
                    provenance.is_file().then_some(provenance),
                ));
            }
            "--predictions" => parsed.instrument = value(&mut index)?,
            "--instrument" => parsed.instrument = value(&mut index)?,
            "--spec-id" => parsed.spec_id = Some(value(&mut index)?),
            "--family" => parsed.family = Some(value(&mut index)?),
            "--partition" => parsed.partition = value(&mut index)?,
            "--timeframe" => parsed.timeframe = Some(parse(&mut index, &value)?),
            "--split-start" => parsed.split_start = Some(value(&mut index)?),
            "--split-end" => parsed.split_end = Some(value(&mut index)?),
            "--horizon" => parsed.horizon = parse(&mut index, &value)?,
            "--outcome" => parsed.outcome = value(&mut index)?,
            "--target-atr" => parsed.target_atr = parse(&mut index, &value)?,
            "--stop-atr" => parsed.stop_atr = parse(&mut index, &value)?,
            "--cost-bps" => parsed.cost_bps = parse(&mut index, &value)?,
            "--random-draws" => parsed.random_draws = parse(&mut index, &value)?,
            "--shuffle-draws" => parsed.shuffle_draws = parse(&mut index, &value)?,
            "--back-lag" => parsed.back_lag = Some(parse(&mut index, &value)?),
            "--seed" => parsed.seed = parse(&mut index, &value)?,
            "--primary-cells" => parsed.primary_cells = parse(&mut index, &value)?,
            "--alpha" => parsed.alpha = parse(&mut index, &value)?,
            "--min-events" => parsed.min_events = parse(&mut index, &value)?,
            "--min-clusters" => parsed.min_clusters = parse(&mut index, &value)?,
            "--min-lift" => parsed.min_lift = parse(&mut index, &value)?,
            "--min-instrument-events" => parsed.min_instrument_events = parse(&mut index, &value)?,
            "--min-instruments" => parsed.min_instruments = parse(&mut index, &value)?,
            "--state-file" => parsed.state_file = Some(value(&mut index)?),
            "--min-state-increment" => parsed.min_state_increment = parse(&mut index, &value)?,
            "--out" => {
                let _ = value(&mut index)?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    if !PARTITIONS.contains(&parsed.partition.as_str()) {
        return Err(format!("unknown partition {}", parsed.partition));
    }
    if parsed.horizon <= 0 || parsed.primary_cells < 1 {
        return Err("horizon and primary cells must be positive".to_owned());
    }
    if !(0.0 < parsed.alpha && parsed.alpha < 1.0) {
        return Err("--alpha must be in (0, 1)".to_owned());
    }
    Ok(parsed)
}

fn parse<T: std::str::FromStr>(
    index: &mut usize,
    value: &impl Fn(&mut usize) -> Result<String, String>,
) -> Result<T, String> {
    value(index)?
        .parse()
        .map_err(|_| "invalid numeric value".to_owned())
}

fn evaluate(args: &Args) -> Result<Value, String> {
    let gate = args
        .state_file
        .as_deref()
        .map(load_state_gate)
        .transpose()?;
    let z = inverse_normal_cdf(1.0 - args.alpha / (2.0 * args.primary_cells as f64));
    let corrected_alpha = args.alpha / args.primary_cells as f64;
    let mut bundles = Vec::new();
    for (index, bundle) in args.bundles.iter().enumerate() {
        bundles.push(evaluate_bundle(index, bundle, args, gate.as_ref())?);
    }
    if bundles.is_empty() {
        return Err("provide --bundle or both --predictions and --readings".to_owned());
    }
    let timeframes: BTreeSet<i64> = bundles
        .iter()
        .map(|bundle| bundle.rows.first().map_or(0, |row| row.timeframe))
        .collect();
    let timeframe = *timeframes.iter().next().unwrap_or(&0);
    let entries: Vec<Entry> = bundles
        .iter()
        .flat_map(|bundle| bundle.entries.clone())
        .collect();
    let unconditional = evaluate_metrics(&bundles, &entries, args, z, false);
    let unconditional_json = metrics_json(&unconditional);
    let (metrics, state_increment, letter, passed, criteria) = if gate.is_some() {
        let state = evaluate_metrics(&bundles, &entries, args, z, true);
        let increment = state
            .lift
            .zip(unconditional.lift)
            .map(|(state, base)| state - base);
        let (letter, passed, criteria) = grade(&state, args, corrected_alpha, increment, true);
        (state, increment, letter, passed, criteria)
    } else {
        let (letter, passed, criteria) = grade(&unconditional, args, corrected_alpha, None, false);
        (unconditional, None, letter, passed, criteria)
    };
    let spec = read_provenance(&args.bundles[0].3);
    let spec_id = args
        .spec_id
        .clone()
        .or_else(|| spec.get("id").and_then(Value::as_str).map(str::to_owned));
    let family = args.family.clone().or_else(|| {
        spec.get("family")
            .and_then(Value::as_str)
            .map(str::to_owned)
    });
    let instrument = if args.bundles.len() == 1 {
        args.bundles[0].0.clone()
    } else {
        "pooled".to_owned()
    };
    let mut limitations = vec![
        "Prediction evaluation only; no execution, position sizing, or P&L.".to_owned(),
        if gate.is_some() {
            "State metrics use a direction-matched base within the declared state; the unconditional metrics use the whole partition.".to_owned()
        } else {
            "Base rate is direction-matched and unconditional within the evaluated partition."
                .to_owned()
        },
        "Overlapping outcomes are clustered by entry bar with a horizon gap.".to_owned(),
        "The forward outcome is joined after the model run; it never enters the model.".to_owned(),
    ];
    if gate.is_some() {
        limitations.push(
            "The state gate is applied at the prediction bar; no later state is read.".to_owned(),
        );
        limitations.push("A state-gated cell promotes only at grade A.".to_owned());
    }
    if args.bundles.len() == 1 {
        limitations
            .push("A single instrument cannot satisfy the replication criterion.".to_owned());
    }
    if args.random_draws == 0 || args.shuffle_draws == 0 {
        limitations.push("One or both controls were disabled by zero draws.".to_owned());
    }
    let mut criteria_value = criteria;
    if gate.is_some() {
        criteria_value.insert(
            "min_state_increment".to_owned(),
            json!(args.min_state_increment),
        );
    }
    Ok(json!({
        "schema": SCHEMA, "spec_id": spec_id, "family": family, "instrument": instrument,
        "timeframe_secs": timeframe, "partition": args.partition,
        "split_start": args.split_start, "split_end": args.split_end,
        "horizon": args.horizon, "outcome": args.outcome,
        "target_atr": if args.outcome == "barrier" { Some(args.target_atr) } else { None },
        "stop_atr": if args.outcome == "barrier" { Some(args.stop_atr) } else { None },
        "cost_bps": args.cost_bps, "primary_cells": args.primary_cells, "alpha": args.alpha,
        "corrected_alpha": corrected_alpha, "z": z, "seed": args.seed,
        "random_draws": args.random_draws, "shuffle_draws": args.shuffle_draws,
        "back_lag": args.back_lag.unwrap_or(args.horizon),
        "criteria": {
            "min_events": args.min_events, "min_clusters": args.min_clusters,
            "min_lift": args.min_lift, "min_instrument_events": args.min_instrument_events,
            "min_instruments": args.min_instruments, "control_alpha": corrected_alpha
        },
        "criteria_status": criteria_value,
        "state_gate": gate.as_ref().map(|gate| gate.raw.clone()),
        "state_increment": state_increment,
        "unconditional_metrics": if gate.is_some() { Some(unconditional_json) } else { None },
        "metrics": metrics_json(&metrics),
        "grade": letter, "verdict": if passed { "pass" } else { "fail" },
        "limitations": limitations,
        "bundles": args.bundles.iter().map(|(name, prediction, readings, _)| json!({
            "name": name, "predictions": prediction.display().to_string(), "readings": readings.display().to_string()
        })).collect::<Vec<_>>(),
    }))
}

fn evaluate_bundle(
    bundle_index: usize,
    bundle: &(String, PathBuf, PathBuf, Option<PathBuf>),
    args: &Args,
    gate: Option<&StateGate>,
) -> Result<BundleData, String> {
    let predictions = read_predictions(&bundle.1)?;
    if predictions.is_empty() {
        return Err(format!("{}: no prediction rows", bundle.1.display()));
    }
    let timeframe = args
        .timeframe
        .or_else(|| predictions.first().map(|row| row.timeframe))
        .unwrap_or(0);
    let predictions: Vec<Prediction> = predictions
        .into_iter()
        .filter(|row| row.timeframe == timeframe)
        .collect();
    if predictions.is_empty() {
        return Err(format!("{}: no rows for {timeframe}s", bundle.1.display()));
    }
    let rows: Vec<Row> = read_readings(&bundle.2)?
        .into_iter()
        .filter(|row| row.timeframe == timeframe)
        .collect();
    if rows.is_empty() {
        return Err(format!(
            "{}: no readings for {timeframe}s",
            bundle.2.display()
        ));
    }
    if let Some(gate) = gate {
        validate_state_columns(&rows, gate)?;
    }
    let index_by_bar: BTreeMap<i64, usize> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.bar, index))
        .collect();
    let split_start = parse_split(args.split_start.as_deref(), false);
    let split_end = parse_split(args.split_end.as_deref(), true);
    let base: BTreeMap<i64, BaseStats> = [-1, 1]
        .into_iter()
        .map(|direction| {
            (
                direction,
                base_statistics(&rows, direction, args, split_start, split_end, None),
            )
        })
        .collect();
    let state_base: Option<BTreeMap<i64, BaseStats>> = gate.map(|gate| {
        [-1, 1]
            .into_iter()
            .map(|direction| {
                (
                    direction,
                    base_statistics(&rows, direction, args, split_start, split_end, Some(gate)),
                )
            })
            .collect()
    });
    let mut entries = Vec::new();
    let mut raw = 0;
    let mut purged = 0;
    for prediction in &predictions {
        if prediction.prediction == 0 {
            continue;
        }
        raw += 1;
        let Some(index) = index_by_bar.get(&prediction.bar).copied() else {
            continue;
        };
        let direction = prediction.prediction;
        let Some(outcome) = outcome_for(&rows, index, direction, args) else {
            continue;
        };
        let mut hit_by_direction = BTreeMap::new();
        let mut valid_sides = true;
        for side in [-1, 1] {
            let Some(side_outcome) = outcome_for(&rows, index, side, args) else {
                valid_sides = false;
                break;
            };
            hit_by_direction.insert(side, side_outcome.hit);
        }
        if !valid_sides {
            continue;
        }
        let timestamp = prediction.timestamp.or(rows[index].timestamp);
        if let Some(timestamp) = timestamp
            && (split_start.is_some_and(|start| timestamp < start)
                || split_end.is_some_and(|end| timestamp > end))
        {
            purged += 1;
            continue;
        }
        if let Some(exit_timestamp) = rows[outcome.exit_index].timestamp
            && split_end.is_some_and(|end| exit_timestamp > end)
        {
            purged += 1;
            continue;
        }
        let year = timestamp.and_then(year_of);
        let fold_base = year.and_then(|year| {
            base.get(&direction).and_then(|base| {
                base.by_year
                    .get(&year)
                    .map(|(hits, valid)| *hits as f64 / *valid as f64)
            })
        });
        let state_ok = gate.is_none_or(|gate| state_matches(&rows[index], gate));
        let state_fold_base = state_base.as_ref().and_then(|state_base| {
            year.and_then(|year| {
                state_base.get(&direction).and_then(|base| {
                    base.by_year
                        .get(&year)
                        .map(|(hits, valid)| *hits as f64 / *valid as f64)
                })
            })
        });
        entries.push(Entry {
            bundle: bundle_index,
            bar: prediction.bar,
            timestamp,
            direction,
            outcome: outcome.clone(),
            base_rate: base.get(&direction).and_then(|base| base.rate),
            fold_base_rate: fold_base,
            hit_by_direction,
            state_ok,
            state_base_rate: state_base
                .as_ref()
                .and_then(|state_base| state_base.get(&direction).and_then(|base| base.rate)),
            state_fold_base_rate: state_fold_base,
        });
    }
    Ok(BundleData {
        name: bundle.0.clone(),
        rows,
        index_by_bar,
        entries,
        base,
        raw_predictions: raw,
        purged,
        state_base,
        state_gate: gate.cloned(),
    })
}

fn metrics_json(metrics: &Metrics) -> Value {
    json!({
        "scope": metrics.scope, "n": metrics.n, "predictions": metrics.predictions,
        "coverage": metrics.coverage, "purged": metrics.purged, "clusters": metrics.clusters,
        "hits": metrics.hits, "hit_rate": metrics.hit_rate, "base_rate": metrics.base_rate,
        "lift": metrics.lift, "lift_lcb": metrics.lift_lcb, "lcb_method": metrics.lcb_method,
        "cluster_se": metrics.cluster_se, "random_mean": metrics.random_mean,
        "random_p95": metrics.random_p95, "random_p": metrics.random_p,
        "shuffle_mean": metrics.shuffle_mean, "shuffle_p95": metrics.shuffle_p95,
        "shuffle_p": metrics.shuffle_p, "back_lag_rate": metrics.back_lag_rate,
        "back_lag_n": metrics.back_lag_n, "back_lag_lift": metrics.back_lag_lift,
        "replication": metrics.replication, "instruments": metrics.instruments,
        "folds": metrics.folds,
    })
}

fn grade(
    metrics: &Metrics,
    args: &Args,
    corrected_alpha: f64,
    state_increment: Option<f64>,
    require_replication: bool,
) -> (String, bool, Map<String, Value>) {
    let sample_ok = metrics.n >= args.min_events && metrics.clusters >= args.min_clusters;
    let lift_ok = metrics.lift.is_some_and(|lift| lift >= args.min_lift)
        && metrics.lift_lcb.is_some_and(|lcb| lcb > 0.0);
    let controls_ok = metrics
        .random_p
        .is_some_and(|value| value <= corrected_alpha)
        && metrics
            .shuffle_p
            .is_some_and(|value| value <= corrected_alpha);
    let back_ok = metrics.back_lag_lift.is_some_and(|value| {
        value <= 0.01 || metrics.lift.is_some_and(|lift| lift - value >= 0.01)
    });
    let replication_ok = metrics.replication >= args.min_instruments;
    let state_ok = state_increment.is_none_or(|value| value >= args.min_state_increment);
    let mut criteria = Map::new();
    criteria.insert("sample".to_owned(), json!(sample_ok));
    criteria.insert("lift".to_owned(), json!(lift_ok));
    criteria.insert("controls".to_owned(), json!(controls_ok));
    criteria.insert("back_lag".to_owned(), json!(back_ok));
    criteria.insert("replication".to_owned(), json!(replication_ok));
    if state_increment.is_some() {
        criteria.insert("state_increment".to_owned(), json!(state_ok));
    }
    if !sample_ok || metrics.lift.is_none_or(|lift| lift <= 0.0) {
        return ("D".to_owned(), false, criteria);
    }
    if !(lift_ok && controls_ok && back_ok && state_ok) {
        return ("C".to_owned(), false, criteria);
    }
    if !replication_ok {
        return ("B".to_owned(), !require_replication, criteria);
    }
    ("A".to_owned(), true, criteria)
}

// The remaining helpers are split into the evaluator submodule.
include!("model_prediction/eval_body.inc");

fn read_provenance(path: &Option<PathBuf>) -> Value {
    path.as_ref()
        .and_then(|path| store::read_json(path).ok())
        .and_then(|document| document.get("spec").cloned())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

#[cfg(test)]
mod tests;
