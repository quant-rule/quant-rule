//! The staged backtest: strategy, execution, and portfolio from a model bundle.
//!
//! The model is not recomputed. A materialized model bundle is copied into the
//! run bundle and the shared stage loop reads the published bars, so the
//! coupled and staged paths share one strategy, execution, and portfolio
//! implementation. The bundle is the handoff; this module never computes an
//! indicator or a model state.

use std::path::{Path, PathBuf};

use crate::artifact::render_report;
use crate::config::{RunConfig, StrategyConfig};
use crate::stages::backtest::{BacktestOutcome, BacktestRunError, BundleFeed, run_stages};
use crate::stages::strategy::{ModelBundle, validate_context};

/// One staged backtest request.
pub struct BacktestFromModelRequest<'a> {
    /// The run configuration path.
    pub config: &'a Path,
    /// The strategy configuration path.
    pub strategy: &'a Path,
    /// The materialized model bundle directory.
    pub model: &'a Path,
    /// The output root; the run configuration's root when `None`.
    pub out: Option<&'a Path>,
}

/// Runs strategy, execution, and portfolio from a published model bundle.
///
/// The model artifact set is copied into `<out>/backtest/model` byte for byte,
/// so the run bundle records the same model the worker consumed. The stage loop
/// then reads the published readings, events, and candles from that bundle.
///
/// # Errors
///
/// Returns [`BacktestRunError`] when the config, strategy, model bundle, stage
/// bundle, manifest, or writer fails.
pub fn run(request: &BacktestFromModelRequest<'_>) -> Result<BacktestOutcome, BacktestRunError> {
    let config = RunConfig::load(request.config).map_err(BacktestRunError::Config)?;
    let (strategy_config, strategy_config_id) =
        crate::config::load_strategy(request.strategy).map_err(BacktestRunError::StrategyConfig)?;
    run_resolved(
        &config,
        &strategy_config,
        &strategy_config_id,
        request.model,
        request.out,
    )
}

/// Runs the staged backtest from an already-resolved run and strategy.
///
/// The v5 worker resolves the pack and artifact references once, then calls
/// this seam. The resolved config is threaded through every stage, so no stage
/// re-reads the run file and loses the references it cannot resolve.
///
/// # Errors
///
/// Returns [`BacktestRunError`] when the model bundle, context, stage bundle,
/// manifest, or writer fails.
pub fn run_resolved(
    config: &RunConfig,
    strategy_config: &StrategyConfig,
    strategy_config_id: &str,
    model: &Path,
    out: Option<&Path>,
) -> Result<BacktestOutcome, BacktestRunError> {
    let out = out.map_or_else(|| config.output_root.clone(), Path::to_path_buf);
    let root = out.join("backtest");
    render_report::ensure_available(&root).map_err(BacktestRunError::Bundle)?;

    let execution = u64::from(config.execution.fill_timeframe.seconds());
    let bundle =
        ModelBundle::load(model).map_err(|error| BacktestRunError::Model(error.to_string()))?;
    validate_context(strategy_config, Some(execution), |timeframe| {
        bundle.contains_timeframe(timeframe)
    })
    .map_err(BacktestRunError::Context)?;
    if !bundle.contains_timeframe(execution) {
        return Err(BacktestRunError::MissingTimeframe {
            timeframe_secs: execution,
        });
    }
    copy_bundle(model, &root.join("model"))?;
    let feed = BundleFeed::new(&bundle, execution);
    run_stages(config, strategy_config, strategy_config_id, &feed, &root)
}

/// Copy a flat model bundle into the run bundle without changing a byte.
///
/// The model writer publishes flat artifacts; a subdirectory would be a different
/// contract, so it is refused rather than silently flattened.
fn copy_bundle(source: &Path, target: &Path) -> Result<(), BacktestRunError> {
    std::fs::create_dir_all(target).map_err(|error| {
        BacktestRunError::Model(format!(
            "cannot create the model bundle directory {}: {error}",
            target.display()
        ))
    })?;
    let mut entries: Vec<PathBuf> = std::fs::read_dir(source)
        .map_err(|error| {
            BacktestRunError::Model(format!(
                "cannot read the model bundle {}: {error}",
                source.display()
            ))
        })?
        .map(|entry| {
            entry.map(|entry| entry.path()).map_err(|error| {
                BacktestRunError::Model(format!(
                    "cannot read the model bundle {}: {error}",
                    source.display()
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    entries.sort();
    for path in entries {
        if !path.is_file() {
            return Err(BacktestRunError::Model(format!(
                "{}: a model bundle holds files only",
                path.display()
            )));
        }
        let Some(name) = path.file_name() else {
            return Err(BacktestRunError::Model(format!(
                "{}: a model artifact has no name",
                path.display()
            )));
        };
        std::fs::copy(&path, target.join(name)).map_err(|error| {
            BacktestRunError::Model(format!(
                "cannot copy the model artifact {}: {error}",
                path.display()
            ))
        })?;
    }
    Ok(())
}
