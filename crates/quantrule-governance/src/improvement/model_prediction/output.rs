//! Model-prediction Markdown rendering.

use serde_json::Value;

pub(super) fn markdown(document: &Value) -> String {
    let metrics = &document["metrics"];
    let mut lines = vec![
        "# Model Prediction Evaluation".to_owned(),
        String::new(),
        format!("- Schema: `{}`", document["schema"].as_str().unwrap_or("")),
        format!(
            "- Spec: `{}` family `{}`",
            document["spec_id"].as_str().unwrap_or("-"),
            document["family"].as_str().unwrap_or("-")
        ),
        format!(
            "- Instrument: `{}`; partition `{}`",
            document["instrument"].as_str().unwrap_or(""),
            document["partition"].as_str().unwrap_or("")
        ),
        format!(
            "- Split: `{}` to `{}`",
            document["split_start"].as_str().unwrap_or("-"),
            document["split_end"].as_str().unwrap_or("-")
        ),
        format!(
            "- Outcome: `{}`; horizon {} bars; cost {} bps",
            document["outcome"].as_str().unwrap_or(""),
            document["horizon"],
            document["cost_bps"]
        ),
        format!(
            "- Grade: **{}**; verdict **{}**",
            document["grade"].as_str().unwrap_or(""),
            document["verdict"].as_str().unwrap_or("")
        ),
        String::new(),
        "| Metric | Value |".to_owned(),
        "|---|---:|".to_owned(),
    ];
    for key in [
        "n",
        "predictions",
        "coverage",
        "purged",
        "clusters",
        "hits",
        "hit_rate",
        "base_rate",
        "lift",
        "lift_lcb",
        "lcb_method",
        "random_p",
        "shuffle_p",
        "back_lag_lift",
        "replication",
    ] {
        lines.push(format!("| {key} | {} |", format_value(metrics.get(key))));
    }
    if document["state_gate"].is_object() {
        let unconditional = &document["unconditional_metrics"];
        lines.extend([
            String::new(),
            "## State Gate".to_owned(),
            String::new(),
            format!("- Gate: `{}`", python_json(&document["state_gate"])),
            format!(
                "- State increment: {}",
                format_value(document.get("state_increment"))
            ),
            String::new(),
            "| Metric | State | Unconditional |".to_owned(),
            "|---|---:|---:|".to_owned(),
        ]);
        for key in [
            "n",
            "clusters",
            "hit_rate",
            "base_rate",
            "lift",
            "lift_lcb",
            "random_p",
            "shuffle_p",
            "back_lag_lift",
            "replication",
        ] {
            lines.push(format!(
                "| {key} | {} | {} |",
                format_value(metrics.get(key)),
                format_value(unconditional.get(key))
            ));
        }
    }
    lines.extend([
        String::new(),
        "## Instruments".to_owned(),
        String::new(),
        "| Instrument | n | Clusters | Hit | Base | Lift |".to_owned(),
        "|---|---:|---:|---:|---:|---:|".to_owned(),
    ]);
    if let Some(instruments) = metrics["instruments"].as_object() {
        for (name, value) in instruments {
            lines.push(format!(
                "| {name} | {} | {} | {} | {} | {} |",
                value["n"],
                value["clusters"],
                format_value(value.get("hit_rate")),
                format_value(value.get("base_rate")),
                format_value(value.get("lift"))
            ));
        }
    }
    lines.extend([
        String::new(),
        "## Folds".to_owned(),
        String::new(),
        "| Year | n | Clusters | Hit | Base | Lift |".to_owned(),
        "|---|---:|---:|---:|---:|---:|".to_owned(),
    ]);
    if let Some(folds) = metrics["folds"].as_object() {
        for (year, value) in folds {
            lines.push(format!(
                "| {year} | {} | {} | {} | {} | {} |",
                value["n"],
                value["clusters"],
                format_value(value.get("hit_rate")),
                format_value(value.get("base_rate")),
                format_value(value.get("lift"))
            ));
        }
    }
    lines.extend([String::new(), "## Limitations".to_owned(), String::new()]);
    if let Some(limitations) = document["limitations"].as_array() {
        for item in limitations {
            lines.push(format!("- {}", item.as_str().unwrap_or("")));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn format_value(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::Number(number)) if number.is_f64() => {
            format!("{:.6}", number.as_f64().unwrap_or(0.0))
        }
        Some(Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
    }
}

fn python_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value).unwrap_or_default(),
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(python_json)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Object(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!(
                    "{}: {}",
                    serde_json::to_string(key).unwrap_or_default(),
                    python_json(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
