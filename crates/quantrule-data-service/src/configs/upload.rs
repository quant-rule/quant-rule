//! A run-config bundle to a catalog record.
//!
//! The bundle is written to a private staging directory, validated by the same
//! loader the repository mount uses, and removed before the function returns.
//! A key that is absolute or contains `..` is refused before any write.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::ConfigOrigin;

use crate::configs::ConfigRecord;
use crate::configs::scan::record;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// Why a bundle could not be catalogued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadError(pub String);

impl std::fmt::Display for UploadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for UploadError {}

/// Validate a bundle, stage it, and return the catalog record.
///
/// # Errors
///
/// Returns [`UploadError`] when the name or paths are invalid, a file cannot be
/// written, or the run config and its references cannot be read.
pub fn build(
    name: &str,
    files: &BTreeMap<String, String>,
    now: &str,
    staging_root: &Path,
) -> Result<ConfigRecord, UploadError> {
    validate(name, files)?;
    let root = staging_root.join(format!(
        "quantrule-config-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _staging = Staging(root.clone());
    std::fs::create_dir_all(&root)
        .map_err(|error| UploadError(format!("create the staging directory: {error}")))?;
    for (relative, body) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| UploadError(format!("create {relative}: {error}")))?;
        }
        std::fs::write(&path, body)
            .map_err(|error| UploadError(format!("write {relative}: {error}")))?;
    }
    let mut record = record(&root, &root.join("run.json"), now, ConfigOrigin::Upload)
        .map_err(|error| UploadError(error.to_string()))?;
    record.name = name.trim().to_owned();
    record.files = files.clone();
    Ok(record)
}

fn validate(name: &str, files: &BTreeMap<String, String>) -> Result<(), UploadError> {
    if name.trim().is_empty() {
        return Err(UploadError("a config needs a name".to_owned()));
    }
    if !files.contains_key("run.json") {
        return Err(UploadError("a bundle must contain run.json".to_owned()));
    }
    if files.len() > 32 {
        return Err(UploadError("a bundle holds at most 32 files".to_owned()));
    }
    let total: usize = files.values().map(String::len).sum();
    if total > 4 * 1024 * 1024 {
        return Err(UploadError("a config bundle exceeds 4 MiB".to_owned()));
    }
    for relative in files.keys() {
        let path = Path::new(relative);
        if relative.is_empty()
            || relative.contains('\\')
            || relative.ends_with('/')
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(UploadError(format!(
                "{relative}: a bundle path must be a relative file path without '..'"
            )));
        }
    }
    Ok(())
}

struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn staging() -> PathBuf {
        // The workspace target is ignored by the cleanup checker; a crate-local
        // target looks like unclassified local data.
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/config-upload-tests")
    }

    fn valid() -> BTreeMap<String, String> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid");
        [
            "run.json",
            "data.json",
            "model.json",
            "execution.json",
            "portfolio.json",
        ]
        .into_iter()
        .map(|name| {
            (
                name.to_owned(),
                std::fs::read_to_string(root.join(name)).expect("fixture"),
            )
        })
        .collect()
    }

    #[test]
    fn a_valid_bundle_is_catalogued() {
        let record =
            build("uploaded", &valid(), "2026-11-13T00:00:00Z", &staging()).expect("build");
        assert!(record.valid);
        assert_eq!(record.origin, ConfigOrigin::Upload);
        assert_eq!(record.name, "uploaded");
        assert!(record.config_hash.is_some());
        assert!(record.config_id.starts_with("cfg_"));
        assert_eq!(record.files.len(), 5);
        assert!(record.files.contains_key("run.json"));
    }

    #[test]
    fn the_same_bundle_has_the_same_id() {
        let first = build("one", &valid(), "2026-11-13T00:00:00Z", &staging()).expect("first");
        let second = build("two", &valid(), "2026-11-13T00:00:00Z", &staging()).expect("second");
        assert_eq!(first.config_id, second.config_id);
    }

    #[test]
    fn a_missing_reference_is_catalogued_as_invalid() {
        let mut files = valid();
        files.remove("data.json");
        let record = build("broken", &files, "2026-11-13T00:00:00Z", &staging()).expect("build");
        assert!(!record.valid);
        assert!(record.config_hash.is_none());
        assert!(
            record
                .message
                .as_deref()
                .is_some_and(|message| message.contains("data.json"))
        );
    }

    #[test]
    fn a_parent_path_is_refused() {
        let mut files = valid();
        files.insert("../escape.json".to_owned(), "{}".to_owned());
        let error = build("bad", &files, "2026-11-13T00:00:00Z", &staging()).expect_err("refused");
        assert!(error.0.contains(".."), "{error}");
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let mut files = valid();
        files.insert("/etc/passwd".to_owned(), "{}".to_owned());
        assert!(build("bad", &files, "2026-11-13T00:00:00Z", &staging()).is_err());
    }

    #[test]
    fn a_bundle_without_run_json_is_refused() {
        let mut files = valid();
        files.remove("run.json");
        assert!(build("bad", &files, "2026-11-13T00:00:00Z", &staging()).is_err());
    }
}
