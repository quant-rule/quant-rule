//! Run one pre-registered research cell and write one result.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::adapters;
use super::program::{self, Cell, Program};

const SCHEMA: &str = "quantrule.research-cell-result/v1";
const STREAM_LIMIT: usize = 8_000;

/// Run `research cell`.
///
/// # Errors
///
/// Returns a message for an invalid program, a failed command, or a write
/// failure.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut program_path = None;
    let mut cell_id = None;
    let mut phase = "dev".to_owned();
    let mut out = None;
    let mut repo_root = PathBuf::from(".");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--program" => program_path = Some(value(args, &mut index, "--program")?),
            "--cell" => cell_id = Some(value(args, &mut index, "--cell")?),
            "--phase" => phase = value(args, &mut index, "--phase")?,
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--repo-root" => repo_root = PathBuf::from(value(args, &mut index, "--repo-root")?),
            "--timeout" => {
                let _ = value(args, &mut index, "--timeout")?;
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let program_path =
        PathBuf::from(program_path.ok_or_else(|| "--program is required".to_owned())?);
    let cell_id = cell_id.ok_or_else(|| "--cell is required".to_owned())?;
    let out = PathBuf::from(out.ok_or_else(|| "--out is required".to_owned())?);
    let loaded = program::load(&program_path, &repo_root)?;
    let cell = loaded.cell(&cell_id)?.clone();
    let record = run_cell(&loaded, &cell, &phase, &out)?;
    println!(
        "ok {} {} -> {}",
        record["cell_id"].as_str().unwrap_or(""),
        record["verdict"].as_str().unwrap_or(""),
        out.display()
    );
    Ok(())
}

fn run_cell(program: &Program, cell: &Cell, phase: &str, out: &Path) -> Result<Value, String> {
    program::require_phase(program, phase)?;
    fs::create_dir_all(out).map_err(|error| format!("cannot create {}: {error}", out.display()))?;
    let command: Vec<String> = cell
        .command
        .iter()
        .map(|token| {
            token
                .replace("{config}", &cell.config_dir)
                .replace("{out}", &out.display().to_string())
                .replace("{phase}", phase)
        })
        .collect();
    let output = Command::new(&command[0])
        .args(&command[1..])
        .current_dir(&program.repo_root)
        .output()
        .map_err(|error| format!("cannot start {}: {error}", command[0]))?;
    let exit_code = output.status.code();
    let stdout = tail(&String::from_utf8_lossy(&output.stdout));
    let stderr = tail(&String::from_utf8_lossy(&output.stderr));
    let mut limitations = Vec::new();
    let mut metrics = Map::new();
    let mut verdict = None;
    match adapters::parse(&cell.result_parser, out) {
        Ok(parsed) => {
            metrics = parsed.metrics;
            verdict = parsed.verdict;
            limitations.extend(parsed.limitations);
        }
        Err(error) => limitations.push(format!("result parse failed: {error}")),
    }
    let verdict = match verdict {
        Some(value) if value == "pass" || value == "fail" => value,
        Some(_) => "fail".to_owned(),
        None => if exit_code == Some(0) { "pass" } else { "fail" }.to_owned(),
    };
    let record = json!({
        "schema": SCHEMA,
        "program_id": program.program_id,
        "cell_id": cell.cell_id,
        "hypothesis": cell.hypothesis,
        "phase": phase,
        "command": command,
        "exit_code": exit_code,
        "stdout": stdout,
        "stderr": stderr,
        "artifacts": artifacts(out)?,
        "metrics": metrics,
        "spec_id": metrics.get("spec_id").cloned().unwrap_or(Value::Null),
        "prediction_digest": metrics.get("prediction_digest").cloned().unwrap_or(Value::Null),
        "verdict": verdict,
        "limitations": limitations,
    });
    crate::improvement::store::write_json(&out.join("cell-result.json"), &record)?;
    Ok(record)
}

fn artifacts(out: &Path) -> Result<Vec<Value>, String> {
    let mut files = Vec::new();
    collect(out, &mut files);
    files.sort();
    let mut values = Vec::new();
    for path in files {
        if path.file_name().and_then(|name| name.to_str()) == Some("cell-result.json") {
            continue;
        }
        values.push(json!({
            "path": path.strip_prefix(out).unwrap_or(&path).to_string_lossy().replace('\\', "/"),
            "sha256": file_digest(&path)?,
        }));
    }
    Ok(values)
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

fn tail(value: &str) -> String {
    if value.chars().count() <= STREAM_LIMIT {
        value.to_owned()
    } else {
        value
            .chars()
            .skip(value.chars().count() - STREAM_LIMIT)
            .collect()
    }
}

fn file_digest(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
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
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[test]
    fn a_none_parser_uses_the_exit_code() {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("qr-cell-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("cell")).expect("tree");
        let program = Program {
            repo_root: root.clone(),
            program_id: "PROGRAM-2026-11-21-01".to_owned(),
            premise: "p".to_owned(),
            tier: "model".to_owned(),
            partition_dev: "model_dev".to_owned(),
            partition_validation: "model_validation".to_owned(),
            validation_access: "none".to_owned(),
            cells: Vec::new(),
        };
        let cell = Cell {
            cell_id: "C1".to_owned(),
            hypothesis: "h".to_owned(),
            primary: true,
            config_dir: root.join("cell").display().to_string(),
            command: vec![
                "sh".to_owned(),
                "-c".to_owned(),
                "echo hi > {out}/x.txt".to_owned(),
            ],
            result_parser: "none".to_owned(),
        };
        let out = root.join("out");
        let record = run_cell(&program, &cell, "dev", &out).expect("run");
        assert_eq!(record["verdict"], "pass");
        assert_eq!(record["artifacts"].as_array().map(Vec::len), Some(1));
        assert!(out.join("cell-result.json").is_file());
        fs::remove_dir_all(root).expect("cleanup");
    }
}
