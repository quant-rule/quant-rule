//! Aggregate a research program and report every cell.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::program::{self, Program};
use crate::research::stats::inverse_normal_cdf;

const SCHEMA: &str = "quantrule.research-program-results/v1";

/// Run `research report`.
///
/// # Errors
///
/// Returns a message for an invalid program or a write failure.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut program_path = None;
    let mut results = None;
    let mut out = None;
    let mut repo_root = PathBuf::from(".");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--program" => program_path = Some(value(args, &mut index, "--program")?),
            "--results" => results = Some(value(args, &mut index, "--results")?),
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--repo-root" => repo_root = PathBuf::from(value(args, &mut index, "--repo-root")?),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let program_path =
        PathBuf::from(program_path.ok_or_else(|| "--program is required".to_owned())?);
    let results = PathBuf::from(results.ok_or_else(|| "--results is required".to_owned())?);
    let out = PathBuf::from(out.ok_or_else(|| "--out is required".to_owned())?);
    let loaded = program::load(&program_path, &repo_root)?;
    let document = build(&loaded, &results);
    fs::create_dir_all(&out)
        .map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    crate::improvement::store::write_json(&out.join("program-results.json"), &document)?;
    fs::write(out.join("program-results.md"), markdown(&document))
        .map_err(|error| format!("cannot write program-results.md: {error}"))?;
    println!("ok report {} -> {}", loaded.program_id, out.display());
    Ok(())
}

fn build(program: &Program, results: &Path) -> Value {
    let path_required = program
        .green_criteria
        .get("path_mean_net_r")
        .and_then(Value::as_str)
        .is_some_and(|value| value.trim() == "> 0");
    let mut rows = Vec::new();
    for cell in &program.cells {
        let record = read_result(&results.join(&cell.cell_id).join("cell-result.json"));
        let (verdict, limitations) = if let Some(record) = &record {
            (
                record
                    .get("verdict")
                    .and_then(Value::as_str)
                    .unwrap_or("fail")
                    .to_owned(),
                record
                    .get("limitations")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .map(|value| value.as_str().unwrap_or("").to_owned())
                    .collect::<Vec<_>>(),
            )
        } else {
            (
                "missing".to_owned(),
                vec!["No cell-result.json under the result root.".to_owned()],
            )
        };
        let mean_net_r = path_mean_net_r(record.as_ref());
        rows.push(json!({
            "cell_id": cell.cell_id,
            "hypothesis": cell.hypothesis,
            "primary": cell.primary,
            "verdict": verdict,
            "metrics": record.as_ref().and_then(|record| record.get("metrics").cloned()).unwrap_or_else(|| json!({})),
            "path_mean_net_r": mean_net_r,
            "path_gate": if path_required { json!(mean_net_r.is_some_and(|value| value > 0.0)) } else { Value::Null },
            "limitations": limitations,
        }));
    }
    let primary: Vec<&Value> = rows
        .iter()
        .filter(|row| row["primary"].as_bool() == Some(true))
        .collect();
    let mut passing: Vec<&Value> = primary
        .iter()
        .copied()
        .filter(|row| {
            row["verdict"].as_str() == Some("pass")
                && (!path_required || row["path_gate"].as_bool() == Some(true))
        })
        .collect();
    passing.sort_by(|left, right| rank_value(Some(right)).total_cmp(&rank_value(Some(left))));
    let candidate = passing
        .first()
        .map(|row| row["cell_id"].clone())
        .unwrap_or(Value::Null);
    let decision = if passing.is_empty() {
        "no_edge — no primary cell passed the development gate"
    } else {
        "candidate — human review required; no validation opened"
    };
    json!({
        "schema": SCHEMA,
        "program_id": program.program_id,
        "premise": program.premise,
        "tier": program.tier,
        "partition_dev": program.partition_dev,
        "partition_validation": program.partition_validation,
        "validation_access": program.validation_access,
        "correction": correction(program),
        "path_gate_required": path_required,
        "cells": rows,
        "primary": primary,
        "candidate": candidate,
        "decision": decision,
    })
}

fn correction(program: &Program) -> Value {
    let method = program
        .correction
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("bonferroni");
    let cells = program
        .correction
        .get("primary_cells")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let alpha = if method == "bonferroni" {
        0.05 / cells as f64
    } else {
        0.05
    };
    json!({
        "method": method,
        "primary_cells": cells,
        "family_alpha": 0.05,
        "cell_alpha": alpha,
        "z": inverse_normal_cdf(1.0 - alpha / 2.0),
    })
}

fn read_result(path: &Path) -> Option<Value> {
    if !path.is_file() {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&text)
        .ok()
        .filter(Value::is_object)
}

fn metric<'a>(record: Option<&'a Value>, key: &str) -> Option<&'a Value> {
    record?.get("metrics")?.get(key)
}

fn numeric(value: Option<&Value>) -> Option<f64> {
    value?.as_f64().filter(|value| value.is_finite())
}

fn path_mean_net_r(record: Option<&Value>) -> Option<f64> {
    numeric(
        metric(record, "path")?
            .get("summaries")?
            .as_array()?
            .first()?
            .get("summary")?
            .get("mean_net_r"),
    )
}

fn rank_value(record: Option<&Value>) -> f64 {
    for key in ["lift_lcb", "average_instrument_return", "profit_factor"] {
        if let Some(value) = numeric(metric(record, key)) {
            return value;
        }
    }
    f64::NEG_INFINITY
}

fn markdown(document: &Value) -> String {
    let correction = &document["correction"];
    let mut lines = vec![
        format!(
            "# Research Program {}",
            document["program_id"].as_str().unwrap_or("")
        ),
        String::new(),
        format!("- Premise: {}", document["premise"].as_str().unwrap_or("")),
        format!("- Tier: `{}`", document["tier"].as_str().unwrap_or("")),
        format!(
            "- Development partition: `{}`",
            document["partition_dev"].as_str().unwrap_or("")
        ),
        format!(
            "- Validation partition: `{}`",
            document["partition_validation"].as_str().unwrap_or("")
        ),
        format!(
            "- Correction: {} over {} primary cells",
            correction["method"].as_str().unwrap_or(""),
            correction["primary_cells"]
        ),
        format!(
            "- Decision: **{}**",
            document["decision"].as_str().unwrap_or("")
        ),
        String::new(),
    ];
    let model_mode = document["cells"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|row| numeric(metric(Some(row), "hit_rate")).is_some());
    let path_required = document["path_gate_required"].as_bool() == Some(true);
    if model_mode {
        let mut header =
            "| Cell | Primary | Verdict | n | Clusters | Hit | Base | Lift | LCB |".to_owned();
        let mut separator = "|---|---|---|---:|---:|---:|---:|---:|---:|".to_owned();
        if path_required {
            header.push_str(" Path R |");
            separator.push_str("---:|");
        }
        header.push_str(" Random p | Shuffle p | Rep | Grade |");
        separator.push_str("---:|---:|---:|---|");
        lines.extend([header, separator]);
    } else {
        lines.extend([
            "| Cell | Primary | Verdict | n | Clusters | Lift | Lift LCB | Grade |".to_owned(),
            "|---|---|---|---:|---:|---:|---:|---|".to_owned(),
        ]);
    }
    for row in document["cells"].as_array().into_iter().flatten() {
        let metrics = &row["metrics"];
        if model_mode {
            let path_cell = if path_required {
                format!(" {} |", fmt(row.get("path_mean_net_r")))
            } else {
                String::new()
            };
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} |{} {} | {} | {} | {} |",
                row["cell_id"].as_str().unwrap_or(""),
                if row["primary"].as_bool() == Some(true) {
                    "yes"
                } else {
                    "no"
                },
                row["verdict"].as_str().unwrap_or(""),
                fmt(metrics.get("n")),
                fmt(metrics.get("clusters")),
                fmt(metrics.get("hit_rate")),
                fmt(metrics.get("base_rate")),
                fmt(metrics.get("lift")),
                fmt(metrics.get("lift_lcb")),
                path_cell,
                fmt(metrics.get("random_p")),
                fmt(metrics.get("shuffle_p")),
                fmt(metrics.get("replication")),
                fmt(metrics.get("grade"))
            ));
        } else {
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                row["cell_id"].as_str().unwrap_or(""),
                if row["primary"].as_bool() == Some(true) {
                    "yes"
                } else {
                    "no"
                },
                row["verdict"].as_str().unwrap_or(""),
                fmt(metrics.get("n")),
                fmt(metrics.get("clusters")),
                fmt(metrics.get("lift")),
                fmt(metrics.get("lift_lcb")),
                fmt(metrics.get("grade"))
            ));
        }
    }
    if document["candidate"].is_string() {
        lines.extend([
            String::new(),
            format!(
                "Candidate for human review: `{}`. No validation was opened by this report.",
                document["candidate"].as_str().unwrap_or("")
            ),
        ]);
    } else {
        lines.extend([
            String::new(),
            "No primary cell passed. The program stops with no edge. Do not re-register a variant."
                .to_owned(),
        ]);
    }
    format!("{}\n", lines.join("\n"))
}

fn fmt(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => "-".to_owned(),
        Some(Value::Number(value)) => {
            if let Some(value) = value.as_i64() {
                value.to_string()
            } else {
                value.as_f64().map_or_else(
                    || value.to_string(),
                    |value| {
                        if value.abs() < 1.0 {
                            format!("{value:+.4}")
                        } else {
                            format!("{value:.4}")
                        }
                    },
                )
            }
        }
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
    }
}

fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_keeps_integers_and_signs_fractions() {
        assert_eq!(fmt(Some(&json!(10))), "10");
        assert_eq!(fmt(Some(&json!(0.05))), "+0.0500");
        assert_eq!(fmt(Some(&json!(1.5))), "1.5000");
        assert_eq!(fmt(None), "-");
    }
}
