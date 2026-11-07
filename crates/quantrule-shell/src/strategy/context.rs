//! Strategy timeframe and context validation shared by the stage commands.
//!
//! The standalone strategy commands and the coupled backtest runner read the
//! same strategy configuration but discover the published timeframes from
//! different artifacts: the model bundle in one path, the in-memory model run in
//! the other. This module owns the one validation rule so the two paths cannot
//! disagree about the execution timeframe or the condition context.

use std::fmt;

use crate::config::StrategyConfig;

/// The strategy configuration does not fit the published model timeframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrategyContextError {
    /// The model does not publish a required timeframe.
    MissingTimeframe {
        /// The timeframe in seconds.
        timeframe_secs: u64,
    },
    /// A condition names a timeframe outside execution plus context.
    ConditionTimeframeNotInContext {
        /// The condition timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The strategy declares a different execution timeframe than the run.
    ExecutionTimeframeMismatch {
        /// The strategy's declared execution timeframe.
        strategy: u64,
        /// The run's execution timeframe.
        run: u64,
    },
}

impl fmt::Display for StrategyContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingTimeframe { timeframe_secs } => write!(
                formatter,
                "the model does not publish the timeframe {timeframe_secs}s"
            ),
            Self::ConditionTimeframeNotInContext { timeframe_secs } => write!(
                formatter,
                "a strategy condition names {timeframe_secs}s, but it is not the execution timeframe or a configured context timeframe"
            ),
            Self::ExecutionTimeframeMismatch { strategy, run } => write!(
                formatter,
                "the strategy declares execution timeframe {strategy}s, but the run's execution timeframe is {run}s"
            ),
        }
    }
}

impl std::error::Error for StrategyContextError {}

/// Validates the strategy's execution and context timeframes.
///
/// `expected_execution` is `Some` in the coupled runner, where the run config
/// owns the execution timeframe; it is `None` in the standalone strategy
/// commands, where the strategy owns it. `published` reports whether the model
/// publishes one timeframe.
///
/// # Errors
///
/// Returns [`StrategyContextError`] when the declared execution timeframe
/// differs from the run's, a required timeframe is not published, or a
/// condition names a timeframe outside execution plus context.
pub(crate) fn validate_context(
    config: &StrategyConfig,
    expected_execution: Option<u64>,
    published: impl Fn(u64) -> bool,
) -> Result<(), StrategyContextError> {
    let execution = u64::from(config.execution_timeframe.seconds());
    if let Some(expected) = expected_execution
        && execution != expected
    {
        return Err(StrategyContextError::ExecutionTimeframeMismatch {
            strategy: execution,
            run: expected,
        });
    }
    if !published(execution) {
        return Err(StrategyContextError::MissingTimeframe {
            timeframe_secs: execution,
        });
    }
    let mut allowed = vec![execution];
    for timeframe in &config.context_timeframes {
        let seconds = u64::from(timeframe.seconds());
        if !published(seconds) {
            return Err(StrategyContextError::MissingTimeframe {
                timeframe_secs: seconds,
            });
        }
        allowed.push(seconds);
    }
    for timeframe in config.condition_timeframes() {
        if !allowed.contains(&timeframe) {
            return Err(StrategyContextError::ConditionTimeframeNotInContext {
                timeframe_secs: timeframe,
            });
        }
    }
    Ok(())
}
