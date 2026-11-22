//! Research program manifest: schema, validation, and phase rules.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub(super) const SCHEMA: &str = "quantrule.research-program/v1";
const TIERS: [&str; 3] = ["model", "strategy", "portfolio"];
const PARTITIONS: [&str; 6] = [
    "model_dev",
    "model_validation",
    "strategy_dev",
    "strategy_validation",
    "portfolio_dev",
    "portfolio_holdout",
];
const PARSERS: [&str; 5] = [
    "path_validation",
    "strategy_evaluation",
    "model_prediction",
    "summary_json",
    "none",
];

/// One pre-registered cell.
#[derive(Clone)]
pub(super) struct Cell {
    pub cell_id: String,
    pub hypothesis: String,
    pub primary: bool,
    pub config_dir: String,
    pub command: Vec<String>,
    pub result_parser: String,
}

/// One validated research program.
pub(super) struct Program {
    pub path: PathBuf,
    pub repo_root: PathBuf,
    pub program_id: String,
    pub premise: String,
    pub tier: String,
    pub partition_dev: String,
    pub partition_validation: String,
    pub validation_access: String,
    pub correction: Value,
    pub green_criteria: Value,
    pub max_parallel: usize,
    pub retired: bool,
    pub retirement_note: Option<String>,
    pub cells: Vec<Cell>,
}

impl Program {
    pub(super) fn primary_cells(&self) -> Vec<&Cell> {
        self.cells.iter().filter(|cell| cell.primary).collect()
    }

    pub(super) fn cell(&self, cell_id: &str) -> Result<&Cell, String> {
        self.cells
            .iter()
            .find(|cell| cell.cell_id == cell_id)
            .ok_or_else(|| format!("unknown cell: {cell_id}"))
    }
}

/// Load and validate a program manifest.
///
/// # Errors
///
/// Returns the first schema or path error.
pub(super) fn load(path: &Path, repo_root: &Path) -> Result<Program, String> {
    if !path.is_file() {
        return Err(format!("program manifest not found: {}", path.display()));
    }
    let document = crate::improvement::store::read_json(path)?;
    let Some(document) = document.as_object() else {
        return Err("program manifest must be an object".to_owned());
    };
    if document.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
        return Err(format!("$.schema must be {SCHEMA}"));
    }
    let program_id = text(document.get("program_id"), "$.program_id")?;
    if !program_id_matches(&program_id) {
        return Err("$.program_id must match PROGRAM-YYYY-MM-DD-NN".to_owned());
    }
    let premise = text(document.get("premise"), "$.premise")?;
    let rationale = text(document.get("economic_rationale"), "$.economic_rationale")?;
    let _ = rationale;
    let tier = text(document.get("tier"), "$.tier")?;
    if !TIERS.contains(&tier.as_str()) {
        return Err("$.tier must be model, strategy, or portfolio".to_owned());
    }
    let changed = text(document.get("changed"), "$.changed")?;
    if changed != tier {
        return Err("$.changed must equal $.tier".to_owned());
    }
    let frozen = array(document.get("frozen"), "$.frozen")?
        .iter()
        .map(|value| text(Some(value), "$.frozen[]"))
        .collect::<Result<Vec<_>, _>>()?;
    if frozen.iter().any(|item| !TIERS.contains(&item.as_str()))
        || frozen.contains(&changed)
        || frozen.iter().collect::<BTreeSet<_>>().len() != frozen.len()
    {
        return Err("$.frozen must not contain $.changed or duplicates".to_owned());
    }
    let partition_dev = text(document.get("partition_dev"), "$.partition_dev")?;
    let partition_validation = text(
        document.get("partition_validation"),
        "$.partition_validation",
    )?;
    if !PARTITIONS.contains(&partition_dev.as_str())
        || !PARTITIONS.contains(&partition_validation.as_str())
    {
        return Err("$.partition_dev and $.partition_validation must be declared".to_owned());
    }
    let costs = mapping(document.get("costs"), "$.costs")?;
    for key in ["commission_bps", "slippage_bps", "funding"] {
        let value = costs.get(key).and_then(Value::as_f64);
        if value.is_none_or(|value| value < 0.0) {
            return Err(format!("$.costs.{key} must be a non-negative number"));
        }
    }
    let green_criteria =
        Value::Object(mapping(document.get("green_criteria"), "$.green_criteria")?.clone());
    let correction = mapping(document.get("correction"), "$.correction")?;
    let method = correction
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("");
    if method != "bonferroni" && method != "none" {
        return Err("$.correction.method must be bonferroni or none".to_owned());
    }
    let primary_cells = integer(
        correction.get("primary_cells"),
        "$.correction.primary_cells",
        1,
        64,
    )?;
    if method == "none" && primary_cells != 1 {
        return Err("$.correction.method none requires one primary cell".to_owned());
    }
    let max_parallel = integer(document.get("max_parallel"), "$.max_parallel", 1, 64)? as usize;
    let _ = integer(document.get("iteration_cap"), "$.iteration_cap", 1, 100)?;
    let retired = document
        .get("retired")
        .map(|value| value.as_bool().ok_or("$.retired must be a boolean"))
        .transpose()?
        .unwrap_or(false);
    let retirement_note = document
        .get("retirement_note")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if retired && retirement_note.is_none() {
        return Err("$.retirement_note is required when $.retired is true".to_owned());
    }
    let validation_access = text(document.get("validation_access"), "$.validation_access")?;
    if validation_access != "none" && validation_access != "one_shot" {
        return Err("$.validation_access must be none or one_shot".to_owned());
    }
    let raw_cells = array(document.get("cells"), "$.cells")?;
    if raw_cells.is_empty() {
        return Err("$.cells must be a non-empty list".to_owned());
    }
    if raw_cells.len() > 64 {
        return Err("$.cells exceeds 64".to_owned());
    }
    let mut cells = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, raw) in raw_cells.iter().enumerate() {
        let prefix = format!("$.cells[{index}]");
        let Some(raw) = raw.as_object() else {
            return Err(format!("{prefix} must be an object"));
        };
        let cell_id = text(raw.get("cell_id"), &format!("{prefix}.cell_id"))?;
        if !seen.insert(cell_id.clone()) {
            return Err(format!("duplicate cell_id: {cell_id}"));
        }
        let hypothesis = text(raw.get("hypothesis"), &format!("{prefix}.hypothesis"))?;
        let Some(primary) = raw.get("primary").and_then(Value::as_bool) else {
            return Err(format!("{prefix}.primary must be a boolean"));
        };
        let config_dir = text(raw.get("config_dir"), &format!("{prefix}.config_dir"))?;
        if !repo_root.join(&config_dir).is_dir() {
            return Err(format!("{prefix}.config_dir does not exist: {config_dir}"));
        }
        let command = array(raw.get("command"), &format!("{prefix}.command"))?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("{prefix}.command must be a non-empty string list"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if !command.iter().any(|item| item.contains("{out}")) {
            return Err(format!("{prefix}.command must contain {{out}}"));
        }
        let parser = raw
            .get("result_parser")
            .and_then(Value::as_str)
            .unwrap_or("none");
        if !PARSERS.contains(&parser) {
            return Err(format!("{prefix}.result_parser is not registered"));
        }
        cells.push(Cell {
            cell_id,
            hypothesis,
            primary,
            config_dir,
            command,
            result_parser: parser.to_owned(),
        });
    }
    let primary_count = cells.iter().filter(|cell| cell.primary).count();
    if primary_count == 0 {
        return Err("$.cells must contain at least one primary cell".to_owned());
    }
    if primary_count != primary_cells as usize {
        return Err("$.correction.primary_cells must equal the number of primary cells".to_owned());
    }
    Ok(Program {
        path: path.to_path_buf(),
        repo_root: repo_root.to_path_buf(),
        program_id,
        premise,
        tier,
        partition_dev,
        partition_validation,
        validation_access,
        correction: Value::Object(correction.clone()),
        green_criteria,
        max_parallel,
        retired,
        retirement_note,
        cells,
    })
}

/// Reject a validation phase when the program has no one-shot access.
///
/// # Errors
///
/// Returns a message for an unknown phase or a closed validation partition.
pub(super) fn require_phase(program: &Program, phase: &str) -> Result<(), String> {
    if phase != "dev" && phase != "validation" {
        return Err("phase must be dev or validation".to_owned());
    }
    if phase == "validation" && program.validation_access != "one_shot" {
        return Err("validation_access is not one_shot".to_owned());
    }
    Ok(())
}

/// Run `research program`.
///
/// # Errors
///
/// Returns a message for an invalid manifest or phase.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let mut path = None;
    let mut phase = "dev".to_owned();
    let mut repo_root = PathBuf::from(".");
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--program" => path = Some(value(args, &mut index, "--program")?),
            "--phase" => phase = value(args, &mut index, "--phase")?,
            "--repo-root" => repo_root = PathBuf::from(value(args, &mut index, "--repo-root")?),
            "--check" => {}
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    let path = PathBuf::from(path.ok_or_else(|| "--program is required".to_owned())?);
    let program = load(&path, &repo_root)?;
    require_phase(&program, &phase)?;
    println!(
        "ok {} cells={} primary={} phase={phase}{}",
        program.program_id,
        program.cells.len(),
        program.primary_cells().len(),
        if program.retired { " retired" } else { "" }
    );
    Ok(())
}

/// Refuse to run a historical program.
///
/// # Errors
///
/// Returns the retirement reason.
pub(super) fn require_live(program: &Program) -> Result<(), String> {
    if program.retired {
        return Err(format!(
            "program {} is retired: {}",
            program.program_id,
            program
                .retirement_note
                .as_deref()
                .unwrap_or("historical record")
        ));
    }
    Ok(())
}

fn value(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}

fn text(value: Option<&Value>, path: &str) -> Result<String, String> {
    value
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim().to_owned())
        .ok_or_else(|| format!("{path} must be a non-empty string"))
}

fn mapping<'a>(
    value: Option<&'a Value>,
    path: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .and_then(Value::as_object)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{path} must be a non-empty object"))
}

fn array<'a>(value: Option<&'a Value>, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path} must be a list"))
}

fn integer(value: Option<&Value>, path: &str, minimum: i64, maximum: i64) -> Result<i64, String> {
    let Some(value) = value.and_then(Value::as_i64) else {
        return Err(format!("{path} must be an integer"));
    };
    if value < minimum || value > maximum {
        return Err(format!("{path} must be in {minimum}..{maximum}"));
    }
    Ok(value)
}

fn program_id_matches(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with("PROGRAM-")
        && value.len() == 21
        && bytes[8..12].iter().all(u8::is_ascii_digit)
        && bytes[12] == b'-'
        && bytes[13..15].iter().all(u8::is_ascii_digit)
        && bytes[15] == b'-'
        && bytes[16..18].iter().all(u8::is_ascii_digit)
        && bytes[18] == b'-'
        && bytes[19..21].iter().all(u8::is_ascii_digit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn manifest(retired: bool, note: Option<&str>) -> Value {
        let mut manifest = serde_json::json!({
            "schema": SCHEMA, "program_id": "PROGRAM-2026-11-21-01", "premise": "p",
            "economic_rationale": "r", "tier": "model", "changed": "model",
            "frozen": ["strategy", "portfolio"], "partition_dev": "model_dev",
            "partition_validation": "model_validation",
            "costs": {"commission_bps": 1, "slippage_bps": 1, "funding": 0},
            "green_criteria": {"min_trades": 1},
            "correction": {"method": "none", "primary_cells": 1},
            "max_parallel": 1, "iteration_cap": 1, "validation_access": "one_shot",
            "stop_rule": "stop",
            "cells": [{"cell_id": "C1", "hypothesis": "h", "primary": true,
                "config_dir": "cell", "command": ["echo", "{out}"], "result_parser": "none"}]
        });
        if retired {
            manifest["retired"] = Value::Bool(true);
        }
        if let Some(note) = note {
            manifest["retirement_note"] = Value::String(note.to_owned());
        }
        manifest
    }

    #[test]
    fn a_valid_program_loads() {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("qr-program-{}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("cell")).expect("tree");
        let manifest = manifest(false, None);
        std::fs::write(
            root.join("program.json"),
            serde_json::to_string(&manifest).expect("json"),
        )
        .expect("write");
        let loaded = load(&root.join("program.json"), &root).expect("load");
        assert_eq!(loaded.program_id, "PROGRAM-2026-11-21-01");
        assert_eq!(loaded.primary_cells().len(), 1);
        require_phase(&loaded, "validation").expect("phase");
        std::fs::remove_dir_all(root).expect("cleanup");
    }
    #[test]
    fn a_retired_program_requires_a_note() {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "qr-program-retired-{}-{unique}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("cell")).expect("tree");
        std::fs::write(
            root.join("program.json"),
            serde_json::to_string(&manifest(true, None)).expect("json"),
        )
        .expect("write");
        let error = match load(&root.join("program.json"), &root) {
            Ok(_) => panic!("retired without note"),
            Err(error) => error,
        };
        assert!(error.contains("retirement_note"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn a_retired_program_is_refused_by_the_runner() {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("qr-program-live-{}-{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("cell")).expect("tree");
        std::fs::write(
            root.join("program.json"),
            serde_json::to_string(&manifest(true, Some("historical evidence"))).expect("json"),
        )
        .expect("write");
        let loaded = load(&root.join("program.json"), &root).expect("load");
        let error = require_live(&loaded).expect_err("retired");
        assert!(error.contains("historical evidence"));
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}
