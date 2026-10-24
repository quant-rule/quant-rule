//! The Momentum axis.
//!
//! Momentum measures RSI 14 as a signed force, the change in that force over
//! exactly three bars, and the signed change in the force magnitude over the
//! same window. It publishes the three continuous values, the compact state
//! code, the size percentile, readiness, the unavailable reason, and the state
//! counters as one record per input bar.
//!
//! The axis reads only `rsi14`. It never reads an EMA, ATR, RVOL, pivot, or
//! another axis's output, so a correlated measurement cannot become a second
//! vote. The module is pure: it performs no I/O, reads no clock, and uses no
//! randomness.

use crate::kernel::{
    PERCENTILE_WINDOW, Readiness, StateCounters, UnavailableReason, clamp_unit, compact_state,
    empirical_percentile, sign,
};
use crate::readings::Readings;

/// The published Momentum values for one closed bar.
///
/// `force` is the primary value. `acceleration` and `intensification` are
/// `None` when the force exactly three bars earlier is unavailable, while
/// `force` itself remains published. A `force` of `Some(0.0)` is a measured
/// neutral force, not missing data. The state counters freeze while
/// `state_code` is `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MomentumBar {
    /// Signed RSI force in `[-1, +1]`, or `None` when RSI 14 is unavailable.
    pub force: Option<f64>,
    /// Signed change in force over exactly three bars, clamped to `[-1, +1]`,
    /// or `None` when either required force is unavailable.
    pub acceleration: Option<f64>,
    /// Signed change in the force magnitude over exactly three bars, clamped
    /// to `[-1, +1]`, or `None` when either required force is unavailable.
    pub intensification: Option<f64>,
    /// Compact state of `force`: -1 below -0.1, 0 through the inclusive ±0.1
    /// band, +1 above +0.1; `None` when force is unavailable.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute force magnitude, in `0..=100`,
    /// or `None` when fewer than 50 valid prior samples exist.
    pub size_percentile: Option<f64>,
    /// Mechanical completeness: unavailable when force is null, provisional
    /// until the percentile has 50 valid prior samples, ready thereafter. A
    /// null change measure is a null output, not a readiness downgrade.
    pub readiness: Readiness,
    /// Why Momentum is unavailable, or `None` when force is available.
    pub unavailable_reason: Option<UnavailableReason>,
    /// Closed bars the compact state has held; zero before the first code.
    pub state_age_bars: u64,
    /// Compact state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The per-bar Momentum series.
///
/// The series has exactly one [`MomentumBar`] per input bar. Its length always
/// equals the input candle count, so a consumer indexes it by bar position.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::axes::momentum::MomentumSeries;
/// use quantrule_model::kernel::Readiness;
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let momentum = MomentumSeries::compute(&readings);
///
/// assert_eq!(momentum.len(), 1);
/// let bar = momentum.get(0).expect("one bar");
/// assert_eq!(bar.force, None);
/// assert_eq!(bar.readiness, Readiness::Unavailable);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MomentumSeries {
    bars: Vec<MomentumBar>,
}

impl MomentumSeries {
    /// Computes Momentum for every bar of the readings.
    ///
    /// The result is aligned to the input length. Each bar is a pure function
    /// of the RSI 14 value at that bar, the RSI 14 value exactly three bars
    /// earlier, and the valid force magnitudes strictly before it; no later
    /// bar is read.
    #[must_use]
    pub fn compute(readings: &Readings) -> Self {
        let mut bars = Vec::with_capacity(readings.len());
        let mut counters = StateCounters::new();
        // Absolute valid force magnitudes in bar order. The current bar is
        // appended only after its percentile is computed, so a bar never
        // samples itself. The kernel never reads beyond 252 samples, so the
        // buffer is sized to that bound rather than to the input length.
        let mut magnitudes = Vec::with_capacity(PERCENTILE_WINDOW);

        for bar in 0..readings.len() {
            match MomentumInputs::from_readings(readings, bar) {
                Ok(inputs) => {
                    let values = inputs.score();
                    let magnitude = values.force.abs();
                    let size_percentile = empirical_percentile(&magnitudes, magnitude);
                    let state_code = compact_state(values.force);
                    let stats = counters.observe(state_code);
                    // §9 names the percentile sample count as the momentum
                    // completeness gate. A null change measure is a null
                    // output, not an incomplete axis.
                    let readiness = if size_percentile.is_some() {
                        Readiness::Ready
                    } else {
                        Readiness::Provisional
                    };

                    bars.push(MomentumBar {
                        force: Some(values.force),
                        acceleration: values.acceleration,
                        intensification: values.intensification,
                        state_code,
                        size_percentile,
                        readiness,
                        unavailable_reason: None,
                        state_age_bars: stats.age_bars,
                        state_changes_50: stats.changes_50,
                    });
                    // The percentile never looks past 252 samples, so retaining
                    // more would only grow memory with the input length.
                    if magnitudes.len() == PERCENTILE_WINDOW {
                        magnitudes.remove(0);
                    }
                    magnitudes.push(magnitude);
                }
                Err(reason) => {
                    let stats = counters.observe(None);
                    bars.push(MomentumBar {
                        force: None,
                        acceleration: None,
                        intensification: None,
                        state_code: None,
                        size_percentile: None,
                        readiness: Readiness::Unavailable,
                        unavailable_reason: Some(reason),
                        state_age_bars: stats.age_bars,
                        state_changes_50: stats.changes_50,
                    });
                }
            }
        }

        Self { bars }
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// The record at `bar`, or `None` when the bar is out of range.
    #[must_use]
    pub fn get(&self, bar: usize) -> Option<MomentumBar> {
        self.bars.get(bar).copied()
    }

    /// Every bar record, in bar order.
    #[must_use]
    pub fn bars(&self) -> &[MomentumBar] {
        &self.bars
    }

    /// The latest bar record, or `None` for an empty series.
    #[must_use]
    pub fn latest(&self) -> Option<MomentumBar> {
        self.bars.last().copied()
    }
}

/// The readings Momentum is allowed to consume.
///
/// Keeping the extraction narrow makes the no-double-count invariant
/// structural: a later reading cannot enter the axis without changing this
/// type. The force is resolved once per bar, before the formulas run.
struct MomentumInputs {
    force: f64,
    previous_force: Option<f64>,
}

impl MomentumInputs {
    fn from_readings(readings: &Readings, bar: usize) -> Result<Self, UnavailableReason> {
        let force = force_at(readings, bar)?;
        // The comparison is the bar exactly three positions earlier, not the
        // third previous valid sample. A missing value there nulls only the
        // two change measures; the current force stays published.
        let previous_force = bar
            .checked_sub(3)
            .and_then(|previous| force_at(readings, previous).ok());

        Ok(Self {
            force,
            previous_force,
        })
    }

    /// Applies the contract's §8.2 formulas.
    fn score(self) -> MomentumValues {
        let (acceleration, intensification) = match self.previous_force {
            Some(previous) => (
                Some(clamp_unit((self.force - previous) / 0.5)),
                Some(clamp_unit(
                    sign(self.force) * (self.force.abs() - previous.abs()) / 0.5,
                )),
            ),
            None => (None, None),
        };

        MomentumValues {
            force: self.force,
            acceleration,
            intensification,
        }
    }
}

/// The three continuous values one bar publishes.
struct MomentumValues {
    force: f64,
    acceleration: Option<f64>,
    intensification: Option<f64>,
}

/// Resolves the force at one bar from RSI 14.
///
/// RSI 14 is bounded to `[0, 100]` by the reading layer, so the force is
/// bounded to `[-1, +1]` without a clamp. A non-finite or out-of-range RSI is
/// treated as an invalid reading rather than clamped into a valid extreme;
/// reason 1 is the closest registry code for a value that cannot be used.
fn force_at(readings: &Readings, bar: usize) -> Result<f64, UnavailableReason> {
    let rsi = readings
        .rsi14
        .get(bar)
        .filter(|value| value.is_finite())
        .ok_or(UnavailableReason::InsufficientBars)?;

    if !(0.0..=100.0).contains(&rsi) {
        return Err(UnavailableReason::InsufficientBars);
    }

    Ok((rsi - 50.0) / 50.0)
}

#[cfg(test)]
mod tests {
    use super::MomentumSeries;
    use crate::kernel::{Readiness, UnavailableReason};
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    /// Builds readings whose RSI 14 values are the supplied values.
    ///
    /// Every other reading is deliberately empty or null: Momentum must not
    /// read it.
    fn readings_for_rsi(rsi: &[Option<f64>]) -> Readings {
        let len = rsi.len();
        Readings {
            ema20: ReadingSeries::new(vec![None; len]),
            ema50: ReadingSeries::new(vec![None; len]),
            ema200: ReadingSeries::new(vec![None; len]),
            rsi14: ReadingSeries::new(rsi.to_vec()),
            atr14: ReadingSeries::new(vec![None; len]),
            rvol20: ReadingSeries::new(vec![None; len]),
            pivots: PivotSeries::compute(&[]),
        }
    }

    fn one_bar(rsi: Option<f64>) -> Readings {
        readings_for_rsi(&[rsi])
    }

    #[test]
    fn force_maps_rsi_to_the_contract_range() {
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(0.0),
            Some(25.0),
            Some(50.0),
            Some(75.0),
            Some(100.0),
        ]));

        let expected = [-1.0, -0.5, 0.0, 0.5, 1.0];
        for (bar, value) in expected.into_iter().enumerate() {
            let record = series.get(bar).expect("a bar");
            assert_close(record.force.expect("a force"), value);
            assert!(record.force.expect("a force").is_finite());
        }
    }

    #[test]
    fn acceleration_uses_exactly_the_three_bar_window() {
        // t-3 is bar 1 (RSI 50, force 0), while t-4 is bar 0 (RSI 80,
        // force 0.6), t-2 is bar 2 (force 0.8), and t-1 is bar 3 (force -0.8).
        // Only the t-3 value gives 0.4.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(80.0),
            Some(50.0),
            Some(90.0),
            Some(10.0),
            Some(60.0),
        ]));

        let bar = series.get(4).expect("a bar");
        assert_close(bar.force.expect("a force"), 0.2);
        assert_close(bar.acceleration.expect("an acceleration"), 0.4);
    }

    #[test]
    fn intensification_tracks_the_signed_magnitude_change() {
        // Positive force expanding: sign +1 * (0.4 - 0.2) / 0.5 = +0.4.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(60.0),
            Some(60.0),
            Some(60.0),
            Some(70.0),
        ]));
        assert_close(
            series
                .get(3)
                .expect("a bar")
                .intensification
                .expect("an intensification"),
            0.4,
        );

        // Negative force expanding: sign -1 * (0.4 - 0.2) / 0.5 = -0.4. The
        // formula's sign factor is applied literally.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(40.0),
            Some(40.0),
            Some(40.0),
            Some(30.0),
        ]));
        assert_close(
            series
                .get(3)
                .expect("a bar")
                .intensification
                .expect("an intensification"),
            -0.4,
        );

        // Positive force fading: +1 * (0.2 - 0.4) / 0.5 = -0.4.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(70.0),
            Some(70.0),
            Some(70.0),
            Some(60.0),
        ]));
        assert_close(
            series
                .get(3)
                .expect("a bar")
                .intensification
                .expect("an intensification"),
            -0.4,
        );

        // Negative force fading: -1 * (0.2 - 0.4) / 0.5 = +0.4.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(30.0),
            Some(30.0),
            Some(30.0),
            Some(40.0),
        ]));
        assert_close(
            series
                .get(3)
                .expect("a bar")
                .intensification
                .expect("an intensification"),
            0.4,
        );
    }

    #[test]
    fn intensification_is_exactly_zero_when_the_magnitude_is_unchanged() {
        // Same sign and same magnitude.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(60.0),
            Some(60.0),
            Some(60.0),
            Some(60.0),
        ]));
        assert_eq!(
            series.get(3).expect("a bar").intensification,
            Some(0.0),
            "the magnitude is unchanged"
        );

        // Opposite sign but the same magnitude: sign * 0 is exactly zero.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(40.0),
            Some(40.0),
            Some(40.0),
            Some(60.0),
        ]));
        assert_eq!(
            series.get(3).expect("a bar").intensification,
            Some(0.0),
            "the magnitude is unchanged across a sign flip"
        );
    }

    #[test]
    fn a_zero_force_reports_zero_intensification_whatever_the_comparison() {
        for previous_rsi in [10.0, 50.0, 90.0] {
            let series = MomentumSeries::compute(&readings_for_rsi(&[
                Some(previous_rsi),
                Some(previous_rsi),
                Some(previous_rsi),
                Some(50.0),
            ]));
            let bar = series.get(3).expect("a bar");

            assert_eq!(bar.force, Some(0.0));
            assert_eq!(bar.state_code, Some(0));
            assert_eq!(
                bar.intensification,
                Some(0.0),
                "a zero force must report exactly zero intensification"
            );
        }
    }

    #[test]
    fn acceleration_and_intensification_are_clamped() {
        // Force -1 -> +1: acceleration 4 clamps to +1; the magnitudes are
        // equal, so intensification is 0.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(0.0),
            Some(0.0),
            Some(0.0),
            Some(100.0),
        ]));
        let bar = series.get(3).expect("a bar");
        assert_eq!(bar.acceleration, Some(1.0));
        assert_eq!(bar.intensification, Some(0.0));

        // Force 0 -> +1: acceleration 2 clamps to +1; intensification 2
        // clamps to +1.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(50.0),
            Some(50.0),
            Some(50.0),
            Some(100.0),
        ]));
        let bar = series.get(3).expect("a bar");
        assert_eq!(bar.acceleration, Some(1.0));
        assert_eq!(bar.intensification, Some(1.0));

        // Force 0 -> -1: acceleration -2 clamps to -1; intensification -2
        // clamps to -1.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(50.0),
            Some(50.0),
            Some(50.0),
            Some(0.0),
        ]));
        let bar = series.get(3).expect("a bar");
        assert_eq!(bar.acceleration, Some(-1.0));
        assert_eq!(bar.intensification, Some(-1.0));
    }

    #[test]
    fn a_missing_current_rsi_makes_the_axis_unavailable() {
        let series = MomentumSeries::compute(&one_bar(None));
        let bar = series.get(0).expect("one bar");

        assert_eq!(bar.force, None);
        assert_eq!(bar.acceleration, None);
        assert_eq!(bar.intensification, None);
        assert_eq!(bar.state_code, None);
        assert_eq!(bar.size_percentile, None);
        assert_eq!(bar.readiness, Readiness::Unavailable);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::InsufficientBars)
        );
        assert_eq!(bar.state_age_bars, 0);
        assert_eq!(bar.state_changes_50, 0);

        // A null after code-bearing bars freezes the counters.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(60.0),
            Some(60.0),
            Some(60.0),
            None,
        ]));
        let bar = series.get(3).expect("a bar");
        assert_eq!(bar.force, None);
        assert_eq!(bar.state_age_bars, 3);
        assert_eq!(bar.state_changes_50, 0);
    }

    #[test]
    fn a_non_finite_or_out_of_range_rsi_is_unavailable() {
        for rsi in [
            Some(f64::NAN),
            Some(f64::INFINITY),
            Some(f64::NEG_INFINITY),
            Some(-1.0),
            Some(101.0),
        ] {
            let series = MomentumSeries::compute(&one_bar(rsi));
            let bar = series.get(0).expect("one bar");

            assert_eq!(bar.force, None, "rsi {rsi:?}");
            assert_eq!(bar.readiness, Readiness::Unavailable, "rsi {rsi:?}");
            assert_eq!(
                bar.unavailable_reason,
                Some(UnavailableReason::InsufficientBars),
                "rsi {rsi:?}"
            );
        }
    }

    #[test]
    fn a_missing_comparison_rsi_makes_only_the_change_measures_null() {
        // Fifty valid forces before bar 54, so the percentile is available.
        // Bar 51 is null and is exactly t-3 at bar 54.
        let mut rsi = vec![Some(60.0); 55];
        rsi[51] = None;
        let series = MomentumSeries::compute(&readings_for_rsi(&rsi));
        let bar = series.get(54).expect("a bar");

        assert_close(bar.force.expect("a force"), 0.2);
        assert_eq!(bar.acceleration, None);
        assert_eq!(bar.intensification, None);
        assert_eq!(bar.state_code, Some(1));
        assert_eq!(bar.unavailable_reason, None);
        assert_eq!(bar.readiness, Readiness::Ready);
        assert_eq!(bar.size_percentile, Some(100.0));
    }

    #[test]
    fn a_non_finite_or_out_of_range_comparison_rsi_nulls_only_the_change_measures() {
        for previous in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 101.0] {
            let series = MomentumSeries::compute(&readings_for_rsi(&[
                Some(previous),
                Some(60.0),
                Some(60.0),
                Some(60.0),
            ]));
            let bar = series.get(3).expect("a bar");

            assert_close(bar.force.expect("a force"), 0.2);
            assert_eq!(bar.acceleration, None, "previous {previous:?}");
            assert_eq!(bar.intensification, None, "previous {previous:?}");
            assert_eq!(bar.unavailable_reason, None, "previous {previous:?}");
        }
    }

    #[test]
    fn the_comparison_ignores_bars_outside_the_exact_window() {
        // t-3 is bar 1 in both cases. A null at t-4 must not matter.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            None,
            Some(60.0),
            Some(60.0),
            Some(60.0),
            Some(60.0),
        ]));
        let bar = series.get(4).expect("a bar");
        assert_close(bar.force.expect("a force"), 0.2);
        assert_eq!(bar.acceleration, Some(0.0));
        assert_eq!(bar.intensification, Some(0.0));

        // A null at t-2 must not matter either.
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(60.0),
            Some(60.0),
            None,
            Some(60.0),
            Some(60.0),
        ]));
        let bar = series.get(4).expect("a bar");
        assert_close(bar.force.expect("a force"), 0.2);
        assert_eq!(bar.acceleration, Some(0.0));
        assert_eq!(bar.intensification, Some(0.0));
    }

    #[test]
    fn the_percentile_uses_only_the_most_recent_252_force_samples() {
        // The first 48 samples have magnitude 1.0; the next 252 have 0.2.
        // The current magnitude is 0.3. Including the older samples would
        // rank it at 84; the 252-sample window ranks it at 100.
        let mut rsi = vec![Some(100.0); 48];
        rsi.extend(vec![Some(60.0); 252]);
        rsi.push(Some(65.0));
        let series = MomentumSeries::compute(&readings_for_rsi(&rsi));
        let bar = series.get(300).expect("a bar");

        assert_close(bar.force.expect("a force"), 0.3);
        assert_eq!(bar.size_percentile, Some(100.0));
        assert_eq!(bar.readiness, Readiness::Ready);
    }

    #[test]
    fn a_valid_zero_force_is_neutral_and_not_missing() {
        let rsi = vec![Some(50.0); 60];
        let series = MomentumSeries::compute(&readings_for_rsi(&rsi));

        let first = series.get(0).expect("a bar");
        assert_eq!(first.force, Some(0.0));
        assert_eq!(first.state_code, Some(0));
        assert_eq!(first.acceleration, None);
        assert_eq!(first.readiness, Readiness::Provisional);
        assert_eq!(first.unavailable_reason, None);

        let fiftieth = series.get(49).expect("a bar");
        assert_eq!(fiftieth.force, Some(0.0));
        assert_eq!(fiftieth.size_percentile, None);
        assert_eq!(fiftieth.readiness, Readiness::Provisional);

        let ready = series.get(50).expect("a bar");
        assert_eq!(ready.force, Some(0.0));
        assert_eq!(ready.state_code, Some(0));
        assert_eq!(ready.acceleration, Some(0.0));
        assert_eq!(ready.intensification, Some(0.0));
        assert_eq!(ready.size_percentile, Some(100.0));
        assert_eq!(ready.readiness, Readiness::Ready);
        assert_eq!(ready.unavailable_reason, None);
        assert_eq!(ready.state_age_bars, 51);
        assert_eq!(ready.state_changes_50, 0);
    }

    #[test]
    fn the_percentile_needs_fifty_prior_samples_and_never_samples_the_current_bar() {
        let mut rsi = vec![Some(60.0); 50];
        rsi.push(Some(55.0));
        rsi.push(Some(60.0));
        let series = MomentumSeries::compute(&readings_for_rsi(&rsi));

        // Bars 0..=48 have fewer than 50 prior samples.
        for bar in 0..=48 {
            let record = series.get(bar).expect("a bar");
            assert_eq!(record.size_percentile, None, "bar {bar}");
            assert_eq!(record.readiness, Readiness::Provisional, "bar {bar}");
        }

        // Bar 49 is the 50th valid force but has only 49 prior samples.
        let fiftieth = series.get(49).expect("a bar");
        assert_eq!(fiftieth.size_percentile, None);
        assert_eq!(fiftieth.readiness, Readiness::Provisional);

        // Bar 50 has 50 prior samples, all 0.2. The current 0.1 ranks at 0,
        // proving that the current bar is not in its own sample.
        let current = series.get(50).expect("a bar");
        assert_eq!(current.size_percentile, Some(0.0));
        assert_eq!(current.readiness, Readiness::Ready);

        // Bar 51 equals the 50 samples of 0.2 and is above the 0.1 sample, so
        // the inclusive comparison ranks it at 100.
        let equal = series.get(51).expect("a bar");
        assert_eq!(equal.size_percentile, Some(100.0));
    }

    #[test]
    fn state_counters_track_changes_and_freeze_on_a_null_force() {
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(60.0),
            Some(60.0),
            None,
            Some(40.0),
            Some(40.0),
            Some(60.0),
            None,
            Some(60.0),
        ]));

        let expected = [
            (Some(1), 1, 0, Readiness::Provisional),
            (Some(1), 2, 0, Readiness::Provisional),
            (None, 2, 0, Readiness::Unavailable),
            (Some(-1), 1, 1, Readiness::Provisional),
            (Some(-1), 2, 1, Readiness::Provisional),
            (Some(1), 1, 2, Readiness::Provisional),
            (None, 1, 2, Readiness::Unavailable),
            (Some(1), 2, 2, Readiness::Provisional),
        ];

        for (bar, (code, age, changes, readiness)) in expected.into_iter().enumerate() {
            let record = series.get(bar).expect("a bar");
            assert_eq!(record.state_code, code, "bar {bar}");
            assert_eq!(record.state_age_bars, age, "bar {bar}");
            assert_eq!(record.state_changes_50, changes, "bar {bar}");
            assert_eq!(record.readiness, readiness, "bar {bar}");
        }
    }

    #[test]
    fn the_state_code_uses_the_inclusive_threshold() {
        let series = MomentumSeries::compute(&readings_for_rsi(&[
            Some(55.0),
            Some(56.0),
            Some(45.0),
            Some(44.0),
        ]));

        assert_eq!(series.get(0).expect("a bar").state_code, Some(0));
        assert_eq!(series.get(1).expect("a bar").state_code, Some(1));
        assert_eq!(series.get(2).expect("a bar").state_code, Some(0));
        assert_eq!(series.get(3).expect("a bar").state_code, Some(-1));
    }

    #[test]
    fn the_axis_does_not_read_other_readings() {
        let rsi = vec![Some(60.0); 60];
        let baseline = readings_for_rsi(&rsi);

        let mut changed = baseline.clone();
        changed.ema20 = ReadingSeries::new(vec![Some(1.0); rsi.len()]);
        changed.ema50 = ReadingSeries::new(vec![Some(2.0); rsi.len()]);
        changed.ema200 = ReadingSeries::new(vec![Some(3.0); rsi.len()]);
        changed.atr14 = ReadingSeries::new(vec![Some(4.0); rsi.len()]);
        changed.rvol20 = ReadingSeries::new(vec![Some(5.0); rsi.len()]);
        changed.pivots = PivotSeries::compute(&[
            Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(1.0)).expect("valid test candle")
        ]);

        assert_eq!(
            MomentumSeries::compute(&baseline),
            MomentumSeries::compute(&changed)
        );
    }

    #[test]
    fn an_empty_series_has_no_bars() {
        let readings = readings_for_rsi(&[]);
        let series = MomentumSeries::compute(&readings);

        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
        assert_eq!(series.get(0), None);
        assert_eq!(series.latest(), None);
        assert_eq!(series.bars(), []);
    }
}
