//! Loading, evaluation, controls, and grading for path validation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{Bundle, CellRow, Event, InstrumentStat, Options, Reading, Selected, StateKey};
use crate::research::eval;
use crate::research::random::Random;

const TIMEFRAME_CODES: [(i64, &str); 4] = [(60, "1m"), (300, "5m"), (900, "15m"), (3600, "1h")];
const EMA_COLUMNS: [(i64, &str); 3] = [(1, "ema_fast"), (2, "ema_mid"), (3, "ema_slow")];
const MIN_EVENTS: usize = 200;
const MIN_CLUSTERS: usize = 20;
const MIN_LIFT: f64 = 0.01;
const MIN_INCREMENTAL_LIFT: f64 = 0.01;

pub(super) fn load_bundle(
    cell: &Value,
    families: &[Value],
    options: &Options,
    timeframes: &BTreeSet<i64>,
) -> Result<Bundle, String> {
    let run = PathBuf::from(
        cell.get("run")
            .and_then(Value::as_str)
            .ok_or_else(|| "cell has no run".to_owned())?,
    );
    let mut bundle = Bundle {
        instrument: string(cell, "instrument"),
        config: string(cell, "config"),
        bases: BTreeMap::new(),
        hits: BTreeMap::new(),
        valid: BTreeMap::new(),
        states: BTreeMap::new(),
        selected: BTreeMap::new(),
        raw: BTreeMap::new(),
        prepared: BTreeMap::new(),
        signature: BTreeMap::new(),
    };
    let rows_by_tf = read_rows(&run.join("readings.csv"))?;
    let events_by_tf = read_events(&run.join("events.csv"))?;
    for timeframe in timeframes {
        let rows = rows_by_tf.get(timeframe).cloned().unwrap_or_default();
        if rows.is_empty() {
            continue;
        }
        let hits = barrier_arrays(&rows, options.horizon, options.target_atr, options.stop_atr);
        let mut bases = BTreeMap::new();
        let mut valid = BTreeMap::new();
        for direction in [-1_i64, 1_i64] {
            let values = &hits[&direction];
            bases.insert(direction, counts(values));
            valid.insert(
                direction,
                values
                    .iter()
                    .enumerate()
                    .filter_map(|(index, hit)| hit.map(|_| index))
                    .collect::<Vec<_>>(),
            );
        }
        bundle.hits.insert(*timeframe, hits);
        bundle.bases.insert(*timeframe, bases);
        bundle.valid.insert(*timeframe, valid);
        bundle
            .states
            .insert(*timeframe, state_bases(&rows, &bundle.hits[timeframe]));
        for family in families {
            let family_id = string(family, "id");
            let (prepared, raw) = select_events(
                events_by_tf
                    .get(timeframe)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                &rows,
                family,
            );
            let prepared_count = prepared.len();
            let mut selected = Vec::new();
            let mut signature = Vec::new();
            for event in prepared {
                let direction = event.direction;
                let index = event.available;
                let hit = bundle.hits[timeframe][&direction][index as usize];
                let Some(hit) = hit else {
                    continue;
                };
                let state_kind = string(family, "state");
                let state_applicable = state_kind != "none";
                let state = if state_applicable {
                    state_key(&state_kind, &rows[index as usize], direction, event.source)
                } else {
                    None
                };
                selected.push(Selected {
                    bar: index,
                    direction,
                    hit,
                    state,
                    state_applicable,
                });
                signature.push((index, direction, event.source));
            }
            let mut sorted_signature = signature;
            sorted_signature.sort();
            let serialized = sorted_signature
                .iter()
                .map(|(a, b, c)| format!("[{a}, {b}, {c}]"))
                .collect::<Vec<_>>()
                .join(", ");
            bundle
                .selected
                .insert((family_id.clone(), *timeframe), selected);
            bundle.raw.insert((family_id.clone(), *timeframe), raw);
            bundle
                .prepared
                .insert((family_id.clone(), *timeframe), prepared_count);
            bundle
                .signature
                .insert((family_id, *timeframe), hex(&format!("[{serialized}]")));
        }
    }
    Ok(bundle)
}

fn read_rows(path: &Path) -> Result<BTreeMap<i64, Vec<Reading>>, String> {
    let (header, rows) = read_csv(path)?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let Some(tf) = index("timeframe_secs") else {
        return Ok(BTreeMap::new());
    };
    let (Some(bar), Some(high), Some(low), Some(close)) =
        (index("bar"), index("high"), index("low"), index("close"))
    else {
        return Ok(BTreeMap::new());
    };
    let atr = index("atr");
    let atr_ratio = index("atr_ratio");
    let rsi = index("rsi");
    let adx_regime = index("adx_regime");
    let adx_slope = index("adx_slope");
    let di_state = index("di_state");
    let ema_stack = index("ema_stack");
    let macd_state = index("macd_state");
    let ema_fast = index("ema_fast");
    let ema_mid = index("ema_mid");
    let ema_slow = index("ema_slow");
    let mut series: BTreeMap<i64, Vec<Reading>> = BTreeMap::new();
    for row in rows {
        let Ok(timeframe) = row.get(tf).map_or("", String::as_str).parse::<i64>() else {
            continue;
        };
        if !TIMEFRAME_CODES.iter().any(|(code, _)| *code == timeframe) {
            continue;
        }
        let (Ok(bar), Ok(high), Ok(low), Ok(close)) = (
            row.get(bar).map_or("", String::as_str).parse::<i64>(),
            row.get(high).map_or("", String::as_str).parse::<f64>(),
            row.get(low).map_or("", String::as_str).parse::<f64>(),
            row.get(close).map_or("", String::as_str).parse::<f64>(),
        ) else {
            continue;
        };
        let field = |index: Option<usize>| {
            index
                .and_then(|index| row.get(index))
                .and_then(|value| number(value))
        };
        series.entry(timeframe).or_default().push(Reading {
            bar,
            high,
            low,
            close,
            atr: field(atr),
            atr_ratio: field(atr_ratio),
            rsi: field(rsi),
            adx_regime: field(adx_regime),
            adx_slope: field(adx_slope),
            di_state: field(di_state),
            ema_stack: field(ema_stack),
            macd_state: field(macd_state),
            ema_fast: field(ema_fast),
            ema_mid: field(ema_mid),
            ema_slow: field(ema_slow),
        });
    }
    for rows in series.values_mut() {
        rows.sort_by_key(|row| row.bar);
    }
    Ok(series)
}

fn read_events(path: &Path) -> Result<BTreeMap<i64, Vec<Event>>, String> {
    let (header, rows) = read_csv(path)?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let Some(tf) = index("timeframe_secs") else {
        return Ok(BTreeMap::new());
    };
    let Some(event_index) = index("event_index") else {
        return Ok(BTreeMap::new());
    };
    let (Some(event_type), Some(subtype), Some(source), Some(available), Some(direction)) = (
        index("event_type_code"),
        index("event_subtype_code"),
        index("source_code"),
        index("available_bar_index"),
        index("direction_code"),
    ) else {
        return Ok(BTreeMap::new());
    };
    let reference = index("reference_value");
    let primary = index("primary_value");
    let mut events: BTreeMap<i64, Vec<Event>> = BTreeMap::new();
    for row in rows {
        let Ok(timeframe) = row.get(tf).map_or("", String::as_str).parse::<i64>() else {
            continue;
        };
        if !TIMEFRAME_CODES.iter().any(|(code, _)| *code == timeframe) {
            continue;
        }
        let parse_i = |index: usize| {
            row.get(index)
                .map_or("", String::as_str)
                .parse::<i64>()
                .ok()
        };
        let (
            Some(index),
            Some(event_type),
            Some(subtype),
            Some(source),
            Some(available),
            Some(direction),
        ) = (
            parse_i(event_index),
            parse_i(event_type),
            parse_i(subtype),
            parse_i(source),
            parse_i(available),
            parse_i(direction),
        )
        else {
            continue;
        };
        events.entry(timeframe).or_default().push(Event {
            index,
            event_type,
            subtype,
            source,
            available,
            direction,
            reference: reference
                .and_then(|index| row.get(index))
                .and_then(|value| number(value)),
            primary: primary
                .and_then(|index| row.get(index))
                .and_then(|value| number(value)),
        });
    }
    for values in events.values_mut() {
        values.sort_by_key(|event| (event.available, event.index));
    }
    Ok(events)
}

pub(super) fn path_hit(
    rows: &[Reading],
    index: i64,
    direction: i64,
    horizon: i64,
    target_atr: f64,
    stop_atr: f64,
) -> Option<bool> {
    if index < 0 || index as usize >= rows.len() {
        return None;
    }
    let entry = &rows[index as usize];
    let atr = entry.atr?;
    if atr <= 0.0 || index + horizon >= rows.len() as i64 {
        return None;
    }
    let target = entry.close + direction as f64 * target_atr * atr;
    let stop = entry.close - direction as f64 * stop_atr * atr;
    for offset in 1..=horizon {
        let row = &rows[(index + offset) as usize];
        if direction > 0 {
            if row.low <= stop {
                return Some(false);
            }
            if row.high >= target {
                return Some(true);
            }
        } else {
            if row.high >= stop {
                return Some(false);
            }
            if row.low <= target {
                return Some(true);
            }
        }
    }
    Some(false)
}

fn barrier_arrays(
    rows: &[Reading],
    horizon: i64,
    target_atr: f64,
    stop_atr: f64,
) -> BTreeMap<i64, Vec<Option<bool>>> {
    let mut arrays = BTreeMap::new();
    for direction in [-1_i64, 1_i64] {
        arrays.insert(
            direction,
            (0..rows.len() as i64)
                .map(|index| path_hit(rows, index, direction, horizon, target_atr, stop_atr))
                .collect(),
        );
    }
    arrays
}

pub(super) fn counts(hits: &[Option<bool>]) -> (usize, usize) {
    let valid: Vec<bool> = hits.iter().filter_map(|hit| *hit).collect();
    (valid.iter().filter(|hit| **hit).count(), valid.len())
}

fn family_matches(family: &Value, event: &Event, row: &Reading) -> bool {
    if event.event_type
        != family
            .get("event_type")
            .and_then(Value::as_i64)
            .unwrap_or(0)
        || event.subtype != family.get("subtype").and_then(Value::as_i64).unwrap_or(0)
    {
        return false;
    }
    if let Some(source) = family.get("source").and_then(Value::as_i64)
        && event.source != source
    {
        return false;
    }
    if let Some(sources) = family.get("sources").and_then(Value::as_array)
        && !sources
            .iter()
            .any(|value| value.as_i64() == Some(event.source))
    {
        return false;
    }
    if let Some(reference) = family.get("reference_value").and_then(Value::as_f64)
        && event
            .reference
            .is_none_or(|value| (value - reference).abs() > 1e-9)
    {
        return false;
    }
    if let Some(minimum) = family.get("primary_min").and_then(Value::as_f64)
        && event.primary.is_none_or(|value| value < minimum)
    {
        return false;
    }
    if let Some(maximum) = family.get("atr_ratio_max").and_then(Value::as_f64)
        && row.atr_ratio.is_none_or(|value| value > maximum)
    {
        return false;
    }
    true
}

fn select_events(events: &[Event], rows: &[Reading], family: &Value) -> (Vec<Event>, usize) {
    let mut raw = 0;
    let mut by_bar: BTreeMap<i64, Option<Event>> = BTreeMap::new();
    for event in events {
        if event.direction != -1 && event.direction != 1 {
            continue;
        }
        if event.available < 0 || event.available as usize >= rows.len() {
            continue;
        }
        if !family_matches(family, event, &rows[event.available as usize]) {
            continue;
        }
        raw += 1;
        match by_bar.get(&event.available) {
            None => {
                by_bar.insert(event.available, Some(event.clone()));
            }
            Some(Some(existing)) if existing.direction != event.direction => {
                by_bar.insert(event.available, None);
            }
            _ => {}
        }
    }
    (by_bar.into_values().flatten().collect(), raw)
}

fn state_key(kind: &str, row: &Reading, direction: i64, source: i64) -> Option<StateKey> {
    match kind {
        "trend" => {
            if row.adx_regime == Some(1.0)
                && row.adx_slope == Some(1.0)
                && row.di_state == Some(direction as f64)
                && row.ema_stack == Some(direction as f64)
                && row.macd_state == Some(direction as f64)
            {
                Some(("trend".to_owned(), direction, None))
            } else {
                None
            }
        }
        "macd_state" => (row.macd_state == Some(direction as f64))
            .then(|| ("macd_state".to_owned(), direction, None)),
        "rsi_side" => row
            .rsi
            .filter(|rsi| (rsi - 50.0) * direction as f64 > 0.0)
            .map(|_| ("rsi_side".to_owned(), direction, None)),
        "ema_side" => {
            let column = EMA_COLUMNS
                .iter()
                .find(|(code, _)| *code == source)
                .map(|(_, name)| *name);
            column
                .and_then(|column| match column {
                    "ema_fast" => row.ema_fast,
                    "ema_mid" => row.ema_mid,
                    _ => row.ema_slow,
                })
                .filter(|ema| (row.close - ema) * direction as f64 > 0.0)
                .map(|_| ("ema_side".to_owned(), direction, Some(source)))
        }
        _ => None,
    }
}

fn state_bases(
    rows: &[Reading],
    hits: &BTreeMap<i64, Vec<Option<bool>>>,
) -> BTreeMap<StateKey, (usize, usize)> {
    let mut totals: BTreeMap<StateKey, (usize, usize)> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        let mut add = |key: StateKey, direction: i64| {
            if let Some(hit) = hits
                .get(&direction)
                .and_then(|values| values.get(index))
                .copied()
                .flatten()
            {
                let entry = totals.entry(key).or_insert((0, 0));
                entry.0 += usize::from(hit);
                entry.1 += 1;
            }
        };
        if let Some(di) = row.di_state.map(|value| value as i64)
            && (di == -1 || di == 1)
            && row.adx_regime == Some(1.0)
            && row.adx_slope == Some(1.0)
            && row.ema_stack == Some(di as f64)
            && row.macd_state == Some(di as f64)
        {
            add(("trend".to_owned(), di, None), di);
        }
        if let Some(macd) = row.macd_state.map(|value| value as i64)
            && (macd == -1 || macd == 1)
        {
            add(("macd_state".to_owned(), macd, None), macd);
        }
        if let Some(rsi) = row.rsi
            && rsi != 50.0
        {
            let direction = if rsi > 50.0 { 1 } else { -1 };
            add(("rsi_side".to_owned(), direction, None), direction);
        }
        for (source, value) in [(1_i64, row.ema_fast), (2, row.ema_mid), (3, row.ema_slow)] {
            if let Some(ema) = value
                && row.close != ema
            {
                let direction = if row.close > ema { 1 } else { -1 };
                add(("ema_side".to_owned(), direction, Some(source)), direction);
            }
        }
    }
    totals
}

pub(super) fn evaluate(
    bundles: &[&Bundle],
    families: &[Value],
    timeframes: &BTreeSet<i64>,
    options: &Options,
    z: f64,
) -> Vec<CellRow> {
    let mut rows = Vec::new();
    for family in families {
        let family_id = string(family, "id");
        for timeframe in timeframes {
            let selected: Vec<(usize, &Selected)> = bundles
                .iter()
                .enumerate()
                .flat_map(|(index, bundle)| {
                    bundle
                        .selected
                        .get(&(family_id.clone(), *timeframe))
                        .into_iter()
                        .flatten()
                        .map(move |event| (index, event))
                })
                .collect();
            let entries: Vec<(usize, i64, bool)> = selected
                .iter()
                .map(|(bundle, event)| (*bundle, event.bar, event.hit))
                .collect();
            let stats = eval::cluster_stats(&entries, options.horizon);
            let n = stats.n;
            let base = event_base(bundles, *timeframe, &selected);
            let (state, state_missing) = state_base(bundles, *timeframe, &selected);
            let state_applicable = string(family, "state") != "none";
            let p = stats.hit_rate;
            let se = stats.se;
            let lift = p.zip(base).map(|(p, base)| p - base);
            let incremental = p.zip(state).map(|(p, state)| p - state);
            let random_controls = random_draws(bundles, *timeframe, &selected, options);
            let shuffled_controls = shuffle_draws(bundles, *timeframe, &selected, options);
            let back_rate = lagged_rate(bundles, *timeframe, &selected, -options.back_lag);
            let forward_rate = lagged_rate(bundles, *timeframe, &selected, options.back_lag);
            let back_lift = back_rate.zip(base).map(|(rate, base)| rate - base);
            let forward_lift = forward_rate.zip(base).map(|(rate, base)| rate - base);
            let instruments = instrument_stats(bundles, *timeframe, &selected);
            let replication = instruments
                .values()
                .filter(|value| {
                    value.n >= 30
                        && value
                            .hit_rate
                            .zip(value.base)
                            .is_some_and(|(rate, base)| rate > base)
                })
                .count();
            let signatures: BTreeSet<String> = bundles
                .iter()
                .filter_map(|bundle| {
                    bundle
                        .signature
                        .get(&(family_id.clone(), *timeframe))
                        .cloned()
                })
                .collect();
            let signature_text = signatures
                .iter()
                .map(|value| format!("\"{value}\""))
                .collect::<Vec<_>>()
                .join(", ");
            rows.push(CellRow {
                family: family_id.clone(),
                label: family
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(&family_id)
                    .to_owned(),
                timeframe: TIMEFRAME_CODES
                    .iter()
                    .find(|(code, _)| code == timeframe)
                    .map_or_else(|| timeframe.to_string(), |(_, name)| (*name).to_owned()),
                timeframe_secs: *timeframe,
                config: bundles
                    .first()
                    .map_or_else(String::new, |bundle| bundle.config.clone()),
                raw: bundles
                    .iter()
                    .map(|bundle| {
                        bundle
                            .raw
                            .get(&(family_id.clone(), *timeframe))
                            .copied()
                            .unwrap_or(0)
                    })
                    .sum(),
                prepared: bundles
                    .iter()
                    .map(|bundle| {
                        bundle
                            .prepared
                            .get(&(family_id.clone(), *timeframe))
                            .copied()
                            .unwrap_or(0)
                    })
                    .sum(),
                n,
                clusters: stats.clusters,
                singleton_bundles: stats.singleton_bundles,
                hits: selected.iter().filter(|(_, event)| event.hit).count(),
                hit_rate: p,
                base_rate: base,
                lift,
                lift_lcb: eval::lower(lift, se, z),
                state_applicable,
                state_rate: state,
                state_missing,
                incremental_lift: incremental,
                incremental_lcb: eval::lower(incremental, se, z),
                cluster_se: se,
                random_mean: mean_valid(&random_controls),
                random_p95: eval::percentile(&random_controls, 0.95),
                random_p: p.and_then(|p| eval::control_p(p, &random_controls)),
                shuffle_mean: mean_valid(&shuffled_controls),
                shuffle_p95: eval::percentile(&shuffled_controls, 0.95),
                shuffle_p: p.and_then(|p| eval::control_p(p, &shuffled_controls)),
                back_lag_rate: back_rate,
                back_lag_n: lagged_n(bundles, *timeframe, &selected, -options.back_lag),
                back_lag_lift: back_lift,
                forward_lag_rate: forward_rate,
                forward_lag_n: lagged_n(bundles, *timeframe, &selected, options.back_lag),
                forward_lag_lift: forward_lift,
                replication,
                instruments,
                event_signature: hex(&format!("[{signature_text}]")),
                grade: String::new(),
                reasons: Vec::new(),
                sensitivity_ok: None,
                sensitivity_reason: None,
            });
        }
    }
    rows
}

fn mean_valid(values: &[Option<f64>]) -> Option<f64> {
    let valid: Vec<f64> = values.iter().filter_map(|value| *value).collect();
    if valid.is_empty() {
        None
    } else {
        Some(valid.iter().sum::<f64>() / valid.len() as f64)
    }
}

fn event_base(bundles: &[&Bundle], timeframe: i64, selected: &[(usize, &Selected)]) -> Option<f64> {
    if selected.is_empty() {
        return None;
    }
    let mut total = 0.0;
    let mut count = 0;
    for (bundle, event) in selected {
        let (hits, valid) = bundles[*bundle].bases[&timeframe][&event.direction];
        if valid == 0 {
            continue;
        }
        total += hits as f64 / valid as f64;
        count += 1;
    }
    (count > 0).then(|| total / count as f64)
}

fn state_base(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
) -> (Option<f64>, usize) {
    let mut total = 0.0;
    let mut count = 0;
    let mut missing = 0;
    for (bundle, event) in selected {
        if !event.state_applicable {
            continue;
        }
        let Some(state) = &event.state else {
            missing += 1;
            continue;
        };
        let (hits, valid) = bundles[*bundle].states[&timeframe]
            .get(state)
            .copied()
            .unwrap_or((0, 0));
        if valid == 0 {
            missing += 1;
            continue;
        }
        total += hits as f64 / valid as f64;
        count += 1;
    }
    ((count > 0).then(|| total / count as f64), missing)
}

fn random_draws(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
    options: &Options,
) -> Vec<Option<f64>> {
    let mut by_bundle: BTreeMap<usize, Vec<&Selected>> = BTreeMap::new();
    for (bundle, event) in selected {
        by_bundle.entry(*bundle).or_default().push(*event);
    }
    let mut rng = Random::new(options.seed);
    let mut draws = Vec::new();
    for _ in 0..options.random_draws {
        let mut hits = 0;
        let mut total = 0;
        for (bundle_index, values) in &by_bundle {
            let bundle = bundles[*bundle_index];
            if bundle.valid[&timeframe][&1].is_empty() {
                continue;
            }
            let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
            for event in values {
                *counts.entry(event.direction).or_default() += 1;
            }
            for (direction, count) in counts {
                let pool = &bundle.valid[&timeframe][&direction];
                if pool.is_empty() || count == 0 {
                    continue;
                }
                for bar in rng.sample(pool, count.min(pool.len())) {
                    if let Some(hit) = bundle.hits[&timeframe][&direction][bar] {
                        hits += usize::from(hit);
                        total += 1;
                    }
                }
            }
        }
        draws.push((total > 0).then(|| hits as f64 / total as f64));
    }
    draws
}

fn shuffle_draws(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
    options: &Options,
) -> Vec<Option<f64>> {
    let mut by_bundle: BTreeMap<usize, Vec<&Selected>> = BTreeMap::new();
    for (bundle, event) in selected {
        by_bundle.entry(*bundle).or_default().push(*event);
    }
    let mut rng = Random::new(options.seed + 1);
    let mut draws = Vec::new();
    for _ in 0..options.shuffle_draws {
        let mut hits = 0;
        let mut total = 0;
        for (bundle_index, values) in &by_bundle {
            let mut directions: Vec<i64> = values.iter().map(|event| event.direction).collect();
            rng.shuffle(&mut directions);
            for (event, direction) in values.iter().zip(directions) {
                if let Some(hit) =
                    bundles[*bundle_index].hits[&timeframe][&direction][event.bar as usize]
                {
                    hits += usize::from(hit);
                    total += 1;
                }
            }
        }
        draws.push((total > 0).then(|| hits as f64 / total as f64));
    }
    draws
}

fn lagged_rate(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
    lag: i64,
) -> Option<f64> {
    let (hits, total) = lagged_counts(bundles, timeframe, selected, lag);
    (total > 0).then(|| hits as f64 / total as f64)
}

fn lagged_n(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
    lag: i64,
) -> usize {
    lagged_counts(bundles, timeframe, selected, lag).1
}

fn lagged_counts(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
    lag: i64,
) -> (usize, usize) {
    let mut hits = 0;
    let mut total = 0;
    for (bundle, event) in selected {
        let bar = event.bar + lag;
        let array = &bundles[*bundle].hits[&timeframe][&event.direction];
        if bar < 0 || bar as usize >= array.len() {
            continue;
        }
        if let Some(hit) = array[bar as usize] {
            hits += usize::from(hit);
            total += 1;
        }
    }
    (hits, total)
}

fn instrument_stats(
    bundles: &[&Bundle],
    timeframe: i64,
    selected: &[(usize, &Selected)],
) -> BTreeMap<String, InstrumentStat> {
    let mut by_instrument: BTreeMap<String, Vec<(usize, &Selected)>> = BTreeMap::new();
    for (bundle, event) in selected {
        by_instrument
            .entry(bundles[*bundle].instrument.clone())
            .or_default()
            .push((*bundle, *event));
    }
    let mut result = BTreeMap::new();
    for (instrument, values) in by_instrument {
        let hits = values.iter().filter(|(_, event)| event.hit).count();
        result.insert(
            instrument,
            InstrumentStat {
                n: values.len(),
                hits,
                hit_rate: (!values.is_empty()).then(|| hits as f64 / values.len() as f64),
                base: event_base(bundles, timeframe, &values),
            },
        );
    }
    result
}

pub(super) fn grade(row: &CellRow, primary: bool) -> (String, Vec<String>) {
    if !primary {
        return ("sensitivity".to_owned(), Vec::new());
    }
    let mut checks: Vec<(&str, bool)> = vec![
        ("n", row.n >= MIN_EVENTS),
        ("clusters", row.clusters >= MIN_CLUSTERS),
        ("lift", row.lift.is_some_and(|lift| lift >= MIN_LIFT)),
        ("lift_lcb", row.lift_lcb.is_some_and(|value| value > 0.0)),
    ];
    if row.state_applicable {
        checks.push((
            "incremental_lift",
            row.incremental_lift
                .is_some_and(|value| value >= MIN_INCREMENTAL_LIFT),
        ));
        checks.push((
            "incremental_lcb",
            row.incremental_lcb.is_some_and(|value| value > 0.0),
        ));
    }
    checks.push(("random", row.random_p.is_some_and(|value| value <= 0.05)));
    checks.push((
        "back_lag",
        row.back_lag_lift.is_some_and(|value| {
            value <= 0.01 || row.lift.is_some_and(|lift| lift - value >= 0.01)
        }),
    ));
    checks.push(("shuffle", row.shuffle_p.is_some_and(|value| value <= 0.05)));
    if row.state_applicable {
        checks.push(("state", row.state_missing == 0));
    }
    checks.push(("replication", row.replication >= 2));
    let reasons: Vec<String> = checks
        .iter()
        .filter(|(_, ok)| !ok)
        .map(|(name, _)| (*name).to_owned())
        .collect();
    if !reasons.is_empty() {
        return ("C".to_owned(), reasons);
    }
    if row.lift.is_some_and(|lift| lift >= 0.02) && row.replication >= 3 {
        ("A".to_owned(), Vec::new())
    } else {
        ("B".to_owned(), Vec::new())
    }
}

pub(super) fn apply_sensitivity(rows: &mut [CellRow], primary: &str, sensitivity: &str) {
    let index: BTreeMap<(String, i64, String), usize> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            (
                (row.family.clone(), row.timeframe_secs, row.config.clone()),
                index,
            )
        })
        .collect();
    for row_index in 0..rows.len() {
        if rows[row_index].config != primary {
            continue;
        }
        let key = (
            rows[row_index].family.clone(),
            rows[row_index].timeframe_secs,
            sensitivity.to_owned(),
        );
        let Some(other_index) = index.get(&key).copied() else {
            rows[row_index].sensitivity_ok = Some(false);
            rows[row_index].sensitivity_reason = Some("missing".to_owned());
            continue;
        };
        let (ok, reason) = if rows[row_index].family == "h1_trend_continuation" {
            (
                rows[other_index].lift.is_some_and(|lift| lift > 0.0),
                if rows[other_index].lift.is_some_and(|lift| lift > 0.0) {
                    "lift"
                } else {
                    "non_positive_lift"
                },
            )
        } else {
            let ok = rows[other_index].event_signature == rows[row_index].event_signature
                && rows[other_index].n == rows[row_index].n
                && rows[other_index].hits == rows[row_index].hits;
            (ok, if ok { "identical" } else { "event_set_changed" })
        };
        rows[row_index].sensitivity_ok = Some(ok);
        rows[row_index].sensitivity_reason = Some(reason.to_owned());
        rows[other_index].sensitivity_ok = Some(ok);
        rows[other_index].sensitivity_reason = Some(reason.to_owned());
    }
}

fn string(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned()
}
fn number(value: &str) -> Option<f64> {
    if value.is_empty() {
        None
    } else {
        value.parse().ok().filter(|value: &f64| value.is_finite())
    }
}
fn hex(value: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(value.as_bytes());
    format!("{:x}", digest.finalize())
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines
        .next()
        .unwrap_or("")
        .split(',')
        .map(|value| value.trim().trim_matches('"').to_owned())
        .collect();
    Ok((
        header,
        lines
            .map(|line| {
                line.split(',')
                    .map(|value| value.trim().trim_matches('"').to_owned())
                    .collect()
            })
            .collect(),
    ))
}
