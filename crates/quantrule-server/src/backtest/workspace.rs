//! Materialize a run config and its catalogued dataset.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::Digest;
use serde_json::{Value, json};

use crate::backtest::worker::WorkerError;
use crate::configs::ConfigRecord;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// A private directory holding one run's config, dataset, and output.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    run: PathBuf,
    strategy: PathBuf,
    out: PathBuf,
}

impl Workspace {
    /// Write the config bundle and dataset, and rewrite `data.json` to point at
    /// the catalogued candles.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError`] for an unsafe bundle path, a missing reference,
    /// a malformed file, or an I/O failure.
    pub fn create(
        config: &ConfigRecord,
        dataset: &[u8],
        data_version: &Digest,
    ) -> Result<Self, WorkerError> {
        let run_value: Value = serde_json::from_str(&config.body)
            .map_err(|error| WorkerError(format!("run config is not JSON: {error}")))?;
        let data_key = run_value
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| WorkerError("run config names no data file".to_owned()))?;
        let strategy_key = run_value
            .get("strategy")
            .and_then(Value::as_str)
            .ok_or_else(|| WorkerError("run config names no strategy".to_owned()))?;

        for key in config.files.keys() {
            safe_key(key)?;
        }
        let root = std::env::temp_dir().join(format!(
            "quantrule-worker-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let workspace = Self {
            run: root.join("run.json"),
            strategy: root.join(strategy_key),
            out: root.join("out"),
            root,
        };
        std::fs::create_dir_all(&workspace.root)
            .map_err(|error| WorkerError(format!("create the workspace: {error}")))?;
        for (relative, body) in &config.files {
            let path = workspace.root.join(relative);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|error| WorkerError(format!("create {relative}: {error}")))?;
            }
            std::fs::write(&path, body)
                .map_err(|error| WorkerError(format!("write {relative}: {error}")))?;
        }

        let data_text = config
            .files
            .get(data_key)
            .ok_or_else(|| WorkerError(format!("the bundle has no {data_key}")))?;
        let mut data: Value = serde_json::from_str(data_text)
            .map_err(|error| WorkerError(format!("{data_key} is not JSON: {error}")))?;
        let source = data
            .get_mut("source")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| WorkerError(format!("{data_key} has no source object")))?;
        source.insert("path".to_owned(), json!("candles.csv"));
        data["data_version"] = json!(data_version.to_string());
        let data_dir = Path::new(data_key)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let candles = workspace.root.join(&data_dir).join("candles.csv");
        std::fs::write(&candles, source_csv(dataset)?)
            .map_err(|error| WorkerError(format!("write candles.csv: {error}")))?;
        std::fs::write(
            workspace.root.join(data_key),
            serde_json::to_string(&data)
                .map_err(|error| WorkerError(format!("encode {data_key}: {error}")))?,
        )
        .map_err(|error| WorkerError(format!("write {data_key}: {error}")))?;

        if !workspace.strategy.is_file() {
            return Err(WorkerError("the bundle has no strategy file".to_owned()));
        }
        if !workspace.run.is_file() {
            return Err(WorkerError("the bundle has no run.json".to_owned()));
        }
        Ok(workspace)
    }

    /// The materialized run config.
    #[must_use]
    pub fn run_config(&self) -> PathBuf {
        self.run.clone()
    }

    /// The materialized strategy.
    #[must_use]
    pub fn strategy(&self) -> PathBuf {
        self.strategy.clone()
    }

    /// The output root.
    #[must_use]
    pub fn out(&self) -> PathBuf {
        self.out.clone()
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn source_csv(dataset: &[u8]) -> Result<String, WorkerError> {
    let text = std::str::from_utf8(dataset)
        .map_err(|_| WorkerError("the dataset is not UTF-8".to_owned()))?;
    let mut out = String::from("Date,Open,High,Low,Close,Volume\n");
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 6 {
            return Err(WorkerError(format!(
                "canonical line {} does not have six fields",
                index + 1
            )));
        }
        let timestamp: i64 = fields[0]
            .parse()
            .map_err(|_| WorkerError(format!("canonical line {} has no timestamp", index + 1)))?;
        let date = quantrule_data::format_utc_timestamp(timestamp)
            .replace('T', " ")
            .replace('Z', "+00:00");
        out.push_str(&format!(
            "{date},{},{},{},{},{}\n",
            fields[1], fields[2], fields[3], fields[4], fields[5]
        ));
    }
    Ok(out)
}

fn safe_key(relative: &str) -> Result<(), WorkerError> {
    let path = Path::new(relative);
    if relative.is_empty()
        || relative.contains('\\')
        || relative.ends_with('/')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(WorkerError(format!(
            "{relative}: a config path must stay inside the workspace"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::scan;
    use crate::data::ingest;
    use quantrule_api::Digest;
    use quantrule_app::config::RunConfig;
    use std::path::Path as StdPath;

    fn fixture() -> (ConfigRecord, Vec<u8>, Digest) {
        let root = StdPath::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config");
        let records = scan(&root, "2026-11-13T00:00:00Z").expect("scan");
        let config = records
            .into_iter()
            .find(|record| record.name == "valid-v4/run.json")
            .expect("v4 config");
        let mut csv = String::from("date,open,high,low,close,volume\n");
        for offset in 0..250 {
            let hour = offset / 60;
            let minute = offset % 60;
            let open = 100.0 + offset as f64 * 0.01;
            csv.push_str(&format!(
                "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
                open + 0.5,
                open - 0.5,
                open + 0.25
            ));
        }
        let dataset = ingest("model.csv", csv.as_bytes()).expect("ingest");
        (config, dataset.content, dataset.data_version)
    }

    #[test]
    fn the_workspace_rewrites_the_data_source_and_loads() {
        let (config, dataset, version) = fixture();
        let workspace = Workspace::create(&config, &dataset, &version).expect("workspace");
        let run = RunConfig::load(&workspace.run_config()).expect("materialized config");
        assert!(run.strategy.is_some());
        assert_eq!(run.data.data_version, version.to_string());
        assert!(workspace.strategy().is_file());
        assert!(workspace.out().to_string_lossy().contains("out"));
    }
}
