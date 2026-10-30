//! The shell composition pipeline.
//!
//! One module owns the source-to-engine path: read the M1 CSV into the data
//! engine's timeframe set, copy the four series into the model input shape, and
//! run either the model or the full backtest. Command families parse arguments
//! and render the result; they do not rebuild the ladder or the inputs.
//!
//! The module performs I/O because the shell owns source access. It does not
//! compute an indicator, a rule, a fill, or an accounting value.

use std::path::Path;

use quantrule_backtest::runner::{BacktestConfig, BacktestError, BacktestRun};
use quantrule_data::{Candle, Timeframe, TimeframeSet};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

use crate::config::RunConfig;
use crate::csv::{self, SourceError};
use crate::source::DateBounds;

/// The input window shared by the model analysis commands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum WindowMode {
    /// Use the full supported series, up to the dataset cap.
    #[default]
    Full,
    /// Use the last [`quantrule_data::DEFAULT_WINDOW`] candles per timeframe.
    Latest,
}

impl WindowMode {
    pub(crate) fn from_code(code: &str) -> Option<Self> {
        match code {
            "full" => Some(Self::Full),
            "latest" => Some(Self::Latest),
            _ => None,
        }
    }

    /// The stable code used in the dashboard header.
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Latest => "latest",
        }
    }
}

/// The default ladder in seconds, fastest first.
///
/// Path-based commands use this ladder. A configured run uses the available
/// ladder from `data.json`.
#[must_use]
pub(crate) fn ladder_secs() -> [u64; 4] {
    Timeframe::DEFAULT_LADDER.map(|timeframe| u64::from(timeframe.seconds()))
}

/// The configured ladder in seconds, fastest first.
#[must_use]
pub(crate) fn ladder_secs_for(timeframes: &[Timeframe]) -> Vec<u64> {
    timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect()
}

/// The owned candle series for the selected ladder.
///
/// `TimeframeInput` borrows a slice, so the vectors must outlive the inputs.
/// This type keeps that ownership in one place and exposes the borrowed view.
pub(crate) struct ModelInputs {
    series: Vec<(Timeframe, Vec<Candle>)>,
}

impl ModelInputs {
    /// Copies the default ladder's series out of a loaded set.
    #[must_use]
    pub(crate) fn from_set(set: &TimeframeSet) -> Self {
        Self::from_set_with_ladder(set, &Timeframe::DEFAULT_LADDER)
    }

    /// Copies the selected ladder's series out of a loaded set.
    #[must_use]
    pub(crate) fn from_set_with_ladder(set: &TimeframeSet, ladder: &[Timeframe]) -> Self {
        let series = ladder
            .iter()
            .map(|timeframe| (*timeframe, set.series(*timeframe).iter().copied().collect()))
            .collect();
        Self { series }
    }

    /// The model inputs, in ladder order.
    #[must_use]
    pub(crate) fn as_inputs(&self) -> Vec<TimeframeInput<'_>> {
        self.series
            .iter()
            .map(|(timeframe, candles)| TimeframeInput {
                timeframe_secs: u64::from(timeframe.seconds()),
                candles,
            })
            .collect()
    }
}

/// The pipeline failed before a command could render a result.
#[derive(Debug)]
pub(crate) enum PipelineError {
    /// The source or the data engine rejected the candles.
    Source(SourceError),
    /// Execution or the portfolio rejected the run.
    Backtest(BacktestError),
    /// The requested period selected no M1 candle.
    EmptyPeriod,
}

/// Loads a full or latest-window set from one CSV.
pub(crate) fn load_set(path: &Path, window: WindowMode) -> Result<TimeframeSet, SourceError> {
    match window {
        WindowMode::Full => csv::load_full_timeframes(path),
        WindowMode::Latest => csv::load_timeframes(path),
    }
}

/// Loads a set bounded by a date range and a refusal limit.
pub(crate) fn load_set_with_limit(
    path: &Path,
    bounds: DateBounds,
    limit: usize,
) -> Result<TimeframeSet, SourceError> {
    csv::load_timeframes_with_limit(path, bounds, limit)
}

/// Loads an extended set bounded by a date range.
pub(crate) fn load_extended_set(
    path: &Path,
    bounds: DateBounds,
) -> Result<TimeframeSet, SourceError> {
    csv::load_extended_timeframes(path, bounds)
}

/// Builds a model run from an already loaded set and an explicit configuration.
#[must_use]
pub(crate) fn model_run_from_set(set: &TimeframeSet, config: &MarketStateConfig) -> ModelRun {
    model_run_from_set_with_ladder(set, &Timeframe::DEFAULT_LADDER, config)
}

/// Builds a model run over an explicit ladder.
#[must_use]
pub(crate) fn model_run_from_set_with_ladder(
    set: &TimeframeSet,
    ladder: &[Timeframe],
    config: &MarketStateConfig,
) -> ModelRun {
    let inputs = ModelInputs::from_set_with_ladder(set, ladder);
    ModelRun::compute(&inputs.as_inputs(), config)
}

/// Loads the configured run and computes its model once.
pub(crate) fn run_model(config: &RunConfig) -> Result<ModelRun, PipelineError> {
    let set = load_set_with_limit(
        &config.data.source.path,
        config.data.bounds,
        config.data.max_candles,
    )
    .map_err(PipelineError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(PipelineError::EmptyPeriod);
    }
    Ok(model_run_from_set_with_ladder(
        &set,
        &config.data.available_timeframes,
        &config.model,
    ))
}

/// Loads the configured run and runs the full backtest once.
pub(crate) fn run_backtest(config: &RunConfig) -> Result<BacktestRun, PipelineError> {
    let set = load_set_with_limit(
        &config.data.source.path,
        config.data.bounds,
        config.data.max_candles,
    )
    .map_err(PipelineError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(PipelineError::EmptyPeriod);
    }
    let inputs = ModelInputs::from_set_with_ladder(&set, &config.data.available_timeframes);
    let backtest = BacktestConfig {
        model: config.model,
        strategy: config.strategy.engine,
        execution: config.execution.engine,
        portfolio: config.portfolio,
        max_position_fraction: config.policy.risk.max_position_fraction,
    };
    quantrule_backtest::runner::run_backtest(&inputs.as_inputs(), &backtest)
        .map_err(PipelineError::Backtest)
}
