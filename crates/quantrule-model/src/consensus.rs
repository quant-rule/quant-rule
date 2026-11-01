//! The cross-axis consensus diagnostic.
//!
//! Direction, Momentum, and Structure vote; Progression is excluded from the
//! directional consensus. The type is separate from the market-state view so
//! the run orchestration and the view can both use it without a module cycle.

use crate::aggregate::TimeframeSnapshot;
use crate::axes::direction::DirectionBar;
use crate::axes::momentum::MomentumBar;
use crate::axes::structure::StructureBar;

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
    /// missing or non-finite value is not counted as neutral. The weakest
    /// directional axis is the lowest valid finite size percentile, with the
    /// lower axis code breaking a tie.
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
        for value in values
            .into_iter()
            .flatten()
            .filter(|value| value.is_finite())
        {
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
            let Some(percentile) = percentile.filter(|value| value.is_finite()) else {
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
