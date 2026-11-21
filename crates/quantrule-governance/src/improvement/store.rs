//! Shared improvement run-store helpers: paths, JSON, templates, and inventory.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

/// Constants shared by the run store.
pub const MANIFEST_SCHEMA: &str = "quantrule.improvement-run/v2";
pub const RESULT_SCHEMA_V2: &str = "quantrule.improvement-result/v2";
pub const CONFIG_SCHEMA: &str = "quantrule.improvement-config/v1";
pub const INDEX_SCHEMA: &str = "quantrule.improvement-index/v1";
pub const TIERS: [&str; 3] = ["model", "strategy", "portfolio"];

/// The resolved configuration paths for one run.
pub struct ConfigPaths {
    /// Run config path.
    pub run_config: PathBuf,
    /// Strategy config path.
    pub strategy_config: Option<PathBuf>,
    /// Qualification criteria path.
    pub qualification_criteria: Option<PathBuf>,
}

/// Read one JSON document.
///
/// # Errors
///
/// Returns a message when the file cannot be read or parsed.
pub fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

/// Write stable pretty JSON with a trailing newline.
///
/// # Errors
///
/// Returns a message when the parent or file cannot be written.
pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("{}: cannot encode JSON: {error}", path.display()))?;
    fs::write(path, format!("{text}\n"))
        .map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Render a value as compact sorted JSON.
#[must_use]
pub fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// SHA-256 of a file, read in fixed-size chunks.
///
/// # Errors
///
/// Returns a message when the file cannot be read.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// List files under `root` with relative POSIX paths and digests.
///
/// # Errors
///
/// Returns a message when a file cannot be read.
pub fn file_inventory(root: &Path) -> Result<Vec<Value>, String> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort_by_key(|path| relative(root, path));
    let mut entries = Vec::new();
    for path in files {
        let mut entry = Map::new();
        entry.insert(
            "path".to_owned(),
            json!(relative(root, &path).replace('\\', "/")),
        );
        entry.insert("sha256".to_owned(), json!(sha256_file(&path)?));
        entries.push(Value::Object(entry));
    }
    Ok(entries)
}

/// Resolve a manifest-relative path, then a repository-relative path.
///
/// # Errors
///
/// Returns a message listing the candidates when no file exists.
pub fn resolve_path(
    manifest_path: &Path,
    value: &str,
    repo_root: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let path = PathBuf::from(expand(value));
    let candidates = if path.is_absolute() {
        vec![path]
    } else {
        vec![
            manifest_path.parent().unwrap_or(Path::new(".")).join(&path),
            repo_root.join(&path),
        ]
    };
    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate
                .canonicalize()
                .unwrap_or_else(|_| candidate.clone()));
        }
    }
    let tried = candidates
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("{label} not found: {value:?} (tried {tried})"))
}

/// Resolve the shell binary without running it.
///
/// # Errors
///
/// Returns a message when the path is empty or not executable.
pub fn resolve_shell(raw: &str, repo_root: &Path) -> Result<PathBuf, String> {
    if raw.is_empty() {
        return Err("shell binary path is empty".to_owned());
    }
    let candidate = PathBuf::from(expand(raw));
    let candidates = if candidate.is_absolute() {
        vec![candidate]
    } else if raw.contains('/') || raw.contains('\\') {
        vec![repo_root.join(candidate)]
    } else {
        let mut found = Vec::new();
        if let Ok(path) = std::env::var("PATH") {
            for directory in std::env::split_paths(&path) {
                let path = directory.join(&candidate);
                if executable(&path) {
                    found.push(path);
                }
            }
        }
        if found.is_empty() {
            vec![repo_root.join(candidate)]
        } else {
            found
        }
    };
    for path in candidates {
        if executable(&path) {
            return Ok(path.canonicalize().unwrap_or(path));
        }
    }
    Err(format!(
        "shell binary not found or not executable: {raw} (build it with `cargo build`, or pass --qr <path>)"
    ))
}

/// Resolve the run root, defaulting to the canonical store layout.
///
/// # Errors
///
/// Returns a message when `run_id` is invalid.
pub fn output_root(
    manifest: &Value,
    store_root: &Path,
    repo_root: &Path,
) -> Result<PathBuf, String> {
    let run_id = manifest
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "run_id is missing".to_owned())?;
    let date = run_date(run_id)?;
    if let Some(configured) = manifest.get("output_root").and_then(Value::as_str) {
        let path = PathBuf::from(expand(configured));
        let path = if path.is_absolute() {
            path
        } else {
            repo_root.join(path)
        };
        return Ok(path.canonicalize().unwrap_or(path));
    }
    Ok(store_root.join(date).join(run_id))
}

/// Extract the date segment from a run id.
///
/// # Errors
///
/// Returns a message when the run id does not match the schema.
pub fn run_date(run_id: &str) -> Result<String, String> {
    let parts: Vec<&str> = run_id.split('-').collect();
    if parts.len() != 5 || !matches!(parts[0], "MODEL" | "STRATEGY" | "PORTFOLIO") {
        return Err(format!("run_id is not valid: {run_id:?}"));
    }
    Ok(format!("{}-{}-{}", parts[1], parts[2], parts[3]))
}

/// Refuse an existing non-empty run root.
///
/// # Errors
///
/// Returns a message when the root is a file or holds entries.
pub fn ensure_empty_root(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    if !root.is_dir() {
        return Err(format!(
            "output root exists and is not a directory: {}",
            root.display()
        ));
    }
    let empty = fs::read_dir(root)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(true);
    if !empty {
        return Err(format!("output root is not empty: {}", root.display()));
    }
    Ok(())
}

/// Copy the run config and its sibling JSON files for provenance.
///
/// # Errors
///
/// Returns a message when a source or destination cannot be read or written.
pub fn prepare_config(
    manifest_path: &Path,
    manifest: &Value,
    config_dir: &Path,
    repo_root: &Path,
) -> Result<ConfigPaths, String> {
    fs::create_dir_all(config_dir)
        .map_err(|error| format!("cannot create {}: {error}", config_dir.display()))?;
    let run_config = resolve_path(
        manifest_path,
        manifest
            .get("run_config")
            .and_then(Value::as_str)
            .unwrap_or(""),
        repo_root,
        "path",
    )?;
    let mut files = Vec::new();
    for source in sibling_configs(&run_config) {
        let name = copy_config(&source, config_dir, "run-")?;
        files.push(json!({
            "source": repo_relative(repo_root, &source),
            "dest": name,
        }));
    }
    let strategy = manifest.get("strategy_config").and_then(Value::as_str);
    let strategy_config = strategy
        .map(|value| resolve_path(manifest_path, value, repo_root, "path"))
        .transpose()?;
    let strategy_name = if let Some(source) = &strategy_config {
        Some(copy_config(source, config_dir, "strategy-")?)
    } else {
        None
    };
    if let Some(source) = &strategy_config {
        files.push(json!({
            "source": repo_relative(repo_root, source),
            "dest": strategy_name,
        }));
    }
    let criteria = manifest
        .get("qualification_criteria")
        .and_then(Value::as_str);
    let qualification_criteria = criteria
        .map(|value| resolve_path(manifest_path, value, repo_root, "path"))
        .transpose()?;
    let criteria_name = if let Some(source) = &qualification_criteria {
        Some(copy_config(source, config_dir, "criteria-")?)
    } else {
        None
    };
    if let Some(source) = &qualification_criteria {
        files.push(json!({
            "source": repo_relative(repo_root, source),
            "dest": criteria_name,
        }));
    }
    let sources = json!({
        "schema": CONFIG_SCHEMA,
        "run_config": repo_relative(repo_root, &run_config),
        "strategy_config": strategy_config.as_ref().map(|path| repo_relative(repo_root, path)),
        "qualification_criteria": qualification_criteria.as_ref().map(|path| repo_relative(repo_root, path)),
        "files": files,
    });
    write_json(&config_dir.join("sources.json"), &sources)?;
    Ok(ConfigPaths {
        run_config,
        strategy_config,
        qualification_criteria,
    })
}

/// Text for the run's initial review document.
#[must_use]
pub fn review_text(manifest: &Value) -> String {
    let run_id = text(manifest, "run_id");
    let frozen = frozen(manifest);
    format!(
        "# Review: {run_id}\n\n- Tier: {}\n- Changed: {}\n- Frozen: {frozen}\n- Partition: {}\n- Hypothesis: {}\n- Green criteria: `{}`\n- Reviewer: Result Analyst\n- Date: <YYYY-MM-DD> (UTC)\n\n## Evidence\n\n- `results.json` records the command, exit code, captured streams, and artifact digests.\n- `artifacts/evidence.json` records the published numbers extracted from the report; run `qr-governance improvement evidence <run-dir>`.\n- Baseline: {}\n- Rerun check:\n- Data coverage and gaps:\n- Open audit findings on the changed path:\n\n## Results\n\n| Metric | Run | Baseline | Difference |\n|---|---|---|---|\n| Return | | | |\n| Max drawdown | | | |\n| Trades | | | |\n| Profit factor | | | |\n| Win rate | | | |\n| Exposure | | | |\n| Cost drag | | | |\n\n## Observations\n\n## Limitations\n\n## Verdict\n\nVerdict: pending\n",
        text(manifest, "tier"),
        text(manifest, "changed"),
        text(manifest, "partition"),
        text(manifest, "hypothesis"),
        compact_json(manifest.get("green_criteria").unwrap_or(&Value::Null)),
        baseline_label(manifest),
    )
}

/// Text for the run's initial decision document.
#[must_use]
pub fn decision_text(manifest: &Value) -> String {
    let run_id = text(manifest, "run_id");
    format!(
        "# Decision: {run_id}\n\n- Tier: {}\n- Changed: {}\n- Frozen: {}\n- Partition: {}\n- Baseline: {}\n- Holdout access: {}\n- Hypothesis: {}\n- Decided by: <role>\n- Date: <YYYY-MM-DD> (UTC)\n\n## Decision\n\nVerdict: pending\n\n## Rationale\n\n## Next hypothesis\n",
        text(manifest, "tier"),
        text(manifest, "changed"),
        frozen(manifest),
        text(manifest, "partition"),
        baseline_label(manifest),
        text(manifest, "holdout_access"),
        text(manifest, "hypothesis"),
    )
}

/// Write UTF-8 text to a file.
///
/// # Errors
///
/// Returns a message when the file cannot be written.
pub fn write_text(path: &Path, text: &str) -> Result<(), String> {
    fs::write(path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

/// Whether `path` resolves inside `parent`.
#[must_use]
pub fn is_within(path: &Path, parent: &Path) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let parent = parent
        .canonicalize()
        .unwrap_or_else(|_| parent.to_path_buf());
    path.starts_with(parent)
}

/// Repository-relative POSIX path, or the absolute path outside.
#[must_use]
pub fn repo_relative(root: &Path, path: &Path) -> String {
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    resolved
        .strip_prefix(root)
        .unwrap_or(&resolved)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The run config directory's JSON files, sorted by name.
#[must_use]
pub fn sibling_configs(config_path: &Path) -> Vec<PathBuf> {
    let mut siblings = Vec::new();
    if let Some(parent) = config_path.parent()
        && let Ok(entries) = fs::read_dir(parent)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
            {
                siblings.push(path);
            }
        }
    }
    let resolved = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());
    if !siblings
        .iter()
        .any(|path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf()) == resolved)
    {
        siblings.push(config_path.to_path_buf());
    }
    siblings.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    });
    siblings
}

/// Run a command in a directory and capture UTF-8 output.
///
/// # Errors
///
/// Returns a message when the process cannot be spawned.
pub fn run_process(command: &[String], cwd: &Path) -> Result<std::process::Output, String> {
    let Some(program) = command.first() else {
        return Err("cannot run an empty command".to_owned());
    };
    Command::new(program)
        .args(&command[1..])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("cannot run {program}: {error}"))
}

fn copy_config(source: &Path, destination: &Path, prefix: &str) -> Result<String, String> {
    let mut name = source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if destination.join(&name).exists() {
        name = format!("{prefix}{name}");
    }
    fs::copy(source, destination.join(&name)).map_err(|error| {
        format!(
            "cannot copy {} to {}: {error}",
            source.display(),
            destination.join(&name).display()
        )
    })?;
    Ok(name)
}

fn baseline_label(manifest: &Value) -> String {
    match manifest.get("baseline") {
        None | Some(Value::Null) => "(none)".to_owned(),
        Some(Value::String(value)) => value.clone(),
        Some(value) => compact_json(value),
    }
}

fn frozen(manifest: &Value) -> String {
    let values: Vec<&str> = manifest
        .get("frozen")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    if values.is_empty() {
        "(none)".to_owned()
    } else {
        values.join(", ")
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn collect_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, found);
        } else {
            found.push(path);
        }
    }
}

fn expand(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    value.to_owned()
}

fn executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}
