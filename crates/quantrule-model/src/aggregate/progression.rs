//! Progression aggregates across timeframes.
//!
//! Stage histograms and their derived statistics are computed per scale, so a
//! minor counter-move is never blended into the major stage distribution. The
//! cross-scale measures count active legs and compare legs for a same-scale
//! match. Stage codes are counted, never averaged.

use super::TimeframeSnapshot;
use crate::axes::progression::ProgressionLeg;
use crate::kernel::Readiness;

/// Stage distribution and derived statistics for one scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleStageAggregates {
    /// Counts for stages 0..5.
    pub stage_histogram: [u32; 6],
    /// Most frequent stage; the lowest stage wins a tie.
    pub stage_mode: Option<u8>,
    /// Minimum observed stage.
    pub stage_min: Option<u8>,
    /// Maximum observed stage.
    pub stage_max: Option<u8>,
    /// Maximum minus minimum.
    pub stage_span: Option<u8>,
    /// Share of valid legs at the modal stage.
    pub modal_stage_share: Option<f64>,
}

/// Progression aggregates across the ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressionAggregates {
    /// Timeframes with a retained leg.
    pub timeframes: Vec<u64>,
    /// The number of configured timeframes.
    pub progression_timeframe_count: usize,
    /// Minor-scale stage aggregates.
    pub minor: ScaleStageAggregates,
    /// Major-scale stage aggregates.
    pub major: ScaleStageAggregates,
    /// Active major legs across the ladder.
    pub active_major_leg_count: u32,
    /// Active minor legs across the ladder.
    pub active_minor_leg_count: u32,
    /// Share of timeframes whose published Progression readiness is ready.
    pub progression_ready_share: Option<f64>,
    /// Share of active legs pointing in the most common direction.
    pub leg_direction_agreement_share: Option<f64>,
    /// Pairs of active legs that match across timeframes.
    pub cross_timeframe_leg_match_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct LegPoint {
    scale_code: u8,
    direction_code: i8,
    origin_time_secs: i64,
    origin_price: f64,
    atr: f64,
    timeframe_secs: u64,
}

/// Builds the progression aggregates.
pub(crate) fn aggregate(ordered: &[&TimeframeSnapshot]) -> ProgressionAggregates {
    let mut minor_stages = Vec::new();
    let mut major_stages = Vec::new();
    let mut active_major = 0_u32;
    let mut active_minor = 0_u32;
    let mut ready_count = 0_usize;
    let mut directions = [0_u32; 2];
    let mut legs = Vec::new();
    let mut timeframes = Vec::new();

    for snapshot in ordered {
        let progression = &snapshot.progression;
        if progression.minor_leg.is_some() || progression.major_leg.is_some() {
            timeframes.push(snapshot.timeframe_secs);
        }
        if progression.readiness == Readiness::Ready {
            ready_count += 1;
        }

        if let Some(stage) = stage_of(progression.minor_leg) {
            minor_stages.push(stage);
        }
        if let Some(stage) = stage_of(progression.major_leg) {
            major_stages.push(stage);
        }

        if progression
            .minor_leg
            .is_some_and(|leg| leg.lifecycle_code == 1)
        {
            active_minor += 1;
        }
        if progression
            .major_leg
            .is_some_and(|leg| leg.lifecycle_code == 1)
        {
            active_major += 1;
        }

        for leg in [progression.minor_leg, progression.major_leg]
            .into_iter()
            .flatten()
        {
            if leg.lifecycle_code != 1 || !matches!(leg.direction_code, -1 | 1) {
                continue;
            }
            directions[(leg.direction_code + 1) as usize / 2] += 1;
        }

        for (leg, origin_time) in [
            (progression.minor_leg, snapshot.minor_origin_time_secs),
            (progression.major_leg, snapshot.major_origin_time_secs),
        ] {
            let (Some(leg), Some(origin_time)) = (leg, origin_time) else {
                continue;
            };
            if leg.lifecycle_code != 1 || !matches!(leg.direction_code, -1 | 1) {
                continue;
            }
            if let Some(atr) = snapshot.atr.filter(|atr| *atr > 0.0) {
                legs.push(LegPoint {
                    scale_code: leg.scale_code,
                    direction_code: leg.direction_code,
                    origin_time_secs: origin_time,
                    origin_price: leg.origin_price,
                    atr,
                    timeframe_secs: snapshot.timeframe_secs,
                });
            }
        }
    }

    let direction_total: u32 = directions.iter().sum();
    let leg_direction_agreement_share = if direction_total == 0 {
        None
    } else {
        let maximum = directions.iter().copied().max().unwrap_or(0);
        Some(f64::from(maximum) / f64::from(direction_total))
    };

    ProgressionAggregates {
        timeframes,
        progression_timeframe_count: ordered.len(),
        minor: scale_stage_aggregates(&minor_stages),
        major: scale_stage_aggregates(&major_stages),
        active_major_leg_count: active_major,
        active_minor_leg_count: active_minor,
        progression_ready_share: share(ready_count, ordered.len()),
        leg_direction_agreement_share,
        cross_timeframe_leg_match_count: leg_matches(&legs),
    }
}

fn stage_of(leg: Option<ProgressionLeg>) -> Option<u8> {
    let stage = leg?.stage_code?;
    if (0..=5).contains(&stage) {
        Some(stage as u8)
    } else {
        None
    }
}

fn scale_stage_aggregates(stages: &[u8]) -> ScaleStageAggregates {
    let mut histogram = [0_u32; 6];
    for stage in stages {
        histogram[*stage as usize] += 1;
    }
    let valid = stages.len();
    let mut mode = None;
    let mut mode_count = 0_u32;
    let mut minimum = None;
    let mut maximum = None;
    for (stage, count) in histogram.iter().enumerate() {
        if *count > 0 {
            if minimum.is_none() {
                minimum = Some(stage as u8);
            }
            maximum = Some(stage as u8);
            if *count > mode_count {
                mode_count = *count;
                mode = Some(stage as u8);
            }
        }
    }
    let span = minimum
        .zip(maximum)
        .map(|(minimum, maximum)| maximum - minimum);
    let modal_stage_share = if valid == 0 {
        None
    } else {
        Some(f64::from(mode_count) / valid as f64)
    };

    ScaleStageAggregates {
        stage_histogram: histogram,
        stage_mode: mode,
        stage_min: minimum,
        stage_max: maximum,
        stage_span: span,
        modal_stage_share,
    }
}

fn leg_matches(legs: &[LegPoint]) -> u32 {
    let mut matches = 0_u32;
    for left in 0..legs.len() {
        for right in (left + 1)..legs.len() {
            let first = legs[left];
            let second = legs[right];
            if first.scale_code != second.scale_code
                || first.direction_code != second.direction_code
            {
                continue;
            }
            let slower = first.timeframe_secs.max(second.timeframe_secs) as i64;
            let origin_gap = (first.origin_time_secs - second.origin_time_secs).abs();
            let price_gap = (first.origin_price - second.origin_price).abs();
            let tolerance = 0.5 * first.atr.max(second.atr);
            if origin_gap <= slower && price_gap <= tolerance {
                matches += 1;
            }
        }
    }
    matches
}

fn share(count: usize, total: usize) -> Option<f64> {
    if total == 0 {
        None
    } else {
        Some(count as f64 / total as f64)
    }
}
