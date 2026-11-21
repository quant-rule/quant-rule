//! Result adapters for the research cell runner.

use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

/// One parsed cell result.
pub struct ParseResult {
    /// Metrics record.
    pub metrics: Map<String, Value>,
    /// `pass`, `fail`, or `None` when the exit code is the verdict.
    pub verdict: Option<String>,
    /// Limitations carried into the research record.
    pub limitations: Vec<String>,
}

/// Parse one artifact with a registered parser.
///
/// # Errors
///
/// Returns a message for a missing or malformed artifact.
pub fn parse(parser: &str, out_dir: &Path) -> Result<ParseResult, String> {
    match parser {
        "path_validation" => parse_path_validation(out_dir),
        "strategy_evaluation" => parse_strategy_evaluation(out_dir),
        "model_prediction" => parse_model_prediction(out_dir),
        "summary_json" => parse_summary_json(out_dir),
        "none" => Ok(ParseResult {
            metrics: Map::new(),
            verdict: None,
            limitations: Vec::new(),
        }),
        other => Err(format!("unknown result parser: {other}")),
    }
}

fn parse_path_validation(out_dir: &Path) -> Result<ParseResult, String> {
    let document = read(&out_dir.join("path_validation.json"))?;
    let cells = document
        .get("cells")
        .and_then(Value::as_array)
        .filter(|cells| !cells.is_empty())
        .ok_or_else(|| "path_validation.json has no cells".to_owned())?;
    let primary_config = document.get("primary_config").and_then(Value::as_str);
    let selection = document.get("selection");
    let mut row = None;
    if let Some(selection) = selection.and_then(Value::as_object) {
        for candidate in cells.iter().filter_map(Value::as_object) {
            if candidate.get("family") == selection.get("family")
                && candidate.get("timeframe") == selection.get("timeframe")
                && candidate.get("config") == selection.get("config")
            {
                row = Some(candidate.clone());
                break;
            }
        }
    }
    let row = if let Some(row) = row {
        row
    } else {
        let mut pool: Vec<&Map<String, Value>> = cells
            .iter()
            .filter_map(Value::as_object)
            .filter(|row| row.get("config").and_then(Value::as_str) == primary_config)
            .collect();
        if pool.is_empty() {
            pool = cells.iter().filter_map(Value::as_object).collect();
        }
        let Some(first) = pool.into_iter().min_by_key(|row| {
            (
                row.get("timeframe")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                row.get("family")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
            )
        }) else {
            return Err("path_validation.json has no usable row".to_owned());
        };
        first.clone()
    };
    let metrics = metric_map(
        &row,
        &[
            "n",
            "clusters",
            "hit_rate",
            "base_rate",
            "lift",
            "lift_lcb",
            "incremental_lift",
            "incremental_lcb",
            "random_p",
            "back_lag_lift",
            "shuffle_p",
            "replication",
            "grade",
        ],
    );
    let grade = row.get("grade").and_then(Value::as_str);
    let passed = selection.is_some_and(Value::is_object) && matches!(grade, Some("A" | "B"));
    let mut limitations = vec!["Barrier path on the model pack; no execution costs.".to_owned()];
    if !passed {
        limitations.push("No primary cell passed the model gate.".to_owned());
    }
    Ok(ParseResult {
        metrics,
        verdict: Some(if passed { "pass" } else { "fail" }.to_owned()),
        limitations,
    })
}

fn parse_strategy_evaluation(out_dir: &Path) -> Result<ParseResult, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(out_dir)
        .map_err(|error| format!("cannot read {}: {error}", out_dir.display()))?
    {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("strategy_") && name.ends_with(".json"))
        {
            files.push(path);
        }
    }
    files.sort();
    let first = files
        .first()
        .ok_or_else(|| format!("no strategy_*.json in {}", out_dir.display()))?;
    let document = read(first)?;
    let Some(pooled) = document.get("pooled").and_then(Value::as_object) else {
        return Err(format!("{} has no pooled summary", first.display()));
    };
    let passed = document.get("passed").and_then(Value::as_bool) == Some(true);
    let metrics = metric_map(
        pooled,
        &[
            "average_instrument_return",
            "worst_instrument_drawdown",
            "trades",
            "profit_factor",
            "positive_instruments",
        ],
    );
    let mut metrics = metrics;
    metrics.insert(
        "criteria".to_owned(),
        document.get("criteria").cloned().unwrap_or(Value::Null),
    );
    metrics.insert(
        "partition".to_owned(),
        document.get("partition").cloned().unwrap_or(Value::Null),
    );
    Ok(ParseResult {
        metrics,
        verdict: Some(if passed { "pass" } else { "fail" }.to_owned()),
        limitations: vec![
            "Backtest after costs; validation is a separate one-shot tier.".to_owned(),
            "Review the fold table; a pooled result can hide a halted fold.".to_owned(),
        ],
    })
}

fn parse_model_prediction(out_dir: &Path) -> Result<ParseResult, String> {
    let document = read(&out_dir.join("model_prediction.json"))?;
    let Some(metrics_doc) = document.get("metrics").and_then(Value::as_object) else {
        return Err("model_prediction.json has no metrics object".to_owned());
    };
    let grade = document.get("grade").and_then(Value::as_str);
    let passed = document.get("verdict").and_then(Value::as_str) == Some("pass")
        && matches!(grade, Some("A" | "B"));
    let mut metrics = metric_map(
        metrics_doc,
        &[
            "n",
            "clusters",
            "hit_rate",
            "base_rate",
            "lift",
            "lift_lcb",
            "lcb_method",
            "random_p",
            "shuffle_p",
            "back_lag_lift",
            "replication",
        ],
    );
    metrics.insert(
        "grade".to_owned(),
        document.get("grade").cloned().unwrap_or(Value::Null),
    );
    for key in [
        "spec_id",
        "family",
        "instrument",
        "prediction_digest",
        "phase",
        "partition",
        "horizon",
        "cost_bps",
        "verdict",
        "condition_file",
        "condition_sha256",
        "model_config_sha256",
        "path",
    ] {
        metrics.insert(
            key.to_owned(),
            document.get(key).cloned().unwrap_or(Value::Null),
        );
    }
    let limitations_value = document.get("limitations");
    let mut limitations = if let Some(values) = limitations_value.and_then(Value::as_array) {
        values.iter().map(text_value).collect()
    } else if let Some(value) = limitations_value.filter(|value| !value.is_null()) {
        vec![text_value(value)]
    } else {
        Vec::new()
    };
    if !passed {
        limitations.push("The model prediction did not pass the gate.".to_owned());
    }
    Ok(ParseResult {
        metrics,
        verdict: Some(if passed { "pass" } else { "fail" }.to_owned()),
        limitations,
    })
}

fn parse_summary_json(out_dir: &Path) -> Result<ParseResult, String> {
    let document = read(&out_dir.join("summary.json"))?;
    let metrics = document
        .get("metrics")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| document.as_object().cloned().unwrap_or_default());
    let verdict = match document.get("verdict").and_then(Value::as_str) {
        Some(value @ ("pass" | "fail")) => value.to_owned(),
        _ if document.get("passed").and_then(Value::as_bool) == Some(true) => "pass".to_owned(),
        _ => "fail".to_owned(),
    };
    let limitations = document
        .get("limitations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|value| value.as_str().unwrap_or(&value.to_string()).to_owned())
        .collect();
    Ok(ParseResult {
        metrics,
        verdict: Some(verdict),
        limitations,
    })
}

fn read(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("result artifact not found: {}", path.display())
        } else {
            format!("cannot read {}: {error}", path.display())
        }
    })?;
    let document: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.display()))?;
    if !document.is_object() {
        return Err(format!("{} must contain an object", path.display()));
    }
    Ok(document)
}

fn text_value(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned)
}

fn metric_map(row: &Map<String, Value>, keys: &[&str]) -> Map<String, Value> {
    keys.iter()
        .map(|key| {
            (
                (*key).to_owned(),
                row.get(*key).cloned().unwrap_or(Value::Null),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn root(name: &str) -> std::path::PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "qr-adapters-{name}-{}-{unique}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        path
    }

    fn document(grade: &str, verdict: &str) -> Value {
        json!({
            "schema": "quantrule.model-prediction/v1", "spec_id": "M2.1", "family": "M2",
            "instrument": "BTC", "partition": "model_dev", "phase": "dev",
            "prediction_digest": "abc123", "horizon": 20, "cost_bps": 6.0,
            "grade": grade, "verdict": verdict,
            "metrics": {"n": 240, "clusters": 24, "hit_rate": 0.56, "base_rate": 0.50,
                "lift": 0.06, "lift_lcb": 0.02, "lcb_method": "cluster", "random_p": 0.01,
                "shuffle_p": 0.02, "back_lag_lift": 0.0, "replication": 2},
            "limitations": ["Prediction evaluation only."]
        })
    }

    #[test]
    fn a_passing_prediction_is_parsed() {
        let root = root("pass");
        fs::write(
            root.join("model_prediction.json"),
            serde_json::to_string(&document("A", "pass")).expect("json"),
        )
        .expect("file");
        let parsed = parse("model_prediction", &root).expect("parse");
        assert_eq!(parsed.verdict.as_deref(), Some("pass"));
        assert_eq!(parsed.metrics["n"], 240);
        assert_eq!(parsed.metrics["grade"], "A");
        assert_eq!(parsed.metrics["cost_bps"], 6.0);
        assert_eq!(parsed.metrics["prediction_digest"], "abc123");
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_failing_prediction_is_parsed() {
        let root = root("fail");
        fs::write(
            root.join("model_prediction.json"),
            serde_json::to_string(&document("C", "fail")).expect("json"),
        )
        .expect("file");
        let parsed = parse("model_prediction", &root).expect("parse");
        assert_eq!(parsed.verdict.as_deref(), Some("fail"));
        assert!(
            parsed
                .limitations
                .iter()
                .any(|value| value.contains("did not pass"))
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
