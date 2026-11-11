//! `run.json`: the composition root.
//!
//! The run file names the data, model, execution, and portfolio files, and the
//! output root. This module reads them in contract
//! order, resolves every path against the file that names it, builds the
//! engine configs, and runs the cross-config checks that need more than one
//! file. No candle is read and no output is written here: a config error
//! leaves the filesystem untouched.
//!
//! The current run has no strategy or analytics file. S3.x passes the strategy
//! file and the qualification criteria as explicit command arguments; the run
//! schema is `quantrule.run/v3`.

use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::portfolio::PortfolioConfig;
use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use serde_json::Value;

use super::data::{self, DataConfig};
use super::error::ConfigError;
use super::execution::{self, ExecutionRunConfig};
use super::model;
use super::portfolio;
use super::read::{self, ROOT};

/// The current run schema.
pub const RUN_SCHEMA: &str = "quantrule.run/v3";

/// One validated run, ready for the pipeline.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// The run name, recorded in provenance.
    pub name: String,
    /// The resolved output root.
    pub output_root: PathBuf,
    /// A stable fingerprint of the five config files.
    pub config_hash: String,
    /// The five resolved config paths.
    pub paths: ConfigPaths,
    /// The optional model identity, such as `M1.0`.
    pub model_id: Option<String>,
    /// The optional execution identity, such as `E1.0`.
    pub execution_id: Option<String>,
    /// The optional portfolio identity, such as `P1.0`.
    pub portfolio_id: Option<String>,
    /// The data engine's source and ladder.
    pub data: DataConfig,
    /// The optional model ladder. `None` means the full data ladder.
    pub model_ladder: Option<Vec<Timeframe>>,
    /// The model engine's configuration.
    pub model: MarketStateConfig,
    /// The execution engine's configuration and fill policy.
    pub execution: ExecutionRunConfig,
    /// The portfolio engine's configuration.
    pub portfolio: PortfolioConfig,
}

/// The five resolved config files.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// The run file itself.
    pub run: PathBuf,
    /// The data file.
    pub data: PathBuf,
    /// The model file.
    pub model: PathBuf,
    /// The execution file.
    pub execution: PathBuf,
    /// The portfolio file.
    pub portfolio: PathBuf,
}

impl RunConfig {
    /// Loads and validates a run from its `run.json`.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the file and JSON path when a file is
    /// missing or unreadable, is not JSON, disagrees with its schema, or fails
    /// a cross-config check.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let run_name = file_name(path);
        let run = parse(&load_document(path, run_name)?, run_name)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));

        let paths = ConfigPaths {
            run: path.to_path_buf(),
            data: base.join(&run.data),
            model: base.join(&run.model),
            execution: base.join(&run.execution),
            portfolio: base.join(&run.portfolio),
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
        let execution = execution::parse(
            &load_document(&paths.execution, display(&run.execution))?,
            display(&run.execution),
        )?;
        let portfolio = portfolio::parse(
            &load_document(&paths.portfolio, display(&run.portfolio))?,
            display(&run.portfolio),
        )?;

        let config_hash = fingerprint(&paths)?;
        let config = Self {
            name: run.name,
            output_root: base.join(&run.output_root),
            config_hash,
            paths,
            model_id: run.model_id,
            execution_id: run.execution_id,
            portfolio_id: run.portfolio_id,
            data,
            model_ladder: run.model_timeframes,
            model,
            execution,
            portfolio,
        };
        config.validate_cross()?;
        config.validate_identities()?;
        Ok(config)
    }

    pub(super) fn validate_identities(&self) -> Result<(), ConfigError> {
        for (field, value, prefix) in [
            ("model_id", self.model_id.as_deref(), "M"),
            ("execution_id", self.execution_id.as_deref(), "E"),
            ("portfolio_id", self.portfolio_id.as_deref(), "P"),
        ] {
            if let Some(identity) = value
                && !has_identity_shape(identity, prefix)
            {
                return Err(ConfigError::range(
                    file_name(&self.paths.run),
                    &format!("$.{field}"),
                    &format!("<prefix><major>.<minor> with prefix {prefix}"),
                    identity,
                ));
            }
        }
        Ok(())
    }

    pub(super) fn validate_cross(&self) -> Result<(), ConfigError> {
        for timeframe in &self.data.available_timeframes {
            if u64::from(timeframe.seconds()) > self.model.max_timeframe_secs() {
                return Err(ConfigError::range(
                    file_name(&self.paths.data),
                    "$.available_timeframes",
                    "timeframes at or below the model ceiling",
                    timeframe.code(),
                ));
            }
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

        if let Some(ladder) = &self.model_ladder {
            for timeframe in ladder {
                if !self.data.available_timeframes.contains(timeframe) {
                    return Err(ConfigError::inconsistent(
                        file_name(&self.paths.run),
                        "$.model_timeframes",
                        "timeframes from the available data ladder",
                        timeframe.code(),
                    ));
                }
            }
            if !ladder.contains(&self.execution.fill_timeframe) {
                return Err(ConfigError::inconsistent(
                    file_name(&self.paths.run),
                    "$.model_timeframes",
                    "a ladder that contains the execution fill timeframe",
                    self.execution.fill_timeframe.code(),
                ));
            }
        }

        if !self
            .data
            .available_timeframes
            .contains(&self.execution.fill_timeframe)
        {
            return Err(ConfigError::inconsistent(
                file_name(&self.paths.execution),
                "$.fill_timeframe",
                "a timeframe from the available data ladder",
                self.execution.fill_timeframe.code(),
            ));
        }
        Ok(())
    }
}

/// The parsed `run.json`, before its referenced files are read.
#[derive(Debug)]
pub(super) struct RunFile {
    pub(super) name: String,
    model_id: Option<String>,
    execution_id: Option<String>,
    portfolio_id: Option<String>,
    data: String,
    model: String,
    execution: String,
    portfolio: String,
    pub(super) model_timeframes: Option<Vec<Timeframe>>,
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
            "model_id",
            "execution_id",
            "portfolio_id",
            "data",
            "model",
            "execution",
            "portfolio",
            "model_timeframes",
            "output_root",
        ],
    )?;
    read::schema(file, object, RUN_SCHEMA)?;

    Ok(RunFile {
        name: read::non_empty_string(file, "$.name", read::field(file, ROOT, object, "name")?)?
            .to_owned(),
        model_id: optional_identity(file, object, "model_id")?,
        execution_id: optional_identity(file, object, "execution_id")?,
        portfolio_id: optional_identity(file, object, "portfolio_id")?,
        data: read::non_empty_string(file, "$.data", read::field(file, ROOT, object, "data")?)?
            .to_owned(),
        model: read::non_empty_string(file, "$.model", read::field(file, ROOT, object, "model")?)?
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
        model_timeframes: object
            .get("model_timeframes")
            .map(|value| parse_model_timeframes(file, value))
            .transpose()?,
        output_root: read::non_empty_string(
            file,
            "$.output_root",
            read::field(file, ROOT, object, "output_root")?,
        )?
        .to_owned(),
    })
}

fn parse_model_timeframes(file: &str, value: &Value) -> Result<Vec<Timeframe>, ConfigError> {
    let values = read::array(file, "$.model_timeframes", value)?;
    if values.is_empty() {
        return Err(ConfigError::range(
            file,
            "$.model_timeframes",
            "a non-empty ascending timeframe list",
            "[]",
        ));
    }
    let mut timeframes: Vec<Timeframe> = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        let path = read::index("$.model_timeframes", index);
        let timeframe = data::timeframe(file, &path, item)?;
        if let Some(previous) = timeframes.last()
            && previous.seconds() >= timeframe.seconds()
        {
            return Err(ConfigError::range(
                file,
                "$.model_timeframes",
                "strictly ascending, without duplicates",
                &read::observed(value),
            ));
        }
        timeframes.push(timeframe);
    }
    Ok(timeframes)
}

fn optional_identity(
    file: &str,
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<String>, ConfigError> {
    match object.get(field) {
        Some(value) => Ok(Some(
            read::non_empty_string(file, &format!("$.{field}"), value)?.to_owned(),
        )),
        None => Ok(None),
    }
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
/// run can be shown to have used the same engine files.
fn fingerprint(paths: &ConfigPaths) -> Result<String, ConfigError> {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for path in [
        &paths.run,
        &paths.data,
        &paths.model,
        &paths.execution,
        &paths.portfolio,
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

/// Whether `identity` is `<prefix><major>.<minor>` with decimal components.
fn has_identity_shape(identity: &str, prefix: &str) -> bool {
    let Some(rest) = identity.strip_prefix(prefix) else {
        return false;
    };
    let Some((major, minor)) = rest.split_once('.') else {
        return false;
    };
    !major.is_empty()
        && !minor.is_empty()
        && major.bytes().all(|byte| byte.is_ascii_digit())
        && minor.bytes().all(|byte| byte.is_ascii_digit())
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
