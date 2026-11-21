// Evaluator helpers included by the model-prediction module.

// Included by `model_prediction.rs`; the evaluator helpers.

fn parse_split(value: Option<&str>, end: bool) -> Option<i64> {
    let value = value?;
    let days = crate::improvement::feasibility::parse_date_text(value)?;
    Some(days * 86_400 + if end { 86_399 } else { 0 })
}

pub(super) fn load_state_gate(path: &str) -> Result<StateGate, String> {
    let document = store::read_json(Path::new(path))?;
    if document.get("schema").and_then(Value::as_str) != Some(STATE_SCHEMA) {
        return Err(format!("state gate schema must be {STATE_SCHEMA}"));
    }
    let Some(raw_clauses) = document.get("all").and_then(Value::as_array).filter(|values| !values.is_empty()) else {
        return Err("state gate all must be a non-empty list".to_owned());
    };
    let mut clauses = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, raw) in raw_clauses.iter().enumerate() {
        let Some(raw) = raw.as_object() else {
            return Err(format!("state gate all[{index}] must be an object"));
        };
        let Some(column) = raw.get("column").and_then(Value::as_str).filter(|column| STATE_COLUMNS.contains(column)) else {
            return Err(format!("state gate all[{index}].column is not a published state"));
        };
        if !seen.insert(column.to_owned()) {
            return Err(format!("state gate column {column:?} is repeated"));
        }
        if let Some(values) = raw.get("values") {
            let values: Vec<i64> = values.as_array().into_iter().flatten().filter_map(Value::as_i64).collect();
            if values.is_empty() {
                return Err(format!("state gate all[{index}].values must be a non-empty integer list"));
            }
            clauses.push(StateClause { column: column.to_owned(), values: Some(values), min_value: None, max_value: None });
        } else {
            let minimum = raw.get("min").and_then(Value::as_i64);
            let maximum = raw.get("max").and_then(Value::as_i64);
            if minimum.is_none() && maximum.is_none() {
                return Err(format!("state gate all[{index}] needs values, min, or max"));
            }
            clauses.push(StateClause { column: column.to_owned(), values: None, min_value: minimum, max_value: maximum });
        }
    }
    Ok(StateGate { clauses, raw: document })
}

fn state_matches(row: &Row, gate: &StateGate) -> bool {
    gate.clauses.iter().all(|clause| {
        let Some(value) = row.states.get(&clause.column).and_then(|value| optional_int(value)) else {
            return false;
        };
        clause.values.as_ref().is_none_or(|values| values.contains(&value))
            && clause.min_value.is_none_or(|minimum| value >= minimum)
            && clause.max_value.is_none_or(|maximum| value <= maximum)
    })
}

fn validate_state_columns(rows: &[Row], gate: &StateGate) -> Result<(), String> {
    let Some(first) = rows.first() else { return Ok(()) };
    let missing: Vec<&str> = gate.clauses.iter().filter(|clause| !first.states.contains_key(&clause.column)).map(|clause| clause.column.as_str()).collect();
    if missing.is_empty() { Ok(()) } else { Err(format!("readings are missing state columns: {missing:?}")) }
}

fn read_predictions(path: &Path) -> Result<Vec<Prediction>, String> {
    let (header, rows) = read_csv(path)?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let (Some(tf), Some(bar), Some(prediction)) = (index("timeframe_secs"), index("bar"), index("prediction")) else {
        return Ok(Vec::new());
    };
    let timestamp = index("timestamp");
    let mut values = Vec::new();
    for row in rows {
        let (Some(timeframe), Some(bar), Some(prediction)) = (
            row.get(tf).and_then(|value| value.parse().ok()),
            row.get(bar).and_then(|value| value.parse().ok()),
            row.get(prediction).and_then(|value| value.parse().ok()),
        ) else { continue };
        if ![-1_i64, 0, 1].contains(&prediction) {
            return Err(format!("{}: prediction {prediction} is not -1, 0, or 1", path.display()));
        }
        values.push(Prediction {
            timeframe,
            bar,
            timestamp: timestamp.and_then(|index| row.get(index)).and_then(|value| optional_int(value)),
            prediction,
        });
    }
    values.sort_by_key(|value| value.bar);
    for pair in values.windows(2) {
        if pair[0].bar == pair[1].bar {
            return Err(format!("{}: duplicate prediction bar {}", path.display(), pair[1].bar));
        }
    }
    Ok(values)
}

fn read_readings(path: &Path) -> Result<Vec<Row>, String> {
    let (header, rows) = read_csv(path)?;
    let index = |name: &str| header.iter().position(|field| field == name);
    let (Some(tf), Some(bar), Some(close), Some(high), Some(low)) = (index("timeframe_secs"), index("bar"), index("close"), index("high"), index("low")) else {
        return Ok(Vec::new());
    };
    let timestamp = index("timestamp");
    let atr = index("atr");
    let mut values = Vec::new();
    for row in rows {
        let (Ok(timeframe), Ok(bar), Ok(close), Ok(high), Ok(low)) = (
            row.get(tf).map_or("", String::as_str).parse(),
            row.get(bar).map_or("", String::as_str).parse(),
            row.get(close).map_or("", String::as_str).parse(),
            row.get(high).map_or("", String::as_str).parse(),
            row.get(low).map_or("", String::as_str).parse(),
        ) else { continue };
        let mut states = BTreeMap::new();
        for column in STATE_COLUMNS {
            if let Some(column_index) = index(column)
                && let Some(value) = row.get(column_index)
            {
                states.insert(column.to_owned(), value.clone());
            }
        }
        values.push(Row {
            timeframe, bar,
            timestamp: timestamp.and_then(|index| row.get(index)).and_then(|value| optional_int(value)),
            close, high, low,
            atr: atr.and_then(|index| row.get(index)).and_then(|value| optional_float(value)),
            states,
        });
    }
    values.sort_by_key(|value| value.bar);
    Ok(values)
}

fn optional_float(value: &str) -> Option<f64> {
    if value.is_empty() { None } else { value.parse().ok().filter(|value: &f64| value.is_finite()) }
}

fn optional_int(value: &str) -> Option<i64> {
    value.parse().ok().or_else(|| value.parse::<f64>().ok().filter(|value| value.is_finite() && value.fract() == 0.0).map(|value| value as i64))
}

fn cost_r(entry: f64, atr: f64, cost_bps: f64) -> f64 {
    cost_bps / 10_000.0 * entry / atr
}

fn outcome_for(rows: &[Row], index: usize, direction: i64, args: &Args) -> Option<Outcome> {
    if args.outcome == "forward" {
        return forward_outcome(rows, index, direction, args.horizon, args.cost_bps);
    }
    barrier_outcome(rows, index, direction, args.horizon, args.target_atr, args.stop_atr, args.cost_bps)
}

fn forward_outcome(rows: &[Row], index: usize, direction: i64, horizon: i64, cost_bps: f64) -> Option<Outcome> {
    if index + horizon as usize >= rows.len() { return None; }
    let atr = rows[index].atr?;
    if atr <= 0.0 { return None; }
    let entry = rows[index].close;
    let gross = direction as f64 * (rows[index + horizon as usize].close - entry) / atr;
    let net = gross - cost_r(entry, atr, cost_bps);
    Some(Outcome { hit: net > 0.0, exit_index: index + horizon as usize })
}

fn barrier_outcome(rows: &[Row], index: usize, direction: i64, horizon: i64, target_atr: f64, stop_atr: f64, cost_bps: f64) -> Option<Outcome> {
    if index + horizon as usize >= rows.len() { return None; }
    let atr = rows[index].atr?;
    if atr <= 0.0 { return None; }
    let entry = rows[index].close;
    let target = entry + direction as f64 * target_atr * atr;
    let stop = entry - direction as f64 * stop_atr * atr;
    let cost = cost_r(entry, atr, cost_bps);
    for offset in 1..=horizon as usize {
        let row = &rows[index + offset];
        let hit_stop = if direction > 0 { row.low <= stop } else { row.high >= stop };
        if hit_stop {
            return Some(Outcome { hit: -stop_atr - cost > 0.0, exit_index: index + offset });
        }
        let hit_target = if direction > 0 { row.high >= target } else { row.low <= target };
        if hit_target {
            return Some(Outcome { hit: target_atr - cost > 0.0, exit_index: index + offset });
        }
    }
    let gross = direction as f64 * (rows[index + horizon as usize].close - entry) / atr;
    let net = gross - cost;
    Some(Outcome { hit: net > 0.0, exit_index: index + horizon as usize })
}

fn base_statistics(rows: &[Row], direction: i64, args: &Args, split_start: Option<i64>, split_end: Option<i64>, gate: Option<&StateGate>) -> BaseStats {
    let mut hits = 0;
    let mut valid = 0;
    let mut valid_indices = Vec::new();
    let mut by_year: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        if gate.is_some_and(|gate| !state_matches(row, gate)) { continue; }
        let Some(outcome) = outcome_for(rows, index, direction, args) else { continue; };
        if let Some(timestamp) = row.timestamp
            && (split_start.is_some_and(|start| timestamp < start)
                || split_end.is_some_and(|end| timestamp > end))
        {
            continue;
        }
        if let Some(exit_timestamp) = rows[outcome.exit_index].timestamp
            && split_end.is_some_and(|end| exit_timestamp > end)
        { continue; }
        valid += 1;
        valid_indices.push(index);
        hits += usize::from(outcome.hit);
        if let Some(year) = row.timestamp.and_then(year_of) {
            let entry = by_year.entry(year).or_insert((0, 0));
            entry.0 += usize::from(outcome.hit);
            entry.1 += 1;
        }
    }
    BaseStats { rate: (valid > 0).then(|| hits as f64 / valid as f64), valid_indices, by_year }
}

fn evaluate_metrics(bundles: &[BundleData], entries: &[Entry], args: &Args, z: f64, state_only: bool) -> Metrics {
    let scoped: Vec<&Entry> = if state_only { entries.iter().filter(|entry| entry.state_ok).collect() } else { entries.iter().collect() };
    let stats = eval::cluster_stats(&scoped.iter().map(|entry| (entry.bundle, entry.bar, entry.outcome.hit)).collect::<Vec<_>>(), args.horizon);
    let hits = scoped.iter().filter(|entry| entry.outcome.hit).count();
    let base = weighted_base(&scoped, state_only);
    let hit_rate = stats.hit_rate;
    let lift = hit_rate.zip(base).map(|(hit, base)| hit - base);
    let (lift_lcb, lcb_method) = if let Some(se) = stats.se {
        (eval::lower(lift, Some(se), z), "cluster")
    } else if stats.n > 0 && base.is_some() {
        let wilson = wilson_lower(hits, stats.n, z);
        (wilson.zip(base).map(|(wilson, base)| wilson - base), "wilson")
    } else {
        (None, "cluster")
    };
    let back_lag = args.back_lag.unwrap_or(args.horizon);
    let mut back_hits = 0;
    let mut back_total = 0;
    for entry in &scoped {
        let bundle = &bundles[entry.bundle];
        let Some(index) = bundle.index_by_bar.get(&(entry.bar - back_lag)).copied() else { continue; };
        if state_only
            && bundle
                .state_gate
                .as_ref()
                .is_some_and(|gate| !state_matches(&bundle.rows[index], gate))
        {
            continue;
        }
        if let Some(outcome) = outcome_for(&bundle.rows, index, entry.direction, args) {
            back_total += 1;
            back_hits += usize::from(outcome.hit);
        }
    }
    let back_rate = (back_total > 0).then(|| back_hits as f64 / back_total as f64);
    let random_draws = random_control(bundles, args, state_only);
    let shuffle_draws = shuffle_control(&scoped, args);
    let raw_predictions = bundles.iter().map(|bundle| bundle.raw_predictions).sum();
    Metrics {
        scope: if state_only { "state" } else { "unconditional" },
        n: stats.n,
        predictions: raw_predictions,
        coverage: if raw_predictions > 0 { stats.n as f64 / raw_predictions as f64 } else { 0.0 },
        purged: bundles.iter().map(|bundle| bundle.purged).sum(),
        clusters: stats.clusters,
        hits,
        hit_rate,
        base_rate: base,
        lift,
        lift_lcb,
        lcb_method,
        cluster_se: stats.se,
        random_mean: mean_optional(&random_draws),
        random_p95: eval::percentile(&random_draws, 0.95),
        random_p: hit_rate.and_then(|hit| eval::control_p(hit, &random_draws)),
        shuffle_mean: mean_optional(&shuffle_draws),
        shuffle_p95: eval::percentile(&shuffle_draws, 0.95),
        shuffle_p: hit_rate.and_then(|hit| eval::control_p(hit, &shuffle_draws)),
        back_lag_rate: back_rate,
        back_lag_n: back_total,
        back_lag_lift: back_rate.zip(base).map(|(rate, base)| rate - base),
        replication: positive_instruments(bundles, args, state_only),
        instruments: instrument_metrics(bundles, args, state_only),
        folds: fold_metrics(&scoped, args, state_only),
    }
}

fn weighted_base(entries: &[&Entry], state_only: bool) -> Option<f64> {
    let values: Vec<f64> = entries.iter().filter_map(|entry| if state_only { entry.state_base_rate } else { entry.base_rate }).collect();
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn random_control(bundles: &[BundleData], args: &Args, state_only: bool) -> Vec<Option<f64>> {
    if args.random_draws == 0 { return Vec::new(); }
    let mut rng = Random::new(args.seed);
    let mut draws = Vec::new();
    for _ in 0..args.random_draws {
        let mut hits = 0;
        let mut total = 0;
        for bundle in bundles {
            let mut counts = BTreeMap::new();
            for entry in bundle.entries.iter().filter(|entry| !state_only || entry.state_ok) {
                *counts.entry(entry.direction).or_insert(0_usize) += 1;
            }
            let base = if state_only { bundle.state_base.as_ref() } else { Some(&bundle.base) };
            let Some(base) = base else { continue; };
            for (direction, count) in counts {
                let Some(stats) = base.get(&direction) else { continue; };
                if count == 0 || stats.valid_indices.is_empty() { continue; }
                for index in rng.sample(&stats.valid_indices, count.min(stats.valid_indices.len())) {
                    if let Some(outcome) = outcome_for(&bundle.rows, index, direction, args) {
                        hits += usize::from(outcome.hit);
                        total += 1;
                    }
                }
            }
        }
        draws.push((total > 0).then(|| hits as f64 / total as f64));
    }
    draws
}

fn shuffle_control(entries: &[&Entry], args: &Args) -> Vec<Option<f64>> {
    if args.shuffle_draws == 0 { return Vec::new(); }
    let mut rng = Random::new(args.seed + 1);
    let mut by_bundle: BTreeMap<usize, Vec<&Entry>> = BTreeMap::new();
    for entry in entries { by_bundle.entry(entry.bundle).or_default().push(entry); }
    let mut draws = Vec::new();
    for _ in 0..args.shuffle_draws {
        let mut hits = 0;
        let mut total = 0;
        for values in by_bundle.values() {
            let mut directions: Vec<i64> = values.iter().map(|entry| entry.direction).collect();
            rng.shuffle(&mut directions);
            for (entry, direction) in values.iter().zip(directions) {
                if let Some(hit) = entry.hit_by_direction.get(&direction) {
                    hits += usize::from(*hit);
                    total += 1;
                }
            }
        }
        draws.push((total > 0).then(|| hits as f64 / total as f64));
    }
    draws
}

fn wilson_lower(hits: usize, n: usize, z: f64) -> Option<f64> {
    if n == 0 { return None; }
    let p = hits as f64 / n as f64;
    let denominator = 1.0 + z * z / n as f64;
    let center = p + z * z / (2.0 * n as f64);
    let margin = z * (p * (1.0 - p) / n as f64 + z * z / (4.0 * n as f64 * n as f64)).sqrt();
    Some(((center - margin) / denominator).max(0.0))
}

fn mean_optional(values: &[Option<f64>]) -> Option<f64> {
    let valid: Vec<f64> = values.iter().filter_map(|value| *value).collect();
    (!valid.is_empty()).then(|| valid.iter().sum::<f64>() / valid.len() as f64)
}

fn positive_instruments(bundles: &[BundleData], args: &Args, state_only: bool) -> usize {
    bundles.iter().filter(|bundle| instrument_metric(bundle, args, state_only)["positive"].as_bool() == Some(true)).count()
}

fn instrument_metric(bundle: &BundleData, args: &Args, state_only: bool) -> Value {
    let entries: Vec<&Entry> = bundle.entries.iter().filter(|entry| !state_only || entry.state_ok).collect();
    let stats = eval::cluster_stats(&entries.iter().map(|entry| (0, entry.bar, entry.outcome.hit)).collect::<Vec<_>>(), args.horizon);
    let base = weighted_base(&entries, state_only);
    let lift = stats.hit_rate.zip(base).map(|(hit, base)| hit - base);
    json!({
        "n": stats.n, "clusters": stats.clusters, "hit_rate": stats.hit_rate,
        "base_rate": base, "lift": lift,
        "positive": stats.n >= args.min_instrument_events && lift.is_some_and(|lift| lift > 0.0)
    })
}

fn instrument_metrics(bundles: &[BundleData], args: &Args, state_only: bool) -> BTreeMap<String, Value> {
    bundles.iter().map(|bundle| (bundle.name.clone(), instrument_metric(bundle, args, state_only))).collect()
}

fn fold_metrics(entries: &[&Entry], args: &Args, state_only: bool) -> BTreeMap<String, Value> {
    let mut grouped: BTreeMap<i64, Vec<&Entry>> = BTreeMap::new();
    for entry in entries {
        if let Some(year) = entry.timestamp.and_then(year_of) {
            grouped.entry(year).or_default().push(entry);
        }
    }
    let mut result = BTreeMap::new();
    for (year, values) in grouped {
        let stats = eval::cluster_stats(&values.iter().map(|entry| (entry.bundle, entry.bar, entry.outcome.hit)).collect::<Vec<_>>(), args.horizon);
        let base_values: Vec<f64> = values.iter().filter_map(|entry| if state_only { entry.state_fold_base_rate } else { entry.fold_base_rate }).collect();
        let base = (!base_values.is_empty()).then(|| base_values.iter().sum::<f64>() / base_values.len() as f64);
        result.insert(year.to_string(), json!({
            "n": stats.n, "clusters": stats.clusters, "hit_rate": stats.hit_rate,
            "base_rate": base, "lift": stats.hit_rate.zip(base).map(|(hit, base)| hit - base)
        }));
    }
    result
}

fn year_of(timestamp: i64) -> Option<i64> {
    let days = timestamp.div_euclid(86_400);
    Some(civil_from_days(days).0)
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 { month_prime + 3 } else { month_prime - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let text = fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header = lines.next().unwrap_or("").split(',').map(|value| value.trim().trim_matches('"').to_owned()).collect();
    Ok((header, lines.map(|line| line.split(',').map(|value| value.trim().trim_matches('"').to_owned()).collect()).collect()))
}

/// Whether a CSV row matches every state clause.
pub(super) fn state_matches_csv(fields: &BTreeMap<String, String>, gate: &StateGate) -> bool {
    gate.clauses.iter().all(|clause| {
        let Some(value) = fields.get(&clause.column).and_then(|value| optional_int(value)) else {
            return false;
        };
        clause.values.as_ref().is_none_or(|values| values.contains(&value))
            && clause.min_value.is_none_or(|minimum| value >= minimum)
            && clause.max_value.is_none_or(|maximum| value <= maximum)
    })
}
