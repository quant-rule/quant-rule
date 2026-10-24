//! The volatility and participation context.
//!
//! The context is a descriptive diagnostic outside the four axes. It publishes
//! `atr_ratio = ATR14 / SMA20(ATR14)`, `rvol_ratio = RVOL20`, and a four-bucket
//! `context_code`. No axis reads it, and it never votes on a direction.
//!
//! The module is pure: it reads only the readings it is given. It performs no
//! I/O, reads no clock, and uses no randomness.

use crate::readings::Readings;

/// The number of ATR values in the volatility baseline.
pub const ATR_BASELINE_BARS: usize = 20;

/// Code 1: expanding — ATR ratio at least 1 and RVOL at least 1.
pub const EXPANDING_CODE: u8 = 1;
/// Code 2: coiling — ATR ratio below 1 and RVOL below 1.
pub const COILING_CODE: u8 = 2;
/// Code 3: settling — ATR ratio below 1 and RVOL at least 1.
pub const SETTLING_CODE: u8 = 3;
/// Code 4: high-range/low-participation — ATR ratio at least 1 and RVOL below 1.
pub const HIGH_RANGE_LOW_PARTICIPATION_CODE: u8 = 4;

/// The context values for one closed bar.
///
/// The two ratios are retained even when the code is null, so a consumer can
/// inspect the underlying measurements. A missing ratio is `None`, never zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextBar {
    /// `ATR14 / SMA20(ATR14)`, or `None` when the baseline is unavailable or zero.
    pub atr_ratio: Option<f64>,
    /// `RVOL20`, or `None` when volume or the volume baseline is unavailable.
    pub rvol_ratio: Option<f64>,
    /// 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation, or
    /// `None` when either ratio is unavailable.
    pub context_code: Option<u8>,
}

/// The per-bar context series.
///
/// The series has exactly one [`ContextBar`] per input bar, so its length always
/// equals the readings length.
///
/// # Examples
///
/// ```
/// use quantrule_model::context::ContextSeries;
/// use quantrule_model::readings::Readings;
///
/// let context = ContextSeries::compute(&Readings::compute(&[]));
///
/// assert!(context.is_empty());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ContextSeries {
    bars: Vec<ContextBar>,
}

impl ContextSeries {
    /// Computes the context for every bar of the readings.
    #[must_use]
    pub fn compute(readings: &Readings) -> Self {
        let mut bars = Vec::with_capacity(readings.len());
        for bar in 0..readings.len() {
            let atr_ratio = atr_ratio(&readings.atr14, bar);
            let rvol_ratio = readings.rvol20.get(bar);
            let context_code = context_code(atr_ratio, rvol_ratio);
            bars.push(ContextBar {
                atr_ratio,
                rvol_ratio,
                context_code,
            });
        }
        Self { bars }
    }

    /// The number of bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the series is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// The context at `bar`, or `None` when out of range.
    #[must_use]
    pub fn get(&self, bar: usize) -> Option<ContextBar> {
        self.bars.get(bar).copied()
    }

    /// Every bar in order.
    #[must_use]
    pub fn bars(&self) -> &[ContextBar] {
        &self.bars
    }

    /// The latest bar, or `None` when the series is empty.
    #[must_use]
    pub fn latest(&self) -> Option<ContextBar> {
        self.bars.last().copied()
    }
}

/// The ATR ratio at `bar`, or `None` before the 20-value baseline exists.
fn atr_ratio(atr: &crate::readings::ReadingSeries, bar: usize) -> Option<f64> {
    let baseline_start = bar.checked_add(1)?.checked_sub(ATR_BASELINE_BARS)?;
    let current = atr.get(bar)?;
    let mut sum = 0.0;
    for index in baseline_start..=bar {
        sum += atr.get(index)?;
    }
    let baseline = sum / ATR_BASELINE_BARS as f64;
    if baseline == 0.0 {
        None
    } else {
        Some(current / baseline)
    }
}

/// The contract's four-bucket context code.
fn context_code(atr_ratio: Option<f64>, rvol_ratio: Option<f64>) -> Option<u8> {
    let atr = atr_ratio?;
    let rvol = rvol_ratio?;
    if atr >= 1.0 && rvol >= 1.0 {
        Some(EXPANDING_CODE)
    } else if atr < 1.0 && rvol < 1.0 {
        Some(COILING_CODE)
    } else if atr < 1.0 && rvol >= 1.0 {
        Some(SETTLING_CODE)
    } else {
        Some(HIGH_RANGE_LOW_PARTICIPATION_CODE)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ATR_BASELINE_BARS, COILING_CODE, ContextSeries, EXPANDING_CODE,
        HIGH_RANGE_LOW_PARTICIPATION_CODE, SETTLING_CODE,
    };
    use crate::axes::direction::DirectionSeries;
    use crate::axes::momentum::MomentumSeries;
    use crate::axes::structure::StructureSeries;
    use crate::readings::{PivotSeries, ReadingSeries, Readings};
    use quantrule_data::Candle;

    fn candle(index: u64) -> Candle {
        Candle::new(index, index as i64, 100.0, 100.5, 99.5, 100.0, Some(10.0))
            .expect("the test candle is valid")
    }

    fn candles(len: usize) -> Vec<Candle> {
        (0..len).map(|index| candle(index as u64)).collect()
    }

    fn series(values: Vec<Option<f64>>) -> ReadingSeries {
        ReadingSeries::new(values)
    }

    fn readings(candles: &[Candle], atr: Vec<Option<f64>>, rvol: Vec<Option<f64>>) -> Readings {
        let len = candles.len();
        Readings {
            ema20: series(vec![None; len]),
            ema50: series(vec![None; len]),
            ema200: series(vec![None; len]),
            rsi14: series(vec![None; len]),
            atr14: series(atr),
            rvol20: series(rvol),
            pivots: PivotSeries::compute(candles),
        }
    }

    fn atr_window(last: f64) -> Vec<Option<f64>> {
        let mut values = vec![Some(2.0); ATR_BASELINE_BARS];
        values.push(Some(last));
        values
    }

    fn rvol_window(last: f64) -> Vec<Option<f64>> {
        let mut values = vec![Some(1.0); ATR_BASELINE_BARS];
        values.push(Some(last));
        values
    }

    #[test]
    fn the_four_context_codes_follow_the_ratio_table() {
        for (last_atr, last_rvol, expected) in [
            (4.0, 1.5, EXPANDING_CODE),
            (4.0, 0.5, HIGH_RANGE_LOW_PARTICIPATION_CODE),
            (1.0, 1.5, SETTLING_CODE),
            (1.0, 0.5, COILING_CODE),
        ] {
            let candles = candles(ATR_BASELINE_BARS + 1);
            let readings = readings(&candles, atr_window(last_atr), rvol_window(last_rvol));
            let context = ContextSeries::compute(&readings);

            let bar = context.latest().expect("one bar");
            assert_eq!(bar.context_code, Some(expected));
            let atr_ratio = bar.atr_ratio.expect("an ATR ratio");
            let rvol_ratio = bar.rvol_ratio.expect("an RVOL ratio");
            assert!((atr_ratio - last_atr / ((19.0 * 2.0 + last_atr) / 20.0)).abs() <= 1e-12);
            assert_eq!(rvol_ratio, last_rvol);
        }
    }

    #[test]
    fn the_atr_ratio_is_null_before_twenty_values() {
        let candles = candles(ATR_BASELINE_BARS);
        let mut atr = vec![Some(2.0); ATR_BASELINE_BARS];
        atr[ATR_BASELINE_BARS - 2] = Some(2.0);
        let readings = readings(&candles, atr, vec![Some(1.0); ATR_BASELINE_BARS]);
        let context = ContextSeries::compute(&readings);

        assert_eq!(
            context.get(ATR_BASELINE_BARS - 2).expect("a bar").atr_ratio,
            None
        );
        assert_eq!(
            context.get(ATR_BASELINE_BARS - 1).expect("a bar").atr_ratio,
            Some(1.0)
        );
    }

    #[test]
    fn a_zero_baseline_gives_a_null_atr_ratio_and_code() {
        let candles = candles(ATR_BASELINE_BARS);
        let readings = readings(
            &candles,
            vec![Some(0.0); ATR_BASELINE_BARS],
            vec![Some(1.0); ATR_BASELINE_BARS],
        );
        let context = ContextSeries::compute(&readings);
        let bar = context.latest().expect("one bar");

        assert_eq!(bar.atr_ratio, None);
        assert_eq!(bar.rvol_ratio, Some(1.0));
        assert_eq!(bar.context_code, None);
    }

    #[test]
    fn a_missing_rvol_leaves_the_code_null_but_keeps_the_atr_ratio() {
        let candles = candles(ATR_BASELINE_BARS);
        let readings = readings(
            &candles,
            vec![Some(2.0); ATR_BASELINE_BARS],
            vec![None; ATR_BASELINE_BARS],
        );
        let context = ContextSeries::compute(&readings);
        let bar = context.latest().expect("one bar");

        assert_eq!(bar.atr_ratio, Some(1.0));
        assert_eq!(bar.rvol_ratio, None);
        assert_eq!(bar.context_code, None);
    }

    #[test]
    fn a_missing_atr_in_the_window_gives_a_null_atr_ratio() {
        let candles = candles(ATR_BASELINE_BARS);
        let mut atr = vec![Some(2.0); ATR_BASELINE_BARS];
        atr[0] = None;
        let readings = readings(&candles, atr, vec![Some(1.0); ATR_BASELINE_BARS]);
        let context = ContextSeries::compute(&readings);

        assert_eq!(context.latest().expect("one bar").atr_ratio, None);
    }

    #[test]
    fn an_empty_series_has_no_context() {
        let context = ContextSeries::compute(&Readings::compute(&[]));
        assert!(context.is_empty());
        assert_eq!(context.len(), 0);
        assert_eq!(context.latest(), None);
    }

    #[test]
    fn a_single_candle_has_no_context() {
        let candles = candles(1);
        let readings = readings(&candles, vec![Some(2.0)], vec![Some(1.0)]);
        let context = ContextSeries::compute(&readings);

        assert_eq!(context.len(), 1);
        assert_eq!(context.get(0).expect("one bar").atr_ratio, None);
        assert_eq!(context.get(0).expect("one bar").context_code, None);
    }

    #[test]
    fn later_candles_do_not_change_earlier_context() {
        let candles = candles(ATR_BASELINE_BARS + 3);
        let mut atr = vec![Some(2.0); candles.len()];
        atr[ATR_BASELINE_BARS] = Some(4.0);
        atr[ATR_BASELINE_BARS + 1] = Some(1.0);
        atr[ATR_BASELINE_BARS + 2] = Some(3.0);
        let mut rvol = vec![Some(1.0); candles.len()];
        rvol[ATR_BASELINE_BARS] = Some(2.0);
        rvol[ATR_BASELINE_BARS + 1] = Some(0.5);
        rvol[ATR_BASELINE_BARS + 2] = Some(1.5);
        let full_readings = readings(&candles, atr.clone(), rvol.clone());
        let full = ContextSeries::compute(&full_readings);

        for length in 0..=candles.len() {
            let prefix = &candles[..length];
            let prefix_readings = readings(prefix, atr[..length].to_vec(), rvol[..length].to_vec());
            let partial = ContextSeries::compute(&prefix_readings);
            assert_eq!(
                partial.bars(),
                &full.bars()[..length],
                "the prefix of length {length} changed an earlier context bar"
            );
        }
    }

    #[test]
    fn the_context_is_deterministic() {
        let candles = candles(ATR_BASELINE_BARS + 1);
        let readings = readings(&candles, atr_window(4.0), rvol_window(1.5));
        let first = ContextSeries::compute(&readings);
        let second = ContextSeries::compute(&readings);
        assert_eq!(first, second);
    }

    #[test]
    fn no_axis_reads_the_context() {
        let candles = candles(ATR_BASELINE_BARS + 1);
        let readings = readings(&candles, atr_window(4.0), rvol_window(1.5));
        let direction_before = DirectionSeries::compute(&readings);
        let momentum_before = MomentumSeries::compute(&readings);
        let structure_before = StructureSeries::compute(&candles, &readings);

        let _ = ContextSeries::compute(&readings);

        assert_eq!(direction_before, DirectionSeries::compute(&readings));
        assert_eq!(momentum_before, MomentumSeries::compute(&readings));
        assert_eq!(
            structure_before,
            StructureSeries::compute(&candles, &readings)
        );
    }
}
