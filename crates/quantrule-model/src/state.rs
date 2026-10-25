//! The assembled market state.
//!
//! The state combines the per-timeframe readings, four axes, context, and event
//! records with the cross-timeframe aggregates and the per-timeframe consensus
//! diagnostic. It publishes the latest closed bar of each timeframe; the
//! per-bar series remain inside the engine for causality tests.
//!
//! The module is pure: it reads the candles and configuration it is given. It
//! performs no I/O, reads no clock, and uses no randomness.

use quantrule_data::Candle;

use crate::aggregate::{CrossTimeframeAggregates, TimeframeSnapshot};
use crate::axes::direction::DirectionBar;
use crate::axes::momentum::MomentumBar;
use crate::axes::structure::registry::ConfirmedLevel;
use crate::axes::structure::StructureBar;
use crate::readings::PivotScale;
use crate::run::ModelRun;

/// The configuration of a market-state run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketStateConfig {
    /// The pivot scale used by Structure and Divergence.
    pub structure_scale: PivotScale,
}

impl MarketStateConfig {
    /// Builds the configuration for a structure scale.
    #[must_use]
    pub const fn new(structure_scale: PivotScale) -> Self {
        Self { structure_scale }
    }
}

impl Default for MarketStateConfig {
    fn default() -> Self {
        Self {
            structure_scale: PivotScale::Major,
        }
    }
}

/// One timeframe's candles, paired with its timeframe in seconds.
#[derive(Debug, Clone, Copy)]
pub struct TimeframeInput<'a> {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The closed candles for this timeframe.
    pub candles: &'a [Candle],
}

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
}

/// The cross-axis consensus for one timeframe.
///
/// The counts cover Direction, Momentum, and Structure only. Progression is
/// excluded from the directional consensus and is published separately on the
/// Progression axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeframeConsensus {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// Number of axes above +0.1.
    pub bullish_axis_count: u8,
    /// Number of axes below -0.1.
    pub bearish_axis_count: u8,
    /// Number of axes in the inclusive ±0.1 band.
    pub neutral_axis_count: u8,
    /// The lowest valid size percentile's axis code: 1 Direction, 2 Momentum,
    /// 3 Structure; null when no percentile is available.
    pub weakest_directional_axis_code: Option<u8>,
}

impl TimeframeConsensus {
    /// Builds the consensus for one timeframe snapshot.
    #[must_use]
    pub fn compute(snapshot: &TimeframeSnapshot) -> Self {
        Self::from_bars(
            snapshot.timeframe_secs,
            &snapshot.direction,
            &snapshot.momentum,
            &snapshot.structure,
        )
    }

    /// Builds the consensus for one closed bar from its three voting bars.
    ///
    /// Direction, Momentum, and Structure vote; Progression is excluded. A
    /// missing value is not counted as neutral. The weakest directional axis is
    /// the lowest valid size percentile, with the lower axis code breaking a
    /// tie.
    #[must_use]
    pub fn from_bars(
        timeframe_secs: u64,
        direction: &DirectionBar,
        momentum: &MomentumBar,
        structure: &StructureBar,
    ) -> Self {
        let values = [direction.direction, momentum.force, structure.score];
        let mut bullish = 0_u8;
        let mut bearish = 0_u8;
        let mut neutral = 0_u8;
        for value in values.into_iter().flatten() {
            if value > 0.1 {
                bullish += 1;
            } else if value < -0.1 {
                bearish += 1;
            } else {
                neutral += 1;
            }
        }

        let candidates = [
            (1_u8, direction.size_percentile),
            (2, momentum.size_percentile),
            (3, structure.size_percentile),
        ];
        let mut weakest: Option<(u8, f64)> = None;
        for (code, percentile) in candidates {
            let Some(percentile) = percentile else {
                continue;
            };
            if weakest.is_none_or(|(weakest_code, weakest_percentile)| {
                percentile < weakest_percentile
                    || (percentile == weakest_percentile && code < weakest_code)
            }) {
                weakest = Some((code, percentile));
            }
        }

        Self {
            timeframe_secs,
            bullish_axis_count: bullish,
            bearish_axis_count: bearish,
            neutral_axis_count: neutral,
            weakest_directional_axis_code: weakest.map(|(code, _)| code),
        }
    }
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
