//! Exponential moving averages.
//!
//! The EMA is seeded with the arithmetic mean of the first `period` closes and
//! then follows `EMA_t = alpha * Close_t + (1 - alpha) * EMA_(t-1)` with
//! `alpha = 2 / (period + 1)`. The value is unavailable before the seed bar, and
//! the seed is the first bar on which the EMA is published.

use quantrule_data::Candle;

use super::ReadingSeries;

/// Computes an EMA series over the candle closes.
///
/// The first `period` closes are the seed. A `period` of zero, or a candle
/// series shorter than `period`, returns an all-null series of the same length.
/// The accumulation runs from the oldest close forward, as the contract fixes.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::readings::ema;
///
/// let candles: Vec<Candle> = [1.0, 2.0, 3.0, 4.0, 5.0]
///     .into_iter()
///     .enumerate()
///     .map(|(index, close)| {
///         Candle::new(index as u64, index as i64, close, close, close, close, None)
///             .expect("the example candle is valid")
///     })
///     .collect();
///
/// let series = ema(&candles, 3);
/// assert_eq!(series.get(1), None);
/// assert_eq!(series.get(2), Some(2.0));
/// assert_eq!(series.get(3), Some(3.0));
/// ```
#[must_use]
pub fn ema(candles: &[Candle], period: usize) -> ReadingSeries {
    let mut values = vec![None; candles.len()];

    if period == 0 || candles.len() < period {
        return ReadingSeries::new(values);
    }

    let alpha = 2.0 / (period as f64 + 1.0);
    let mut seed = 0.0;
    for candle in &candles[..period] {
        seed += candle.close();
    }
    seed /= period as f64;
    values[period - 1] = Some(seed);

    let mut previous = seed;
    for (index, candle) in candles.iter().enumerate().skip(period) {
        previous = alpha * candle.close() + (1.0 - alpha) * previous;
        values[index] = Some(previous);
    }

    ReadingSeries::new(values)
}

#[cfg(test)]
mod tests {
    use super::ema;
    use crate::readings::test_support::{candle, candles_from_closes};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_seed_is_the_sma_of_the_first_period_closes() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let series = ema(&candles, 3);

        assert_eq!(series.get(0), None);
        assert_eq!(series.get(1), None);
        assert_eq!(series.get(2), Some(2.0));
        assert_close(series.get(3).expect("the fourth bar has an EMA"), 3.0);
        assert_close(series.get(4).expect("the fifth bar has an EMA"), 4.0);
    }

    #[test]
    fn a_constant_series_keeps_the_constant() {
        let candles = candles_from_closes(&[7.0; 10]);
        let series = ema(&candles, 3);

        for bar in 2..10 {
            assert_eq!(series.get(bar), Some(7.0));
        }
    }

    #[test]
    fn a_series_shorter_than_the_period_is_all_null() {
        let candles = candles_from_closes(&[1.0, 2.0]);
        let series = ema(&candles, 3);

        assert_eq!(series.len(), 2);
        assert_eq!(series.get(0), None);
        assert_eq!(series.get(1), None);
    }

    #[test]
    fn a_zero_period_is_all_null() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0]);
        let series = ema(&candles, 0);

        assert_eq!(series.values(), [None, None, None]);
    }

    #[test]
    fn a_period_of_one_repeats_the_close() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0]);
        let series = ema(&candles, 1);

        assert_eq!(series.get(0), Some(1.0));
        assert_eq!(series.get(1), Some(2.0));
        assert_eq!(series.get(2), Some(3.0));
    }

    #[test]
    fn an_empty_series_is_empty() {
        let series = ema(&[], 20);

        assert!(series.is_empty());
        assert_eq!(series.get(0), None);
    }

    #[test]
    fn a_single_candle_below_the_period_is_null() {
        let series = ema(&[candle(10.0)], 20);

        assert_eq!(series.len(), 1);
        assert_eq!(series.get(0), None);
    }

    #[test]
    fn appended_bars_do_not_change_earlier_output() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let full = ema(&candles, 3);
        let prefix = ema(&candles[..4], 3);

        for bar in 0..4 {
            assert_eq!(full.get(bar), prefix.get(bar));
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles = candles_from_closes(&[1.0, 3.0, 2.0, 5.0, 4.0, 7.0]);
        let first = ema(&candles, 3);
        let second = ema(&candles, 3);

        assert_eq!(first, second);
    }
}
