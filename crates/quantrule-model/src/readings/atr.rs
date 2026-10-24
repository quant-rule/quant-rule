//! Average true range with Wilder smoothing.
//!
//! The first true range is at bar 1 because bar 0 has no previous close. The
//! seed is the arithmetic mean of the first `period` true ranges, and each
//! later bar applies the Wilder recurrence
//! `ATR_t = (ATR_(t-1) * (period - 1) + TR_t) / period`.

use quantrule_data::Candle;

use super::ReadingSeries;

/// Computes an ATR series over the candles.
///
/// The first true range is at bar 1, and the seed uses the true ranges at bars
/// `1..=period`, so the first ATR is published at bar `period`. A `period` of
/// zero, or a candle series shorter than `period + 1`, returns an all-null
/// series of the same length. A flat series produces a valid zero.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::readings::atr;
///
/// let candles = [
///     Candle::new(0, 0, 9.0, 10.0, 8.0, 9.0, None).expect("valid candle"),
///     Candle::new(1, 1, 10.0, 11.0, 9.0, 10.0, None).expect("valid candle"),
///     Candle::new(2, 2, 11.0, 12.0, 10.0, 11.0, None).expect("valid candle"),
/// ];
///
/// let series = atr(&candles, 2);
/// assert_eq!(series.get(1), None);
/// assert_eq!(series.get(2), Some(2.0));
/// ```
#[must_use]
pub fn atr(candles: &[Candle], period: usize) -> ReadingSeries {
    let mut values = vec![None; candles.len()];

    if period == 0 || candles.len() <= period {
        return ReadingSeries::new(values);
    }

    let mut seed = 0.0;
    for index in 1..=period {
        seed += true_range(candles, index);
    }
    seed /= period as f64;
    values[period] = Some(seed);

    let mut previous = seed;
    let divisor = period as f64;
    let previous_weight = divisor - 1.0;
    for (index, value) in values.iter_mut().enumerate().skip(period + 1) {
        let range = true_range(candles, index);
        previous = (previous * previous_weight + range) / divisor;
        *value = Some(previous);
    }

    ReadingSeries::new(values)
}

fn true_range(candles: &[Candle], index: usize) -> f64 {
    let candle = candles[index];
    let previous_close = candles[index - 1].close();
    let mut range = candle.high() - candle.low();
    let high_gap = (candle.high() - previous_close).abs();
    if high_gap > range {
        range = high_gap;
    }
    let low_gap = (candle.low() - previous_close).abs();
    if low_gap > range {
        range = low_gap;
    }
    range
}

#[cfg(test)]
mod tests {
    use super::atr;
    use crate::readings::test_support::{candle, candles_from_ohlcv};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_seed_is_the_mean_of_the_first_true_ranges() {
        let candles = candles_from_ohlcv(&[
            (10.0, 8.0, 9.0),
            (11.0, 9.0, 10.0),
            (12.0, 10.0, 11.0),
            (15.0, 13.0, 14.0),
        ]);
        let series = atr(&candles, 2);

        assert_eq!(series.get(0), None);
        assert_eq!(series.get(1), None);
        assert_eq!(series.get(2), Some(2.0));
        assert_close(series.get(3).expect("the fourth bar has an ATR"), 3.0);
    }

    #[test]
    fn a_flat_series_has_a_valid_zero() {
        let candles = candles_from_ohlcv(&[(5.0, 5.0, 5.0); 5]);
        let series = atr(&candles, 2);

        assert_eq!(series.get(2), Some(0.0));
        assert_eq!(series.get(3), Some(0.0));
        assert_eq!(series.get(4), Some(0.0));
    }

    #[test]
    fn a_series_shorter_than_period_plus_one_is_all_null() {
        let candles = candles_from_ohlcv(&[(10.0, 9.0, 9.5); 14]);
        let series = atr(&candles, 14);

        assert_eq!(series.len(), 14);
        for bar in 0..14 {
            assert_eq!(series.get(bar), None);
        }
    }

    #[test]
    fn the_fifteenth_closed_bar_is_the_first_value() {
        let candles = candles_from_ohlcv(&[(10.0, 9.0, 9.5); 15]);
        let series = atr(&candles, 14);

        assert_eq!(series.get(13), None);
        assert!(series.get(14).is_some());
    }

    #[test]
    fn a_zero_period_is_all_null() {
        let candles = candles_from_ohlcv(&[(10.0, 9.0, 9.5); 3]);
        let series = atr(&candles, 0);

        assert_eq!(series.values(), [None, None, None]);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let series = atr(&[], 14);

        assert!(series.is_empty());
    }

    #[test]
    fn a_single_candle_is_null() {
        let series = atr(&[candle(10.0)], 14);

        assert_eq!(series.len(), 1);
        assert_eq!(series.get(0), None);
    }

    #[test]
    fn appended_bars_do_not_change_earlier_output() {
        let candles = candles_from_ohlcv(&[
            (10.0, 8.0, 9.0),
            (11.0, 9.0, 10.0),
            (12.0, 10.0, 11.0),
            (15.0, 13.0, 14.0),
            (16.0, 14.0, 15.0),
        ]);
        let full = atr(&candles, 2);
        let prefix = atr(&candles[..4], 2);

        for bar in 0..4 {
            assert_eq!(full.get(bar), prefix.get(bar));
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles = candles_from_ohlcv(&[
            (10.0, 8.0, 9.0),
            (11.0, 9.0, 10.0),
            (12.0, 10.0, 11.0),
            (15.0, 13.0, 14.0),
        ]);
        let first = atr(&candles, 2);
        let second = atr(&candles, 2);

        assert_eq!(first, second);
    }
}
