//! Path-validation document and Markdown rendering.

use serde_json::{Map, Value, json};

use super::CellRow;

pub(super) fn row_json(row: &CellRow) -> Value {
    json!({
        "family": row.family, "label": row.label, "timeframe": row.timeframe,
        "timeframe_secs": row.timeframe_secs, "config": row.config,
        "raw": row.raw, "prepared": row.prepared, "n": row.n, "clusters": row.clusters,
        "singleton_bundles": row.singleton_bundles, "hits": row.hits,
        "hit_rate": row.hit_rate, "base_rate": row.base_rate, "lift": row.lift,
        "lift_lcb": row.lift_lcb, "state_applicable": row.state_applicable,
        "state_rate": row.state_rate, "state_missing": row.state_missing,
        "incremental_lift": row.incremental_lift, "incremental_lcb": row.incremental_lcb,
        "cluster_se": row.cluster_se, "random_mean": row.random_mean,
        "random_p95": row.random_p95, "random_p": row.random_p,
        "shuffle_mean": row.shuffle_mean, "shuffle_p95": row.shuffle_p95,
        "shuffle_p": row.shuffle_p, "back_lag_rate": row.back_lag_rate,
        "back_lag_n": row.back_lag_n, "back_lag_lift": row.back_lag_lift,
        "forward_lag_rate": row.forward_lag_rate, "forward_lag_n": row.forward_lag_n,
        "forward_lag_lift": row.forward_lag_lift, "replication": row.replication,
        "instruments": row.instruments.iter().map(|(name, value)| (name.clone(), json!({
            "n": value.n, "hits": value.hits, "hit_rate": value.hit_rate, "base": value.base
        }))).collect::<Map<String, Value>>(),
        "event_signature": row.event_signature, "grade": row.grade, "reasons": row.reasons,
        "sensitivity_ok": row.sensitivity_ok, "sensitivity_reason": row.sensitivity_reason,
    })
}

pub(super) fn markdown(document: &Value) -> String {
    let primary_config = document["primary_config"].as_str().unwrap_or("");
    let sensitivity_config = document["sensitivity_config"].as_str().unwrap_or("");
    let mut lines = vec![
        concat!("# Model v", "4 Path Validation").to_owned(), String::new(),
        format!("- Horizon: {} bars", document["horizon_bars"]),
        format!("- Target / stop: {} / {} ATR", document["target_atr"], document["stop_atr"]),
        format!("- Seed: {}; random draws: {}; shuffle draws: {}; back lag: {}", document["seed"], document["random_draws"], document["shuffle_draws"], document["back_lag"]),
        format!("- Primary config: `{primary_config}`; sensitivity config: `{sensitivity_config}`"),
        String::new(), "## Primary cells".to_owned(), String::new(),
        "| Family | TF | n | J | Hit | Base | Lift | Lift LCB | State | Incr. | Incr. LCB | Rand p | Back lag | Shuffle p | Rep | Grade |".to_owned(),
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|".to_owned(),
    ];
    let cells = document["cells"].as_array().cloned().unwrap_or_default();
    let primary: Vec<&Value> = cells
        .iter()
        .filter(|row| row["config"].as_str() == Some(primary_config))
        .collect();
    for row in &primary {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row["family"].as_str().unwrap_or(""),
            row["timeframe"].as_str().unwrap_or(""),
            row["n"],
            row["clusters"],
            fmt_value(row.get("hit_rate")),
            fmt_value(row.get("base_rate")),
            fmt_signed_value(row.get("lift")),
            fmt_value(row.get("lift_lcb")),
            fmt_value(row.get("state_rate")),
            fmt_signed_value(row.get("incremental_lift")),
            fmt_value(row.get("incremental_lcb")),
            fmt_value(row.get("random_p")),
            fmt_signed_value(row.get("back_lag_lift")),
            fmt_value(row.get("shuffle_p")),
            row["replication"],
            row["grade"].as_str().unwrap_or("")
        ));
    }
    lines.extend([
        String::new(),
        "`-` in State/Incr. means the family declares no state (`state: none`); the state-conditioned incremental test is not applicable.".to_owned(),
    ]);
    let failed: Vec<&Value> = primary
        .iter()
        .copied()
        .filter(|row| {
            row["reasons"]
                .as_array()
                .is_some_and(|values| !values.is_empty())
        })
        .collect();
    if !failed.is_empty() {
        lines.extend([
            String::new(),
            "### Failed checks".to_owned(),
            String::new(),
            "| Family | TF | Failed checks |".to_owned(),
            "|---|---|---|".to_owned(),
        ]);
        for row in failed {
            let reasons = row["reasons"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "| {} | {} | {reasons} |",
                row["family"].as_str().unwrap_or(""),
                row["timeframe"].as_str().unwrap_or("")
            ));
        }
    }
    lines.extend([
        String::new(),
        "## Sensitivity cells".to_owned(),
        String::new(),
        "| Family | TF | n | Hit | Base | Lift | Incr. | Rand p | Back lag | Grade | Isolation |"
            .to_owned(),
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---|---|".to_owned(),
    ]);
    for row in cells
        .iter()
        .filter(|row| row["config"].as_str() == Some(sensitivity_config))
    {
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row["family"].as_str().unwrap_or(""),
            row["timeframe"].as_str().unwrap_or(""),
            row["n"],
            fmt_value(row.get("hit_rate")),
            fmt_value(row.get("base_rate")),
            fmt_signed_value(row.get("lift")),
            fmt_signed_value(row.get("incremental_lift")),
            fmt_value(row.get("random_p")),
            fmt_signed_value(row.get("back_lag_lift")),
            row["grade"].as_str().unwrap_or(""),
            row.get("sensitivity_reason")
                .and_then(Value::as_str)
                .unwrap_or("-")
        ));
    }
    lines.extend([
        String::new(),
        "## Per-instrument (primary config)".to_owned(),
        String::new(),
        "| Family | TF | Instrument | n | Hit | Base | Lift |".to_owned(),
        "|---|---|---|---:|---:|---:|---:|".to_owned(),
    ]);
    for row in primary {
        if let Some(instruments) = row["instruments"].as_object() {
            for (instrument, value) in instruments {
                let lift = value["hit_rate"]
                    .as_f64()
                    .zip(value["base"].as_f64())
                    .map(|(hit, base)| hit - base);
                lines.push(format!(
                    "| {} | {} | {instrument} | {} | {} | {} | {} |",
                    row["family"].as_str().unwrap_or(""),
                    row["timeframe"].as_str().unwrap_or(""),
                    value["n"],
                    fmt_value(value.get("hit_rate")),
                    fmt_value(value.get("base")),
                    fmt_signed_num(lift)
                ));
            }
        }
    }
    lines.extend([
        String::new(),
        "## Decision".to_owned(),
        String::new(),
        document["decision"].as_str().unwrap_or("").to_owned(),
        String::new(),
    ]);
    lines.join("\n")
}

fn fmt_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_f64)
        .map_or_else(|| "-".to_owned(), |value| format!("{value:.4}"))
}

fn fmt_signed_value(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_f64)
        .map_or_else(|| "-".to_owned(), |value| format!("{value:+.4}"))
}

fn fmt_signed_num(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_owned(), |value| format!("{value:+.4}"))
}
