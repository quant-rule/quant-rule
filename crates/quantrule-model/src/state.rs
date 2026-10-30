//! The assembled market state view.
//!
//! The state combines the per-timeframe snapshots, confirmed levels, and the
//! cross-timeframe aggregates and consensus into the latest published view. It
//! is a view over a completed [`crate::run::ModelRun`]; the input contract lives
//! in [`crate::input`] and the consensus diagnostic in [`crate::consensus`].
//!
//! The module is pure: it reads the run it is given. It performs no I/O, reads
//! no clock, and uses no randomness.

use crate::aggregate::{CrossTimeframeAggregates, TimeframeSnapshot};
use crate::axes::structure::registry::ConfirmedLevel;
use crate::consensus::TimeframeConsensus;
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::run::ModelRun;

/// One timeframe's published state.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeMarketState {
    /// The latest snapshot used by the aggregation.
    pub snapshot: TimeframeSnapshot,
    /// The confirmed-level registry knowable at the latest bar.
    pub levels: Vec<ConfirmedLevel>,
    /// The stable series index of the latest candle.
    pub latest_series_index: u64,
    /// The UTC Unix-second open time of the latest candle.
    pub latest_open_time: i64,
    /// The first bar position in the loaded model window.
    pub window_start: usize,
    /// The number of bars in the loaded model window.
    pub window_len: usize,
}

/// The assembled market state across the ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketState {
    /// Per-timeframe states, fastest first.
    pub timeframes: Vec<TimeframeMarketState>,
    /// Cross-timeframe aggregates.
    pub aggregates: CrossTimeframeAggregates,
    /// Per-timeframe consensus diagnostics, fastest first.
    pub consensus: Vec<TimeframeConsensus>,
}

impl MarketState {
    /// Computes the state for every non-empty timeframe input.
    ///
    /// Empty inputs are omitted. The inputs are sorted by timeframe, so their
    /// order does not affect the result.
    #[must_use]
    pub fn compute(inputs: &[TimeframeInput<'_>], config: &MarketStateConfig) -> Self {
        let run = ModelRun::compute(inputs, config);
        Self::from_run(&run)
    }

    /// Builds the latest market state from a completed model run.
    #[must_use]
    pub fn from_run(run: &ModelRun) -> Self {
        let timeframes = run
            .timeframes()
            .iter()
            .map(|timeframe| {
                let latest = timeframe.snapshot().latest_bar_index;
                let candle = timeframe.candle(latest);
                TimeframeMarketState {
                    snapshot: timeframe.snapshot().clone(),
                    levels: timeframe.levels().to_vec(),
                    latest_series_index: candle.map_or(0, |candle| candle.series_index()),
                    latest_open_time: candle.map_or(0, |candle| candle.timestamp()),
                    window_start: 0,
                    window_len: timeframe.len(),
                }
            })
            .collect();

        Self {
            timeframes,
            aggregates: run.aggregates().clone(),
            consensus: run.consensus().to_vec(),
        }
    }

    /// The state for one timeframe, or `None` when it is absent.
    #[must_use]
    pub fn timeframe(&self, timeframe_secs: u64) -> Option<&TimeframeMarketState> {
        self.timeframes
            .iter()
            .find(|state| state.snapshot.timeframe_secs == timeframe_secs)
    }
}

#[cfg(test)]
mod tests;
