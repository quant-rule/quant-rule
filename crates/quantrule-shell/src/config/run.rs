//! `run.json`: the composition root.
//!
//! The run file names the policy blueprint, the engine files, and the output
//! root. This module reads them in contract order, resolves every path against
//! the file that names it, builds the engine configs, and runs the cross-config
//! checks that need more than one file. No candle is read and no output is
//! written here: a config error leaves the filesystem untouched.

use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::portfolio::{DrawdownAction, PortfolioConfig};
use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use quantrule_model::readings::PivotScale;
use serde_json::Value;

use super::analytics::{self, AnalyticsConfig};
use super::data::{self, DataConfig};
use super::error::ConfigError;
use super::execution::{self, ExecutionRunConfig};
use super::model;
use super::policy::{self, PolicyConfig};
use super::portfolio;
use super::read::{self, ROOT};
use super::strategy::{self, StrategyRunConfig};

/// One validated run, ready for the pipeline.
#[derive(Debug, Clone)]
pub(crate) struct RunConfig {
    /// The run name, recorded in provenance.
    pub(crate) name: String,
    /// The resolved output root.
    pub(crate) output_root: PathBuf,
    /// A stable fingerprint of the eight config files.
    pub(crate) config_hash: String,
    /// The eight resolved config paths.
    pub(crate) paths: ConfigPaths,
    /// The human-owned policy envelope.
    pub(crate) policy: PolicyConfig,
    /// The data engine's source and ladder.
    pub(crate) data: DataConfig,
    /// The model engine's configuration.
    pub(crate) model: MarketStateConfig,
    /// The explainable strategy blueprint and thresholds.
    pub(crate) strategy: StrategyRunConfig,
    /// The execution engine's configuration and fill policy.
    pub(crate) execution: ExecutionRunConfig,
    /// The portfolio engine's configuration.
    pub(crate) portfolio: PortfolioConfig,
    /// The analytical report's configuration.
    pub(crate) analytics: AnalyticsConfig,
}

/// The eight resolved config files.
#[derive(Debug, Clone)]
pub(crate) struct ConfigPaths {
    /// The run file itself.
    pub(crate) run: PathBuf,
    /// The policy blueprint.
    pub(crate) policy: PathBuf,
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
            policy: base.join(&run.policy),
            data: base.join(&run.data),
            model: base.join(&run.model),
            strategy: base.join(&run.strategy),
            execution: base.join(&run.execution),
            portfolio: base.join(&run.portfolio),
            analytics: base.join(&run.analytics),
        };

        let policy = policy::parse(
            &load_document(&paths.policy, display(&run.policy))?,
            display(&run.policy),
        )?;
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
            strategy.stop_atr_multiple,
            strategy.target_atr_multiple(),
        )?;
        let portfolio = portfolio::parse(
            &load_document(&paths.portfolio, display(&run.portfolio))?,
            display(&run.portfolio),
        )?;
        let analytics = analytics::parse(
            &load_document(&paths.analytics, display(&run.analytics))?,
            display(&run.analytics),
        )?;

        let config_hash = fingerprint(&paths)?;
        let config = Self {
            name: run.name,
            output_root: base.join(&run.output_root),
            config_hash,
            paths,
            policy,
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

    pub(super) fn validate_cross(&self) -> Result<(), ConfigError> {
        let strategy_secs = self.strategy.engine.timeframe_secs();
        let Some(strategy_timeframe) = self
            .data
            .available_timeframes
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

        for timeframe in Timeframe::ALL {
            if self.model.is_overridden(timeframe)
                && !self.data.available_timeframes.contains(&timeframe)
            {
                return Err(ConfigError::inconsistent(
                    file_name(&self.paths.model),
                    &format!("$.timeframes.{}", timeframe.code()),
                    "a timeframe from the available data ladder",
                    timeframe.code(),
                ));
            }
        }

        for timeframe in &self.analytics.timeframes {
            if !self.data.available_timeframes.contains(timeframe) {
                return Err(ConfigError::inconsistent(
                    file_name(&self.paths.analytics),
                    "$.timeframes",
                    "timeframes from the available data ladder",
                    timeframe.code(),
                ));
            }
        }

        for timeframe in &self.policy.universe.timeframes {
            if !self.data.available_timeframes.contains(timeframe) {
                return Err(ConfigError::inconsistent(
                    file_name(&self.paths.policy),
                    "$.universe.timeframes",
                    "timeframes from the available data ladder",
                    timeframe.code(),
                ));
            }
        }

        if !self
            .policy
            .universe
            .timeframes
            .iter()
            .any(|timeframe| timeframe.seconds() == strategy_timeframe.seconds())
        {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.strategy),
                "$.timeframe",
                "a timeframe from the policy universe",
                strategy_timeframe.code(),
            ));
        }

        if !self
            .policy
            .universe
            .tickets
            .iter()
            .any(|ticket| ticket == &self.data.instrument)
        {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.data),
                "$.instrument",
                "an instrument from the policy universe",
                &self.data.instrument,
            ));
        }

        if self.policy.account.initial_cash != self.portfolio.initial_cash {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.initial_cash",
                &self.policy.account.initial_cash.to_string(),
                &self.portfolio.initial_cash.to_string(),
            ));
        }

        let Some(drawdown_limit) = self.portfolio.max_drawdown.limit() else {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.max_drawdown.limit",
                "a limit at or below the policy maximum",
                "null",
            ));
        };
        if drawdown_limit > self.policy.objective.max_drawdown {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.max_drawdown.limit",
                &self.policy.objective.max_drawdown.to_string(),
                &drawdown_limit.to_string(),
            ));
        }
        if self.portfolio.max_drawdown.action() != DrawdownAction::Halt {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.max_drawdown.action",
                "halt, because the policy declares a drawdown limit",
                self.portfolio.max_drawdown.action().code(),
            ));
        }
        if self.portfolio.leverage > self.policy.risk.max_leverage {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.leverage",
                &self.policy.risk.max_leverage.to_string(),
                &self.portfolio.leverage.to_string(),
            ));
        }
        if self.portfolio.max_open_positions > self.policy.risk.max_open_positions {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.portfolio),
                "$.max_open_positions",
                &self.policy.risk.max_open_positions.to_string(),
                &self.portfolio.max_open_positions.to_string(),
            ));
        }
        match self.portfolio.sizing {
            quantrule_backtest::portfolio::SizingMode::FixedNotional { notional, .. } => {
                let maximum =
                    self.policy.account.initial_cash * self.policy.risk.max_position_fraction;
                if notional > maximum {
                    return Err(ConfigError::inconsistent(
                        file_name(&self.paths.portfolio),
                        "$.sizing.notional",
                        &format!("at most {maximum}"),
                        &notional.to_string(),
                    ));
                }
            }
            quantrule_backtest::portfolio::SizingMode::EquityFraction { fraction, .. } => {
                if fraction > self.policy.risk.max_position_fraction {
                    return Err(ConfigError::inconsistent(
                        file_name(&self.paths.portfolio),
                        "$.sizing.fraction",
                        &self.policy.risk.max_position_fraction.to_string(),
                        &fraction.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

/// The parsed `run.json`, before its referenced files are read.
#[derive(Debug)]
pub(super) struct RunFile {
    pub(super) name: String,
    policy: String,
    data: String,
    model: String,
    strategy: String,
    execution: String,
    portfolio: String,
    analytics: String,
    pub(super) output_root: String,
}

pub(super) fn parse(value: &Value, file: &str) -> Result<RunFile, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    read::check_fields(
        file,
        ROOT,
        object,
        &[
            "schema",
            "name",
            "policy",
            "data",
            "model",
            "strategy",
            "execution",
            "portfolio",
            "analytics",
            "output_root",
        ],
    )?;
    read::schema(file, object, "quantrule.run/v2")?;

    Ok(RunFile {
        name: read::non_empty_string(file, "$.name", read::field(file, ROOT, object, "name")?)?
            .to_owned(),
        policy: read::non_empty_string(
            file,
            "$.policy",
            read::field(file, ROOT, object, "policy")?,
        )?
        .to_owned(),
        data: read::non_empty_string(file, "$.data", read::field(file, ROOT, object, "data")?)?
            .to_owned(),
        model: read::non_empty_string(file, "$.model", read::field(file, ROOT, object, "model")?)?
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

/// A stable FNV-1a fingerprint of the config file bytes.
///
/// The hash is an identity aid, not a security primitive. It is stable for the
/// same bytes and the same binary, and it is recorded in provenance so a later
/// run can be shown to have used the same policy and engine files.
fn fingerprint(paths: &ConfigPaths) -> Result<String, ConfigError> {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for path in [
        &paths.run,
        &paths.policy,
        &paths.data,
        &paths.model,
        &paths.strategy,
        &paths.execution,
        &paths.portfolio,
        &paths.analytics,
    ] {
        let bytes = std::fs::read(path).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => ConfigError::missing_file(file_name(path)),
            _ => ConfigError::unreadable_file(file_name(path)),
        })?;
        for byte in bytes {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
        // A separator prevents the concatenation of two files from colliding
        // with a different split.
        hash ^= 0xff;
        hash = hash.wrapping_mul(PRIME);
    }
    Ok(format!("{hash:016x}"))
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
