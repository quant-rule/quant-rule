//! The Direction axis.
//!
//! Direction measures the alignment of the EMA 20/50/200 stack, scaled by
//! ATR 14. It publishes the signed alignment score, the compact state code,
//! the size percentile, readiness, the unavailable reason, and the state
//! counters as one record per input bar.
//!
//! The axis reads only `ema20`, `ema50`, `ema200`, and `atr14`. It never reads
//! an EMA-distance feature, a Structure field, or any other axis's output, so
//! a correlated measurement cannot become a second vote. The module is pure:
//! it performs no I/O, reads no clock, and uses no randomness.

use crate::kernel::{
    PERCENTILE_WINDOW, Readiness, StateCounters, UnavailableReason, clamp_unit, compact_state,
    empirical_percentile,
};
use crate::readings::Readings;

/// The published Direction values for one closed bar.
///
/// A `direction` of `Some(0.0)` is a measured neutral alignment. `None` means
/// the axis could not publish a value; `unavailable_reason` then says why.
/// The state counters freeze while `state_code` is `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionBar {
    /// Directional alignment in `[-1, +1]`, or `None` when unavailable.
    pub direction: Option<f64>,
    /// Compact state: -1 below -0.1, 0 through the inclusive ±0.1 band, +1
    /// above +0.1; `None` when Direction is unavailable.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute Direction magnitude, in `0..=100`,
    /// or `None` when fewer than 50 valid prior samples exist.
    pub size_percentile: Option<f64>,
    /// Mechanical completeness of the axis.
    pub readiness: Readiness,
    /// Why Direction is unavailable, or `None` when it is available.
    pub unavailable_reason: Option<UnavailableReason>,
    /// Closed bars the compact state has held; zero before the first code.
    pub state_age_bars: u64,
    /// Compact state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The per-bar Direction series.
///
/// The series has exactly one [`DirectionBar`] per input bar. Its length always
/// equals the input candle count, so a consumer indexes it by bar position.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::axes::direction::DirectionSeries;
/// use quantrule_model::kernel::Readiness;
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let direction = DirectionSeries::compute(&readings);
///
/// assert_eq!(direction.len(), 1);
/// let bar = direction.get(0).expect("one bar");
/// assert_eq!(bar.direction, None);
/// assert_eq!(bar.readiness, Readiness::Unavailable);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionSeries {
    bars: Vec<DirectionBar>,
}

impl DirectionSeries {
    /// Computes Direction for every bar of the readings.
    ///
    /// The result is aligned to the input length. Each bar is a pure function
    /// of the readings at that bar and the valid Direction magnitudes strictly
    /// before it; no later bar is read.
    #[must_use]
    pub fn compute(readings: &Readings) -> Self {
        let mut bars = Vec::with_capacity(readings.len());
        let mut counters = StateCounters::new();
        // Absolute valid magnitudes in bar order. The current bar is appended
        // only after its percentile is computed, so a bar never samples itself.
        let mut magnitudes = Vec::with_capacity(readings.len());

        for bar in 0..readings.len() {
            match DirectionInputs::from_readings(readings, bar).and_then(DirectionInputs::score) {
                Ok(direction) => {
                    let magnitude = direction.abs();
                    let size_percentile = empirical_percentile(&magnitudes, magnitude);
                    let state_code = compact_state(direction);
                    let stats = counters.observe(state_code);
                    let readiness = if size_percentile.is_some() {
                        Readiness::Ready
                    } else {
                        Readiness::Provisional
                    };

                    bars.push(DirectionBar {
                        direction: Some(direction),
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
                    bars.push(DirectionBar {
                        direction: None,
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
    pub fn get(&self, bar: usize) -> Option<DirectionBar> {
        self.bars.get(bar).copied()
    }

    /// Every bar record, in bar order.
    #[must_use]
    pub fn bars(&self) -> &[DirectionBar] {
        &self.bars
    }

    /// The latest bar record, or `None` for an empty series.
    #[must_use]
    pub fn latest(&self) -> Option<DirectionBar> {
        self.bars.last().copied()
    }
}

/// The four readings Direction is allowed to consume.
///
/// Keeping the extraction narrow makes the no-double-count invariant
/// structural: a later reading cannot enter the axis without changing this
/// type. The readings are resolved once per bar, before the formula runs.
struct DirectionInputs {
    ema20: f64,
    ema50: f64,
    ema200: f64,
    atr14: f64,
}

impl DirectionInputs {
    fn from_readings(readings: &Readings, bar: usize) -> Result<Self, UnavailableReason> {
        let ema20 = finite(readings.ema20.get(bar))?;
        let ema50 = finite(readings.ema50.get(bar))?;
        let ema200 = finite(readings.ema200.get(bar))?;
        let atr14 = readings
            .atr14
            .get(bar)
            .ok_or(UnavailableReason::InsufficientBars)?;

        // ATR is a true-range average, so a non-positive or non-finite value is
        // not a usable denominator. A measured zero ATR means a flat series,
        // not missing data, and receives its own reason.
        if !atr14.is_finite() || atr14 <= 0.0 {
            return Err(UnavailableReason::InvalidDenominator);
        }

        Ok(Self {
            ema20,
            ema50,
            ema200,
            atr14,
        })
    }

    /// Applies the contract's §8.1 formula.
    fn score(self) -> Result<f64, UnavailableReason> {
        let d_short = clamp_unit((self.ema20 - self.ema50) / (2.0 * self.atr14));
        let d_long = clamp_unit((self.ema50 - self.ema200) / (4.0 * self.atr14));
        let direction = (d_short + d_long) / 2.0;

        // The inputs are finite and ATR is positive, so this cannot fail. The
        // guard keeps a defect from publishing NaN as a score or a state.
        if direction.is_finite() {
            Ok(direction)
        } else {
            Err(UnavailableReason::InvalidDenominator)
        }
    }
}

fn finite(value: Option<f64>) -> Result<f64, UnavailableReason> {
    value
        .filter(|value| value.is_finite())
        .ok_or(UnavailableReason::InsufficientBars)
}

#[cfg(test)]
mod tests {
    use super::{DirectionInputs, DirectionSeries};
    use crate::kernel::{Readiness, UnavailableReason};
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;

    fn inputs(ema20: f64, ema50: f64, ema200: f64, atr14: f64) -> DirectionInputs {
        DirectionInputs {
            ema20,
            ema50,
            ema200,
            atr14,
        }
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    /// Builds readings whose Direction values are the supplied values.
    ///
    /// With EMA50 = EMA200 = 0, ATR = 1, and EMA20 = 4 * value, the short leg
    /// is 2 * value and the long leg is 0, so Direction is the supplied value
    /// for magnitudes up to 0.5. A `None` value makes ATR unavailable. The
    /// pivot series is deliberately empty: Direction must not read it.
    fn readings_for(directions: &[Option<f64>]) -> Readings {
        let ema20: Vec<Option<f64>> = directions
            .iter()
            .map(|direction| direction.map(|value| 4.0 * value))
            .collect();
        let atr14: Vec<Option<f64>> = directions
            .iter()
            .map(|direction| direction.map(|_| 1.0))
            .collect();
        let len = directions.len();
        Readings {
            ema20: ReadingSeries::new(ema20),
            ema50: ReadingSeries::new(vec![Some(0.0); len]),
            ema200: ReadingSeries::new(vec![Some(0.0); len]),
            rsi14: ReadingSeries::new(vec![None; len]),
            atr14: ReadingSeries::new(atr14),
            rvol20: ReadingSeries::new(vec![None; len]),
            pivots: PivotSeries::compute(&[]),
        }
    }

    fn one_bar(
        ema20: Option<f64>,
        ema50: Option<f64>,
        ema200: Option<f64>,
        atr14: Option<f64>,
    ) -> Readings {
        Readings {
            ema20: ReadingSeries::new(vec![ema20]),
            ema50: ReadingSeries::new(vec![ema50]),
            ema200: ReadingSeries::new(vec![ema200]),
            rsi14: ReadingSeries::new(vec![None]),
            atr14: ReadingSeries::new(vec![atr14]),
            rvol20: ReadingSeries::new(vec![None]),
            pivots: PivotSeries::compute(&[]),
        }
    }

    #[test]
    fn score_applies_the_contract_formula() {
        // d_short = (110 - 100) / 20 = 0.5; d_long = (100 - 80) / 40 = 0.5.
        let score = inputs(110.0, 100.0, 80.0, 10.0)
            .score()
            .expect("the inputs are available");
        assert_close(score, 0.5);

        // Each leg is clamped before the average.
        let score = inputs(200.0, 100.0, 0.0, 1.0)
            .score()
            .expect("the inputs are available");
        assert_close(score, 1.0);

        let score = inputs(0.0, 100.0, 200.0, 1.0)
            .score()
            .expect("the inputs are available");
        assert_close(score, -1.0);

        // Only the short leg is clamped here: d_short = +1, d_long = +0.5.
        // Clamping only the average would publish a different value.
        let score = inputs(200.0, 100.0, 80.0, 10.0)
            .score()
            .expect("the inputs are available");
        assert_close(score, 0.75);

        // The legs can offset one another and produce a measured zero.
        let score = inputs(110.0, 100.0, 120.0, 10.0)
            .score()
            .expect("the inputs are available");
        assert_close(score, 0.0);
    }

    #[test]
    fn a_missing_required_reading_is_insufficient_bars() {
        let cases = [
            one_bar(None, Some(1.0), Some(1.0), Some(1.0)),
            one_bar(Some(1.0), None, Some(1.0), Some(1.0)),
            one_bar(Some(1.0), Some(1.0), None, Some(1.0)),
            one_bar(Some(1.0), Some(1.0), Some(1.0), None),
        ];

        for readings in cases {
            let bar = DirectionSeries::compute(&readings).get(0).expect("one bar");
            assert_eq!(bar.direction, None);
            assert_eq!(bar.state_code, None);
            assert_eq!(bar.size_percentile, None);
            assert_eq!(bar.readiness, Readiness::Unavailable);
            assert_eq!(
                bar.unavailable_reason,
                Some(UnavailableReason::InsufficientBars)
            );
            assert_eq!(bar.state_age_bars, 0);
            assert_eq!(bar.state_changes_50, 0);
        }
    }

    #[test]
    fn a_non_finite_ema_is_insufficient_bars() {
        let readings = one_bar(Some(f64::NAN), Some(1.0), Some(1.0), Some(1.0));
        let bar = DirectionSeries::compute(&readings).get(0).expect("one bar");

        assert_eq!(bar.direction, None);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::InsufficientBars)
        );
    }

    #[test]
    fn a_zero_negative_or_non_finite_atr_is_an_invalid_denominator() {
        for atr in [Some(0.0), Some(-1.0), Some(f64::NAN), Some(f64::INFINITY)] {
            let readings = one_bar(Some(1.0), Some(1.0), Some(1.0), atr);
            let bar = DirectionSeries::compute(&readings).get(0).expect("one bar");

            assert_eq!(bar.direction, None, "atr {atr:?}");
            assert_eq!(bar.state_code, None, "atr {atr:?}");
            assert_eq!(bar.size_percentile, None, "atr {atr:?}");
            assert_eq!(bar.readiness, Readiness::Unavailable, "atr {atr:?}");
            assert_eq!(
                bar.unavailable_reason,
                Some(UnavailableReason::InvalidDenominator),
                "atr {atr:?}"
            );
        }
    }

    #[test]
    fn a_valid_zero_is_neutral_and_not_missing() {
        let directions = vec![Some(0.0); 51];
        let series = DirectionSeries::compute(&readings_for(&directions));

        for bar in 0..49 {
            let record = series.get(bar).expect("a bar");
            assert_eq!(record.direction, Some(0.0));
            assert_eq!(record.state_code, Some(0));
            assert_eq!(record.size_percentile, None);
            assert_eq!(record.readiness, Readiness::Provisional);
            assert_eq!(record.unavailable_reason, None);
        }

        let ready = series.get(50).expect("a bar");
        assert_eq!(ready.direction, Some(0.0));
        assert_eq!(ready.state_code, Some(0));
        assert_eq!(ready.size_percentile, Some(100.0));
        assert_eq!(ready.readiness, Readiness::Ready);
        assert_eq!(ready.unavailable_reason, None);
        assert_eq!(ready.state_age_bars, 51);
        assert_eq!(ready.state_changes_50, 0);
    }

    #[test]
    fn the_percentile_needs_fifty_prior_samples_and_never_samples_the_current_bar() {
        let mut directions = vec![Some(0.2); 50];
        directions.push(Some(0.1));
        directions.push(Some(0.2));
        let series = DirectionSeries::compute(&readings_for(&directions));

        // Bars 0..=48 have fewer than 50 prior samples.
        for bar in 0..=48 {
            let record = series.get(bar).expect("a bar");
            assert_eq!(record.size_percentile, None, "bar {bar}");
            assert_eq!(record.readiness, Readiness::Provisional, "bar {bar}");
        }

        // Bar 49 is the 50th valid value but has only 49 prior samples.
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
    fn state_counters_track_changes_and_freeze_on_a_null_code() {
        let directions = [
            Some(0.2),
            Some(0.2),
            None,
            Some(-0.2),
            Some(-0.2),
            Some(0.2),
            None,
            Some(0.2),
        ];
        let series = DirectionSeries::compute(&readings_for(&directions));

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
        let series = DirectionSeries::compute(&readings_for(&[
            Some(0.1),
            Some(0.2),
            Some(-0.1),
            Some(-0.2),
        ]));

        assert_eq!(series.get(0).expect("a bar").state_code, Some(0));
        assert_eq!(series.get(1).expect("a bar").state_code, Some(1));
        assert_eq!(series.get(2).expect("a bar").state_code, Some(0));
        assert_eq!(series.get(3).expect("a bar").state_code, Some(-1));
    }

    #[test]
    fn the_axis_does_not_read_other_readings() {
        let directions = vec![Some(0.2); 60];
        let baseline = readings_for(&directions);

        let mut changed = baseline.clone();
        changed.rsi14 = ReadingSeries::new(vec![Some(99.0); directions.len()]);
        changed.rvol20 = ReadingSeries::new(vec![Some(5.0); directions.len()]);
        changed.pivots = PivotSeries::compute(&[
            Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(1.0)).expect("valid test candle")
        ]);

        let baseline_series = DirectionSeries::compute(&baseline);
        let changed_series = DirectionSeries::compute(&changed);

        assert_eq!(baseline_series, changed_series);
    }

    #[test]
    fn an_empty_series_has_no_bars() {
        let readings = Readings {
            ema20: ReadingSeries::new(Vec::new()),
            ema50: ReadingSeries::new(Vec::new()),
            ema200: ReadingSeries::new(Vec::new()),
            rsi14: ReadingSeries::new(Vec::new()),
            atr14: ReadingSeries::new(Vec::new()),
            rvol20: ReadingSeries::new(Vec::new()),
            pivots: PivotSeries::compute(&[]),
        };
        let series = DirectionSeries::compute(&readings);

        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
        assert_eq!(series.get(0), None);
        assert_eq!(series.latest(), None);
        assert_eq!(series.bars(), []);
    }
}
