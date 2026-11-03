//! Moving Average Convergence Divergence.
//!
//! The line is the fast EMA minus the slow EMA. The signal is an EMA of the
//! line, and the histogram is the line minus the signal. Every series is aligned
//! to the input candles; a value is unavailable until its seed and its inputs
//! are available. The module is pure and reads only the supplied candles.

use quantrule_data::Candle;

use super::{ReadingSeries, ema};

/// One MACD reading series.
///
/// The line is available once the slow EMA is available. The signal and
/// histogram are available once the signal EMA has received its seed of line
/// values, so a short series publishes nulls rather than a guessed value.
#[derive(Debug, Clone, PartialEq)]
pub struct MacdSeries {
    /// The MACD line.
    pub line: ReadingSeries,
    /// The signal EMA of the line.
    pub signal: ReadingSeries,
    /// The line minus the signal.
    pub histogram: ReadingSeries,
}

impl MacdSeries {
    /// Computes the MACD series for the supplied candles.
    ///
    /// A zero or non-increasing period pair produces all-null series. The
    /// caller's model config rejects that shape before a run.
    #[must_use]
    pub fn compute(candles: &[Candle], fast: usize, slow: usize, signal: usize) -> Self {
        if fast == 0 || slow == 0 || signal == 0 || fast >= slow {
            return Self::empty(candles.len());
        }
        let fast_ema = ema(candles, fast);
        let slow_ema = ema(candles, slow);
        let line_values: Vec<Option<f64>> = fast_ema
            .values()
            .iter()
            .zip(slow_ema.values())
            .map(|(fast, slow)| match (fast, slow) {
                (Some(fast), Some(slow)) => Some(fast - slow),
                _ => None,
            })
            .collect();
        let line = ReadingSeries::new(line_values);
        let signal = ema_of_series(&line, signal);
        let histogram_values: Vec<Option<f64>> = line
            .values()
            .iter()
            .zip(signal.values())
            .map(|(line, signal)| match (line, signal) {
                (Some(line), Some(signal)) => Some(line - signal),
                _ => None,
            })
            .collect();
        Self {
            line,
            signal,
            histogram: ReadingSeries::new(histogram_values),
        }
    }

    /// An all-null series of the input length.
    #[must_use]
    pub fn empty(len: usize) -> Self {
        Self {
            line: ReadingSeries::new(vec![None; len]),
            signal: ReadingSeries::new(vec![None; len]),
            histogram: ReadingSeries::new(vec![None; len]),
        }
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.line.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.line.is_empty()
    }
}

/// An EMA over a series that may begin with nulls.
///
/// The seed is the mean of the first `period` non-null values, and the seed bar
/// is the bar of the `period`-th non-null value. Later non-null values follow the
/// usual recurrence. A null value is skipped rather than treated as zero, so a
/// missing source value never produces a fabricated MACD signal.
fn ema_of_series(series: &ReadingSeries, period: usize) -> ReadingSeries {
    let mut values = vec![None; series.len()];
    if period == 0 {
        return ReadingSeries::new(values);
    }
    let indices: Vec<usize> = series
        .values()
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.map(|_| index))
        .collect();
    if indices.len() < period {
        return ReadingSeries::new(values);
    }
    let alpha = 2.0 / (period as f64 + 1.0);
    let mut seed = 0.0;
    for index in &indices[..period] {
        seed += series.get(*index).unwrap_or(0.0);
    }
    seed /= period as f64;
    let seed_index = indices[period - 1];
    values[seed_index] = Some(seed);
    let mut previous = seed;
    for index in indices.into_iter().skip(period) {
        let value = series.get(index).unwrap_or(0.0);
        previous = alpha * value + (1.0 - alpha) * previous;
        values[index] = Some(previous);
    }
    ReadingSeries::new(values)
}

#[cfg(test)]
mod tests {
    use super::MacdSeries;
    use crate::readings::test_support::candles_from_closes;

    fn close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("the value is available");
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_line_is_the_fast_minus_the_slow_ema() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0, 10.0]);
        let macd = MacdSeries::compute(&candles, 2, 3, 2);

        close(macd.line.get(2), 0.5);
        close(macd.line.get(3), 0.5);
        close(macd.line.get(4), 1.333_333_333_333_333_3);
        close(macd.signal.get(3), 0.5);
        close(macd.signal.get(4), 1.055_555_555_555_555_6);
        close(macd.histogram.get(3), 0.0);
        close(macd.histogram.get(4), 0.277_777_777_777_777_7);
    }

    #[test]
    fn a_constant_series_has_a_zero_line_and_histogram() {
        let candles = candles_from_closes(&[7.0; 10]);
        let macd = MacdSeries::compute(&candles, 3, 5, 4);

        for bar in 7..10 {
            close(macd.line.get(bar), 0.0);
            close(macd.signal.get(bar), 0.0);
            close(macd.histogram.get(bar), 0.0);
        }
    }

    #[test]
    fn the_signal_waits_for_its_seed_of_line_values() {
        let candles = candles_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let macd = MacdSeries::compute(&candles, 2, 3, 3);

        assert_eq!(macd.signal.get(3), None);
        assert!(macd.signal.get(4).is_some());
    }

    #[test]
    fn appended_bars_do_not_change_earlier_output() {
        let candles = candles_from_closes(&[1.0, 3.0, 2.0, 5.0, 4.0, 7.0, 6.0]);
        let full = MacdSeries::compute(&candles, 2, 3, 2);
        let prefix = MacdSeries::compute(&candles[..5], 2, 3, 2);

        for bar in 0..5 {
            assert_eq!(full.line.get(bar), prefix.line.get(bar));
            assert_eq!(full.signal.get(bar), prefix.signal.get(bar));
            assert_eq!(full.histogram.get(bar), prefix.histogram.get(bar));
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles = candles_from_closes(&[1.0, 3.0, 2.0, 5.0, 4.0, 7.0]);
        let first = MacdSeries::compute(&candles, 2, 3, 2);
        let second = MacdSeries::compute(&candles, 2, 3, 2);
        assert_eq!(first, second);
    }

    #[test]
    fn short_and_invalid_inputs_are_all_null() {
        let candles = candles_from_closes(&[1.0, 2.0]);
        let short = MacdSeries::compute(&candles, 2, 3, 2);
        assert_eq!(short.line.get(0), None);
        assert_eq!(short.signal.get(1), None);

        let invalid = MacdSeries::compute(&candles, 3, 2, 2);
        assert_eq!(invalid.line.get(0), None);
        assert_eq!(invalid.histogram.get(1), None);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let macd = MacdSeries::compute(&[], 12, 26, 9);
        assert!(macd.is_empty());
        assert_eq!(macd.len(), 0);
    }
}
