//! Write a research program record from a program-results report.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::program::{self, Program};

/// Run `research registry`.
///
/// # Errors
///
/// Returns a message for an invalid program or report.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut program_path = None;
    let mut report_path = None;
    let mut out = None;
    let mut accepted_by = None;
    let mut accepted_on = None;
    let mut repo_root = PathBuf::from(".");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--program" => program_path = Some(value(args, &mut index, "--program")?),
            "--report" => report_path = Some(value(args, &mut index, "--report")?),
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--accepted-by" => accepted_by = Some(value(args, &mut index, "--accepted-by")?),
            "--accepted-on" => accepted_on = Some(value(args, &mut index, "--accepted-on")?),
            "--repo-root" => repo_root = PathBuf::from(value(args, &mut index, "--repo-root")?),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let program_path =
        PathBuf::from(program_path.ok_or_else(|| "--program is required".to_owned())?);
    let report_path = PathBuf::from(report_path.ok_or_else(|| "--report is required".to_owned())?);
    let loaded = program::load(&program_path, &repo_root)?;
    let report = read_report(&report_path)?;
    let state = status(&report, accepted_by.as_deref(), accepted_on.as_deref())?;
    let date = &loaded.program_id[8..18];
    let out = out.map_or_else(
        || {
            repo_root
                .join("docs/improvement")
                .join(date)
                .join(format!("{}.md", loaded.program_id))
        },
        PathBuf::from,
    );
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    fs::write(
        &out,
        markdown(
            &loaded,
            &report,
            &state,
            accepted_by.as_deref(),
            accepted_on.as_deref(),
        ),
    )
    .map_err(|error| format!("cannot write {}: {error}", out.display()))?;
    println!(
        "ok registry {} {} -> {}",
        loaded.program_id,
        state,
        out.display()
    );
    Ok(())
}

fn read_report(path: &Path) -> Result<Value, String> {
    let document = crate::improvement::store::read_json(path)?;
    if document.get("schema").and_then(Value::as_str)
        != Some("quantrule.research-program-results/v1")
    {
        return Err("report schema is not recognized".to_owned());
    }
    Ok(document)
}

fn status(
    report: &Value,
    accepted_by: Option<&str>,
    accepted_on: Option<&str>,
) -> Result<String, String> {
    if accepted_by.is_some() || accepted_on.is_some() {
        if accepted_by.is_none() || accepted_on.is_none() {
            return Err("--accepted-by and --accepted-on must be supplied together".to_owned());
        }
        return Ok("accepted".to_owned());
    }
    let decision = report.get("decision").and_then(Value::as_str).unwrap_or("");
    Ok(if decision.starts_with("candidate") {
        "candidate"
    } else {
        "no_edge"
    }
    .to_owned())
}

fn markdown(
    program: &Program,
    report: &Value,
    state: &str,
    accepted_by: Option<&str>,
    accepted_on: Option<&str>,
) -> String {
    let mut lines = vec![
        format!("# Program {}", program.program_id),
        String::new(),
        format!("- Date: {} (UTC)", &program.program_id[8..18]),
        format!("- Status: **{state}**"),
        format!("- Tier: `{}`", program.tier),
        format!("- Premise: {}", program.premise),
        format!("- Development: `{}`", program.partition_dev),
        format!("- Validation: `{}`", program.partition_validation),
        format!(
            "- Correction: {} over {} primary cells",
            report["correction"]["method"].as_str().unwrap_or(""),
            report["correction"]["primary_cells"]
        ),
        format!("- Decision: {}", report["decision"].as_str().unwrap_or("")),
        format!("- Accepted by: {}", accepted_by.unwrap_or("—")),
        format!("- Accepted on: {}", accepted_on.unwrap_or("—")),
        String::new(),
        "| Cell | Primary | Verdict | n | Clusters | Lift | Lift LCB | Grade |".to_owned(),
        "|---|---|---|---:|---:|---:|---:|---|".to_owned(),
    ];
    for row in report
        .get("cells")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let metrics = &row["metrics"];
        lines.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            row["cell_id"].as_str().unwrap_or("?"),
            if row["primary"].as_bool() == Some(true) {
                "yes"
            } else {
                "no"
            },
            row["verdict"].as_str().unwrap_or("?"),
            fmt(metrics.get("n")),
            fmt(metrics.get("clusters")),
            fmt(metrics.get("lift")),
            fmt(metrics.get("lift_lcb")),
            fmt(metrics.get("grade"))
        ));
    }
    lines.extend([
        String::new(),
        "The registry does not accept a baseline. A candidate requires an independent Quant Validator record and human acceptance.".to_owned(),
    ]);
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
