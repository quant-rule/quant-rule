//! Build a deterministic index of improvement runs.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::manifest;
use super::store;

const TIER_RANK: [(&str, usize); 3] = [("model", 0), ("strategy", 1), ("portfolio", 2)];
const COLUMNS: [&str; 10] = [
    "run id",
    "date",
    "tier",
    "hypothesis",
    "changed",
    "frozen",
    "baseline",
    "partition",
    "verdict",
    "artifacts",
];

/// Rebuild `index.json` and `index.md` under `root`.
///
/// # Errors
///
/// Returns a message for an unreadable run, an invalid manifest, or a write
/// failure.
pub fn build(root: &Path, repo_root: &Path) -> Result<usize, String> {
    let rows = scan(root, repo_root)?;
    let mut document = Map::new();
    document.insert("schema".to_owned(), json!(store::INDEX_SCHEMA));
    document.insert("count".to_owned(), json!(rows.len()));
    document.insert("runs".to_owned(), Value::Array(rows.clone()));
    store::write_json(&root.join("index.json"), &Value::Object(document))?;
    fs::write(root.join("index.md"), render_markdown(&rows))
        .map_err(|error| format!("cannot write {}: {error}", root.join("index.md").display()))?;
    Ok(rows.len())
}

fn scan(root: &Path, repo_root: &Path) -> Result<Vec<Value>, String> {
    if !root.is_dir() {
        return Err(format!("improvement root not found: {}", root.display()));
    }
    let mut date_dirs: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|error| format!("cannot read {}: {error}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|name| valid_date(name.to_string_lossy().as_ref()))
        })
        .collect();
    date_dirs.sort();
    let mut rows = Vec::new();
    for date_dir in date_dirs {
        let mut run_dirs: Vec<PathBuf> = fs::read_dir(&date_dir)
            .map_err(|error| format!("cannot read {}: {error}", date_dir.display()))?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        run_dirs.sort();
        for run_dir in run_dirs {
            let manifest_path = run_dir.join("manifest.json");
            if !manifest_path.is_file() {
                continue;
            }
            let value = store::read_json(&manifest_path)?;
            let errors = manifest::validate(&value, repo_root);
            if !errors.is_empty() {
                return Err(format!(
                    "{}: invalid manifest: {}",
                    manifest_path.display(),
                    errors.join("; ")
                ));
            }
            let run_id = value
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            let date = store::run_date(&run_id)?;
            if date
                != date_dir
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            {
                return Err(format!(
                    "{}: run_id date {date} does not match directory {}",
                    manifest_path.display(),
                    date_dir.display()
                ));
            }
            let mut row = Map::new();
            row.insert("run_id".to_owned(), json!(run_id));
            row.insert("date".to_owned(), json!(date));
            row.insert(
                "tier".to_owned(),
                value.get("tier").cloned().unwrap_or(Value::Null),
            );
            row.insert(
                "hypothesis".to_owned(),
                json!(collapse(
                    value
                        .get("hypothesis")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                )),
            );
            row.insert(
                "changed".to_owned(),
                value.get("changed").cloned().unwrap_or(Value::Null),
            );
            row.insert(
                "frozen".to_owned(),
                json!(
                    value
                        .get("frozen")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            row.insert(
                "baseline".to_owned(),
                json!(baseline_cell(value.get("baseline"))),
            );
            row.insert(
                "partition".to_owned(),
                value.get("partition").cloned().unwrap_or(Value::Null),
            );
            row.insert("verdict".to_owned(), json!(verdict(&run_dir)?));
            row.insert("artifacts".to_owned(), json!(artifact_count(&run_dir)));
            rows.push(Value::Object(row));
        }
    }
    rows.sort_by(|left, right| {
        let key = |row: &Value| {
            (
                row.get("date")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
                TIER_RANK
                    .iter()
                    .find(|(tier, _)| row.get("tier").and_then(Value::as_str) == Some(*tier))
                    .map_or(3, |(_, rank)| *rank),
                row.get("run_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned(),
            )
        };
        key(left).cmp(&key(right))
    });
    Ok(rows)
}

fn verdict(run_dir: &Path) -> Result<String, String> {
    let results_path = run_dir.join("results.json");
    if results_path.is_file() {
        let results = store::read_json(&results_path)?;
        if let Some(results) = results.as_object() {
            if let Some(verdict) = results.get("verdict").and_then(Value::as_str)
                && !verdict.trim().is_empty()
            {
                return Ok(verdict.trim().to_owned());
            }
            if results.get("ok").and_then(Value::as_bool) == Some(false) {
                return Ok("failed".to_owned());
            }
        }
    }
    let decision_path = run_dir.join("decision.md");
    if decision_path.is_file() {
        let text = fs::read_to_string(&decision_path)
            .map_err(|error| format!("cannot read {}: {error}", decision_path.display()))?;
        for line in text.lines() {
            if let Some(value) = verdict_line(line) {
                if !value.is_empty() {
                    return Ok(value);
                }
                break;
            }
        }
    }
    Ok("pending".to_owned())
}

fn verdict_line(line: &str) -> Option<String> {
    let mut rest = line.trim_start();
    if let Some(value) = rest.strip_prefix("- ").or_else(|| rest.strip_prefix("* ")) {
        rest = value;
    }
    if let Some(value) = rest.strip_prefix("**") {
        rest = value;
    }
    let rest = rest
        .strip_prefix("Verdict")
        .or_else(|| rest.strip_prefix("verdict"))?;
    let rest = rest.strip_prefix("**").unwrap_or(rest);
    let rest = rest.strip_prefix(':')?.trim();
    Some(
        rest.trim_matches(|ch| matches!(ch, '*' | '`' | '_' | ' '))
            .to_owned(),
    )
}

fn artifact_count(run_dir: &Path) -> usize {
    let artifacts = run_dir.join("artifacts");
    let mut files = Vec::new();
    if artifacts.is_dir() {
        collect(&artifacts, &mut files);
    }
    files.len()
}

fn baseline_cell(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.trim().to_owned(),
        Some(Value::Object(object)) => object.get("run_id").and_then(Value::as_str).map_or_else(
            || store::compact_json(value.unwrap_or(&Value::Null)),
            |value| value.trim().to_owned(),
        ),
        Some(value) => store::compact_json(value),
    }
}

fn render_markdown(rows: &[Value]) -> String {
    let mut lines = vec![
        "# Improvement run index".to_owned(),
        String::new(),
        format!("Runs: {}.", rows.len()),
        String::new(),
        format!("| {} |", COLUMNS.join(" | ")),
        format!(
            "|{}|",
            COLUMNS.iter().map(|_| "---").collect::<Vec<_>>().join("|")
        ),
    ];
    for row in rows {
        let values = [
            cell(row.get("run_id")),
            cell(row.get("date")),
            cell(row.get("tier")),
            cell(row.get("hypothesis")),
            cell(row.get("changed")),
            cell(row.get("frozen")),
            cell(row.get("baseline")),
            cell(row.get("partition")),
            cell(row.get("verdict")),
            cell(row.get("artifacts")),
        ];
        lines.push(format!("| {} |", values.join(" | ")));
    }
    format!("{}\n", lines.join("\n"))
}

fn cell(value: Option<&Value>) -> String {
    let text = match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => value.to_string(),
    };
    collapse(&text).replace('|', "\\|")
}

fn collapse(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn valid_date(name: &str) -> bool {
    name.len() == 10
        && name.as_bytes()[4] == b'-'
        && name.as_bytes()[7] == b'-'
        && name
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else {
            found.push(path);
        }
    }
}
