//! Average Directional Index and the directional movement indicators.
//!
//! The family is Wilder's DMI. ADX measures trend strength and is published as
//! context; `+DI` and `-DI` carry direction. The three series are aligned to the
//! input candles, and a value is `None` until its Wilder seed exists.
//!
//! The module is pure: it reads only the candles it is given, performs no I/O,
//! reads no clock, and uses no randomness.

use quantrule_data::Candle;

use super::ReadingSeries;

/// The configured ADX state at one bar.
///
/// ADX itself is non-directional. `adx_regime` says whether strength is at or
/// above the configured threshold; `di_state` supplies the direction. A missing
/// input produces `None`, never a zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdxState {
    /// 1 when ADX is at or above the threshold, 0 when below, `None` when ADX is
    /// unavailable.
    pub adx_regime: Option<i8>,
    /// Sign of `ADX(t) - ADX(t - slope_bars)`, or `None` when either value is
    /// unavailable.
    pub adx_slope: Option<i8>,
    /// Sign of `+DI - -DI`, or `None` when either DI is unavailable.
    pub di_state: Option<i8>,
}

/// One timeframe's ADX and directional-movement series.
///
/// `adx` is the Wilder-smoothed DX. `di_plus` and `di_minus` are the two
/// directional indicators. Every series has exactly one entry per input candle.
#[derive(Debug, Clone, PartialEq)]
pub struct AdxSeries {
    /// Wilder-smoothed ADX.
    pub adx: ReadingSeries,
    /// Positive directional indicator.
    pub di_plus: ReadingSeries,
    /// Negative directional indicator.
    pub di_minus: ReadingSeries,
}

impl AdxSeries {
    /// Computes the ADX/DI family with the configured period.
    ///
    /// The first true range and directional movement are at bar 1. The seed uses
    /// bars `1..=period`, so DI and DX first appear at bar `period`. ADX needs
    /// `period` DX values and first appears at bar `2 * period - 1`. A period of
    /// zero, or a series shorter than `2 * period`, returns all-null series.
    #[must_use]
    pub fn compute(candles: &[Candle], period: usize) -> Self {
        let len = candles.len();
        if period == 0 || len < period.saturating_mul(2) {
            return Self::empty(len);
        }

        let mut tr = vec![0.0; len];
        let mut plus_dm = vec![0.0; len];
        let mut minus_dm = vec![0.0; len];
        for index in 1..len {
            let current = candles[index];
            let previous = candles[index - 1];
            tr[index] = true_range(current, previous.close());
            let up = current.high() - previous.high();
            let down = previous.low() - current.low();
            if up > down && up > 0.0 {
                plus_dm[index] = up;
            }
            if down > up && down > 0.0 {
                minus_dm[index] = down;
            }
        }

        let smoothed_tr = wilder_series(&tr, period);
        let smoothed_plus = wilder_series(&plus_dm, period);
        let smoothed_minus = wilder_series(&minus_dm, period);

        let mut di_plus = vec![None; len];
        let mut di_minus = vec![None; len];
        let mut dx = vec![None; len];
        for index in period..len {
            let (Some(tr), Some(plus), Some(minus)) = (
                smoothed_tr[index],
                smoothed_plus[index],
                smoothed_minus[index],
            ) else {
                continue;
            };
            if tr <= 0.0 {
                continue;
            }
            let plus_di = 100.0 * plus / tr;
            let minus_di = 100.0 * minus / tr;
            di_plus[index] = Some(plus_di);
            di_minus[index] = Some(minus_di);
            let sum = plus_di + minus_di;
            if sum > 0.0 {
                dx[index] = Some(100.0 * (plus_di - minus_di).abs() / sum);
            }
        }

        let adx = adx_from_dx(&dx, period);
        Self {
            adx,
            di_plus: ReadingSeries::new(di_plus),
            di_minus: ReadingSeries::new(di_minus),
        }
    }

    /// An all-null family of the input length.
    #[must_use]
    pub fn empty(len: usize) -> Self {
        Self {
            adx: ReadingSeries::new(vec![None; len]),
            di_plus: ReadingSeries::new(vec![None; len]),
            di_minus: ReadingSeries::new(vec![None; len]),
        }
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.adx.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.adx.is_empty()
    }

    /// The ADX state at `bar`.
    ///
    /// The state is a pure read of the published series. It does not infer a
    /// value from a missing input.
    #[must_use]
    pub fn state(&self, bar: usize, threshold: f64, slope_bars: usize) -> AdxState {
        let adx = self.adx.get(bar);
        let adx_regime = adx.map(|value| i8::from(value >= threshold));
        let adx_slope = match (
            adx,
            bar.checked_sub(slope_bars)
                .and_then(|prior| self.adx.get(prior)),
        ) {
            (Some(current), Some(prior)) => Some(sign(current - prior)),
            _ => None,
        };
        let di_state = match (self.di_plus.get(bar), self.di_minus.get(bar)) {
            (Some(plus), Some(minus)) => Some(sign(plus - minus)),
            _ => None,
        };
        AdxState {
            adx_regime,
            adx_slope,
            di_state,
        }
    }
}

/// Wilder's smoothing of a non-negative series.
///
/// The seed at bar `period` is the mean of bars `1..=period`. Earlier entries
/// stay `None`. The recurrence is
/// `smoothed_t = (smoothed_(t-1) * (period - 1) + value_t) / period`.
fn wilder_series(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut smoothed = vec![None; values.len()];
    if period == 0 || values.len() <= period {
        return smoothed;
    }
    let mut sum = 0.0;
    for value in values.iter().take(period + 1).skip(1) {
        sum += *value;
    }
    let seed = sum / period as f64;
    smoothed[period] = Some(seed);
    let mut previous = seed;
    let divisor = period as f64;
    for (index, value) in values.iter().enumerate().skip(period + 1) {
        previous = (previous * (divisor - 1.0) + value) / divisor;
        smoothed[index] = Some(previous);
    }
    smoothed
}

/// Wilder's ADX smoothing of the DX series.
///
/// A null DX resets the seed window. ADX is published only after `period`
/// consecutive DX values, so a flat market stays null instead of receiving a
/// fabricated zero.
fn adx_from_dx(dx: &[Option<f64>], period: usize) -> ReadingSeries {
    let mut adx = vec![None; dx.len()];
    let mut seed_window: Vec<f64> = Vec::with_capacity(period);
    let mut previous: Option<f64> = None;
    for (index, value) in dx.iter().enumerate() {
        let Some(value) = value else {
            seed_window.clear();
            previous = None;
            continue;
        };
        match previous {
            Some(prior) => {
                let next = (prior * (period as f64 - 1.0) + value) / period as f64;
                adx[index] = Some(next);
                previous = Some(next);
            }
            None => {
                seed_window.push(*value);
                if seed_window.len() == period {
                    let seed = seed_window.iter().sum::<f64>() / period as f64;
                    adx[index] = Some(seed);
                    previous = Some(seed);
                    seed_window.clear();
                }
            }
        }
    }
    ReadingSeries::new(adx)
}

fn true_range(current: Candle, previous_close: f64) -> f64 {
    let mut range = current.high() - current.low();
    let high_gap = (current.high() - previous_close).abs();
    if high_gap > range {
        range = high_gap;
    }
    let low_gap = (current.low() - previous_close).abs();
    if low_gap > range {
        range = low_gap;
    }
    range
}

fn sign(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{AdxSeries, sign};

    fn candle(index: usize, open: f64, high: f64, low: f64, close: f64) -> Candle {
        let high = high.max(open).max(close);
        let low = low.min(open).min(close);
        Candle::new(
            index as u64,
            index as i64,
            open,
            high,
            low,
            close,
            Some(1.0),
        )
        .expect("the test candle is valid")
    }

    fn trend(len: usize, step: f64) -> Vec<Candle> {
        (0..len)
            .map(|index| {
                let close = 100.0 + index as f64 * step;
                candle(index, close - step, close + 0.5, close - 0.5, close)
            })
            .collect()
    }

    fn trend_after_range(len: usize, direction: f64) -> Vec<Candle> {
        let mut price = 100.0;
        (0..len)
            .map(|index| {
                if index < 30 {
                    let widening = 0.1 * index as f64;
                    return candle(index, 100.0, 101.0 + widening, 99.0 - widening, 100.0);
                }
                let step = direction * (0.5 + 0.02 * (index - 30) as f64);
                price += step;
                candle(index, price - step, price + 0.5, price - 0.5, price)
            })
            .collect()
    }

    fn range(len: usize) -> Vec<Candle> {
        (0..len)
            .map(|index| {
                let widening = if index % 2 == 0 { 0.0 } else { 0.5 };
                candle(index, 100.0, 101.0 + widening, 99.0 - widening, 100.0)
            })
            .collect()
    }

    fn assert_close(actual: Option<f64>, expected: f64) {
        let actual = actual.expect("the value is available");
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn an_uptrend_has_positive_direction_and_strength() {
        let candles = trend_after_range(60, 1.0);
        let series = AdxSeries::compute(&candles, 14);

        let state = series.state(59, 25.0, 3);
        assert_eq!(state.adx_regime, Some(1));
        assert_eq!(state.di_state, Some(1));
        assert!(series.adx.get(59).expect("ADX") > 25.0);
        assert!(series.di_plus.get(59).expect("+DI") > series.di_minus.get(59).expect("-DI"));
    }

    #[test]
    fn a_downtrend_has_negative_direction_and_strength() {
        let candles = trend_after_range(60, -1.0);
        let series = AdxSeries::compute(&candles, 14);

        let state = series.state(59, 25.0, 3);
        assert_eq!(state.adx_regime, Some(1));
        assert_eq!(state.di_state, Some(-1));
        assert!(series.di_minus.get(59).expect("-DI") > series.di_plus.get(59).expect("+DI"));
    }

    #[test]
    fn a_range_does_not_reach_the_trend_regime() {
        let candles = range(120);
        let series = AdxSeries::compute(&candles, 14);

        let state = series.state(59, 25.0, 3);
        assert_ne!(state.adx_regime, Some(1));
        assert!(series.adx.get(59).is_none_or(|value| value < 25.0));
    }

    #[test]
    fn a_flat_series_is_null_not_zero() {
        let candles = (0..60)
            .map(|index| candle(index, 100.0, 100.0, 100.0, 100.0))
            .collect::<Vec<_>>();
        let series = AdxSeries::compute(&candles, 14);

        assert_eq!(series.adx.get(59), None);
        assert_eq!(series.di_plus.get(59), None);
        assert_eq!(series.di_minus.get(59), None);
        assert_eq!(series.state(59, 25.0, 3).di_state, None);
    }

    #[test]
    fn the_first_adx_is_at_twice_the_period_minus_one() {
        let candles = trend(30, 1.0);
        let series = AdxSeries::compute(&candles, 14);

        assert_eq!(series.adx.get(26), None);
        assert!(series.adx.get(27).is_some());
        assert_eq!(series.di_plus.get(13), None);
        assert!(series.di_plus.get(14).is_some());
    }

    #[test]
    fn an_empty_or_short_series_is_all_null() {
        let series = AdxSeries::compute(&[], 14);
        assert!(series.is_empty());
        assert_eq!(series.adx.get(0), None);

        let candles = trend(10, 1.0);
        let series = AdxSeries::compute(&candles, 14);
        assert_eq!(series.adx.len(), 10);
        assert_eq!(series.adx.get(9), None);
    }

    #[test]
    fn a_zero_period_is_all_null() {
        let candles = trend(30, 1.0);
        let series = AdxSeries::compute(&candles, 0);
        assert_eq!(series.adx.get(29), None);
        assert_eq!(series.di_plus.get(29), None);
        assert_eq!(series.di_minus.get(29), None);
    }

    #[test]
    fn the_series_is_causal() {
        let candles = trend(80, 0.7);
        let full = AdxSeries::compute(&candles, 14);
        let prefix = AdxSeries::compute(&candles[..60], 14);

        for bar in 0..60 {
            assert_eq!(full.adx.get(bar), prefix.adx.get(bar));
            assert_eq!(full.di_plus.get(bar), prefix.di_plus.get(bar));
            assert_eq!(full.di_minus.get(bar), prefix.di_minus.get(bar));
        }
    }

    #[test]
    fn the_series_is_deterministic() {
        let candles = trend(80, 0.3);
        let first = AdxSeries::compute(&candles, 14);
        let second = AdxSeries::compute(&candles, 14);
        assert_eq!(first, second);
    }

    #[test]
    fn the_state_reads_the_configured_slope() {
        use super::{AdxSeries, ReadingSeries};
        let series = AdxSeries {
            adx: ReadingSeries::new(vec![Some(20.0), Some(22.0), Some(24.0), Some(26.0)]),
            di_plus: ReadingSeries::new(vec![Some(30.0); 4]),
            di_minus: ReadingSeries::new(vec![Some(10.0); 4]),
        };
        let state = series.state(3, 25.0, 2);
        assert_eq!(state.adx_regime, Some(1));
        assert_eq!(state.adx_slope, Some(1));
        assert_eq!(state.di_state, Some(1));
    }

    #[test]
    fn the_slope_uses_the_prior_bar_only() {
        let candles = trend(40, 1.0);
        let series = AdxSeries::compute(&candles, 14);
        let state = series.state(39, 25.0, 3);
        let current = series.adx.get(39).expect("current ADX");
        let prior = series.adx.get(36).expect("prior ADX");
        assert_eq!(state.adx_slope, Some(sign(current - prior)));
    }

    #[test]
    fn a_zero_range_keeps_di_zero_when_true_range_is_positive() {
        let candles = trend(40, 0.0);
        let series = AdxSeries::compute(&candles, 14);
        assert_close(series.di_plus.get(39), 0.0);
        assert_close(series.di_minus.get(39), 0.0);
        assert_eq!(series.adx.get(39), None);
    }
}
