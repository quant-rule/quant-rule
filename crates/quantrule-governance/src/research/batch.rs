//! Run pre-registered research cells and write a batch summary.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};

use super::cell;
use super::program::{self, Cell, Program};

const SCHEMA: &str = "quantrule.research-batch/v1";

/// Run `research batch`.
///
/// # Errors
///
/// Returns a message for an invalid program, a failed worktree command, or a
/// write failure.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut program_path = None;
    let mut phase = "dev".to_owned();
    let mut mode = "config".to_owned();
    let mut max_parallel = None;
    let mut cells_raw = None;
    let mut worktree_root = None;
    let mut base = "HEAD".to_owned();
    let mut prepare_only = false;
    let mut out = None;
    let mut repo_root = PathBuf::from(".");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--program" => program_path = Some(value(args, &mut index, "--program")?),
            "--phase" => phase = value(args, &mut index, "--phase")?,
            "--mode" => mode = value(args, &mut index, "--mode")?,
            "--max-parallel" => {
                max_parallel = Some(
                    value(args, &mut index, "--max-parallel")?
                        .parse::<usize>()
                        .map_err(|_| "--max-parallel invalid".to_owned())?,
                )
            }
            "--cells" => cells_raw = Some(value(args, &mut index, "--cells")?),
            "--worktree-root" => worktree_root = Some(value(args, &mut index, "--worktree-root")?),
            "--base" => base = value(args, &mut index, "--base")?,
            "--prepare-only" => prepare_only = true,
            "--out" => out = Some(value(args, &mut index, "--out")?),
            "--repo-root" => repo_root = PathBuf::from(value(args, &mut index, "--repo-root")?),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let program_path =
        PathBuf::from(program_path.ok_or_else(|| "--program is required".to_owned())?);
    let program = program::load(&program_path, &repo_root)?;
    program::require_phase(&program, &phase)?;
    if phase == "validation" && cells_raw.is_none() {
        return Err("validation requires --cells promotion".to_owned());
    }
    let cells = selected_cells(&program, cells_raw.as_deref())?;
    let max_parallel = max_parallel.unwrap_or(program.max_parallel).clamp(1, 64);
    let out_root = out.map_or_else(
        || {
            program
                .repo_root
                .join("data/research")
                .join(&program.program_id)
                .join(&phase)
        },
        PathBuf::from,
    );
    let summary = if mode == "config" {
        run_config(&program, &phase, &cells, max_parallel, &out_root)?
    } else {
        let root =
            PathBuf::from(worktree_root.ok_or_else(|| "--worktree-root is required".to_owned())?);
        if prepare_only {
            let summary = prepare_worktrees(&program, &cells, &root, &base)?;
            fs::create_dir_all(&out_root)
                .map_err(|error| format!("cannot create {}: {error}", out_root.display()))?;
            crate::improvement::store::write_json(&out_root.join("batch-summary.json"), &summary)?;
            summary
        } else {
            run_worktrees(
                &program,
                &phase,
                &cells,
                &root,
                &base,
                max_parallel,
                &out_root,
            )?
        }
    };
    println!(
        "ok batch {} {} cells={}",
        summary["program_id"].as_str().unwrap_or(""),
        summary["phase"].as_str().unwrap_or(""),
        summary["cells"].as_array().map_or(0, Vec::len)
    );
    Ok(())
}

fn selected_cells(program: &Program, raw: Option<&str>) -> Result<Vec<Cell>, String> {
    let Some(raw) = raw else {
        return Ok(program.cells.clone());
    };
    let ids: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();
    if ids.is_empty() {
        return Err("--cells selected no cell".to_owned());
    }
    ids.into_iter()
        .map(|cell_id| program.cell(cell_id).cloned())
        .collect()
}

fn run_config(
    program: &Program,
    phase: &str,
    cells: &[Cell],
    max_parallel: usize,
    out_root: &Path,
) -> Result<Value, String> {
    let _ = max_parallel;
    fs::create_dir_all(out_root)
        .map_err(|error| format!("cannot create {}: {error}", out_root.display()))?;
    let mut results = Vec::new();
    for cell in cells {
        let record = cell::run_cell(program, cell, phase, &out_root.join(&cell.cell_id)).unwrap_or_else(|error| {
            json!({"cell_id": cell.cell_id, "verdict": "fail", "metrics": {}, "limitations": [format!("runner error: {error}")]})
        });
        results.push(json!({"cell_id": cell.cell_id, "out": out_root.join(&cell.cell_id).display().to_string(), "verdict": record["verdict"]}));
    }
    results.sort_by(|left, right| left["cell_id"].as_str().cmp(&right["cell_id"].as_str()));
    let summary = json!({
        "schema": SCHEMA, "program_id": program.program_id, "phase": phase,
        "mode": "config", "max_parallel": max_parallel, "cells": results
    });
    crate::improvement::store::write_json(&out_root.join("batch-summary.json"), &summary)?;
    Ok(summary)
}

fn prepare_worktrees(
    program: &Program,
    cells: &[Cell],
    root: &Path,
    base: &str,
) -> Result<Value, String> {
    fs::create_dir_all(root)
        .map_err(|error| format!("cannot create {}: {error}", root.display()))?;
    let mut entries = Vec::new();
    for cell in cells {
        let path = root.join(&cell.cell_id);
        if path.exists() {
            return Err(format!("worktree path already exists: {}", path.display()));
        }
        let branch = format!("research/{}-{}", program.program_id, cell.cell_id);
        let output = Command::new("git")
            .arg("-C")
            .arg(&program.repo_root)
            .args([
                "worktree",
                "add",
                "-b",
                &branch,
                &path.display().to_string(),
                base,
            ])
            .output()
            .map_err(|error| format!("cannot start git worktree: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "git worktree add failed for {}: {}",
                cell.cell_id,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        entries.push(json!({"cell_id": cell.cell_id, "worktree": path.display().to_string(), "branch": branch, "status": "prepared"}));
    }
    Ok(
        json!({"schema": SCHEMA, "program_id": program.program_id, "phase": "prepare", "mode": "worktree", "cells": entries}),
    )
}

fn run_worktrees(
    program: &Program,
    phase: &str,
    cells: &[Cell],
    root: &Path,
    base: &str,
    max_parallel: usize,
    out_root: &Path,
) -> Result<Value, String> {
    let prepared = prepare_worktrees(program, cells, root, base)?;
    let relative_program = program
        .path
        .strip_prefix(&program.repo_root)
        .unwrap_or(&program.path)
        .to_path_buf();
    fs::create_dir_all(out_root)
        .map_err(|error| format!("cannot create {}: {error}", out_root.display()))?;
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate current executable: {error}"))?;
    let mut entries = Vec::new();
    for entry in prepared["cells"].as_array().into_iter().flatten() {
        let worktree = PathBuf::from(entry["worktree"].as_str().unwrap_or(""));
        let cell_id = entry["cell_id"].as_str().unwrap_or("");
        let cell_out = worktree
            .join("data/research")
            .join(&program.program_id)
            .join(phase)
            .join(cell_id);
        let output = Command::new(&executable)
            .current_dir(&worktree)
            .args([
                "research",
                "cell",
                "--program",
                &relative_program.display().to_string(),
                "--cell",
                cell_id,
                "--phase",
                phase,
                "--out",
                &cell_out.display().to_string(),
                "--repo-root",
                ".",
            ])
            .output()
            .map_err(|error| format!("cannot start cell {cell_id}: {error}"))?;
        let copied = out_root.join(cell_id).join("cell-result.json");
        if let Some(parent) = copied.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        let source = cell_out.join("cell-result.json");
        if source.is_file() {
            fs::copy(&source, &copied)
                .map_err(|error| format!("cannot copy cell result: {error}"))?;
        }
        entries.push(json!({
            "cell_id": cell_id, "worktree": worktree.display().to_string(),
            "branch": entry["branch"].clone(), "out": cell_out.display().to_string(),
            "exit_code": output.status.code(), "stderr": String::from_utf8_lossy(&output.stderr).trim(),
            "status": "ran"
        }));
    }
    entries.sort_by(|left, right| left["cell_id"].as_str().cmp(&right["cell_id"].as_str()));
    let summary = json!({"schema": SCHEMA, "program_id": program.program_id, "phase": phase, "mode": "worktree", "max_parallel": max_parallel, "cells": entries});
    crate::improvement::store::write_json(&out_root.join("batch-summary.json"), &summary)?;
    Ok(summary)
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

    fn program() -> Program {
        Program {
            path: PathBuf::from("program.json"),
            repo_root: PathBuf::from("."),
            program_id: "PROGRAM-2026-11-21-01".to_owned(),
            premise: "p".to_owned(),
            tier: "model".to_owned(),
            partition_dev: "model_dev".to_owned(),
            partition_validation: "model_validation".to_owned(),
            validation_access: "none".to_owned(),
            correction: json!({}),
            green_criteria: json!({}),
            max_parallel: 1,
            cells: vec![Cell {
                cell_id: "C1".to_owned(),
                hypothesis: "h".to_owned(),
                primary: true,
                config_dir: ".".to_owned(),
                command: vec!["echo".to_owned(), "{out}".to_owned()],
                result_parser: "none".to_owned(),
            }],
        }
    }

    #[test]
    fn selected_cells_filters_and_rejects_empty() {
        assert_eq!(selected_cells(&program(), None).expect("all").len(), 1);
        assert_eq!(
            selected_cells(&program(), Some("C1")).expect("one").len(),
            1
        );
        assert!(selected_cells(&program(), Some(",")).is_err());
    }
}
