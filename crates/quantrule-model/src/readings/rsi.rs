//! Wilder's relative strength index.
//!
//! The first average gain and the first average loss are the arithmetic means
//! of the first `period` changes. Each later bar applies the Wilder recurrence
//! `avg_t = (avg_(t-1) * (period - 1) + value_t) / period`. The first RSI is
//! published on the bar that completes the first `period` changes, so a series
//! needs `period + 1` closed bars before the first value exists.

use quantrule_data::Candle;

use super::ReadingSeries;

/// Computes an RSI series over the candle closes.
///
/// A `period` of zero, or a candle series shorter than `period + 1`, returns an
/// all-null series of the same length. If both Wilder averages are zero, the
/// bar is unavailable. If the average loss is zero and the average gain is
/// positive, the value is 100. A zero average gain with a positive average loss
/// is a valid zero.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::readings::rsi;
///
/// let closes = [10.0, 11.0, 10.0, 12.0, 11.0];
/// let candles: Vec<Candle> = closes
///     .into_iter()
///     .enumerate()
///     .map(|(index, close)| {
///         Candle::new(index as u64, index as i64, close, close, close, close, None)
///             .expect("the example candle is valid")
///     })
///     .collect();
///
/// let series = rsi(&candles, 2);
/// assert_eq!(series.get(1), None);
/// assert_eq!(series.get(2), Some(50.0));
/// ```
#[must_use]
pub fn rsi(candles: &[Candle], period: usize) -> ReadingSeries {
    let mut values = vec![None; candles.len()];

    if period == 0 || candles.len() <= period {
        return ReadingSeries::new(values);
    }

    let mut gain_sum = 0.0;
    let mut loss_sum = 0.0;
    for index in 1..=period {
        let change = candles[index].close() - candles[index - 1].close();
        if change > 0.0 {
            gain_sum += change;
        } else if change < 0.0 {
            loss_sum -= change;
        }
    }

    let mut average_gain = gain_sum / period as f64;
    let mut average_loss = loss_sum / period as f64;
    values[period] = rsi_value(average_gain, average_loss);

    let divisor = period as f64;
    let previous_weight = divisor - 1.0;
    for (index, value) in values.iter_mut().enumerate().skip(period + 1) {
        let change = candles[index].close() - candles[index - 1].close();
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };
        average_gain = (average_gain * previous_weight + gain) / divisor;
        average_loss = (average_loss * previous_weight + loss) / divisor;
        *value = rsi_value(average_gain, average_loss);
    }

    ReadingSeries::new(values)
}

fn rsi_value(average_gain: f64, average_loss: f64) -> Option<f64> {
    if average_gain == 0.0 && average_loss == 0.0 {
        None
    } else if average_loss == 0.0 {
        Some(100.0)
    } else {
        Some(100.0 - 100.0 / (1.0 + average_gain / average_loss))
    }
}

#[cfg(test)]
mod tests {
    use super::rsi;
    use crate::readings::test_support::{candle, candles_from_closes};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_first_value_matches_a_hand_computed_two_change_window() {
        let candles = candles_from_closes(&[10.0, 11.0, 10.0, 12.0, 11.0]);
        let series = rsi(&candles, 2);

        assert_eq!(series.get(0), None);
        assert_eq!(series.get(1), None);
        assert_eq!(series.get(2), Some(50.0));
        assert_close(
            series.get(3).expect("the fourth bar has an RSI"),
            83.333_333_333_333_33,
        );
        assert_eq!(series.get(4), Some(50.0));
    }

    #[test]
    fn all_gains_produce_one_hundred() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let series = rsi(&candles, 3);

        for bar in 3..6 {
            assert_eq!(series.get(bar), Some(100.0));
        }
    }

    #[test]
    fn all_losses_produce_zero() {
        let candles = candles_from_closes(&[6.0, 5.0, 4.0, 3.0, 2.0, 1.0]);
        let series = rsi(&candles, 3);

        for bar in 3..6 {
            assert_eq!(series.get(bar), Some(0.0));
        }
    }

    #[test]
    fn a_flat_window_is_unavailable() {
        let candles = candles_from_closes(&[5.0; 10]);
        let series = rsi(&candles, 3);

        for bar in 3..10 {
            assert_eq!(series.get(bar), None);
        }
    }

    #[test]
    fn a_flat_window_that_moves_later_becomes_available() {
        let mut closes = vec![5.0; 4];
        closes.push(6.0);
        closes.push(7.0);
        let candles = candles_from_closes(&closes);
        let series = rsi(&candles, 3);

        assert_eq!(series.get(3), None);
        assert_eq!(series.get(4), Some(100.0));
        assert_eq!(series.get(5), Some(100.0));
    }

    #[test]
    fn a_series_shorter_than_period_plus_one_is_all_null() {
        let candles = candles_from_closes(&[1.0; 14]);
        let series = rsi(&candles, 14);

        assert_eq!(series.len(), 14);
        for bar in 0..14 {
            assert_eq!(series.get(bar), None);
        }
    }

    #[test]
    fn the_fifteenth_closed_bar_is_the_first_value() {
        let mut closes: Vec<f64> = (1_u32..=15).map(f64::from).collect();
        closes[14] = 14.5;
        let candles = candles_from_closes(&closes);
        let series = rsi(&candles, 14);

        assert_eq!(series.get(13), None);
        assert!(series.get(14).is_some());
    }

    #[test]
    fn a_zero_period_is_all_null() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0]);
        let series = rsi(&candles, 0);

        assert_eq!(series.values(), [None, None, None]);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let series = rsi(&[], 14);

        assert!(series.is_empty());
    }

    #[test]
    fn a_single_candle_is_null() {
        let series = rsi(&[candle(1.0)], 14);

        assert_eq!(series.len(), 1);
        assert_eq!(series.get(0), None);
    }

    #[test]
    fn appended_bars_do_not_change_earlier_output() {
        let candles = candles_from_closes(&[10.0, 11.0, 10.0, 12.0, 11.0, 13.0]);
        let full = rsi(&candles, 2);
        let prefix = rsi(&candles[..4], 2);

        for bar in 0..4 {
            assert_eq!(full.get(bar), prefix.get(bar));
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles = candles_from_closes(&[10.0, 11.0, 10.0, 12.0, 11.0, 13.0]);
        let first = rsi(&candles, 2);
        let second = rsi(&candles, 2);

        assert_eq!(first, second);
    }
}
