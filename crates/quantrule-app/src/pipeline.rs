//! The application-service composition pipeline.
//!
//! One module owns the source-to-engine path: read the M1 CSV into the data
//! engine's timeframe set, copy the selected ladder's series into the model
//! input shape, and run the model. Command families parse arguments and render
//! the result; they do not rebuild the ladder or the inputs.
//!
//! The module performs I/O because the application service owns local source access. It does not
//! compute an indicator, a rule, a fill, or an accounting value.

use std::path::Path;

use quantrule_data::{Candle, Timeframe, TimeframeSet};
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::run::ModelRun;

use crate::config::RunConfig;
use crate::csv::{self, SourceError};
use crate::date::DateBounds;

/// The input window shared by the model analysis commands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WindowMode {
    /// Use the full supported series, up to the dataset cap.
    #[default]
    Full,
    /// Use the last [`quantrule_data::DEFAULT_WINDOW`] candles per timeframe.
    Latest,
}

impl WindowMode {
    /// Parses the command-facing window code.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "full" => Some(Self::Full),
            "latest" => Some(Self::Latest),
            _ => None,
        }
    }
}

/// The configured ladder in seconds, fastest first.
#[must_use]
pub fn ladder_secs_for(timeframes: &[Timeframe]) -> Vec<u64> {
    timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect()
}

/// The owned candle series for the selected ladder.
///
/// `TimeframeInput` borrows a slice, so the vectors must outlive the inputs.
/// This type keeps that ownership in one place and exposes the borrowed view.
pub struct ModelInputs {
    series: Vec<(Timeframe, Vec<Candle>)>,
}

impl ModelInputs {
    /// Copies the selected ladder's series out of a loaded set.
    #[must_use]
    pub fn from_set_with_ladder(set: &TimeframeSet, ladder: &[Timeframe]) -> Self {
        let series = ladder
            .iter()
            .map(|timeframe| (*timeframe, set.series(*timeframe).iter().copied().collect()))
            .collect();
        Self { series }
    }

    /// The model inputs, in ladder order.
    #[must_use]
    pub fn as_inputs(&self) -> Vec<TimeframeInput<'_>> {
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
pub enum PipelineError {
    /// The source or the data engine rejected the candles.
    Source(SourceError),
    /// The requested period selected no M1 candle.
    EmptyPeriod,
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => write!(formatter, "{error}"),
            Self::EmptyPeriod => write!(formatter, "the selected period contains no M1 candle"),
        }
    }
}

impl std::error::Error for PipelineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::EmptyPeriod => None,
        }
    }
}

/// Loads a full or latest-window set from one CSV.
pub fn load_set(path: &Path, window: WindowMode) -> Result<TimeframeSet, SourceError> {
    match window {
        WindowMode::Full => csv::load_full_timeframes(path),
        WindowMode::Latest => csv::load_timeframes(path),
    }
}

/// Loads a set bounded by a date range and a refusal limit.
pub fn load_set_with_limit(
    path: &Path,
    bounds: DateBounds,
    limit: usize,
) -> Result<TimeframeSet, SourceError> {
    csv::load_timeframes_with_limit(path, bounds, limit)
}

/// Builds a model run from an already loaded set and an explicit configuration.
#[must_use]
pub fn model_run_from_set(set: &TimeframeSet, config: &MarketStateConfig) -> ModelRun {
    model_run_from_set_with_ladder(set, &Timeframe::DEFAULT_LADDER, config)
}

/// Builds a model run over an explicit ladder.
#[must_use]
pub fn model_run_from_set_with_ladder(
    set: &TimeframeSet,
    ladder: &[Timeframe],
    config: &MarketStateConfig,
) -> ModelRun {
    let inputs = ModelInputs::from_set_with_ladder(set, ladder);
    ModelRun::compute(&inputs.as_inputs(), config)
}

/// Loads the configured run and computes its model over the active ladder.
pub fn run_model(config: &RunConfig) -> Result<ModelRun, PipelineError> {
    let ladder: &[Timeframe] = match &config.model_ladder {
        Some(ladder) => ladder,
        None => &config.data.available_timeframes,
    };
    run_model_with_ladder(config, ladder)
}

/// Loads the configured run and computes its model over an explicit ladder.
///
/// The spec engine uses this to run only the timeframes the spec declares.
pub fn run_model_with_ladder(
    config: &RunConfig,
    ladder: &[Timeframe],
) -> Result<ModelRun, PipelineError> {
    let set = load_set_with_limit(
        &config.data.source.path,
        config.data.bounds,
        config.data.max_candles,
    )
    .map_err(PipelineError::Source)?;
    if set.series(Timeframe::M1).is_empty() {
        return Err(PipelineError::EmptyPeriod);
    }
    Ok(model_run_from_set_with_ladder(&set, ladder, &config.model))
}
