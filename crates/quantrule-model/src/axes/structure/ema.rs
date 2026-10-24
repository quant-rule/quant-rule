//! The EMA dynamic-level features of the Structure axis.
//!
//! For each EMA period in `{20, 50, 200}` the module publishes the distance of
//! the close from the EMA in ATR units, the close/EMA cross code, and the
//! retest code of the EMA zone. These features describe an EMA's structural
//! role as a dynamic level; they are additional context and never enter the
//! swing score.
//!
//! The module is pure: it reads the candle and readings it is given and
//! performs no I/O, reads no clock, and uses no randomness.

use crate::readings::{ReadingSeries, Readings};
use quantrule_data::Candle;

/// The half-width of the retest zone in ATR units.
const RETEST_ZONE_ATR: f64 = 0.2;

/// The three EMA features for one bar, one entry per [`EMA_PERIODS`] entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EmaFeatures {
    /// `(Close - EMA_n) / ATR14` for each period.
    pub(crate) distance_atr: [Option<f64>; 3],
    /// Close/EMA cross code for each period.
    pub(crate) cross_code: [Option<i8>; 3],
    /// EMA-zone retest code for each period.
    pub(crate) retest_code: [Option<i8>; 3],
}

impl EmaFeatures {
    /// Computes the three features for `bar`.
    ///
    /// A missing candle at `bar`, a missing reading, or a non-finite input
    /// makes the affected feature null. A zero or negative ATR is not a usable
    /// denominator for distance or retest; the cross does not use ATR and
    /// remains available.
    pub(crate) fn compute(candles: &[Candle], readings: &Readings, bar: usize) -> Self {
        let series = [&readings.ema20, &readings.ema50, &readings.ema200];
        let mut features = Self {
            distance_atr: [None; 3],
            cross_code: [None; 3],
            retest_code: [None; 3],
        };

        for (index, ema) in series.into_iter().enumerate() {
            features.distance_atr[index] = distance_atr(candles, readings, ema, bar);
            features.cross_code[index] = cross_code(candles, ema, bar);
            features.retest_code[index] = retest_code(candles, readings, ema, bar);
        }

        features
    }
}

fn finite(value: f64) -> Option<f64> {
    value.is_finite().then_some(value)
}

fn valid_atr(readings: &Readings, bar: usize) -> Option<f64> {
    readings
        .atr14
        .get(bar)
        .and_then(finite)
        .filter(|atr| *atr > 0.0)
}

fn distance_atr(
    candles: &[Candle],
    readings: &Readings,
    ema: &ReadingSeries,
    bar: usize,
) -> Option<f64> {
    let close = candles.get(bar).and_then(|candle| finite(candle.close()))?;
    let ema = ema.get(bar).and_then(finite)?;
    let atr = valid_atr(readings, bar)?;
    finite((close - ema) / atr)
}

fn cross_code(candles: &[Candle], ema: &ReadingSeries, bar: usize) -> Option<i8> {
    let previous_bar = bar.checked_sub(1)?;
    let previous_close = candles
        .get(previous_bar)
        .and_then(|candle| finite(candle.close()))?;
    let close = candles.get(bar).and_then(|candle| finite(candle.close()))?;
    let previous_ema = ema.get(previous_bar).and_then(finite)?;
    let ema = ema.get(bar).and_then(finite)?;

    if previous_close <= previous_ema && close > ema {
        Some(1)
    } else if previous_close >= previous_ema && close < ema {
        Some(-1)
    } else {
        // A close exactly on the EMA, or a bar that stayed on one side, is a
        // measured absence of a cross rather than missing data.
        Some(0)
    }
}

fn retest_code(
    candles: &[Candle],
    readings: &Readings,
    ema: &ReadingSeries,
    bar: usize,
) -> Option<i8> {
    let candle = candles.get(bar)?;
    let close = finite(candle.close())?;
    let high = finite(candle.high())?;
    let low = finite(candle.low())?;
    let ema = ema.get(bar).and_then(finite)?;
    let atr = valid_atr(readings, bar)?;
    let zone_half_width = RETEST_ZONE_ATR * atr;

    // The bar range and the EMA zone are both closed intervals, so touching a
    // zone boundary is a test.
    let tested = low <= ema + zone_half_width && high >= ema - zone_half_width;
    if !tested {
        return Some(0);
    }

    if close > ema {
        Some(1)
    } else if close < ema {
        Some(-1)
    } else {
        // The contract has no separate code for a tested close exactly on the
        // EMA; it publishes the neutral 0, as a Level event does for a close
        // exactly on a level.
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{EmaFeatures, RETEST_ZONE_ATR};
    use crate::readings::{PivotSeries, ReadingSeries, Readings};

    fn candle(open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(0, 0, open, high, low, close, Some(10.0)).expect("the test candle is valid")
    }

    fn readings(
        ema20: Vec<Option<f64>>,
        ema50: Vec<Option<f64>>,
        ema200: Vec<Option<f64>>,
        atr14: Vec<Option<f64>>,
    ) -> Readings {
        let len = atr14.len();
        Readings {
            ema20: ReadingSeries::new(ema20),
            ema50: ReadingSeries::new(ema50),
            ema200: ReadingSeries::new(ema200),
            rsi14: ReadingSeries::new(vec![None; len]),
            atr14: ReadingSeries::new(atr14),
            rvol20: ReadingSeries::new(vec![None; len]),
            pivots: PivotSeries::compute(&[]),
        }
    }

    fn single_bar_readings(
        ema20: Option<f64>,
        ema50: Option<f64>,
        ema200: Option<f64>,
        atr14: Option<f64>,
    ) -> Readings {
        readings(vec![ema20], vec![ema50], vec![ema200], vec![atr14])
    }

    #[test]
    fn distance_is_close_minus_ema_over_atr() {
        let candles = [candle(110.0, 110.0, 110.0, 110.0)];
        let readings = single_bar_readings(Some(100.0), Some(120.0), Some(110.0), Some(10.0));
        let features = EmaFeatures::compute(&candles, &readings, 0);

        assert_eq!(features.distance_atr, [Some(1.0), Some(-1.0), Some(0.0)]);
    }

    #[test]
    fn distance_is_null_on_an_invalid_denominator_or_missing_input() {
        let candles = [candle(110.0, 110.0, 110.0, 110.0)];

        for atr in [
            Some(0.0),
            Some(-1.0),
            Some(f64::NAN),
            Some(f64::INFINITY),
            None,
        ] {
            let readings = single_bar_readings(Some(100.0), Some(100.0), Some(100.0), atr);
            let features = EmaFeatures::compute(&candles, &readings, 0);
            assert_eq!(
                features.distance_atr,
                [None, None, None],
                "ATR {atr:?} must not be a usable denominator"
            );
        }

        for ema in [Some(f64::NAN), Some(f64::INFINITY), None] {
            let readings = single_bar_readings(ema, ema, ema, Some(10.0));
            let features = EmaFeatures::compute(&candles, &readings, 0);
            assert_eq!(features.distance_atr, [None, None, None]);
        }

        let readings = single_bar_readings(Some(100.0), Some(100.0), Some(100.0), Some(10.0));
        let features = EmaFeatures::compute(&[], &readings, 0);
        assert_eq!(features.distance_atr, [None, None, None]);
    }

    #[test]
    fn cross_compares_the_previous_and_current_close() {
        let candles = [
            candle(99.0, 99.0, 99.0, 99.0),
            candle(101.0, 101.0, 101.0, 101.0),
        ];
        let readings = readings(
            vec![Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0)],
            vec![Some(10.0), Some(10.0)],
        );
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(1), Some(1), Some(1)]);

        let candles = [
            candle(101.0, 101.0, 101.0, 101.0),
            candle(99.0, 99.0, 99.0, 99.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(-1), Some(-1), Some(-1)]);
    }

    #[test]
    fn a_touch_or_a_one_sided_bar_is_not_a_cross() {
        let readings = readings(
            vec![Some(100.0), Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0), Some(100.0)],
            vec![Some(10.0), Some(10.0), Some(10.0)],
        );

        // Close exactly on the EMA, then below: a cross from at-or-above.
        let candles = [
            candle(100.0, 100.0, 100.0, 100.0),
            candle(99.0, 99.0, 99.0, 99.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(-1), Some(-1), Some(-1)]);

        // Close exactly on the EMA, then above: a cross from at-or-below.
        let candles = [
            candle(100.0, 100.0, 100.0, 100.0),
            candle(101.0, 101.0, 101.0, 101.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(1), Some(1), Some(1)]);

        // Stay above the EMA: no cross.
        let candles = [
            candle(101.0, 101.0, 101.0, 101.0),
            candle(102.0, 102.0, 102.0, 102.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(0), Some(0), Some(0)]);

        // Stay below the EMA: no cross.
        let candles = [
            candle(99.0, 99.0, 99.0, 99.0),
            candle(98.0, 98.0, 98.0, 98.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(0), Some(0), Some(0)]);

        // Current close exactly on the EMA from above: no cross.
        let candles = [
            candle(101.0, 101.0, 101.0, 101.0),
            candle(100.0, 100.0, 100.0, 100.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(0), Some(0), Some(0)]);

        // Current close exactly on the EMA from below: no cross.
        let candles = [
            candle(99.0, 99.0, 99.0, 99.0),
            candle(100.0, 100.0, 100.0, 100.0),
        ];
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.cross_code, [Some(0), Some(0), Some(0)]);

        // The first bar has no previous bar.
        let features = EmaFeatures::compute(&candles, &readings, 0);
        assert_eq!(features.cross_code, [None, None, None]);
    }

    #[test]
    fn cross_does_not_require_a_valid_atr() {
        let candles = [
            candle(99.0, 99.0, 99.0, 99.0),
            candle(101.0, 101.0, 101.0, 101.0),
        ];

        for atr in [Some(0.0), Some(-1.0), Some(f64::NAN), None] {
            let readings = readings(
                vec![Some(100.0), Some(100.0)],
                vec![Some(100.0), Some(100.0)],
                vec![Some(100.0), Some(100.0)],
                vec![atr, atr],
            );
            let features = EmaFeatures::compute(&candles, &readings, 1);
            assert_eq!(features.cross_code, [Some(1), Some(1), Some(1)]);
            assert_eq!(features.distance_atr, [None, None, None]);
            assert_eq!(features.retest_code, [None, None, None]);
        }
    }

    #[test]
    fn cross_is_null_when_a_required_reading_is_missing() {
        let candles = [
            candle(99.0, 99.0, 99.0, 99.0),
            candle(101.0, 101.0, 101.0, 101.0),
        ];

        let with_missing_previous = readings(
            vec![None, Some(100.0)],
            vec![Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0)],
            vec![Some(10.0), Some(10.0)],
        );
        let features = EmaFeatures::compute(&candles, &with_missing_previous, 1);
        assert_eq!(features.cross_code[0], None);
        assert_eq!(features.cross_code[1], Some(1));

        let with_missing_current = readings(
            vec![Some(100.0), None],
            vec![Some(100.0), Some(100.0)],
            vec![Some(100.0), Some(100.0)],
            vec![Some(10.0), Some(10.0)],
        );
        let features = EmaFeatures::compute(&candles, &with_missing_current, 1);
        assert_eq!(features.cross_code[0], None);
    }

    #[test]
    fn retest_reports_the_side_of_a_tested_close() {
        // EMA 100, ATR 10, so the zone is [98, 102].
        let candles = [
            candle(101.0, 103.0, 97.0, 101.0),
            candle(99.0, 103.0, 97.0, 99.0),
            candle(100.0, 103.0, 97.0, 100.0),
        ];
        let readings = readings(
            vec![Some(100.0); 3],
            vec![Some(100.0); 3],
            vec![Some(100.0); 3],
            vec![Some(10.0); 3],
        );

        let features = EmaFeatures::compute(&candles, &readings, 0);
        assert_eq!(features.retest_code, [Some(1), Some(1), Some(1)]);
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.retest_code, [Some(-1), Some(-1), Some(-1)]);
        let features = EmaFeatures::compute(&candles, &readings, 2);
        assert_eq!(features.retest_code, [Some(0), Some(0), Some(0)]);
    }

    #[test]
    fn retest_zone_boundaries_are_inclusive() {
        // A bar whose low is exactly the lower zone boundary is a test, and a
        // bar whose high is exactly the upper boundary is a test.
        let candles = [
            candle(99.0, 99.0, 98.0, 99.0),
            candle(101.0, 102.0, 101.0, 101.0),
        ];
        let readings = readings(
            vec![Some(100.0); 2],
            vec![Some(100.0); 2],
            vec![Some(100.0); 2],
            vec![Some(10.0); 2],
        );

        let features = EmaFeatures::compute(&candles, &readings, 0);
        assert_eq!(features.retest_code, [Some(-1), Some(-1), Some(-1)]);
        let features = EmaFeatures::compute(&candles, &readings, 1);
        assert_eq!(features.retest_code, [Some(1), Some(1), Some(1)]);
    }

    #[test]
    fn a_bar_outside_the_zone_is_a_measured_zero() {
        let candles = [
            candle(97.0, 97.0, 96.0, 97.0),
            candle(103.0, 104.0, 103.0, 103.0),
        ];
        let readings = readings(
            vec![Some(100.0); 2],
            vec![Some(100.0); 2],
            vec![Some(100.0); 2],
            vec![Some(10.0); 2],
        );

        for bar in 0..2 {
            let features = EmaFeatures::compute(&candles, &readings, bar);
            assert_eq!(features.retest_code, [Some(0), Some(0), Some(0)]);
        }
    }

    #[test]
    fn retest_is_null_on_an_invalid_atr_or_missing_input() {
        let candles = [candle(101.0, 103.0, 97.0, 101.0)];

        for atr in [Some(0.0), Some(-1.0), Some(f64::NAN), None] {
            let readings = single_bar_readings(Some(100.0), Some(100.0), Some(100.0), atr);
            let features = EmaFeatures::compute(&candles, &readings, 0);
            assert_eq!(features.retest_code, [None, None, None]);
        }

        let readings = single_bar_readings(Some(f64::NAN), Some(100.0), Some(100.0), Some(10.0));
        let features = EmaFeatures::compute(&candles, &readings, 0);
        assert_eq!(features.retest_code[0], None);
        assert_eq!(features.retest_code[1], Some(1));

        let readings = single_bar_readings(Some(100.0), Some(100.0), Some(100.0), Some(10.0));
        let features = EmaFeatures::compute(&[], &readings, 0);
        assert_eq!(features.retest_code, [None, None, None]);
    }

    #[test]
    fn the_retest_zone_is_twenty_percent_of_atr() {
        assert_eq!(RETEST_ZONE_ATR, 0.2);
    }
}
