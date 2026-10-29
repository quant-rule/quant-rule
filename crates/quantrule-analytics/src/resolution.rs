//! Per-axis resolution: a continuous value in `[-1, +1]` and an ordinal bucket
//! in `[0, 9]`.
//!
//! The mapping is the research output contract, §6. The continuous value is the
//! axis's primary measurement, except Progression, whose unclamped progress is
//! clamped for comparability while the raw value stays in the per-bar metrics.
//! The ordinal is an equal-width bucket of the continuous value and is counted,
//! never averaged.
//!
//! The module is pure and total: every input produces a record, and a missing
//! axis stays null with the axis's own unavailable-reason code.

use quantrule_model::axes::direction::DirectionBar;
use quantrule_model::axes::momentum::MomentumBar;
use quantrule_model::axes::progression::ProgressionBar;
use quantrule_model::axes::structure::StructureBar;
use quantrule_model::kernel::UnavailableReason;

/// The nine interior bucket boundaries, in ascending order.
///
/// The mapping compares the value against these constants instead of
/// recomputing `-1 + k / 5`. A division would classify `-0.8` as bucket 0
/// through floating-point error, and the contract's left-closed rule requires
/// bucket 1.
const BOUNDARIES: [f64; 9] = [-0.8, -0.6, -0.4, -0.2, 0.0, 0.2, 0.4, 0.6, 0.8];

/// The resolution of one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisResolution {
    /// The continuous value in `[-1, +1]`, or `None` when the axis is unavailable.
    pub continuous: Option<f64>,
    /// The equal-width ordinal bucket in `[0, 9]`, or `None`.
    pub ordinal: Option<u8>,
    /// The axis's unavailable-reason code when the continuous value is null.
    pub reason: Option<u8>,
}

impl AxisResolution {
    /// Builds a resolution from an already clamped value and a reason.
    ///
    /// `continuous` must already be in `[-1, +1]`; the constructor does not
    /// clamp it, because silently clamping a published axis value would hide a
    /// model defect. Use [`clamp_unit`] for a raw value such as Progression's
    /// `progress`. When the value is present the reason is discarded, so a
    /// measured value never carries a reason.
    #[must_use]
    pub fn new(continuous: Option<f64>, reason: Option<u8>) -> Self {
        Self {
            continuous,
            ordinal: continuous.map(ordinal_bucket),
            reason: if continuous.is_some() { None } else { reason },
        }
    }

    /// A resolution with no value and the reason it is missing.
    #[must_use]
    pub const fn unavailable(reason: Option<u8>) -> Self {
        Self {
            continuous: None,
            ordinal: None,
            reason,
        }
    }
}

/// The four axis resolutions for one bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisResolutions {
    /// Direction.
    pub direction: AxisResolution,
    /// Momentum.
    pub momentum: AxisResolution,
    /// Structure.
    pub structure: AxisResolution,
    /// Progression.
    pub progression: AxisResolution,
}

impl AxisResolutions {
    /// Builds the four resolutions from the model's published axis bars.
    ///
    /// Progression's raw `progress` is clamped to `[-1, +1]` for the resolution;
    /// the unclamped value remains on the model's bar and in Layer 1. The other
    /// axis values are published in range by the model and are used as they
    /// are.
    #[must_use]
    pub fn from_bars(
        direction: &DirectionBar,
        momentum: &MomentumBar,
        structure: &StructureBar,
        progression: &ProgressionBar,
    ) -> Self {
        Self {
            direction: AxisResolution::new(
                direction.direction,
                reason_code(direction.unavailable_reason),
            ),
            momentum: AxisResolution::new(momentum.force, reason_code(momentum.unavailable_reason)),
            structure: AxisResolution::new(
                structure.score,
                reason_code(structure.unavailable_reason),
            ),
            progression: AxisResolution::new(
                progression.progress.map(clamp_unit),
                reason_code(progression.unavailable_reason),
            ),
        }
    }
}

/// The equal-width bucket of a continuous value in `[-1, +1]`.
///
/// The bucket is the count of the nine interior boundaries that are less than
/// or equal to the value, so buckets are closed on the left and open on the
/// right. The last bucket is closed on both ends. A value outside the range is
/// clamped first, which keeps the function total: `-2.0` is bucket 0 and `2.0`
/// is bucket 9. `-1.0` is 0, `-0.8` is 1, `0.0` is 5, `+0.8` is 9, and `+1.0`
/// is 9.
#[must_use]
pub fn ordinal_bucket(value: f64) -> u8 {
    let clamped = clamp_unit(value);
    BOUNDARIES
        .iter()
        .filter(|boundary| clamped >= **boundary)
        .count() as u8
}

/// Clamps a value to `[-1, +1]`.
///
/// The model publishes finite values or nulls; a non-finite value is outside
/// the contract and the export rejects it. This function stays total for any
/// `f64`: `NaN` maps to zero, and the infinities clamp to the ends.
#[must_use]
pub fn clamp_unit(value: f64) -> f64 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(-1.0, 1.0)
    }
}

fn reason_code(reason: Option<UnavailableReason>) -> Option<u8> {
    reason.map(UnavailableReason::code)
}

#[cfg(test)]
mod tests {
    use quantrule_model::axes::direction::DirectionBar;
    use quantrule_model::axes::momentum::MomentumBar;
    use quantrule_model::axes::progression::ProgressionBar;
    use quantrule_model::axes::structure::StructureBar;
    use quantrule_model::kernel::{Readiness, UnavailableReason};

    use super::{AxisResolution, AxisResolutions, clamp_unit, ordinal_bucket};

    #[test]
    fn the_contract_boundaries_map_to_the_contract_buckets() {
        assert_eq!(ordinal_bucket(-1.0), 0);
        assert_eq!(ordinal_bucket(-0.8), 1);
        assert_eq!(ordinal_bucket(-0.79), 1);
        assert_eq!(ordinal_bucket(-0.2), 4);
        assert_eq!(ordinal_bucket(-0.0), 5);
        assert_eq!(ordinal_bucket(0.0), 5);
        assert_eq!(ordinal_bucket(0.199), 5);
        assert_eq!(ordinal_bucket(0.2), 6);
        assert_eq!(ordinal_bucket(0.8), 9);
        assert_eq!(ordinal_bucket(1.0), 9);
    }

    #[test]
    fn every_bucket_is_reachable() {
        let samples = [-0.9, -0.7, -0.5, -0.3, -0.1, 0.1, 0.3, 0.5, 0.7, 0.9];
        for (expected, sample) in samples.into_iter().enumerate() {
            assert_eq!(ordinal_bucket(sample), expected as u8);
        }
    }

    #[test]
    fn a_value_below_a_boundary_stays_in_the_lower_bucket() {
        assert_eq!(ordinal_bucket(-0.800_000_1), 0);
        assert_eq!(ordinal_bucket(-0.199_999_9), 4);
        assert_eq!(ordinal_bucket(0.799_999_9), 8);
    }

    #[test]
    fn out_of_range_values_are_clamped() {
        assert_eq!(ordinal_bucket(-2.0), 0);
        assert_eq!(ordinal_bucket(2.0), 9);
    }

    #[test]
    fn the_continuous_value_is_clamped() {
        assert_eq!(clamp_unit(-2.0), -1.0);
        assert_eq!(clamp_unit(2.0), 1.0);
        assert_eq!(clamp_unit(0.25), 0.25);
        assert_eq!(clamp_unit(f64::NAN), 0.0);
        assert_eq!(clamp_unit(f64::INFINITY), 1.0);
        assert_eq!(clamp_unit(f64::NEG_INFINITY), -1.0);
    }

    #[test]
    fn a_missing_value_is_null_with_its_reason() {
        let resolution = AxisResolution::unavailable(Some(1));
        assert_eq!(resolution.continuous, None);
        assert_eq!(resolution.ordinal, None);
        assert_eq!(resolution.reason, Some(1));
    }

    #[test]
    fn a_measured_value_has_no_reason() {
        let resolution = AxisResolution::new(Some(0.0), Some(1));
        assert_eq!(resolution.continuous, Some(0.0));
        assert_eq!(resolution.ordinal, Some(5));
        assert_eq!(resolution.reason, None);
    }

    #[test]
    fn each_axis_resolves_from_its_published_value() {
        let resolutions = AxisResolutions::from_bars(
            &direction_bar(Some(0.5), None),
            &momentum_bar(Some(-0.5), None),
            &structure_bar(Some(0.0), None),
            &progression_bar(Some(1.5), None),
        );

        assert_eq!(
            resolutions.direction,
            AxisResolution {
                continuous: Some(0.5),
                ordinal: Some(7),
                reason: None,
            }
        );
        assert_eq!(resolutions.momentum.ordinal, Some(2));
        assert_eq!(resolutions.structure.ordinal, Some(5));
        assert_eq!(
            resolutions.progression,
            AxisResolution {
                continuous: Some(1.0),
                ordinal: Some(9),
                reason: None,
            }
        );
    }

    #[test]
    fn a_raw_negative_progress_is_clamped_for_the_resolution() {
        let resolutions = AxisResolutions::from_bars(
            &direction_bar(None, Some(UnavailableReason::InsufficientBars)),
            &momentum_bar(None, Some(UnavailableReason::InsufficientBars)),
            &structure_bar(None, Some(UnavailableReason::InsufficientConfirmedPivots)),
            &progression_bar(Some(-1.5), None),
        );

        assert_eq!(resolutions.progression.continuous, Some(-1.0));
        assert_eq!(resolutions.progression.ordinal, Some(0));
        assert_eq!(resolutions.progression.reason, None);
    }

    #[test]
    fn an_unavailable_axis_stays_null_with_its_own_reason() {
        let resolutions = AxisResolutions::from_bars(
            &direction_bar(None, Some(UnavailableReason::InsufficientBars)),
            &momentum_bar(None, Some(UnavailableReason::InvalidDenominator)),
            &structure_bar(None, Some(UnavailableReason::InsufficientConfirmedPivots)),
            &progression_bar(None, Some(UnavailableReason::OriginUnconfirmed)),
        );

        assert_eq!(resolutions.direction, AxisResolution::unavailable(Some(1)));
        assert_eq!(resolutions.momentum, AxisResolution::unavailable(Some(3)));
        assert_eq!(resolutions.structure, AxisResolution::unavailable(Some(2)));
        assert_eq!(
            resolutions.progression,
            AxisResolution::unavailable(Some(6))
        );
    }

    fn direction_bar(value: Option<f64>, reason: Option<UnavailableReason>) -> DirectionBar {
        DirectionBar {
            direction: value,
            state_code: None,
            size_percentile: None,
            readiness: Readiness::Ready,
            unavailable_reason: reason,
            state_age_bars: 0,
            state_changes_50: 0,
        }
    }

    fn momentum_bar(value: Option<f64>, reason: Option<UnavailableReason>) -> MomentumBar {
        MomentumBar {
            force: value,
            acceleration: None,
            intensification: None,
            state_code: None,
            size_percentile: None,
            readiness: Readiness::Ready,
            unavailable_reason: reason,
            state_age_bars: 0,
            state_changes_50: 0,
        }
    }

    fn structure_bar(value: Option<f64>, reason: Option<UnavailableReason>) -> StructureBar {
        StructureBar {
            high_relation_code: None,
            low_relation_code: None,
            score: value,
            state_code: None,
            scale_code: 2,
            ema_distance_atr_20: None,
            ema_distance_atr_50: None,
            ema_distance_atr_200: None,
            ema_cross_code_20: None,
            ema_cross_code_50: None,
            ema_cross_code_200: None,
            ema_retest_code_20: None,
            ema_retest_code_50: None,
            ema_retest_code_200: None,
            size_percentile: None,
            readiness: Readiness::Ready,
            unavailable_reason: reason,
            state_age_bars: 0,
            state_changes_50: 0,
        }
    }

    fn progression_bar(value: Option<f64>, reason: Option<UnavailableReason>) -> ProgressionBar {
        ProgressionBar {
            progress: value,
            stage_code: None,
            leg_direction: None,
            leg_scale_code: None,
            published_leg: None,
            minor_leg: None,
            major_leg: None,
            size_percentile: None,
            readiness: Readiness::Ready,
            unavailable_reason: reason,
            state_age_bars: 0,
            state_changes_50: 0,
        }
    }
}
