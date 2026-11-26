//! Materialize a run config and its catalogued dataset.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

use crate::backtest::worker::WorkerError;
use quantrule_service_api::ConfigBundle;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// A private directory holding one run's config, dataset, and output.
#[derive(Debug)]
pub struct Workspace {
    root: PathBuf,
    run: PathBuf,
    strategy: PathBuf,
    model: PathBuf,
    out: PathBuf,
}

impl Workspace {
    /// Write the config bundle and dataset, and rewrite `data.json` to point at
    /// the catalogued candles.
    ///
    /// The config's declared `data_version` is preserved. The worker verifies
    /// the dataset bytes against the run row's digest and never substitutes that
    /// digest for the config's own label, so the materialized config is the same
    /// input the coupled `backtest run` writer consumes.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError`] for an unsafe bundle path, a missing reference,
    /// a malformed file, or an I/O failure.
    pub fn create(config: &ConfigBundle, dataset: &[u8]) -> Result<Self, WorkerError> {
        let run_text = config
            .files
            .get("run.json")
            .ok_or_else(|| WorkerError("the bundle has no run.json".to_owned()))?;
        let run_value: Value = serde_json::from_str(run_text)
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
            model: root.join("model"),
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

    /// The model bundle directory the worker materializes.
    #[must_use]
    pub fn model(&self) -> PathBuf {
        self.model.clone()
    }

    /// Write one materialized model bundle artifact.
    ///
    /// The caller verifies the artifact against its manifest hash before this
    /// write. A path outside the model directory is refused.
    ///
    /// # Errors
    ///
    /// Returns [`WorkerError`] for an unsafe artifact name or an I/O failure.
    pub fn write_model_artifact(&self, name: &str, bytes: &[u8]) -> Result<(), WorkerError> {
        safe_key(name)?;
        let path = self.model.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| WorkerError(format!("create {name}: {error}")))?;
        }
        std::fs::write(&path, bytes).map_err(|error| WorkerError(format!("write {name}: {error}")))
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

pub(crate) fn source_csv(dataset: &[u8]) -> Result<String, WorkerError> {
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

pub(crate) fn safe_key(relative: &str) -> Result<(), WorkerError> {
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
    use quantrule_api::Digest;
    use quantrule_app::config::RunConfig;
    use std::collections::BTreeMap;
    use std::path::Path as StdPath;

    /// Canonicalize a date-and-price CSV for a test, the way ingest does.
    fn canonical_csv(csv: &str) -> Vec<u8> {
        use quantrule_data::{Candle, RawCandle};

        let mut out = Vec::new();
        let mut index = 0_u64;
        for line in csv.lines() {
            let fields: Vec<&str> = line.split(',').map(str::trim).collect();
            if fields.len() < 5 || !fields[0].starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            let raw = RawCandle {
                date: fields[0],
                open: fields[1],
                high: fields[2],
                low: fields[3],
                close: fields[4],
                volume: fields.get(5).copied(),
            };
            let candle = Candle::parse(index, raw).expect("test candle");
            out.extend_from_slice(candle.canonical_line().as_bytes());
            out.push(b'\n');
            index += 1;
        }
        out
    }

    fn fixture() -> (ConfigBundle, Vec<u8>, Digest) {
        let root = StdPath::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid-v4");
        let run_path = root.join("run.json");
        let run_text = std::fs::read_to_string(&run_path).expect("run");
        let config = RunConfig::load(&run_path).expect("config");
        let value: Value = serde_json::from_str(&run_text).expect("run json");
        let mut files = BTreeMap::from([("run.json".to_owned(), run_text)]);
        let pairs = [
            ("data", Some(&config.paths.data)),
            ("model", Some(&config.paths.model)),
            ("execution", config.paths.execution.as_ref()),
            ("portfolio", config.paths.portfolio.as_ref()),
        ];
        for (key, path) in pairs {
            let Some(path) = path else {
                continue;
            };
            let relative = value
                .get(key)
                .and_then(Value::as_str)
                .expect("a relative path");
            files.insert(
                relative.to_owned(),
                std::fs::read_to_string(path).expect("config file"),
            );
        }
        if let Some(strategy) = &config.paths.strategy {
            let relative = value
                .get("strategy")
                .and_then(Value::as_str)
                .expect("a relative strategy path");
            files.insert(
                relative.to_owned(),
                std::fs::read_to_string(strategy).expect("strategy"),
            );
        }
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
        let dataset = canonical_csv(&csv);
        let version = Digest::sha256(&dataset);
        (
            ConfigBundle {
                config_id: "cfg_test".to_owned(),
                references: Vec::new(),
                files,
                valid: true,
                message: None,
                config_hash: Some(version.clone()),
            },
            dataset,
            version,
        )
    }

    #[test]
    fn the_workspace_rewrites_the_data_source_and_keeps_the_declared_version() {
        let (config, dataset, _version) = fixture();
        let declared: Value = serde_json::from_str(
            config
                .files
                .get("data.json")
                .expect("the fixture data config"),
        )
        .expect("data config JSON");
        let workspace = Workspace::create(&config, &dataset).expect("workspace");
        let run = RunConfig::load(&workspace.run_config()).expect("materialized config");
        assert!(run.strategy.is_some());
        assert_eq!(
            run.data.data_version,
            declared["data_version"].as_str().expect("a string")
        );
        assert!(workspace.strategy().is_file());
        assert!(workspace.out().to_string_lossy().contains("out"));
    }
}
