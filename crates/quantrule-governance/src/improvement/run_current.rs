//! Execute one improvement run on the current stage-command surface.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::index;
use super::manifest;
use super::store::{self, ConfigPaths};

const DEFAULT_SHELL: &str = "target/debug/quantrule";

/// Build the ordered commands for one manifest.
#[must_use]
pub fn commands_for(
    manifest: &Value,
    qr: &Path,
    root: &Path,
    paths: &ConfigPaths,
) -> Vec<Vec<String>> {
    let artifacts = root.join("artifacts");
    let tier = manifest.get("tier").and_then(Value::as_str).unwrap_or("");
    if tier == "model" {
        return vec![vec![
            qr.display().to_string(),
            "report".to_owned(),
            "model".to_owned(),
            "--config".to_owned(),
            paths.run_config.display().to_string(),
            "--out".to_owned(),
            artifacts.display().to_string(),
            "--json".to_owned(),
        ]];
    }
    let mut commands = vec![vec![
        qr.display().to_string(),
        "backtest".to_owned(),
        "run".to_owned(),
        "--config".to_owned(),
        paths.run_config.display().to_string(),
        "--strategy".to_owned(),
        paths
            .strategy_config
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        "--out".to_owned(),
        artifacts.display().to_string(),
        "--json".to_owned(),
    ]];
    if let Some(criteria) = &paths.qualification_criteria {
        commands.push(vec![
            qr.display().to_string(),
            "qualify".to_owned(),
            "--evidence".to_owned(),
            artifacts.join("backtest").display().to_string(),
            "--criteria".to_owned(),
            criteria.display().to_string(),
            "--out".to_owned(),
            artifacts.display().to_string(),
            "--json".to_owned(),
        ]);
    }
    if manifest
        .get("analytics")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        commands.push(vec![
            qr.display().to_string(),
            "analytics".to_owned(),
            "evaluate".to_owned(),
            "--evidence".to_owned(),
            artifacts.join("backtest").display().to_string(),
            "--out".to_owned(),
            artifacts.display().to_string(),
            "--json".to_owned(),
        ]);
    }
    commands
}

/// Parse one JSON document from stdout.
#[must_use]
pub fn parse_stdout(stdout: &str) -> (Option<Value>, Option<String>) {
    match serde_json::from_str(stdout) {
        Ok(value) => (Some(value), None),
        Err(error) => (
            None,
            Some(format!("stdout is not one JSON document: {error}")),
        ),
    }
}

/// Execute one run.
///
/// # Errors
///
/// Returns a message for an invalid manifest, a preparation failure, a failed
/// command, or an index failure.
pub fn run(
    manifest_path: &Path,
    qr_raw: &str,
    repo_root: &Path,
    store_root: &Path,
) -> Result<(), String> {
    let manifest_path = manifest_path
        .canonicalize()
        .unwrap_or_else(|_| manifest_path.to_path_buf());
    let manifest = manifest::load(&manifest_path, repo_root)?;
    let root = store::output_root(&manifest, store_root, repo_root)?;
    store::ensure_empty_root(&root)?;
    let qr = store::resolve_shell(qr_raw, repo_root)?;
    let paths = store::prepare_config(&manifest_path, &manifest, &root.join("config"), repo_root)?;

    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot prepare run root {}: {error}", root.display()))?;
    fs::copy(&manifest_path, root.join("manifest.json"))
        .map_err(|error| format!("cannot prepare run root {}: {error}", root.display()))?;
    fs::create_dir_all(root.join("artifacts"))
        .map_err(|error| format!("cannot prepare run root {}: {error}", root.display()))?;
    store::write_text(&root.join("review.md"), &store::review_text(&manifest))?;
    store::write_text(&root.join("decision.md"), &store::decision_text(&manifest))?;

    let commands = commands_for(&manifest, &qr, &root, &paths);
    let mut records = Vec::new();
    let mut failed = false;
    for command in commands {
        let mut record = Map::new();
        record.insert("command".to_owned(), json!(command));
        match store::run_process(&command, repo_root) {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let (stdout_json, stdout_json_error) = parse_stdout(&stdout);
                record.insert("return_code".to_owned(), json!(output.status.code()));
                record.insert("ok".to_owned(), json!(output.status.success()));
                record.insert("stdout".to_owned(), json!(stdout));
                record.insert("stderr".to_owned(), json!(stderr));
                record.insert("stdout_json".to_owned(), stdout_json.unwrap_or(Value::Null));
                record.insert("stdout_json_error".to_owned(), json!(stdout_json_error));
                if !output.status.success() {
                    failed = true;
                }
            }
            Err(error) => {
                record.insert("return_code".to_owned(), Value::Null);
                record.insert("ok".to_owned(), json!(false));
                record.insert("stdout".to_owned(), json!(""));
                record.insert("stderr".to_owned(), json!(error));
                record.insert("stdout_json".to_owned(), Value::Null);
                record.insert("stdout_json_error".to_owned(), Value::Null);
                failed = true;
            }
        }
        let stop = !record.get("ok").and_then(Value::as_bool).unwrap_or(false);
        records.push(Value::Object(record));
        if stop {
            break;
        }
    }

    let run_id = manifest.get("run_id").and_then(Value::as_str).unwrap_or("");
    let mut result = Map::new();
    result.insert("schema".to_owned(), json!(store::RESULT_SCHEMA_V2));
    result.insert("run_id".to_owned(), json!(run_id));
    result.insert(
        "tier".to_owned(),
        manifest.get("tier").cloned().unwrap_or(Value::Null),
    );
    result.insert("commands".to_owned(), Value::Array(records.clone()));
    result.insert("ok".to_owned(), json!(!failed));
    result.insert(
        "artifacts".to_owned(),
        Value::Array(store::file_inventory(&root.join("artifacts"))?),
    );
    result.insert(
        "failure".to_owned(),
        if failed {
            json!("a command exited non-zero")
        } else {
            Value::Null
        },
    );
    let results_path = root.join("results.json");
    store::write_json(&results_path, &Value::Object(result))?;

    if failed {
        for record in &records {
            if record.get("ok").and_then(Value::as_bool) != Some(true)
                && let Some(stderr) = record.get("stderr").and_then(Value::as_str)
                && !stderr.trim().is_empty()
            {
                eprintln!("shell stderr:");
                eprintln!("{}", stderr.trim_end());
            }
        }
        return Err(format!("run failed; see {}", results_path.display()));
    }

    if !store::is_within(&root, store_root) {
        eprintln!(
            "note: output root is outside data/improvement; the canonical index does not include it"
        );
    }
    fs::create_dir_all(store_root)
        .map_err(|error| format!("cannot create {}: {error}", store_root.display()))?;
    let count = index::build(store_root, repo_root)?;
    println!("indexed {count} run(s) from {}", store_root.display());
    println!("ok {run_id} -> {}", root.display());
    Ok(())
}

/// The default shell path.
#[must_use]
pub fn default_shell() -> PathBuf {
    PathBuf::from(DEFAULT_SHELL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_manifest() -> Value {
        json!({
            "schema": store::MANIFEST_SCHEMA,
            "run_id": "MODEL-2026-11-07-01",
            "tier": "model"
        })
    }

    fn strategy_manifest() -> Value {
        json!({
            "schema": store::MANIFEST_SCHEMA,
            "run_id": "STRATEGY-2026-11-07-01",
            "tier": "strategy",
            "strategy_config": "config/strategy/example.json",
            "qualification_criteria": "config/qualification/example.json"
        })
    }

    fn paths() -> ConfigPaths {
        ConfigPaths {
            run_config: PathBuf::from("/config/run.json"),
            strategy_config: Some(PathBuf::from("/config/strategy.json")),
            qualification_criteria: Some(PathBuf::from("/config/criteria.json")),
        }
    }

    #[test]
    fn model_uses_report_model() {
        let commands = commands_for(
            &model_manifest(),
            Path::new("/qr"),
            Path::new("/run"),
            &paths(),
        );
        assert_eq!(commands.len(), 1);
        assert_eq!(&commands[0][..3], ["/qr", "report", "model"]);
    }

    #[test]
    fn strategy_uses_backtest_qualify_analytics() {
        let commands = commands_for(
            &strategy_manifest(),
            Path::new("/qr"),
            Path::new("/run"),
            &paths(),
        );
        assert_eq!(commands.len(), 3);
        assert_eq!(&commands[0][..3], ["/qr", "backtest", "run"]);
        assert_eq!(&commands[1][..2], ["/qr", "qualify"]);
        assert_eq!(&commands[2][..3], ["/qr", "analytics", "evaluate"]);
    }

    #[test]
    fn analytics_can_be_disabled() {
        let mut manifest = strategy_manifest();
        manifest["analytics"] = json!(false);
        let commands = commands_for(&manifest, Path::new("/qr"), Path::new("/run"), &paths());
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn parse_stdout_accepts_one_json_document() {
        let (value, error) = parse_stdout("{\"ok\": true}\n");
        assert_eq!(value, Some(json!({"ok": true})));
        assert!(error.is_none());
    }

    #[test]
    fn parse_stdout_rejects_text() {
        let (value, error) = parse_stdout("not json\n");
        assert!(value.is_none());
        assert!(error.is_some());
    }
}
