//! `run.json`: the composition root.
//!
//! The run file names the six engine files and the output root. This module
//! reads them in contract order, resolves every path against the file that
//! names it, builds the engine configs, and runs the cross-config checks that
//! need more than one file. No candle is read and no output is written here: a
//! config error leaves the filesystem untouched.

use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::portfolio::PortfolioConfig;
use quantrule_backtest::strategy::StrategyConfig;
use quantrule_model::readings::PivotScale;
use quantrule_model::state::MarketStateConfig;
use serde_json::Value;

use super::analytics::{self, AnalyticsConfig};
use super::data::{self, DataConfig};
use super::error::ConfigError;
use super::execution::{self, ExecutionRunConfig};
use super::model;
use super::portfolio;
use super::read::{self, ROOT};
use super::strategy;

/// One validated run, ready for the pipeline.
#[derive(Debug, Clone)]
pub(crate) struct RunConfig {
    /// The run name, recorded in provenance.
    pub(crate) name: String,
    /// The resolved output root.
    pub(crate) output_root: PathBuf,
    /// The seven resolved config paths.
    pub(crate) paths: ConfigPaths,
    /// The data engine's source and ladder.
    pub(crate) data: DataConfig,
    /// The model engine's configuration.
    pub(crate) model: MarketStateConfig,
    /// The strategy's configuration.
    pub(crate) strategy: StrategyConfig,
    /// The execution engine's configuration and fill policy.
    pub(crate) execution: ExecutionRunConfig,
    /// The portfolio engine's configuration.
    pub(crate) portfolio: PortfolioConfig,
    /// The analytical report's configuration.
    pub(crate) analytics: AnalyticsConfig,
}

/// The seven resolved config files.
#[derive(Debug, Clone)]
pub(crate) struct ConfigPaths {
    /// The run file itself.
    pub(crate) run: PathBuf,
    /// The data file.
    pub(crate) data: PathBuf,
    /// The model file.
    pub(crate) model: PathBuf,
    /// The strategy file.
    pub(crate) strategy: PathBuf,
    /// The execution file.
    pub(crate) execution: PathBuf,
    /// The portfolio file.
    pub(crate) portfolio: PathBuf,
    /// The analytics file.
    pub(crate) analytics: PathBuf,
}

impl RunConfig {
    /// Loads and validates a run from its `run.json`.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the file and JSON path when a file is
    /// missing or unreadable, is not JSON, disagrees with its schema, or fails
    /// a cross-config check.
    pub(crate) fn load(path: &Path) -> Result<Self, ConfigError> {
        let run_name = file_name(path);
        let run = parse(&load_document(path, run_name)?, run_name)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));

        let paths = ConfigPaths {
            run: path.to_path_buf(),
            data: base.join(&run.data),
            model: base.join(&run.model),
            strategy: base.join(&run.strategy),
            execution: base.join(&run.execution),
            portfolio: base.join(&run.portfolio),
            analytics: base.join(&run.analytics),
        };

        let data = data::parse(
            &load_document(&paths.data, display(&run.data))?,
            display(&run.data),
            paths.data.parent().unwrap_or(base),
        )?;
        let model = model::parse(
            &load_document(&paths.model, display(&run.model))?,
            display(&run.model),
        )?;
        let strategy = strategy::parse(
            &load_document(&paths.strategy, display(&run.strategy))?,
            display(&run.strategy),
        )?;
        let execution = execution::parse(
            &load_document(&paths.execution, display(&run.execution))?,
            display(&run.execution),
        )?;
        let portfolio = portfolio::parse(
            &load_document(&paths.portfolio, display(&run.portfolio))?,
            display(&run.portfolio),
        )?;
        let analytics = analytics::parse(
            &load_document(&paths.analytics, display(&run.analytics))?,
            display(&run.analytics),
        )?;

        let config = Self {
            name: run.name,
            output_root: base.join(&run.output_root),
            paths,
            data,
            model,
            strategy,
            execution,
            portfolio,
            analytics,
        };
        config.validate_cross()?;
        Ok(config)
    }

    fn validate_cross(&self) -> Result<(), ConfigError> {
        let strategy_secs = self.strategy.timeframe_secs();
        let Some(strategy_timeframe) = self
            .data
            .ladder
            .iter()
            .copied()
            .find(|timeframe| u64::from(timeframe.seconds()) == strategy_secs)
        else {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.strategy),
                "$.timeframe",
                "a member of the data ladder",
                &strategy_secs.to_string(),
            ));
        };

        if self.execution.fill_timeframe != strategy_timeframe {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.execution),
                "$.fill_timeframe",
                strategy_timeframe.code(),
                self.execution.fill_timeframe.code(),
            ));
        }

        if self.analytics.structure_scale != self.model.structure_scale {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.analytics),
                "$.structure_scale",
                scale_code(self.model.structure_scale),
                scale_code(self.analytics.structure_scale),
            ));
        }

        for timeframe in &self.analytics.timeframes {
            if !self.data.ladder.contains(timeframe) {
                return Err(ConfigError::inconsistent(
                    file_name(&self.paths.analytics),
                    "$.timeframes",
                    "timeframes from the data ladder",
                    timeframe.code(),
                ));
            }
        }

        Ok(())
    }
}

/// The parsed `run.json`, before its referenced files are read.
struct RunFile {
    name: String,
    data: String,
    model: String,
    strategy: String,
    execution: String,
    portfolio: String,
    analytics: String,
    output_root: String,
}

fn parse(value: &Value, file: &str) -> Result<RunFile, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "name",
            "data",
            "model",
            "strategy",
            "execution",
            "portfolio",
            "analytics",
            "output_root",
        ],
    )?;
    read::schema(file, object, "quantrule.run/v1")?;

    Ok(RunFile {
        name: read::non_empty_string(
            file,
            "$.name",
            read::field(file, ROOT, object, "name")?,
        )?
        .to_owned(),
        data: read::non_empty_string(file, "$.data", read::field(file, ROOT, object, "data")?)?
            .to_owned(),
        model: read::non_empty_string(
            file,
            "$.model",
            read::field(file, ROOT, object, "model")?,
        )?
        .to_owned(),
        strategy: read::non_empty_string(
            file,
            "$.strategy",
            read::field(file, ROOT, object, "strategy")?,
        )?
        .to_owned(),
        execution: read::non_empty_string(
            file,
            "$.execution",
            read::field(file, ROOT, object, "execution")?,
        )?
        .to_owned(),
        portfolio: read::non_empty_string(
            file,
            "$.portfolio",
            read::field(file, ROOT, object, "portfolio")?,
        )?
        .to_owned(),
        analytics: read::non_empty_string(
            file,
            "$.analytics",
            read::field(file, ROOT, object, "analytics")?,
        )?
        .to_owned(),
        output_root: read::non_empty_string(
            file,
            "$.output_root",
            read::field(file, ROOT, object, "output_root")?,
        )?
        .to_owned(),
    })
}

fn load_document(path: &Path, file: &str) -> Result<Value, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ConfigError::missing_file(file),
        _ => ConfigError::unreadable_file(file),
    })?;
    serde_json::from_str(&text).map_err(|error| ConfigError::invalid_json(file, &error.to_string()))
}

/// The file's own name, for an error message that never prints its path.
fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("run.json")
}

/// The referenced name's final component, for the same reason.
fn display(name: &str) -> &str {
    Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(name)
}

fn scale_code(scale: PivotScale) -> &'static str {
    match scale {
        PivotScale::Major => "major",
        PivotScale::Minor => "minor",
    }
}
