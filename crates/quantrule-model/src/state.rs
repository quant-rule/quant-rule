//! The assembled market state.
//!
//! The state combines the per-timeframe readings, four axes, context, and event
//! records with the cross-timeframe aggregates and the per-timeframe consensus
//! diagnostic. It publishes the latest closed bar of each timeframe; the
//! per-bar series remain inside the engine for causality tests.
//!
//! The module is pure: it reads the candles and configuration it is given. It
//! performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::aggregate::{CrossTimeframeAggregates, ReferenceSet, TimeframeSnapshot};
use crate::axes::direction::DirectionSeries;
use crate::axes::momentum::MomentumSeries;
use crate::axes::progression::{ProgressionConfig, ProgressionSeries};
use crate::axes::structure::registry::{ConfirmedLevel, ConfirmedLevelRegistry};
use crate::axes::structure::{StructureConfig, StructureSeries};
use crate::context::ContextSeries;
use crate::events::EventRecord;
use crate::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use crate::events::level::{LevelEventConfig, LevelEventSeries};
use crate::events::signal::{SignalConfig, SignalEventSeries};
use crate::readings::{PivotKind, PivotScale, Readings};

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
        let values = [
            snapshot.direction.direction,
            snapshot.momentum.force,
            snapshot.structure.score,
        ];
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
            (1_u8, snapshot.direction.size_percentile),
            (2, snapshot.momentum.size_percentile),
            (3, snapshot.structure.size_percentile),
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
            timeframe_secs: snapshot.timeframe_secs,
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
        let mut states = Vec::new();
        for input in inputs {
            if input.candles.is_empty() {
                continue;
            }
            if let Some(state) = build_timeframe(input, config) {
                states.push(state);
            }
        }
        states.sort_by_key(|state| state.snapshot.timeframe_secs);

        let snapshots: Vec<TimeframeSnapshot> =
            states.iter().map(|state| state.snapshot.clone()).collect();
        let aggregates = CrossTimeframeAggregates::compute(&snapshots);
        let consensus = states
            .iter()
            .map(|state| TimeframeConsensus::compute(&state.snapshot))
            .collect();

        Self {
            timeframes: states,
            aggregates,
            consensus,
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

fn build_timeframe(
    input: &TimeframeInput<'_>,
    config: &MarketStateConfig,
) -> Option<TimeframeMarketState> {
    let timeframe_secs = NonZeroU64::new(input.timeframe_secs)?;
    let readings = Readings::compute(input.candles);
    let direction = DirectionSeries::compute(&readings);
    let momentum = MomentumSeries::compute(&readings);
    let structure = StructureSeries::compute_with_config(
        input.candles,
        &readings,
        StructureConfig::new(config.structure_scale),
    );
    let progression = ProgressionSeries::compute_with_config(
        input.candles,
        &readings,
        ProgressionConfig::new(input.timeframe_secs),
    );
    let context = ContextSeries::compute(&readings);

    let level_events = LevelEventSeries::compute(
        input.candles,
        &readings,
        &LevelEventConfig::new(timeframe_secs),
    );
    let divergence_events = DivergenceEventSeries::compute(
        &readings,
        &DivergenceConfig::new(timeframe_secs, config.structure_scale),
    );
    let signal_events = SignalEventSeries::compute(
        &readings,
        &direction,
        &momentum,
        &structure,
        &SignalConfig::new(timeframe_secs),
    );

    let latest = input.candles.len().checked_sub(1)?;
    let direction_bar = direction.latest()?;
    let momentum_bar = momentum.latest()?;
    let structure_bar = structure.latest()?;
    let progression_bar = progression.latest()?;
    let context_bar = context.latest()?;

    let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
    let levels: Vec<ConfirmedLevel> = registry
        .levels()
        .iter()
        .copied()
        .filter(|level| level.available_at <= latest)
        .collect();
    let (minor_high, minor_low, major_high, major_low) = latest_swings(&levels);
    let references = ReferenceSet::from_parts(
        readings.ema20.get(latest),
        readings.ema50.get(latest),
        readings.ema200.get(latest),
        minor_high.map(|level| level.price),
        minor_low.map(|level| level.price),
        major_high.map(|level| level.price),
        major_low.map(|level| level.price),
    );

    let mut events: Vec<EventRecord> = level_events
        .events()
        .iter()
        .chain(divergence_events.events())
        .chain(signal_events.events())
        .copied()
        .collect();
    events.sort_by_key(|event| {
        (
            event.available_bar_index,
            event.occurred_bar_index,
            event.event_type.code(),
            event.event_subtype_code,
            event.source_code,
            event.reference_code.unwrap_or(0),
        )
    });

    let minor_origin_time_secs = progression_bar.minor_leg.and_then(|leg| {
        input
            .candles
            .get(leg.origin_occurred_at)
            .map(Candle::timestamp)
    });
    let major_origin_time_secs = progression_bar.major_leg.and_then(|leg| {
        input
            .candles
            .get(leg.origin_occurred_at)
            .map(Candle::timestamp)
    });

    Some(TimeframeMarketState {
        snapshot: TimeframeSnapshot {
            timeframe_secs: input.timeframe_secs,
            latest_bar_index: latest,
            direction: direction_bar,
            momentum: momentum_bar,
            structure: structure_bar,
            progression: progression_bar,
            context: context_bar,
            atr: readings.atr14.get(latest),
            references,
            events,
            minor_origin_time_secs,
            major_origin_time_secs,
        },
        levels,
    })
}

fn latest_swings(
    levels: &[ConfirmedLevel],
) -> (
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
) {
    let mut minor_high = None;
    let mut minor_low = None;
    let mut major_high = None;
    let mut major_low = None;
    for level in levels {
        match (level.scale, level.kind) {
            (PivotScale::Minor, PivotKind::High) => minor_high = Some(*level),
            (PivotScale::Minor, PivotKind::Low) => minor_low = Some(*level),
            (PivotScale::Major, PivotKind::High) => major_high = Some(*level),
            (PivotScale::Major, PivotKind::Low) => major_low = Some(*level),
        }
    }
    (minor_high, minor_low, major_high, major_low)
}

#[cfg(test)]
mod tests;
