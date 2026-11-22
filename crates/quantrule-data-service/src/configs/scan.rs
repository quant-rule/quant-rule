//! Scan the read-only repository mount for run configs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use quantrule_api::{ConfigOrigin, Digest};
use quantrule_app::config::RunConfig;

use crate::configs::ConfigRecord;

/// Why the repository mount could not be scanned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError(pub String);

impl std::fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScanError {}

/// Read every `run.json` under the root and validate it.
///
/// The order is sorted by path, so the same mount always produces the same
/// records. A valid config is hashed over its five files; an invalid one is
/// hashed over its run file so it can still be listed and fixed.
///
/// # Errors
///
/// Returns [`ScanError`] when the root cannot be read or a file cannot be read.
pub fn scan(root: &Path, now: &str) -> Result<Vec<ConfigRecord>, ScanError> {
    let mut paths = find_runs(root)?;
    paths.sort();
    paths
        .iter()
        .map(|path| record(root, path, now, ConfigOrigin::Repository))
        .collect()
}

fn find_runs(root: &Path) -> Result<Vec<PathBuf>, ScanError> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| ScanError(format!("read {}: {error}", directory.display())))?;
        for entry in entries {
            let entry = entry.map_err(|error| ScanError(error.to_string()))?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|name| name == "run.json") {
                found.push(path);
            }
        }
    }
    Ok(found)
}

pub(crate) fn record(
    root: &Path,
    path: &Path,
    now: &str,
    origin: ConfigOrigin,
) -> Result<ConfigRecord, ScanError> {
    let body = std::fs::read_to_string(path).map_err(|error| ScanError(error.to_string()))?;
    let name = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    match RunConfig::load(path) {
        Ok(config) => {
            let digest = hash_files(&config)?;
            let files = bundle_files(&body, &config);
            Ok(ConfigRecord {
                config_id: id(&digest),
                name,
                origin,
                valid: true,
                message: None,
                config_hash: Some(digest),
                body,
                files,
                created_at: now.to_owned(),
            })
        }
        Err(error) => {
            let digest = Digest::sha256(body.as_bytes());
            Ok(ConfigRecord {
                config_id: id(&digest),
                name,
                origin,
                valid: false,
                message: Some(error.to_string()),
                config_hash: None,
                body: body.clone(),
                files: BTreeMap::from([("run.json".to_owned(), body)]),
                created_at: now.to_owned(),
            })
        }
    }
}

fn bundle_files(body: &str, config: &RunConfig) -> BTreeMap<String, String> {
    let value: serde_json::Value = serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
    let mut files = BTreeMap::from([("run.json".to_owned(), body.to_owned())]);
    let mut pairs: Vec<(&str, Option<&PathBuf>)> = vec![
        ("data", Some(&config.paths.data)),
        ("model", Some(&config.paths.model)),
        ("execution", config.paths.execution.as_ref()),
        ("portfolio", config.paths.portfolio.as_ref()),
    ];
    if let Some(strategy) = &config.paths.strategy {
        pairs.push(("strategy", Some(strategy)));
    }
    for (key, path) in pairs {
        let Some(path) = path else {
            continue;
        };
        let Some(relative) = value.get(key).and_then(|value| value.as_str()) else {
            continue;
        };
        if let Ok(content) = std::fs::read_to_string(path) {
            files.insert(relative.to_owned(), content);
        }
    }
    files
}

/// The catalog's `config_hash` is the app loader's identity.
///
/// Recomputing the files here would be a second definition; a v5 identity also
/// covers the pack and signal hashes, which this scanner does not hold. The
/// loader has already read and validated every referenced file.
fn hash_files(config: &RunConfig) -> Result<Digest, ScanError> {
    Digest::parse(&config.config_hash).map_err(|error| ScanError(error.to_string()))
}

fn id(digest: &Digest) -> String {
    format!("cfg_{}", &digest.as_hex()[..12])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config")
    }

    #[test]
    fn the_scan_is_ordered_and_finds_both_runs() {
        let records = scan(&root(), "2026-11-13T00:00:00Z").expect("scan");
        let names: Vec<&str> = records.iter().map(|record| record.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["invalid/run.json", "valid/run.json", "valid-v4/run.json"]
        );
    }

    #[test]
    fn a_valid_run_carries_a_hash() {
        let records = scan(&root(), "2026-11-13T00:00:00Z").expect("scan");
        let valid = records
            .iter()
            .find(|record| record.name == "valid/run.json")
            .expect("valid");
        assert!(valid.valid);
        assert!(valid.message.is_none());
        assert!(valid.config_hash.is_some());
        assert!(valid.config_id.starts_with("cfg_"));
        assert_eq!(valid.files.len(), 5);
        assert!(valid.files.contains_key("run.json"));
        assert!(valid.files.contains_key("data.json"));

        // The app's `RunConfig` and the catalog must name the same identity.
        let config = RunConfig::load(&root().join("valid/run.json")).expect("the valid config");
        assert_eq!(
            config.config_hash,
            valid
                .config_hash
                .as_ref()
                .expect("the catalog hash")
                .to_string()
        );

        let v4 = records
            .iter()
            .find(|record| record.name == "valid-v4/run.json")
            .expect("v4");
        assert!(v4.valid);
        assert_eq!(v4.files.len(), 6);
        assert!(v4.files.contains_key("strategy.json"));
    }

    fn copy_tree(source: &Path, target: &Path) {
        std::fs::create_dir_all(target).expect("create the copy");
        for entry in std::fs::read_dir(source).expect("read the source") {
            let entry = entry.expect("the source entry");
            let path = entry.path();
            let destination = target.join(entry.file_name());
            if path.is_dir() {
                copy_tree(&path, &destination);
            } else {
                std::fs::copy(&path, &destination).expect("copy the file");
            }
        }
    }

    #[test]
    fn the_config_hash_covers_a_strategy_change() {
        let source = root().join("valid-v4");
        let target = std::env::temp_dir().join(format!(
            "quantrule-scan-strategy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("the clock")
                .as_nanos()
        ));
        copy_tree(&source, &target);
        let before = scan(&target, "2026-11-13T00:00:00Z").expect("scan")[0]
            .config_hash
            .clone()
            .expect("the original hash");
        let original = RunConfig::load(&target.join("run.json")).expect("the original config");
        let strategy = target.join("strategy.json");
        let mut text = std::fs::read_to_string(&strategy).expect("the strategy");
        text.push('\n');
        std::fs::write(&strategy, text).expect("write the changed strategy");
        let after = scan(&target, "2026-11-13T00:00:00Z").expect("scan")[0]
            .config_hash
            .clone()
            .expect("the changed hash");
        assert_ne!(before, after);

        // The model-input identity survives a strategy-only change.
        let changed = RunConfig::load(&target.join("run.json")).expect("the changed config");
        assert_eq!(original.model_input_hash, changed.model_input_hash);

        // A model-config change moves it.
        let model = target.join("model.json");
        let mut model_text = std::fs::read_to_string(&model).expect("the model config");
        model_text.push('\n');
        std::fs::write(&model, model_text).expect("write the changed model");
        let model_changed = RunConfig::load(&target.join("run.json")).expect("the model change");
        assert_ne!(original.model_input_hash, model_changed.model_input_hash);
        std::fs::remove_dir_all(&target).expect("remove the copy");
    }

    #[test]
    fn an_invalid_run_names_its_missing_file() {
        let records = scan(&root(), "2026-11-13T00:00:00Z").expect("scan");
        let invalid = records
            .iter()
            .find(|record| record.name == "invalid/run.json")
            .expect("invalid");
        assert!(!invalid.valid);
        assert!(invalid.config_hash.is_none());
        let message = invalid.message.as_deref().expect("message");
        assert!(message.contains("missing-data.json"), "{message}");
    }

    #[test]
    fn the_same_mount_gives_the_same_records() {
        assert_eq!(
            scan(&root(), "2026-11-13T00:00:00Z").expect("first"),
            scan(&root(), "2026-11-13T00:00:00Z").expect("second")
        );
    }
}
