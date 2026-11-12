//! The shared application service.
//!
//! One [`AppService`] binds a local workspace to the use cases every adapter
//! needs: list datasets, load and validate a run, compute the model over the
//! configured ladder, and export a deterministic run summary. The command shell
//! and the desktop UI call these methods instead of reaching into the engine
//! crates or repeating path resolution.
//!
//! The service owns file access. The engines remain pure: the service passes
//! validated configuration and loaded candles into them and returns their
//! values unchanged.

use std::fs;
use std::path::{Path, PathBuf};

use quantrule_data::Timeframe;
use quantrule_model::run::ModelRun;

use crate::config::{RUN_SCHEMA, RunConfig};
use crate::dataset::{Dataset, DatasetSummary};
use crate::error::AppError;
use crate::export;
use crate::memory::{MemoryError, NewConfig, NewDataset, NewRun, NewWorkspace, ResearchMemory};
use crate::pipeline;
use crate::workspace::Workspace;

/// The application service for one local workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppService {
    workspace: Workspace,
}

impl AppService {
    /// Opens an existing local workspace.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Workspace`] when the root is missing or is not a
    /// directory.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, AppError> {
        Ok(Self {
            workspace: Workspace::open(root)?,
        })
    }

    /// Discovers the nearest enclosing workspace from a file or directory.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Workspace`] when no enclosing workspace exists.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, AppError> {
        Ok(Self {
            workspace: Workspace::discover(start)?,
        })
    }

    /// Opens the workspace that owns a run configuration.
    ///
    /// A configuration inside a workspace uses that workspace. A standalone
    /// configuration uses its own directory, so validation never requires the
    /// caller to invent a workspace root.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Workspace`] when neither an enclosing workspace nor
    /// the configuration's directory exists.
    pub fn open_for_config(config: impl AsRef<Path>) -> Result<Self, AppError> {
        let config = config.as_ref();
        if let Ok(service) = Self::discover(config) {
            return Ok(service);
        }
        let parent = config
            .parent()
            .filter(|parent| parent.is_dir())
            .unwrap_or_else(|| Path::new("."));
        Self::open(parent)
    }

    /// The workspace this service operates on.
    #[must_use]
    pub fn workspace(&self) -> &Workspace {
        &self.workspace
    }

    /// Lists the local candle datasets in sorted order.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] when a source directory cannot be read.
    pub fn datasets(&self) -> Result<Vec<Dataset>, AppError> {
        self.workspace.datasets()
    }

    /// Summarises one local candle dataset.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Dataset`] or [`AppError::Source`] when the path is
    /// not a valid M1 CSV source.
    pub fn dataset_summary(&self, path: impl AsRef<Path>) -> Result<DatasetSummary, AppError> {
        Dataset::open(path)?.summary()
    }

    /// Loads and validates one run configuration.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] when any file is missing, malformed, or
    /// inconsistent with another file.
    pub fn load_run(&self, path: impl AsRef<Path>) -> Result<RunConfig, AppError> {
        Ok(RunConfig::load(path.as_ref())?)
    }

    /// Loads one run and returns its validated summary.
    ///
    /// This is the same work as `run check`: it reads every configuration file,
    /// resolves every path, and applies every cross-config rule without reading
    /// a candle.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] when the run is invalid.
    pub fn check_run(&self, path: impl AsRef<Path>) -> Result<RunSummary, AppError> {
        let config = self.load_run(path)?;
        Ok(RunSummary::from_config(&config))
    }

    /// Loads one run, computes its model over the configured ladder, and
    /// returns the model plus its summary.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] for an invalid run, and
    /// [`AppError::Source`] or [`AppError::Pipeline`] when the candles cannot
    /// be loaded or the selected period is empty.
    pub fn run_model(&self, path: impl AsRef<Path>) -> Result<ModelRunOutcome, AppError> {
        let config = self.load_run(path)?;
        let model = pipeline::run_model(&config)?;
        let summary = ModelRunSummary::from_run(&model);
        Ok(ModelRunOutcome {
            run: RunSummary::from_config(&config),
            model,
            summary,
        })
    }

    /// Records one validated run in the local research memory.
    ///
    /// The service writes the workspace, dataset, configuration, and run in one
    /// call. Metrics, claims, and notes are added separately through the
    /// memory. The caller supplies every timestamp and the stable run key. The
    /// three parent writes are idempotent; a duplicate run key leaves them in
    /// place for the next successful record.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] for an invalid run, [`AppError::Dataset`]
    /// when the source is missing, [`AppError::Source`] when it is not a valid
    /// M1 source, and [`AppError::Memory`] when the memory rejects a record.
    pub fn record_run(
        &self,
        memory: &ResearchMemory,
        config: impl AsRef<Path>,
        run_key: &str,
        status: &str,
        started_at: i64,
        finished_at: Option<i64>,
    ) -> Result<RecordedRun, AppError> {
        self.record_run_with_model(
            memory,
            RecordRunInput {
                config: config.as_ref(),
                run_key,
                status,
                started_at,
                finished_at,
                model: None,
            },
        )
    }

    /// Records one validated run together with its computed model counts.
    ///
    /// The model summary is additive in the run's `summary_json`; the run and
    /// its parents are still written in one call. A caller that has not
    /// computed the model uses [`Self::record_run`].
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::record_run`].
    pub fn record_run_with_model(
        &self,
        memory: &ResearchMemory,
        input: RecordRunInput<'_>,
    ) -> Result<RecordedRun, AppError> {
        let run = self.load_run(input.config)?;
        let summary = RunSummary::from_config(&run);
        let dataset = Dataset::open(&summary.data_source)?.summary()?;
        let root = self.workspace.root().to_string_lossy();
        let workspace_name = self.workspace.name();
        let workspace_id = memory.upsert_workspace(NewWorkspace {
            root: &root,
            name: &workspace_name,
            created_at: input.started_at,
        })?;
        let dataset_path = dataset.path.to_string_lossy();
        let candles = i64::try_from(dataset.candles).map_err(|_| {
            AppError::Memory(MemoryError::InvalidRecord(String::from(
                "dataset candle count does not fit i64",
            )))
        })?;
        let dataset_id = memory.upsert_dataset(NewDataset {
            workspace_id,
            name: &dataset.name,
            path: &dataset_path,
            data_version: &summary.data_version,
            instrument: &summary.instrument,
            candles,
            first_open_time: dataset.first_open_time,
            last_open_time: dataset.last_open_time,
            created_at: input.started_at,
        })?;
        let config_path =
            fs::canonicalize(&run.paths.run).unwrap_or_else(|_| run.paths.run.clone());
        let config_path = config_path.to_string_lossy();
        let config_id = memory.upsert_config(NewConfig {
            workspace_id,
            name: &summary.name,
            path: &config_path,
            config_hash: &summary.config_hash,
            schema_version: RUN_SCHEMA,
            created_at: input.started_at,
        })?;
        let mut summary_value = export::run_summary_json(&summary);
        if let Some(model) = input.model {
            summary_value["model"] = export::model_summary_json(model);
        }
        let summary_json = serde_json::to_string(&summary_value).map_err(MemoryError::Json)?;
        let run_id = memory.insert_run(NewRun {
            workspace_id,
            config_id: Some(config_id),
            run_key: input.run_key,
            data_version: &summary.data_version,
            config_hash: &summary.config_hash,
            status: input.status,
            started_at: input.started_at,
            finished_at: input.finished_at,
            summary_json: &summary_json,
        })?;
        Ok(RecordedRun {
            workspace_id,
            dataset_id,
            config_id,
            run_id,
        })
    }

    /// Exports the validated summary of one run as deterministic JSON.
    ///
    /// The file is written only after the run validates. An existing file is
    /// replaced; the caller chooses a fresh path when it needs an immutable
    /// artifact.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Config`] for an invalid run, and [`AppError::Io`]
    /// when the destination cannot be written.
    pub fn export_run_summary(
        &self,
        config: impl AsRef<Path>,
        out: impl AsRef<Path>,
    ) -> Result<RunSummary, AppError> {
        let summary = self.check_run(config)?;
        export::write_run_summary_file(&summary, out.as_ref())?;
        Ok(summary)
    }
}

/// The validated, adapter-facing view of one run.
///
/// Timeframes are published as their stable wire codes, in ladder order. The
/// summary carries configuration identity and composition only; it carries no
/// reading or decision.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSummary {
    /// The run name.
    pub name: String,
    /// The resolved output root.
    pub output_root: PathBuf,
    /// The stable fingerprint of the run's configuration files.
    pub config_hash: String,
    /// The optional model identity.
    pub model_id: Option<String>,
    /// The optional execution identity.
    pub execution_id: Option<String>,
    /// The optional portfolio identity.
    pub portfolio_id: Option<String>,
    /// The configured instrument.
    pub instrument: String,
    /// The caller-supplied data version.
    pub data_version: String,
    /// The resolved M1 source path.
    pub data_source: PathBuf,
    /// The available data ladder, fastest first.
    pub data_timeframes: Vec<String>,
    /// The model ladder, fastest first; the data ladder when not overridden.
    pub model_timeframes: Vec<String>,
    /// The execution fill timeframe.
    pub execution_fill_timeframe: String,
    /// The portfolio's initial cash.
    pub initial_cash: f64,
    /// The portfolio's leverage.
    pub leverage: f64,
    /// The portfolio's position limit.
    pub max_open_positions: u32,
}

impl RunSummary {
    /// Builds the summary from a validated run.
    #[must_use]
    pub fn from_config(config: &RunConfig) -> Self {
        let data_timeframes = codes(&config.data.available_timeframes);
        let model_timeframes = config
            .model_ladder
            .as_deref()
            .map(codes)
            .unwrap_or_else(|| data_timeframes.clone());
        let data_source = fs::canonicalize(&config.data.source.path)
            .unwrap_or_else(|_| config.data.source.path.clone());
        Self {
            name: config.name.clone(),
            output_root: config.output_root.clone(),
            config_hash: config.config_hash.clone(),
            model_id: config.model_id.clone(),
            execution_id: config.execution_id.clone(),
            portfolio_id: config.portfolio_id.clone(),
            instrument: config.data.instrument.clone(),
            data_version: config.data.data_version.clone(),
            data_source,
            data_timeframes,
            model_timeframes,
            execution_fill_timeframe: config.execution.fill_timeframe.code().to_owned(),
            initial_cash: config.portfolio.initial_cash,
            leverage: config.portfolio.leverage,
            max_open_positions: config.portfolio.max_open_positions,
        }
    }
}

/// One run to record, optionally with its computed model counts.
#[derive(Debug, Clone, Copy)]
pub struct RecordRunInput<'a> {
    /// The run configuration path.
    pub config: &'a Path,
    /// The caller-supplied stable run key.
    pub run_key: &'a str,
    /// The run status code.
    pub status: &'a str,
    /// The caller-supplied UTC Unix start second.
    pub started_at: i64,
    /// The caller-supplied UTC Unix finish second.
    pub finished_at: Option<i64>,
    /// The computed model counts, when the caller has run the model.
    pub model: Option<&'a ModelRunSummary>,
}

/// The stable IDs written by [`AppService::record_run`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordedRun {
    /// The workspace row.
    pub workspace_id: i64,
    /// The dataset row.
    pub dataset_id: i64,
    /// The configuration row.
    pub config_id: i64,
    /// The run row.
    pub run_id: i64,
}

/// A completed model run and the adapter-facing views of it.
#[derive(Debug)]
pub struct ModelRunOutcome {
    /// The validated run summary.
    pub run: RunSummary,
    /// The computed model, per timeframe.
    pub model: ModelRun,
    /// Counts derived from the computed model.
    pub summary: ModelRunSummary,
}

/// Deterministic counts for one model run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelRunSummary {
    /// One entry per computed timeframe, fastest first.
    pub timeframes: Vec<TimeframeModelSummary>,
    /// The total published events across the ladder.
    pub events: usize,
}

impl ModelRunSummary {
    /// Counts the computed model without altering it.
    #[must_use]
    pub fn from_run(model: &ModelRun) -> Self {
        let timeframes: Vec<TimeframeModelSummary> = model
            .timeframes()
            .iter()
            .map(|timeframe| TimeframeModelSummary {
                timeframe_secs: timeframe.timeframe_secs(),
                bars: timeframe.len(),
                events: timeframe.events().len(),
                levels: timeframe.levels().len(),
            })
            .collect();
        let events = timeframes.iter().map(|timeframe| timeframe.events).sum();
        Self { timeframes, events }
    }
}

/// Counts for one computed timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeframeModelSummary {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// The number of closed bars.
    pub bars: usize,
    /// The number of published events.
    pub events: usize,
    /// The number of confirmed levels.
    pub levels: usize,
}

fn codes(timeframes: &[Timeframe]) -> Vec<String> {
    timeframes
        .iter()
        .map(|timeframe| timeframe.code().to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::AppService;
    use crate::config::RUN_SCHEMA;

    fn fixture_run() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-shell/tests/fixtures/config-run/run.json")
    }

    fn workspace() -> AppService {
        AppService::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .expect("the repository is a workspace")
    }

    #[test]
    fn the_example_run_checks_without_reading_a_candle() {
        let summary = workspace()
            .check_run(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../config/examples/btcusd-2024/run.json"
            ))
            .expect("the example run is valid");
        assert_eq!(summary.name, "btcusd-2024");
        assert_eq!(summary.instrument, "BTCUSD");
        assert_eq!(summary.model_timeframes, ["1m", "5m", "15m", "1h"]);
        assert_eq!(summary.execution_fill_timeframe, "5m");
    }

    #[test]
    fn a_model_run_is_deterministic() {
        let service = workspace();
        let first = service.run_model(fixture_run()).expect("the fixture runs");
        let second = service.run_model(fixture_run()).expect("the fixture runs");
        assert_eq!(first.run, second.run);
        assert_eq!(first.summary, second.summary);
        assert_eq!(first.model, second.model);
        assert!(first.summary.events > 0);
        assert!(
            first
                .summary
                .timeframes
                .windows(2)
                .all(|pair| pair[0].timeframe_secs < pair[1].timeframe_secs)
        );
    }

    #[test]
    fn recording_a_run_writes_every_record() {
        let memory = crate::memory::ResearchMemory::open_in_memory().expect("the memory opens");
        let recorded = workspace()
            .record_run(
                &memory,
                fixture_run(),
                "run-1",
                "complete",
                1_763_000_000,
                Some(1_763_000_060),
            )
            .expect("the run is recorded");
        let expected_name = workspace().workspace().name();
        let workspace = memory
            .workspace(recorded.workspace_id)
            .expect("the workspace is read")
            .expect("the workspace exists");
        assert_eq!(workspace.name, expected_name);
        assert_ne!(workspace.name, "fixture");
        let datasets = memory
            .datasets(recorded.workspace_id)
            .expect("the datasets are read");
        assert_eq!(datasets.len(), 1);
        assert_eq!(datasets[0].data_version, "test:v1");
        assert!(datasets[0].candles > 0);
        let configs = memory
            .configs(recorded.workspace_id)
            .expect("the configs are read");
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].schema_version, RUN_SCHEMA);
        let run = memory
            .run(recorded.run_id)
            .expect("the run is read")
            .expect("the run exists");
        assert_eq!(run.run_key, "run-1");
        assert_eq!(run.status, "complete");
        assert!(run.summary_json.contains("\"fixture\""));
    }

    #[test]
    fn recording_a_model_run_stores_the_model_summary() {
        let memory = crate::memory::ResearchMemory::open_in_memory().expect("the memory opens");
        let service = workspace();
        let outcome = service.run_model(fixture_run()).expect("the model runs");
        let config = fixture_run();
        let recorded = service
            .record_run_with_model(
                &memory,
                super::RecordRunInput {
                    config: &config,
                    run_key: "model-run-1",
                    status: "complete",
                    started_at: 1,
                    finished_at: Some(2),
                    model: Some(&outcome.summary),
                },
            )
            .expect("the model run is recorded");
        let run = memory
            .run(recorded.run_id)
            .expect("the run is read")
            .expect("the run exists");
        let summary: serde_json::Value =
            serde_json::from_str(&run.summary_json).expect("the summary is JSON");
        assert_eq!(summary["model"]["events"], outcome.summary.events);
        assert_eq!(
            summary["model"]["timeframes"]
                .as_array()
                .expect("the model timeframes are an array")
                .len(),
            outcome.summary.timeframes.len()
        );
    }

    #[test]
    fn the_workspace_name_is_the_root_name() {
        let memory = crate::memory::ResearchMemory::open_in_memory().expect("the memory opens");
        let service = workspace();
        let expected_name = service.workspace().name();
        let recorded = service
            .record_run(&memory, fixture_run(), "name-1", "complete", 1, None)
            .expect("the run is recorded");
        let workspace = memory
            .workspace(recorded.workspace_id)
            .expect("the workspace is read")
            .expect("the workspace exists");
        assert_eq!(workspace.name, expected_name);
        assert_ne!(workspace.name, "fixture");
    }

    #[test]
    fn the_same_workspace_root_is_recorded_once() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let first = AppService::open(&root).expect("the absolute root opens");
        let second = AppService::open(root.join(".")).expect("the dotted root opens");
        let memory = crate::memory::ResearchMemory::open_in_memory().expect("the memory opens");
        first
            .record_run(&memory, fixture_run(), "canonical-1", "complete", 1, None)
            .expect("the first run is recorded");
        second
            .record_run(&memory, fixture_run(), "canonical-2", "complete", 2, None)
            .expect("the second run is recorded");
        let workspaces = memory.workspaces().expect("the workspaces are read");
        assert_eq!(workspaces.len(), 1);
        let workspace = workspaces[0].id;
        assert_eq!(memory.datasets(workspace).expect("datasets").len(), 1);
        assert_eq!(memory.configs(workspace).expect("configs").len(), 1);
        assert_eq!(memory.runs(workspace).expect("runs").len(), 2);
    }

    #[test]
    fn recording_the_same_run_key_twice_is_rejected() {
        let memory = crate::memory::ResearchMemory::open_in_memory().expect("the memory opens");
        let service = workspace();
        service
            .record_run(&memory, fixture_run(), "run-1", "complete", 1, None)
            .expect("the first run is recorded");
        assert!(
            service
                .record_run(&memory, fixture_run(), "run-1", "complete", 2, None)
                .is_err()
        );
    }

    #[test]
    fn an_export_is_byte_deterministic() {
        let out =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/app-service-tests/export");
        std::fs::create_dir_all(&out).expect("the export directory is created");
        let first = out.join("first.json");
        let second = out.join("second.json");
        let service = workspace();
        service
            .export_run_summary(fixture_run(), &first)
            .expect("the first export is written");
        service
            .export_run_summary(fixture_run(), &second)
            .expect("the second export is written");
        assert_eq!(
            std::fs::read(&first).expect("the first export is readable"),
            std::fs::read(&second).expect("the second export is readable")
        );
    }
}
