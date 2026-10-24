//! Cross-timeframe aggregation.
//!
//! The aggregator combines the per-timeframe snapshots over an ordered
//! fastest-to-slowest ladder. Each axis uses a method that suits it: means and
//! shares for Direction and Momentum, histograms and spans for Structure and
//! Progression, and counts for events. Ordinal codes are counted, never
//! averaged, and every aggregate names the timeframes it came from.
//!
//! The aggregator is pure: it reads only the snapshots it is given. It performs
//! no I/O, reads no clock, and uses no randomness.

pub mod events;
pub mod levels;
pub mod progression;

pub use events::{EventAggregates, EventSubtypeCounts, TimeframeEventSummary};
pub use levels::{LevelClusterAggregates, ReferenceSet};
pub use progression::{ProgressionAggregates, ScaleStageAggregates};

use crate::axes::direction::DirectionBar;
use crate::axes::momentum::MomentumBar;
use crate::axes::progression::ProgressionBar;
use crate::axes::structure::StructureBar;
use crate::context::ContextBar;
use crate::events::EventRecord;
use crate::kernel::Readiness;

/// One timeframe's latest state, as the aggregator's input.
///
/// The caller supplies the snapshot in ladder order; the aggregator sorts by
/// `timeframe_secs` so the result does not depend on the input order. The
/// `minor_origin_time_secs` and `major_origin_time_secs` are the origin
/// timestamps of the retained leg at each scale, in UTC Unix seconds; they are
/// needed for the cross-timeframe leg match.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeSnapshot {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The index of the latest closed bar in this series.
    pub latest_bar_index: usize,
    /// The Direction bar.
    pub direction: DirectionBar,
    /// The Momentum bar.
    pub momentum: MomentumBar,
    /// The Structure bar.
    pub structure: StructureBar,
    /// The Progression bar.
    pub progression: ProgressionBar,
    /// The context bar.
    pub context: ContextBar,
    /// ATR 14 at the latest bar, used by the level clusters and leg matching.
    pub atr: Option<f64>,
    /// The bounded A6 reference set for the latest bar.
    pub references: ReferenceSet,
    /// The retained event records for this timeframe.
    pub events: Vec<EventRecord>,
    /// The minor leg's origin time in UTC Unix seconds, when a leg exists.
    pub minor_origin_time_secs: Option<i64>,
    /// The major leg's origin time in UTC Unix seconds, when a leg exists.
    pub major_origin_time_secs: Option<i64>,
}

/// The cross-timeframe aggregate output.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTimeframeAggregates {
    /// The ordered timeframe ladder, fastest first.
    pub ladder: Vec<u64>,
    /// Direction aggregates.
    pub direction: DirectionAggregates,
    /// Momentum aggregates.
    pub momentum: MomentumAggregates,
    /// Structure aggregates, including the level clusters.
    pub structure: StructureAggregates,
    /// Progression aggregates.
    pub progression: ProgressionAggregates,
    /// Event aggregates.
    pub events: EventAggregates,
}

impl CrossTimeframeAggregates {
    /// Computes every aggregate over the ladder.
    #[must_use]
    pub fn compute(snapshots: &[TimeframeSnapshot]) -> Self {
        let mut ordered: Vec<&TimeframeSnapshot> = snapshots.iter().collect();
        ordered.sort_by_key(|snapshot| snapshot.timeframe_secs);
        let ladder: Vec<u64> = ordered
            .iter()
            .map(|snapshot| snapshot.timeframe_secs)
            .collect();
        let levels = levels::cluster(&ordered);
        let direction = direction_aggregates(&ordered);
        let momentum = momentum_aggregates(&ordered);
        let structure = structure_aggregates(&ordered, levels);
        let progression = progression::aggregate(&ordered);
        let events = events::aggregate(&ordered);

        Self {
            ladder,
            direction,
            momentum,
            structure,
            progression,
            events,
        }
    }
}

/// Direction aggregates.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionAggregates {
    /// Timeframes with a valid Direction score.
    pub timeframes: Vec<u64>,
    /// The number of configured timeframes.
    pub direction_timeframe_count: usize,
    /// Mean of the valid scores.
    pub direction_mean: Option<f64>,
    /// Share of valid scores above +0.1.
    pub direction_bullish_share: Option<f64>,
    /// Share of valid scores below -0.1.
    pub direction_bearish_share: Option<f64>,
    /// Share of valid scores in the inclusive ±0.1 band.
    pub direction_neutral_share: Option<f64>,
    /// Slowest score minus fastest score.
    pub direction_gradient: Option<f64>,
    /// Timeframes whose Direction readiness is ready.
    pub direction_ready_count: usize,
}

/// Momentum aggregates.
#[derive(Debug, Clone, PartialEq)]
pub struct MomentumAggregates {
    /// Timeframes with a valid force.
    pub timeframes: Vec<u64>,
    /// The number of configured timeframes.
    pub momentum_timeframe_count: usize,
    /// Mean of the valid forces.
    pub momentum_force_mean: Option<f64>,
    /// Mean of the valid acceleration values.
    pub momentum_acceleration_mean: Option<f64>,
    /// Share of valid forces above +0.1.
    pub momentum_positive_share: Option<f64>,
    /// Share of valid forces below -0.1.
    pub momentum_negative_share: Option<f64>,
    /// Fastest force minus slowest force.
    pub momentum_force_fast_slow_gap: Option<f64>,
    /// Fastest acceleration minus slowest acceleration.
    pub momentum_acceleration_fast_slow_gap: Option<f64>,
    /// Slowest force minus fastest force.
    pub momentum_gradient: Option<f64>,
    /// Timeframes whose Momentum readiness is ready.
    pub momentum_ready_count: usize,
}

/// Structure aggregates.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureAggregates {
    /// Timeframes with a valid structure score.
    pub timeframes: Vec<u64>,
    /// The number of configured timeframes.
    pub structure_timeframe_count: usize,
    /// Counts for codes -2..+2, indexed by code + 2.
    pub structure_code_histogram: [u32; 5],
    /// Share of valid scores above +0.1.
    pub structure_bullish_share: Option<f64>,
    /// Share of valid scores below -0.1.
    pub structure_bearish_share: Option<f64>,
    /// Share of valid scores in the inclusive ±0.1 band.
    pub structure_neutral_share: Option<f64>,
    /// Maximum valid code minus minimum valid code.
    pub structure_state_span: Option<u8>,
    /// Slowest score minus fastest score.
    pub structure_gradient: Option<f64>,
    /// Share of all timeframes whose Structure readiness is ready.
    pub structure_ready_share: Option<f64>,
    /// Timeframes with opposite high and low relation signs.
    pub structure_conflict_timeframe_count: usize,
    /// Number of single-linkage level clusters.
    pub level_cluster_count: u32,
    /// Distinct timeframes in the largest cluster.
    pub largest_level_cluster_timeframe_count: u32,
    /// Price span of the largest cluster divided by its maximum ATR.
    pub largest_level_cluster_span_atr: Option<f64>,
}

fn direction_aggregates(ordered: &[&TimeframeSnapshot]) -> DirectionAggregates {
    let valid: Vec<(u64, f64)> = ordered
        .iter()
        .filter_map(|snapshot| {
            snapshot
                .direction
                .direction
                .map(|value| (snapshot.timeframe_secs, value))
        })
        .collect();
    let values: Vec<f64> = valid.iter().map(|(_, value)| *value).collect();
    let bullish = values.iter().filter(|value| **value > 0.1).count();
    let bearish = values.iter().filter(|value| **value < -0.1).count();
    let neutral = values.iter().filter(|value| value.abs() <= 0.1).count();
    let ready_count = ordered
        .iter()
        .filter(|snapshot| snapshot.direction.readiness == Readiness::Ready)
        .count();

    DirectionAggregates {
        timeframes: valid.iter().map(|(timeframe, _)| *timeframe).collect(),
        direction_timeframe_count: ordered.len(),
        direction_mean: mean(&values),
        direction_bullish_share: share(bullish, values.len()),
        direction_bearish_share: share(bearish, values.len()),
        direction_neutral_share: share(neutral, values.len()),
        direction_gradient: gradient(ordered, |snapshot| snapshot.direction.direction),
        direction_ready_count: ready_count,
    }
}

fn momentum_aggregates(ordered: &[&TimeframeSnapshot]) -> MomentumAggregates {
    let forces: Vec<(u64, f64)> = ordered
        .iter()
        .filter_map(|snapshot| {
            snapshot
                .momentum
                .force
                .map(|value| (snapshot.timeframe_secs, value))
        })
        .collect();
    let force_values: Vec<f64> = forces.iter().map(|(_, value)| *value).collect();
    let accelerations: Vec<f64> = ordered
        .iter()
        .filter_map(|snapshot| snapshot.momentum.acceleration)
        .collect();
    let positive = force_values.iter().filter(|value| **value > 0.1).count();
    let negative = force_values.iter().filter(|value| **value < -0.1).count();
    let ready_count = ordered
        .iter()
        .filter(|snapshot| snapshot.momentum.readiness == Readiness::Ready)
        .count();
    let force_fast_slow = endpoint_gap(ordered, |snapshot| snapshot.momentum.force);
    let acceleration_fast_slow = endpoint_gap(ordered, |snapshot| snapshot.momentum.acceleration);

    MomentumAggregates {
        timeframes: forces.iter().map(|(timeframe, _)| *timeframe).collect(),
        momentum_timeframe_count: ordered.len(),
        momentum_force_mean: mean(&force_values),
        momentum_acceleration_mean: mean(&accelerations),
        momentum_positive_share: share(positive, force_values.len()),
        momentum_negative_share: share(negative, force_values.len()),
        momentum_force_fast_slow_gap: force_fast_slow,
        momentum_acceleration_fast_slow_gap: acceleration_fast_slow,
        momentum_gradient: force_fast_slow.map(|gap| -gap),
        momentum_ready_count: ready_count,
    }
}

fn structure_aggregates(
    ordered: &[&TimeframeSnapshot],
    levels: LevelClusterAggregates,
) -> StructureAggregates {
    let valid: Vec<(u64, f64)> = ordered
        .iter()
        .filter_map(|snapshot| {
            snapshot
                .structure
                .score
                .map(|value| (snapshot.timeframe_secs, value))
        })
        .collect();
    let values: Vec<f64> = valid.iter().map(|(_, value)| *value).collect();
    let bullish = values.iter().filter(|value| **value > 0.1).count();
    let bearish = values.iter().filter(|value| **value < -0.1).count();
    let neutral = values.iter().filter(|value| value.abs() <= 0.1).count();
    let mut histogram = [0_u32; 5];
    let mut minimum: Option<i8> = None;
    let mut maximum: Option<i8> = None;
    for snapshot in ordered {
        if let Some(code) = snapshot.structure.state_code
            && (-2..=2).contains(&code)
        {
            histogram[(code + 2) as usize] += 1;
            minimum = Some(minimum.map_or(code, |current| current.min(code)));
            maximum = Some(maximum.map_or(code, |current| current.max(code)));
        }
    }
    let ready_count = ordered
        .iter()
        .filter(|snapshot| snapshot.structure.readiness == Readiness::Ready)
        .count();
    let conflicts = ordered
        .iter()
        .filter(|snapshot| {
            matches!(
                (snapshot.structure.high_relation_code, snapshot.structure.low_relation_code),
                (Some(high), Some(low)) if high * low < 0
            )
        })
        .count();

    StructureAggregates {
        timeframes: valid.iter().map(|(timeframe, _)| *timeframe).collect(),
        structure_timeframe_count: ordered.len(),
        structure_code_histogram: histogram,
        structure_bullish_share: share(bullish, values.len()),
        structure_bearish_share: share(bearish, values.len()),
        structure_neutral_share: share(neutral, values.len()),
        structure_state_span: minimum
            .zip(maximum)
            .map(|(minimum, maximum)| (maximum - minimum) as u8),
        structure_gradient: gradient(ordered, |snapshot| snapshot.structure.score),
        structure_ready_share: share(ready_count, ordered.len()),
        structure_conflict_timeframe_count: conflicts,
        level_cluster_count: levels.level_cluster_count,
        largest_level_cluster_timeframe_count: levels.largest_level_cluster_timeframe_count,
        largest_level_cluster_span_atr: levels.largest_level_cluster_span_atr,
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        let sum: f64 = values.iter().sum();
        Some(sum / values.len() as f64)
    }
}

fn share(count: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(count as f64 / total as f64)
    }
}

fn gradient(
    ordered: &[&TimeframeSnapshot],
    value: impl Fn(&TimeframeSnapshot) -> Option<f64>,
) -> Option<f64> {
    let fastest = value(ordered.first()?)?;
    let slowest = value(ordered.last()?)?;
    Some(slowest - fastest)
}

fn endpoint_gap(
    ordered: &[&TimeframeSnapshot],
    value: impl Fn(&TimeframeSnapshot) -> Option<f64>,
) -> Option<f64> {
    let fastest = value(ordered.first()?)?;
    let slowest = value(ordered.last()?)?;
    Some(fastest - slowest)
}

#[cfg(test)]
mod tests;
