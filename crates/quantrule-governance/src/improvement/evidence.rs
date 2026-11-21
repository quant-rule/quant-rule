//! Write the tier-specific evidence summary for one improvement run.

use std::fs;
use std::path::Path;

use serde_json::{Map, Value, json};

const EVIDENCE_SCHEMA: &str = "quantrule.improvement-evidence/v1";
const TIERS: [&str; 3] = ["model", "strategy", "portfolio"];

/// Run `qr-governance improvement evidence <run_dir>`.
///
/// # Errors
///
/// Returns a message when the run folder, manifest, report, or output cannot be
/// read or written.
pub fn run(run_dir: &Path) -> Result<(), String> {
    if !run_dir.is_dir() {
        return Err(format!("run directory not found: {}", run_dir.display()));
    }
    let manifest_path = run_dir.join("manifest.json");
    if !manifest_path.is_file() {
        return Err(format!("manifest not found: {}", manifest_path.display()));
    }
    let manifest = read_json(&manifest_path)?;
    let Some(manifest) = manifest.as_object() else {
        return Err(format!(
            "{}: manifest must be a JSON object",
            manifest_path.display()
        ));
    };
    let tier = manifest.get("tier").and_then(Value::as_str).unwrap_or("");
    if !TIERS.contains(&tier) {
        return Err(format!(
            "{}: tier must be one of {}",
            manifest_path.display(),
            TIERS.join(", ")
        ));
    }
    let Some(run_id) = manifest
        .get("run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Err(format!("{}: run_id is missing", manifest_path.display()));
    };
    let artifacts = run_dir.join("artifacts");
    fs::create_dir_all(&artifacts)
        .map_err(|error| format!("cannot create {}: {error}", artifacts.display()))?;

    let mut notes = Vec::new();
    let (source_name, source, mut summary) = if tier == "model" {
        (
            "artifacts/model/report.json".to_owned(),
            artifacts.join("model/report.json"),
            empty_model_summary(),
        )
    } else {
        let candidates = [
            (
                "artifacts/analytics/metrics.json",
                artifacts.join("analytics/metrics.json"),
            ),
            (
                "artifacts/portfolio/metrics.json",
                artifacts.join("portfolio/metrics.json"),
            ),
            (
                "artifacts/portfolio/report.json",
                artifacts.join("portfolio/report.json"),
            ),
        ];
        let (name, path) = candidates
            .iter()
            .find(|(_, path)| path.is_file())
            .unwrap_or(&candidates[2]);
        ((*name).to_owned(), path.clone(), empty_portfolio_summary())
    };
    if !source.is_file() {
        notes.push(format!("{source_name} is missing; summary fields are null"));
    } else {
        let report = read_json(&source)?;
        let (extracted, extraction_notes) = if tier == "model" {
            model_metrics(&report)
        } else {
            portfolio_metrics(&report)
        };
        summary = extracted;
        notes.extend(extraction_notes);
    }

    let mut qualification = Value::Null;
    let qualification_path = artifacts.join("qualification/qualification.json");
    if qualification_path.is_file() {
        let record = read_json(&qualification_path)?;
        if let Some(record) = record.as_object() {
            let mut output = Map::new();
            output.insert(
                "verdict".to_owned(),
                record.get("verdict").cloned().unwrap_or(Value::Null),
            );
            output.insert(
                "human_decision".to_owned(),
                record.get("human_decision").cloned().unwrap_or(Value::Null),
            );
            output.insert(
                "reasons".to_owned(),
                record.get("reasons").cloned().unwrap_or(Value::Null),
            );
            qualification = Value::Object(output);
        } else {
            notes.push("artifacts/qualification/qualification.json is not an object".to_owned());
        }
    }

    let mut document = Map::new();
    document.insert("schema".to_owned(), json!(EVIDENCE_SCHEMA));
    document.insert("run_id".to_owned(), json!(run_id));
    document.insert("tier".to_owned(), json!(tier));
    document.insert(
        "source".to_owned(),
        if source.is_file() {
            json!(source_name)
        } else {
            Value::Null
        },
    );
    document.insert("summary".to_owned(), summary);
    document.insert("qualification".to_owned(), qualification);
    document.insert(
        "notes".to_owned(),
        Value::Array(notes.into_iter().map(Value::String).collect()),
    );
    let destination = artifacts.join("evidence.json");
    let text = serde_json::to_string_pretty(&Value::Object(document))
        .map_err(|error| format!("cannot encode {}: {error}", destination.display()))?;
    fs::write(&destination, format!("{text}\n"))
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    println!("ok {run_id} evidence -> {}", destination.display());
    Ok(())
}

fn empty_portfolio_summary() -> Value {
    let fields = [
        "return",
        "max_drawdown",
        "trades",
        "profit_factor",
        "win_rate",
        "halted",
        "final_equity",
        "exposure",
        "time_to_target_days",
        "target_reached",
        "max_drawdown_peak_to_trough_days",
        "max_drawdown_recovery_days",
        "longest_underwater_days",
        "benchmark_buy_and_hold_return",
        "benchmark_random_median_return",
        "strategy_percentile_vs_random",
    ];
    let mut values = Map::new();
    for field in fields {
        values.insert(field.to_owned(), Value::Null);
    }
    Value::Object(values)
}

fn empty_model_summary() -> Value {
    let mut values = Map::new();
    for field in [
        "model_version",
        "view_contract",
        "data_version",
        "requested_period",
        "ladder_seconds",
    ] {
        values.insert(field.to_owned(), Value::Null);
    }
    values.insert("timeframes".to_owned(), Value::Array(Vec::new()));
    Value::Object(values)
}

/// Extract the portfolio metrics fields evidence and comparison share.
#[must_use]
pub fn portfolio_metrics(report: &Value) -> (Value, Vec<String>) {
    let fields = [
        "return",
        "max_drawdown",
        "trades",
        "profit_factor",
        "win_rate",
        "halted",
        "final_equity",
        "exposure",
        "time_to_target_days",
        "target_reached",
        "max_drawdown_peak_to_trough_days",
        "max_drawdown_recovery_days",
        "longest_underwater_days",
        "benchmark_buy_and_hold_return",
        "benchmark_random_median_return",
        "strategy_percentile_vs_random",
    ];
    let mut values = Map::new();
    for field in fields {
        values.insert(field.to_owned(), Value::Null);
    }
    let mut notes = Vec::new();
    let Some(report) = report.as_object() else {
        return (
            Value::Object(values),
            vec!["portfolio report is not a JSON object".to_owned()],
        );
    };
    let (source, prefix) = if report.get("metrics").is_some_and(Value::is_object) {
        (report["metrics"].as_object().unwrap_or(report), "metrics")
    } else if report.get("summary").is_some_and(Value::is_object) {
        (report["summary"].as_object().unwrap_or(report), "summary")
    } else {
        (report, "report")
    };
    let sources: [(&str, &[&str]); 16] = [
        ("return", &["total_return"]),
        ("max_drawdown", &["max_drawdown"]),
        ("trades", &["trade_count", "trades"]),
        ("profit_factor", &["profit_factor"]),
        ("win_rate", &["win_rate"]),
        ("halted", &["halted"]),
        ("final_equity", &["final_equity"]),
        ("exposure", &["exposure"]),
        ("time_to_target_days", &["time_to_target_days"]),
        ("target_reached", &["target_reached"]),
        (
            "max_drawdown_peak_to_trough_days",
            &["max_drawdown_peak_to_trough_days"],
        ),
        (
            "max_drawdown_recovery_days",
            &["max_drawdown_recovery_days"],
        ),
        ("longest_underwater_days", &["longest_underwater_days"]),
        (
            "benchmark_buy_and_hold_return",
            &["benchmark_buy_and_hold_return"],
        ),
        (
            "benchmark_random_median_return",
            &["benchmark_random_median_return"],
        ),
        (
            "strategy_percentile_vs_random",
            &["strategy_percentile_vs_random"],
        ),
    ];
    for (field, keys) in sources {
        let Some(key) = keys.iter().find(|key| source.contains_key(**key)) else {
            notes.push(format!("{prefix}.{} is missing", keys[0]));
            continue;
        };
        let value = &source[*key];
        if field == "halted" || field == "target_reached" {
            if value.is_boolean() || value.is_null() {
                values.insert(field.to_owned(), value.clone());
            } else {
                notes.push(format!("{prefix}.{key} is not a boolean"));
            }
        } else if is_number(value) {
            values.insert(field.to_owned(), value.clone());
        } else if value.is_null()
            && (field == "time_to_target_days" || field == "max_drawdown_recovery_days")
        {
            values.insert(field.to_owned(), Value::Null);
        } else {
            notes.push(format!("{prefix}.{key} is not a finite number"));
        }
    }
    (Value::Object(values), notes)
}

/// Summarize the model report's versions and per-timeframe coverage.
#[must_use]
pub fn model_metrics(report: &Value) -> (Value, Vec<String>) {
    let mut values = empty_model_summary();
    let mut notes = Vec::new();
    let Some(report) = report.as_object() else {
        return (values, vec!["model report is not a JSON object".to_owned()]);
    };
    for field in ["model_version", "view_contract", "data_version"] {
        if report
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        {
            values[field] = report[field].clone();
        } else {
            notes.push(format!("{field} is missing"));
        }
    }
    if report.get("requested_period").is_some_and(Value::is_object) {
        values["requested_period"] = report["requested_period"].clone();
    } else {
        notes.push("requested_period is missing".to_owned());
    }
    let Some(market) = report.get("market_report").and_then(Value::as_object) else {
        notes.push("market_report is missing".to_owned());
        return (values, notes);
    };
    if market
        .get("ladder")
        .and_then(Value::as_array)
        .is_some_and(|ladder| ladder.iter().all(is_number))
    {
        values["ladder_seconds"] = market["ladder"].clone();
    } else {
        notes.push("market_report.ladder is missing or not numeric".to_owned());
    }
    let Some(timeframes) = market.get("timeframes").and_then(Value::as_array) else {
        notes.push("market_report.timeframes is missing".to_owned());
        return (values, notes);
    };
    let mut rows = Vec::new();
    for (index, timeframe) in timeframes.iter().enumerate() {
        let Some(timeframe) = timeframe.as_object() else {
            notes.push(format!(
                "market_report.timeframes[{index}] is not an object"
            ));
            continue;
        };
        let mut row = Map::new();
        for field in ["timeframe_secs", "bars", "missing_bars", "coverage_share"] {
            let value = timeframe.get(field).cloned().unwrap_or(Value::Null);
            if is_number(&value) {
                row.insert(field.to_owned(), value);
            } else {
                row.insert(field.to_owned(), Value::Null);
                notes.push(format!(
                    "market_report.timeframes[{index}].{field} is missing"
                ));
            }
        }
        rows.push(Value::Object(row));
    }
    values["timeframes"] = Value::Array(rows);
    (values, notes)
}

fn is_number(value: &Value) -> bool {
    value
        .as_f64()
        .is_some_and(|value| value.is_finite() && !value.is_nan())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytics_metrics_shape() {
        let report = json!({
            "metrics": {
                "total_return": 0.1,
                "max_drawdown": 0.2,
                "trade_count": 5,
                "profit_factor": 1.5,
                "win_rate": 0.6,
                "halted": false,
                "final_equity": 11000.0,
                "exposure": 0.4
            }
        });
        let (values, notes) = portfolio_metrics(&report);
        assert_eq!(values["return"], 0.1);
        assert_eq!(values["trades"], 5);
        assert_eq!(values["final_equity"], 11000.0);
        assert_eq!(values["exposure"], 0.4);
        assert!(notes.iter().all(|note| note.contains("missing")));
    }

    #[test]
    fn portfolio_metrics_shape() {
        let report = json!({
            "total_return": -0.1,
            "max_drawdown": 0.3,
            "trade_count": 2,
            "profit_factor": 0.0,
            "win_rate": 0.0,
            "halted": true,
            "final_equity": 9000.0
        });
        let (values, notes) = portfolio_metrics(&report);
        assert_eq!(values["return"], -0.1);
        assert_eq!(values["trades"], 2);
        assert_eq!(values["halted"], true);
        assert!(notes.contains(&"report.exposure is missing".to_owned()));
    }

    #[test]
    fn historical_summary_shape() {
        let report = json!({
            "summary": {
                "total_return": 0.2,
                "max_drawdown": 0.1,
                "trades": 3,
                "profit_factor": 2.0,
                "win_rate": 0.5,
                "halted": false,
                "final_equity": 12000.0
            }
        });
        let (values, _) = portfolio_metrics(&report);
        assert_eq!(values["return"], 0.2);
        assert_eq!(values["trades"], 3);
        assert_eq!(values["final_equity"], 12000.0);
    }

    #[test]
    fn missing_values_are_null() {
        let (values, notes) = portfolio_metrics(&json!({}));
        assert!(values["return"].is_null());
        assert!(!notes.is_empty());
    }

    #[test]
    fn model_report_shape() {
        let report = json!({
            "model_version": "M1.0",
            "view_contract": "2026-11-05",
            "data_version": "test:v1",
            "requested_period": {"from": "2025-01-01", "to": "2025-01-02"},
            "market_report": {
                "ladder": [60, 300],
                "timeframes": [{
                    "timeframe_secs": 60,
                    "bars": 10,
                    "missing_bars": 0,
                    "coverage_share": 1.0
                }]
            }
        });
        let (values, notes) = model_metrics(&report);
        assert_eq!(values["model_version"], "M1.0");
        assert_eq!(values["ladder_seconds"], json!([60, 300]));
        assert_eq!(values["timeframes"].as_array().map(Vec::len), Some(1));
        assert!(notes.is_empty());
    }
}
