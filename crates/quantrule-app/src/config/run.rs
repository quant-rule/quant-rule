//! `run.json`: the composition root.
//!
//! The run file names the data, model, execution, and portfolio files, and the
//! output root. This module reads them in contract
//! order, resolves every path against the file that names it, builds the
//! engine configs, and runs the cross-config checks that need more than one
//! file. No candle is read and no output is written here: a config error
//! leaves the filesystem untouched.
//!
//! A `quantrule.run/v4` file names the strategy and the four engine files; a
//! `quantrule.run/v3` file is model-only and must not carry a strategy. A
//! `quantrule.run/v5` file names packs and upstream artifacts by content hash
//! instead of naming strategy, execution, and portfolio files. Because the
//! persistent registry does not exist yet, a v5 run is loaded through
//! [`RunConfig::load_with`] with a supplied [`RunReferenceResolver`]. The legacy
//! [`RunConfig::load`] stays the v3/v4 path and rejects a v5 reference it was
//! not given bytes for.

use std::io;
use std::path::{Component, Path, PathBuf};

use quantrule_api::Digest;
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
use super::run_v5::{self, RunV5, V5RunFile};
use super::strategy::StrategyConfig;
use crate::pack::{ContentReference, ReferenceKind, RunReferenceResolver, RunReferences};

/// The v4 run schema: the strategy is part of the run.
pub const RUN_SCHEMA: &str = "quantrule.run/v4";
/// The model-only schema, accepted without a strategy.
pub const RUN_SCHEMA_V3: &str = "quantrule.run/v3";
/// The research-artifact schema, which references packs by content hash.
pub const RUN_SCHEMA_V5: &str = "quantrule.run/v5";

/// One validated run, ready for the pipeline.
#[derive(Debug, Clone)]
pub struct RunConfig {
    /// The run name, recorded in provenance.
    pub name: String,
    /// The resolved output root.
    pub output_root: PathBuf,
    /// The schema the run file declared.
    pub schema_version: String,
    /// A stable fingerprint of the config files and content references.
    pub config_hash: String,
    /// The model-input identity: data version label, model config bytes, and
    /// the effective model ladder. It is invariant under a strategy-only
    /// change, so a model bundle can be reused across strategy cells.
    pub model_input_hash: String,
    /// The resolved config paths. The engine files are absent in a v5 run.
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
    /// The human-owned strategy, when the run config names one.
    pub strategy: Option<StrategyConfig>,
    /// The execution engine's configuration and fill policy.
    pub execution: ExecutionRunConfig,
    /// The portfolio engine's configuration.
    pub portfolio: PortfolioConfig,
    /// The resolved v5 packs and references, absent for v3 and v4.
    pub v5: Option<RunV5>,
}

/// The resolved config files.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// The run file itself.
    pub run: PathBuf,
    /// The data file.
    pub data: PathBuf,
    /// The model file.
    pub model: PathBuf,
    /// The execution file, absent in a v5 run.
    pub execution: Option<PathBuf>,
    /// The portfolio file, absent in a v5 run.
    pub portfolio: Option<PathBuf>,
    /// The strategy file, absent in a v3 or v5 run.
    pub strategy: Option<PathBuf>,
}

impl RunConfig {
    /// Loads and validates a run from its `run.json`.
    ///
    /// This is the v3/v4 path. A v5 run needs [`RunConfig::load_with`] and a
    /// reference resolver, because the registry that would hold the pack bytes
    /// is a later work item.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the file and JSON path when a file is
    /// missing or unreadable, is not JSON, disagrees with its schema, fails a
    /// cross-config check, or is a v5 run whose references were not supplied.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        Self::load_with(path, &NoReferences)
    }

    /// Loads and validates a run, resolving v5 pack and artifact references.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the file and JSON path when a file is
    /// missing or unreadable, is not JSON, disagrees with its schema, fails a
    /// cross-config check, or when a reference is missing, mismatched, or has
    /// the wrong kind.
    pub fn load_with(
        path: &Path,
        references: &dyn RunReferenceResolver,
    ) -> Result<Self, ConfigError> {
        let run_name = file_name(path);
        let run = parse(&load_document(path, run_name)?, run_name)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));

        let paths = ConfigPaths {
            run: path.to_path_buf(),
            data: referenced_path(base, &run.data, run_name, "$.data")?,
            model: referenced_path(base, &run.model, run_name, "$.model")?,
            execution: run
                .execution
                .as_deref()
                .map(|name| referenced_path(base, name, run_name, "$.execution"))
                .transpose()?,
            portfolio: run
                .portfolio
                .as_deref()
                .map(|name| referenced_path(base, name, run_name, "$.portfolio"))
                .transpose()?,
            strategy: run
                .strategy
                .as_deref()
                .map(|name| referenced_path(base, name, run_name, "$.strategy"))
                .transpose()?,
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

        let (strategy, execution, portfolio, v5) = match &run.v5 {
            Some(v5_file) => {
                let ladder = run
                    .model_timeframes
                    .as_deref()
                    .unwrap_or(&data.available_timeframes);
                let resolved = run_v5::resolve(
                    run_name,
                    v5_file,
                    &data,
                    run.model_id.as_deref(),
                    ladder,
                    references,
                )?;
                (
                    Some(resolved.strategy),
                    resolved.execution,
                    resolved.portfolio,
                    Some(resolved.v5),
                )
            }
            None => {
                let execution_path = paths
                    .execution
                    .as_ref()
                    .ok_or_else(|| ConfigError::missing_field(run_name, "$.execution"))?;
                let portfolio_path = paths
                    .portfolio
                    .as_ref()
                    .ok_or_else(|| ConfigError::missing_field(run_name, "$.portfolio"))?;
                let execution_name = run.execution.as_deref().unwrap_or("execution.json");
                let portfolio_name = run.portfolio.as_deref().unwrap_or("portfolio.json");
                let execution = execution::parse(
                    &load_document(execution_path, display(execution_name))?,
                    display(execution_name),
                )?;
                let portfolio = portfolio::parse(
                    &load_document(portfolio_path, display(portfolio_name))?,
                    display(portfolio_name),
                )?;
                let strategy = match &paths.strategy {
                    Some(path) => Some(super::strategy::load(path)?.0),
                    None => None,
                };
                (strategy, execution, portfolio, None)
            }
        };

        let config_hash = match &v5 {
            Some(v5) => fingerprint_v5(&paths, &v5.references)?,
            None => fingerprint_legacy(&paths)?,
        };
        let model_input_hash = model_input_hash(&paths, &data, run.model_timeframes.as_deref())?;
        let config = Self {
            name: run.name,
            schema_version: run.schema,
            output_root: base.join(&run.output_root),
            config_hash,
            model_input_hash,
            paths,
            model_id: run.model_id,
            execution_id: run.execution_id,
            portfolio_id: run.portfolio_id,
            data,
            model_ladder: run.model_timeframes,
            model,
            strategy,
            execution,
            portfolio,
            v5,
        };
        config.validate_cross()?;
        config.validate_identities()?;
        Ok(config)
    }

    /// The v5 references, when this is a v5 run.
    #[must_use]
    pub fn references(&self) -> Option<&RunReferences> {
        self.v5.as_ref().map(|v5| &v5.references)
    }

    /// The v5 reference for one kind, when present.
    #[must_use]
    pub fn reference(&self, kind: ReferenceKind) -> Option<&ContentReference> {
        self.references()
            .and_then(|references| references.for_kind(kind))
    }

    /// The provenance value for the v5 references, when this is a v5 run.
    ///
    /// The backtest manifest and provenance record this value, and the
    /// canonical digest excludes both files, so a reference never changes the
    /// result digest.
    #[must_use]
    pub fn references_provenance(&self) -> Option<serde_json::Value> {
        self.references().map(RunReferences::to_value)
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
        let run_file = file_name(&self.paths.run);
        let execution_file = self
            .paths
            .execution
            .as_ref()
            .map_or(run_file, |path| file_name(path));
        let execution_path = if self.v5.is_some() {
            "$.execution_model"
        } else {
            "$.fill_timeframe"
        };

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

        if let Some(strategy) = &self.strategy {
            let strategy_file = self
                .paths
                .strategy
                .as_ref()
                .map_or(run_file, |path| file_name(path));
            let strategy_path = if self.v5.is_some() {
                "$.strategy_pack"
            } else {
                "$.execution_timeframe"
            };
            if strategy.execution_timeframe != self.execution.fill_timeframe {
                return Err(ConfigError::inconsistent(
                    strategy_file,
                    strategy_path,
                    self.execution.fill_timeframe.code(),
                    strategy.execution_timeframe.code(),
                ));
            }
            if !self
                .data
                .available_timeframes
                .contains(&strategy.execution_timeframe)
            {
                return Err(ConfigError::inconsistent(
                    strategy_file,
                    strategy_path,
                    "a timeframe from the available data ladder",
                    strategy.execution_timeframe.code(),
                ));
            }
            for timeframe in &strategy.context_timeframes {
                if !self.data.available_timeframes.contains(timeframe) {
                    return Err(ConfigError::inconsistent(
                        strategy_file,
                        strategy_path,
                        "timeframes from the available data ladder",
                        timeframe.code(),
                    ));
                }
            }
        }

        if let Some(ladder) = &self.model_ladder {
            for timeframe in ladder {
                if !self.data.available_timeframes.contains(timeframe) {
                    return Err(ConfigError::inconsistent(
                        run_file,
                        "$.model_timeframes",
                        "timeframes from the available data ladder",
                        timeframe.code(),
                    ));
                }
            }
            if !ladder.contains(&self.execution.fill_timeframe) {
                return Err(ConfigError::inconsistent(
                    run_file,
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
                execution_file,
                execution_path,
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
    pub(super) schema: String,
    model_id: Option<String>,
    execution_id: Option<String>,
    portfolio_id: Option<String>,
    data: String,
    model: String,
    execution: Option<String>,
    portfolio: Option<String>,
    pub(super) model_timeframes: Option<Vec<Timeframe>>,
    pub(super) output_root: String,
    pub(super) strategy: Option<String>,
    pub(super) v5: Option<V5RunFile>,
}

pub(super) fn parse(value: &Value, file: &str) -> Result<RunFile, ConfigError> {
    let object = read::object(file, ROOT, value)?;
    let schema =
        read::string(file, "$.schema", read::field(file, ROOT, object, "schema")?)?.to_owned();
    let allowed: &[&str] = match schema.as_str() {
        RUN_SCHEMA_V5 => &[
            "schema",
            "name",
            "model_id",
            "execution_id",
            "portfolio_id",
            "data",
            "model",
            "model_artifact",
            "strategy_pack",
            "signal_artifact",
            "portfolio_policy",
            "execution_model",
            "account",
            "execution",
            "portfolio",
            "strategy",
            "model_timeframes",
            "output_root",
        ],
        RUN_SCHEMA => &[
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
            "strategy",
        ],
        RUN_SCHEMA_V3 => &[
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
            "strategy",
        ],
        other => {
            return Err(ConfigError::schema(
                file,
                "$.schema",
                "quantrule.run/v5, quantrule.run/v4 or quantrule.run/v3",
                other,
            ));
        }
    };
    read::check_fields(file, ROOT, object, allowed)?;

    let v5 = if schema == RUN_SCHEMA_V5 {
        for (field, replacement) in [
            ("execution", "$.execution_model"),
            ("portfolio", "$.portfolio_policy"),
            ("strategy", "$.strategy_pack"),
            ("execution_id", "$.execution_model"),
            ("portfolio_id", "$.portfolio_policy"),
        ] {
            if object.contains_key(field) {
                return Err(ConfigError::unsupported(
                    file,
                    &format!("$.{field}"),
                    &format!("absent in quantrule.run/v5; use {replacement}"),
                    "present",
                ));
            }
        }
        Some(V5RunFile::parse(file, object)?)
    } else {
        None
    };

    let strategy = match schema.as_str() {
        RUN_SCHEMA => Some(
            read::non_empty_string(
                file,
                "$.strategy",
                read::field(file, ROOT, object, "strategy")?,
            )?
            .to_owned(),
        ),
        RUN_SCHEMA_V3 => {
            if object.contains_key("strategy") {
                return Err(ConfigError::schema(
                    file,
                    "$.strategy",
                    "absent in quantrule.run/v3",
                    "present",
                ));
            }
            None
        }
        RUN_SCHEMA_V5 => None,
        _ => None,
    };
    let execution = if v5.is_some() {
        None
    } else {
        Some(
            read::non_empty_string(
                file,
                "$.execution",
                read::field(file, ROOT, object, "execution")?,
            )?
            .to_owned(),
        )
    };
    let portfolio = if v5.is_some() {
        None
    } else {
        Some(
            read::non_empty_string(
                file,
                "$.portfolio",
                read::field(file, ROOT, object, "portfolio")?,
            )?
            .to_owned(),
        )
    };

    Ok(RunFile {
        name: read::non_empty_string(file, "$.name", read::field(file, ROOT, object, "name")?)?
            .to_owned(),
        schema,
        model_id: optional_identity(file, object, "model_id")?,
        execution_id: optional_identity(file, object, "execution_id")?,
        portfolio_id: optional_identity(file, object, "portfolio_id")?,
        data: read::non_empty_string(file, "$.data", read::field(file, ROOT, object, "data")?)?
            .to_owned(),
        model: read::non_empty_string(file, "$.model", read::field(file, ROOT, object, "model")?)?
            .to_owned(),
        execution,
        portfolio,
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
        strategy,
        v5,
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

/// The canonical SHA-256 identity of the legacy config file bytes.
///
/// The data service hashes the same files with the same order and a NUL
/// separator, so the catalog's `config_hash`, the run record, and the stage
/// provenance name one value. The hash is an identity, not a security
/// primitive; it is stable for the same bytes.
fn fingerprint_legacy(paths: &ConfigPaths) -> Result<String, ConfigError> {
    let mut files: Vec<&PathBuf> = vec![&paths.run, &paths.data, &paths.model];
    if let Some(execution) = &paths.execution {
        files.push(execution);
    }
    if let Some(portfolio) = &paths.portfolio {
        files.push(portfolio);
    }
    if let Some(strategy) = &paths.strategy {
        files.push(strategy);
    }
    Ok(Digest::sha256(&file_bytes(&files)?).to_string())
}

/// The v5 identity: the run, data, and model files, then every content hash.
///
/// The run file already carries the references, so changing any pack or signal
/// changes the run bytes. The explicit tags keep the identity readable and make
/// the contribution of each reference independent of the run file's key order.
fn fingerprint_v5(paths: &ConfigPaths, references: &RunReferences) -> Result<String, ConfigError> {
    let mut bytes = file_bytes(&[&paths.run, &paths.data, &paths.model])?;
    for (kind, reference) in references.each() {
        bytes.extend_from_slice(kind.code().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(reference.hash.to_string().as_bytes());
        bytes.push(0);
    }
    Ok(Digest::sha256(&bytes).to_string())
}

fn file_bytes(files: &[&PathBuf]) -> Result<Vec<u8>, ConfigError> {
    let mut bytes = Vec::new();
    for path in files {
        let content = std::fs::read(path).map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => ConfigError::missing_file(file_name(path)),
            _ => ConfigError::unreadable_file(file_name(path)),
        })?;
        bytes.extend_from_slice(&content);
        // A separator prevents the concatenation of two files from colliding
        // with a different split. The data service uses the same byte.
        bytes.push(0);
    }
    Ok(bytes)
}

/// The model-input identity: the data version label, the model config bytes,
/// and the effective model ladder.
///
/// The full [`fingerprint_legacy`] and [`fingerprint_v5`] identities cover the
/// strategy and every pack, so they change when a research object changes. The
/// model artifact must survive that change; this hash covers only the inputs the
/// model consumes.
fn model_input_hash(
    paths: &ConfigPaths,
    data: &DataConfig,
    model_ladder: Option<&[Timeframe]>,
) -> Result<String, ConfigError> {
    let model_bytes = std::fs::read(&paths.model).map_err(|error| match error.kind() {
        io::ErrorKind::NotFound => ConfigError::missing_file(file_name(&paths.model)),
        _ => ConfigError::unreadable_file(file_name(&paths.model)),
    })?;
    let ladder: Vec<String> = model_ladder
        .unwrap_or(&data.available_timeframes)
        .iter()
        .map(|timeframe| timeframe.code().to_owned())
        .collect();
    Ok(quantrule_api::model_input_hash(&data.data_version, &model_bytes, &ladder).to_string())
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

/// Resolve one config reference inside the run file's directory.
///
/// A client never sends a filesystem path; a `run.json` reference is a name
/// inside the bundle. An absolute path or a `..` component is refused before
/// the loader reads it, so an uploaded bundle cannot escape its staging root.
fn referenced_path(
    base: &Path,
    name: &str,
    run_name: &str,
    field: &str,
) -> Result<PathBuf, ConfigError> {
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(ConfigError::range(
            run_name,
            field,
            "a relative path inside the config directory",
            name,
        ));
    }
    Ok(base.join(path))
}

/// The referenced name's final component, for the same reason.
fn display(name: &str) -> &str {
    Path::new(name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(name)
}

/// The no-op resolver used by [`RunConfig::load`].
///
/// A v5 run loaded this way fails at its first reference with a missing-file
/// error, which names the field a caller must supply bytes for.
struct NoReferences;

impl RunReferenceResolver for NoReferences {
    fn resolve(&self, _kind: ReferenceKind, _reference: &ContentReference) -> Option<&[u8]> {
        None
    }
}
