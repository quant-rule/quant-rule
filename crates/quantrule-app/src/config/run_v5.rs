//! `quantrule.run/v5`: references to the model artifact and research packs.
//!
//! A v5 run carries no strategy, execution, or portfolio file. It names the
//! strategy pack, portfolio policy pack and execution-simulation pack by id and
//! hash, and optionally names the model artifact and the signal artifact. The
//! account's starting cash and currency stay in the run; the packs do not
//! duplicate them.
//!
//! The loader resolves every reference through a [`RunReferenceResolver`],
//! verifies the content hash, parses only the kind the field promises, and
//! materializes the pipeline's strategy and engine configurations from the
//! validated payloads. The resolved payloads are retained in [`RunV5`] so a
//! later stage can use the full policy instead of a lossy projection. The
//! persistent registry, signal computation and strategy service are later work
//! items.

use quantrule_backtest::portfolio::{
    Currency, MarkToMarket, PortfolioConfig, PortfolioConfigError,
};
use quantrule_data::Timeframe;
use serde_json::Value;

use crate::config::read;
use crate::config::{ConfigError, StrategyConfig};
use crate::pack::references::{self, parse_model_artifact, parse_signal_artifact};
use crate::pack::{
    ContentReference, ExecutionPack, ExecutionPackPayload, PortfolioPack, PortfolioPackPayload,
    ReferenceKind, RunReferenceResolver, RunReferences, SignalArtifactHeader, StrategyMode,
    StrategyPack, StrategyPackPayload,
};

use super::data::DataConfig;
use super::execution::ExecutionRunConfig;

/// The account inputs a run owns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunAccount {
    /// The starting cash in the quote currency.
    pub initial_cash: f64,
    /// The quote currency; only USD is supported in v1.
    pub currency: Currency,
}

/// The fully resolved v5 composition.
///
/// The pipeline fields are the materialized engine values. The payload fields
/// retain the complete validated payload, so a field the current fold does not
/// yet apply is still recorded rather than silently dropped.
#[derive(Debug, Clone)]
pub struct RunV5 {
    /// The five content references the run cites.
    pub references: RunReferences,
    /// The validated strategy payload.
    pub strategy: StrategyPackPayload,
    /// The signal artifact header, present only in signal mode.
    pub signal_artifact: Option<SignalArtifactHeader>,
    /// The validated portfolio policy payload.
    pub portfolio: PortfolioPackPayload,
    /// The validated execution-simulation payload.
    pub execution: ExecutionPackPayload,
    /// The account cash and currency from the run.
    pub account: RunAccount,
}

/// The v5-only fields, as written.
#[derive(Debug, Clone)]
pub(super) struct V5RunFile {
    model_artifact: Option<ContentReference>,
    strategy_pack: ContentReference,
    signal_artifact: Option<ContentReference>,
    portfolio_policy: ContentReference,
    execution_model: ContentReference,
    account: RunAccount,
}

/// The materialized values the run loader needs from a v5 document.
pub(super) struct V5Resolved {
    pub(super) strategy: StrategyConfig,
    pub(super) execution: ExecutionRunConfig,
    pub(super) portfolio: PortfolioConfig,
    pub(super) v5: RunV5,
}

impl V5RunFile {
    /// The five references, without resolving the referenced bytes.
    ///
    /// This is the structural seam the data service uses to populate
    /// `ConfigBundle.references` before evidence resolves the content.
    pub(crate) fn references(&self) -> RunReferences {
        RunReferences {
            model_artifact: self.model_artifact.clone(),
            strategy_pack: self.strategy_pack.clone(),
            signal_artifact: self.signal_artifact.clone(),
            portfolio_policy: self.portfolio_policy.clone(),
            execution_model: self.execution_model.clone(),
        }
    }

    /// Parses the v5-only fields; the caller has already checked unknown fields.
    pub(super) fn parse(
        file: &str,
        object: &serde_json::Map<String, Value>,
    ) -> Result<Self, ConfigError> {
        let model_artifact = optional_reference(file, object, "model_artifact")?;
        let strategy_pack = required_reference(file, object, "strategy_pack")?;
        let signal_artifact = optional_reference(file, object, "signal_artifact")?;
        let portfolio_policy = required_reference(file, object, "portfolio_policy")?;
        let execution_model = required_reference(file, object, "execution_model")?;
        let account = parse_account(file, object)?;
        Ok(Self {
            model_artifact,
            strategy_pack,
            signal_artifact,
            portfolio_policy,
            execution_model,
            account,
        })
    }
}

/// Parses a v5 `run.json` value into its reference set.
///
/// The caller has already established the `quantrule.run/v5` schema. This
/// function checks every reference's id/hash pair but does not read a pack or
/// artifact body; the data service resolves those from its own registry.
pub fn parse_references(file: &str, value: &Value) -> Result<RunReferences, ConfigError> {
    let object = read::object(file, "$", value)?;
    Ok(V5RunFile::parse(file, object)?.references())
}

/// Resolves and verifies every reference, then builds the engine values.
///
/// # Errors
///
/// Returns a [`ConfigError`] when a reference is missing or hashes to other
/// bytes, a pack is the wrong kind, a signal or model artifact disagrees with
/// the run, the account or policy is invalid, or the strategy's timeframe and
/// universe disagree with the execution pack and data ladder.
pub(super) fn resolve(
    file_name: &str,
    file: &V5RunFile,
    data: &DataConfig,
    model_id: Option<&str>,
    ladder: &[Timeframe],
    resolver: &dyn RunReferenceResolver,
) -> Result<V5Resolved, ConfigError> {
    let strategy_pack = resolve_pack::<StrategyPack>(
        resolver,
        ReferenceKind::StrategyPack,
        &file.strategy_pack,
        file_name,
        "$.strategy_pack",
        StrategyPack::from_stored_bytes,
    )?;
    let execution_pack = resolve_pack::<ExecutionPack>(
        resolver,
        ReferenceKind::ExecutionModel,
        &file.execution_model,
        file_name,
        "$.execution_model",
        ExecutionPack::from_stored_bytes,
    )?;
    let portfolio_pack = resolve_pack::<PortfolioPack>(
        resolver,
        ReferenceKind::PortfolioPolicy,
        &file.portfolio_policy,
        file_name,
        "$.portfolio_policy",
        PortfolioPack::from_stored_bytes,
    )?;

    if matches!(
        (strategy_pack.payload().mode(), &file.signal_artifact),
        (StrategyMode::Signal, None)
    ) {
        return Err(ConfigError::missing_field(file_name, "$.signal_artifact"));
    }
    if matches!(
        (strategy_pack.payload().mode(), &file.signal_artifact),
        (StrategyMode::Fold, Some(_))
    ) {
        return Err(ConfigError::inconsistent(
            file_name,
            "$.signal_artifact",
            "absent for a fold-mode strategy pack",
            "present",
        ));
    }

    if let Some(reference) = &file.model_artifact {
        resolve_model_artifact(resolver, reference, file_name, model_id, ladder)?;
    }
    let signal_artifact = match &file.signal_artifact {
        Some(reference) => Some(resolve_signal_artifact(
            resolver,
            reference,
            &file.strategy_pack,
            &file.model_artifact,
            file_name,
        )?),
        None => None,
    };

    let execution = ExecutionRunConfig {
        engine: execution_pack.payload().costs(),
        fill_timeframe: execution_pack.payload().fill_timeframe(),
    };
    if let Some(signal) = &signal_artifact
        && signal.timeframe != execution.fill_timeframe
    {
        return Err(ConfigError::inconsistent(
            file_name,
            "$.signal_artifact",
            &format!("timeframe {}", execution.fill_timeframe.code()),
            signal.timeframe.code(),
        ));
    }
    let strategy = strategy_pack.payload().strategy().clone();
    if strategy.execution_timeframe != execution.fill_timeframe {
        return Err(ConfigError::inconsistent(
            file_name,
            "$.execution_model",
            &format!(
                "the strategy execution timeframe {}",
                strategy.execution_timeframe.code()
            ),
            execution.fill_timeframe.code(),
        ));
    }
    if let Some(universe) = strategy_universe(&strategy_pack)
        && !universe
            .iter()
            .any(|instrument| instrument == &data.instrument)
    {
        return Err(ConfigError::inconsistent(
            file_name,
            "$.strategy_pack",
            &format!("a universe containing {}", data.instrument),
            &universe.join(","),
        ));
    }

    let portfolio =
        materialize_portfolio(&file.account, &portfolio_pack, &execution_pack, file_name)?;
    let references = RunReferences {
        model_artifact: file.model_artifact.clone(),
        strategy_pack: file.strategy_pack.clone(),
        signal_artifact: file.signal_artifact.clone(),
        portfolio_policy: file.portfolio_policy.clone(),
        execution_model: file.execution_model.clone(),
    };
    Ok(V5Resolved {
        strategy,
        execution,
        portfolio,
        v5: RunV5 {
            references,
            strategy: strategy_pack.payload().clone(),
            signal_artifact,
            portfolio: portfolio_pack.payload().clone(),
            execution: execution_pack.payload().clone(),
            account: file.account,
        },
    })
}

fn resolve_pack<T>(
    resolver: &dyn RunReferenceResolver,
    kind: ReferenceKind,
    reference: &ContentReference,
    file: &str,
    path: &str,
    parse: impl FnOnce(&[u8], &str) -> Result<T, crate::pack::PackError>,
) -> Result<T, ConfigError> {
    let bytes = crate::pack::references::resolve_bytes(resolver, kind, reference, file, path)?;
    parse(bytes, file).map_err(|error| references::pack_error(file, path, error))
}

fn resolve_model_artifact(
    resolver: &dyn RunReferenceResolver,
    reference: &ContentReference,
    file: &str,
    model_id: Option<&str>,
    ladder: &[Timeframe],
) -> Result<crate::pack::ModelArtifactHeader, ConfigError> {
    // A model artifact is a file set, so its content address is the canonical
    // digest over its `(name, sha256)` entries, not the hash of the manifest
    // bytes. The resolver returns the manifest because that is the record the
    // registry stores; the artifact-set digest is recomputed and checked here.
    let bytes = resolver
        .resolve(ReferenceKind::ModelArtifact, reference)
        .ok_or_else(|| {
            ConfigError::missing_reference(
                file,
                "$.model_artifact",
                "a resolvable model_artifact reference",
            )
        })?;
    let header = parse_model_artifact(bytes, file, "$.model_artifact")?;
    if header.artifact_id != reference.hash {
        return Err(ConfigError::inconsistent(
            file,
            "$.model_artifact",
            &format!(
                "an artifact set whose canonical digest is {}",
                reference.hash
            ),
            &header.artifact_id.to_string(),
        ));
    }
    if let Some(model_id) = model_id
        && header.model_version != model_id
    {
        return Err(ConfigError::inconsistent(
            file,
            "$.model_artifact",
            &format!("model version {model_id}"),
            &header.model_version,
        ));
    }
    let expected: Vec<String> = ladder
        .iter()
        .map(|timeframe| timeframe.code().to_owned())
        .collect();
    if header.ladder != expected {
        return Err(ConfigError::inconsistent(
            file,
            "$.model_artifact",
            &format!("the run's model ladder [{}]", expected.join(", ")),
            &header.ladder.join(","),
        ));
    }
    Ok(header)
}

fn resolve_signal_artifact(
    resolver: &dyn RunReferenceResolver,
    reference: &ContentReference,
    strategy_pack: &ContentReference,
    model_artifact: &Option<ContentReference>,
    file: &str,
) -> Result<SignalArtifactHeader, ConfigError> {
    let bytes = crate::pack::references::resolve_bytes(
        resolver,
        ReferenceKind::SignalArtifact,
        reference,
        file,
        "$.signal_artifact",
    )?;
    let header = parse_signal_artifact(bytes, file, "$.signal_artifact")?;
    if header.strategy_pack_id != strategy_pack.hash {
        return Err(ConfigError::inconsistent(
            file,
            "$.signal_artifact",
            &format!("strategy pack {}", strategy_pack.hash),
            &header.strategy_pack_id.to_string(),
        ));
    }
    let Some(model_artifact) = model_artifact else {
        return Err(ConfigError::missing_field(file, "$.model_artifact"));
    };
    if header.model_artifact_id != model_artifact.hash {
        return Err(ConfigError::inconsistent(
            file,
            "$.signal_artifact",
            &format!("model artifact {}", model_artifact.hash),
            &header.model_artifact_id.to_string(),
        ));
    }
    Ok(header)
}

fn strategy_universe(pack: &StrategyPack) -> Option<Vec<String>> {
    let universe = pack.payload().universe();
    if universe.is_empty() {
        None
    } else {
        Some(universe.to_vec())
    }
}

fn materialize_portfolio(
    account: &RunAccount,
    portfolio: &PortfolioPack,
    execution: &ExecutionPack,
    file: &str,
) -> Result<PortfolioConfig, ConfigError> {
    let config = PortfolioConfig {
        initial_cash: account.initial_cash,
        currency: account.currency,
        sizing: portfolio.payload().sizing(),
        leverage: 1.0,
        max_open_positions: portfolio.payload().max_open_positions(),
        commission: execution.payload().commission(),
        funding: execution.payload().funding(),
        max_drawdown: portfolio.payload().drawdown(),
        mark_to_market: MarkToMarket::BarClose,
    };
    config
        .validate()
        .map_err(|error| portfolio_error(file, error))?;
    Ok(config)
}

fn portfolio_error(file: &str, error: PortfolioConfigError) -> ConfigError {
    ConfigError::range(
        file,
        "$.portfolio_policy",
        "a portfolio policy the current ledger can apply",
        &error.to_string(),
    )
}

fn parse_account(
    file: &str,
    object: &serde_json::Map<String, Value>,
) -> Result<RunAccount, ConfigError> {
    let path = "$.account";
    let account = read::object(file, path, read::field(file, "$", object, "account")?)?;
    read::check_fields(file, path, account, &["initial_cash", "currency"])?;
    let initial_cash = read::number(
        file,
        &read::child(path, "initial_cash"),
        read::field(file, path, account, "initial_cash")?,
    )?;
    if initial_cash <= 0.0 {
        return Err(ConfigError::range(
            file,
            &read::child(path, "initial_cash"),
            "> 0",
            &initial_cash.to_string(),
        ));
    }
    let currency = read::string(
        file,
        &read::child(path, "currency"),
        read::field(file, path, account, "currency")?,
    )?;
    let currency = Currency::parse(currency).map_err(|_| {
        ConfigError::unsupported(
            file,
            &read::child(path, "currency"),
            Currency::Usd.code(),
            currency,
        )
    })?;
    Ok(RunAccount {
        initial_cash,
        currency,
    })
}

fn required_reference(
    file: &str,
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<ContentReference, ConfigError> {
    let path = read::child("$", field);
    references::parse_content_reference(file, &path, read::field(file, "$", object, field)?)
}

fn optional_reference(
    file: &str,
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<Option<ContentReference>, ConfigError> {
    let Some(value) = object.get(field) else {
        return Ok(None);
    };
    let path = read::child("$", field);
    references::parse_content_reference(file, &path, value).map(Some)
}

#[cfg(test)]
mod tests;
