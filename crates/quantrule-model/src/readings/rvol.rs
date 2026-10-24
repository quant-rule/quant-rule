//! Relative volume.
//!
//! `RVOL20 = Volume / SMA20(Volume)`. The simple moving average includes the
//! current bar, matching the contract's literal `SMA20(Volume)`. The reading is
//! context only; it never votes on an axis.

use quantrule_data::Candle;

use super::ReadingSeries;

/// Computes a relative-volume series.
///
/// The baseline is the arithmetic mean of the `period` volumes ending at the
/// current bar. The value is unavailable when the current volume is absent,
/// when any volume inside the baseline window is absent, or when the baseline
/// is zero. A measured zero volume against a positive baseline is a valid zero.
/// A `period` of zero, or a candle series shorter than `period`, returns an
/// all-null series of the same length.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::readings::rvol;
///
/// let volumes = [1.0, 2.0, 3.0];
/// let candles: Vec<Candle> = volumes
///     .into_iter()
///     .enumerate()
///     .map(|(index, volume)| {
///         Candle::new(index as u64, index as i64, 10.0, 10.0, 10.0, 10.0, Some(volume))
///             .expect("the example candle is valid")
///     })
///     .collect();
///
/// let series = rvol(&candles, 3);
/// assert_eq!(series.get(1), None);
/// assert_eq!(series.get(2), Some(1.5));
/// ```
#[must_use]
pub fn rvol(candles: &[Candle], period: usize) -> ReadingSeries {
    let mut values = vec![None; candles.len()];

    if period == 0 || candles.len() < period {
        return ReadingSeries::new(values);
    }

    for index in (period - 1)..candles.len() {
        let Some(current_volume) = candles[index].volume() else {
            continue;
        };

        let mut sum = 0.0;
        let mut complete = true;
        for candle in &candles[(index + 1 - period)..=index] {
            match candle.volume() {
                Some(volume) => sum += volume,
                None => {
                    complete = false;
                    break;
                }
            }
        }
        if !complete {
            continue;
        }

        let baseline = sum / period as f64;
        if baseline == 0.0 {
            continue;
        }

        values[index] = Some(current_volume / baseline);
    }

    ReadingSeries::new(values)
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::rvol;
    use crate::readings::test_support::candle_with_volume;

    fn candles_with_volumes(volumes: &[Option<f64>]) -> Vec<Candle> {
        volumes
            .iter()
            .map(|volume| candle_with_volume(10.0, *volume))
            .collect()
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn the_baseline_includes_the_current_bar() {
        let candles =
            candles_with_volumes(&[Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)]);
        let series = rvol(&candles, 3);

        assert_eq!(series.get(1), None);
        assert_eq!(series.get(2), Some(1.5));
        assert_close(series.get(3).expect("the fourth bar has RVOL"), 4.0 / 3.0);
        assert_eq!(series.get(4), Some(1.25));
    }

    #[test]
    fn a_missing_current_volume_is_null() {
        let candles = candles_with_volumes(&[Some(1.0), Some(2.0), None]);
        let series = rvol(&candles, 3);

        assert_eq!(series.get(2), None);
    }

    #[test]
    fn a_missing_volume_inside_the_window_is_null_until_it_leaves() {
        let candles = candles_with_volumes(&[Some(1.0), None, Some(3.0), Some(4.0), Some(5.0)]);
        let series = rvol(&candles, 3);

        assert_eq!(series.get(2), None);
        assert_eq!(series.get(3), None);
        assert_eq!(series.get(4), Some(1.25));
    }

    #[test]
    fn a_zero_baseline_is_null() {
        let candles = candles_with_volumes(&[Some(0.0), Some(0.0), Some(0.0)]);
        let series = rvol(&candles, 3);

        assert_eq!(series.get(2), None);
    }

    #[test]
    fn a_zero_volume_against_a_positive_baseline_is_zero() {
        let candles = candles_with_volumes(&[Some(1.0), Some(2.0), Some(0.0)]);
        let series = rvol(&candles, 3);

        assert_eq!(series.get(2), Some(0.0));
    }

    #[test]
    fn a_series_shorter_than_the_period_is_all_null() {
        let candles = candles_with_volumes(&[Some(1.0), Some(2.0)]);
        let series = rvol(&candles, 3);

        assert_eq!(series.len(), 2);
        assert_eq!(series.values(), [None, None]);
    }

    #[test]
    fn a_zero_period_is_all_null() {
        let candles = candles_with_volumes(&[Some(1.0), Some(2.0), Some(3.0)]);
        let series = rvol(&candles, 0);

        assert_eq!(series.values(), [None, None, None]);
    }

    #[test]
    fn an_empty_series_is_empty() {
        let series = rvol(&[], 20);

        assert!(series.is_empty());
    }

    #[test]
    fn a_single_candle_below_the_period_is_null() {
        let candles = candles_with_volumes(&[Some(1.0)]);
        let series = rvol(&candles, 20);

        assert_eq!(series.len(), 1);
        assert_eq!(series.get(0), None);
    }

    #[test]
    fn appended_bars_do_not_change_earlier_output() {
        let candles =
            candles_with_volumes(&[Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)]);
        let full = rvol(&candles, 3);
        let prefix = rvol(&candles[..4], 3);

        for bar in 0..4 {
            assert_eq!(full.get(bar), prefix.get(bar));
        }
    }

    #[test]
    fn repeated_runs_are_identical() {
        let candles =
            candles_with_volumes(&[Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)]);
        let first = rvol(&candles, 3);
        let second = rvol(&candles, 3);

        assert_eq!(first, second);
    }
}
