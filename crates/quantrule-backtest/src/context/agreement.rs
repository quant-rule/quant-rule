//! The priority agreement across the higher-timeframe context.
//!
//! The agreement is a strategy-owned as-of view. It does not average the four
//! axes; it applies the approved priority: the 15m regime sets Direction and
//! Progression, the 5m confirmation may be neutral but not opposing; Structure
//! has no conflict and no opposing reading; Momentum does not oppose. The view
//! also carries the number of consecutive execution bars the agreement has held.

use super::{AxisContext, ProgressionContext, StructureContext};

/// The priority agreement for a long and a short entry.
///
/// `long_bars` and `short_bars` count consecutive execution bars on which the
/// agreement held. A missing context resets both to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AgreementContext {
    /// Both higher timeframes agree on the long priority.
    pub long: bool,
    /// Both higher timeframes agree on the short priority.
    pub short: bool,
    /// Consecutive execution bars the long agreement has held.
    pub long_bars: u64,
    /// Consecutive execution bars the short agreement has held.
    pub short_bars: u64,
}

impl AgreementContext {
    /// The unavailable agreement: no side and no age.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            long: false,
            short: false,
            long_bars: 0,
            short_bars: 0,
        }
    }
}

/// Applies the approved priority to one closed context.
///
/// A valid reading on both higher timeframes is required. Direction and
/// Progression use the weighted shares, so a 15m reading plus a neutral 5m
/// confirmation can pass while a 5m opposition cannot. Structure requires no
/// conflict and no opposing score; Momentum allows neutral but not an opposing
/// reading. The weights are the strategy's per-axis, per-timeframe weights.
#[must_use]
pub fn priority_sides(
    direction: &AxisContext,
    momentum: &AxisContext,
    structure: &StructureContext,
    progression: &ProgressionContext,
) -> (bool, bool) {
    let long = direction.bullish_count == 2
        && direction.bearish_count == 0
        && progression.direction.bullish_count == 2
        && progression.direction.bearish_count == 0
        && structure.conflict_count == 0
        && structure.score.bearish_count == 0
        && momentum.bearish_count == 0;
    let short = direction.bearish_count == 2
        && direction.bullish_count == 0
        && progression.direction.bearish_count == 2
        && progression.direction.bullish_count == 0
        && structure.conflict_count == 0
        && structure.score.bullish_count == 0
        && momentum.bullish_count == 0;
    (long, short)
}

#[cfg(test)]
mod tests {
    use super::{AgreementContext, priority_sides};
    use crate::context::{AgreementCode, AxisContext, ProgressionContext, StructureContext};

    fn axis(bullish: u32, bearish: u32, neutral: u32) -> AxisContext {
        let total = f64::from(bullish + bearish + neutral);
        AxisContext {
            timeframes: vec![300, 900],
            bullish_count: bullish,
            bearish_count: bearish,
            neutral_count: neutral,
            weighted_value: Some(0.0),
            bullish_share: Some(if total > 0.0 {
                f64::from(bullish) / total
            } else {
                0.0
            }),
            bearish_share: Some(if total > 0.0 {
                f64::from(bearish) / total
            } else {
                0.0
            }),
            neutral_share: Some(if total > 0.0 {
                f64::from(neutral) / total
            } else {
                1.0
            }),
            agreement: AgreementCode::Mixed,
        }
    }

    fn progression(bullish: u32, bearish: u32) -> ProgressionContext {
        ProgressionContext {
            direction: axis(bullish, bearish, 2 - bullish - bearish),
            stage_histogram: [0.0; 6],
            late_or_terminal_count: 0,
        }
    }

    fn structure(bullish: u32, bearish: u32, conflict: u32) -> StructureContext {
        StructureContext {
            score: axis(bullish, bearish, 2 - bullish - bearish),
            conflict_count: conflict,
        }
    }

    #[test]
    fn both_directions_and_progressions_agree_long() {
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 0, 1),
                &structure(1, 0, 0),
                &progression(2, 0),
            ),
            (true, false)
        );
    }

    #[test]
    fn one_neutral_direction_or_progression_blocks_the_long() {
        assert_eq!(
            priority_sides(
                &axis(1, 0, 1),
                &axis(1, 0, 1),
                &structure(1, 0, 0),
                &progression(2, 0),
            ),
            (false, false)
        );
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 0, 1),
                &structure(1, 0, 0),
                &progression(1, 0),
            ),
            (false, false)
        );
    }

    #[test]
    fn a_neutral_structure_is_not_opposing() {
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 0, 1),
                &structure(0, 0, 0),
                &progression(2, 0),
            ),
            (true, false)
        );
    }

    #[test]
    fn an_opposing_structure_or_momentum_blocks_the_long() {
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 0, 1),
                &structure(1, 1, 0),
                &progression(2, 0),
            ),
            (false, false)
        );
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 1, 0),
                &structure(1, 0, 0),
                &progression(2, 0),
            ),
            (false, false)
        );
        assert_eq!(
            priority_sides(
                &axis(2, 0, 0),
                &axis(1, 0, 1),
                &structure(1, 0, 1),
                &progression(2, 0),
            ),
            (false, false)
        );
    }

    #[test]
    fn the_short_mirrors_the_long() {
        assert_eq!(
            priority_sides(
                &axis(0, 2, 0),
                &axis(0, 1, 1),
                &structure(0, 1, 0),
                &progression(0, 2),
            ),
            (false, true)
        );
    }

    #[test]
    fn the_unavailable_context_has_no_age() {
        assert_eq!(AgreementContext::unavailable(), AgreementContext::default());
    }
}
