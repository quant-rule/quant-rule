//! Conditional trend-continuation events.
//!
//! The event is a conjunction of published readings, not a weighted score:
//! ADX must be in its trend regime and rising, DI must have a direction, the EMA
//! stack must agree, and the MACD state must agree. The model emits one event
//! when the conjunction becomes true, not on every bar it remains true.
//!
//! The module is pure: it reads the published readings and MACD/ADX series. It
//! performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use crate::events::dmi::ADX_SOURCE_CODE;
use crate::events::signal::SignalSubtype;
use crate::events::{EventRecord, EventType};
use crate::readings::{AdxSeries, MacdSeries, Readings};

/// The configuration of a trend-continuation event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrendContinuationConfig {
    timeframe_secs: NonZeroU64,
    adx_threshold: f64,
    adx_slope_bars: usize,
}

impl TrendContinuationConfig {
    /// Builds a configuration from a non-zero timeframe and ADX parameters.
    #[must_use]
    pub const fn new(
        timeframe_secs: NonZeroU64,
        adx_threshold: f64,
        adx_slope_bars: usize,
    ) -> Self {
        Self {
            timeframe_secs,
            adx_threshold,
            adx_slope_bars,
        }
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }

    /// The ADX threshold.
    #[must_use]
    pub const fn adx_threshold(self) -> f64 {
        self.adx_threshold
    }

    /// The ADX slope lookback.
    #[must_use]
    pub const fn adx_slope_bars(self) -> usize {
        self.adx_slope_bars
    }
}

/// The trend-continuation events detected over one readings set.
#[derive(Debug, Clone, PartialEq)]
pub struct TrendContinuationSeries {
    events: Vec<EventRecord>,
}

impl TrendContinuationSeries {
    /// Computes the trend-continuation events.
    ///
    /// An event is emitted on the first bar of each continuous conjunction. A
    /// missing reading, a neutral DI state, or a disagreement produces no event.
    #[must_use]
    pub fn compute(
        readings: &Readings,
        macd: &MacdSeries,
        adx: &AdxSeries,
        config: &TrendContinuationConfig,
    ) -> Self {
        let len = readings.len().min(macd.len()).min(adx.len());
        let mut events = Vec::new();
        let mut previous = false;
        for bar in 0..len {
            let active = conjunction(readings, macd, adx, bar, config);
            if !previous && let Some(direction) = active {
                events.push(EventRecord {
                    event_type: EventType::Signal,
                    event_subtype_code: SignalSubtype::TrendContinuation.code(),
                    source_code: ADX_SOURCE_CODE,
                    timeframe_secs: config.timeframe_secs(),
                    occurred_bar_index: bar,
                    available_bar_index: bar,
                    direction_code: direction,
                    reference_code: None,
                    primary_value: adx.adx.get(bar),
                    reference_value: Some(config.adx_threshold),
                });
            }
            previous = active.is_some();
        }
        Self { events }
    }

    /// Every detected event, in bar order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// The number of detected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// The direction of the conjunction at `bar`, or `None` when it does not hold.
fn conjunction(
    readings: &Readings,
    macd: &MacdSeries,
    adx: &AdxSeries,
    bar: usize,
    config: &TrendContinuationConfig,
) -> Option<i8> {
    let state = adx.state(bar, config.adx_threshold, config.adx_slope_bars);
    if state.adx_regime != Some(1) || state.adx_slope != Some(1) {
        return None;
    }
    let direction = state.di_state?;
    if direction == 0 {
        return None;
    }
    if ema_stack(readings, bar) != Some(direction) {
        return None;
    }
    if macd_state(macd, bar) != Some(direction) {
        return None;
    }
    Some(direction)
}

fn ema_stack(readings: &Readings, bar: usize) -> Option<i8> {
    let fast = readings.ema20.get(bar)?;
    let mid = readings.ema50.get(bar)?;
    let slow = readings.ema200.get(bar)?;
    if fast > mid && mid > slow {
        Some(1)
    } else if fast < mid && mid < slow {
        Some(-1)
    } else {
        Some(0)
    }
}

fn macd_state(macd: &MacdSeries, bar: usize) -> Option<i8> {
    let line = macd.line.get(bar)?;
    let signal = macd.signal.get(bar)?;
    if line > signal {
        Some(1)
    } else if line < signal {
        Some(-1)
    } else {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use quantrule_data::Candle;

    use super::{TrendContinuationConfig, TrendContinuationSeries};
    use crate::readings::{AdxSeries, MacdSeries, Readings};

    fn candle(index: usize, close: f64, step: f64) -> Candle {
        let open = close - step;
        let high = (close + 0.5).max(open);
        let low = (close - 0.5).min(open);
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

    fn trend_after_range(len: usize, direction: f64) -> Vec<Candle> {
        let mut price = 100.0;
        (0..len)
            .map(|index| {
                if index < 50 {
                    let widening = 0.1 * index as f64;
                    let high = 101.0 + widening;
                    let low = 99.0 - widening;
                    return Candle::new(
                        index as u64,
                        index as i64,
                        100.0,
                        high,
                        low,
                        100.0,
                        Some(1.0),
                    )
                    .expect("the test candle is valid");
                }
                let step = direction * (0.3 + 0.005 * (index - 50) as f64);
                price += step;
                candle(index, price, step)
            })
            .collect()
    }

    fn config() -> TrendContinuationConfig {
        TrendContinuationConfig::new(NonZeroU64::new(300).expect("non-zero timeframe"), 25.0, 3)
    }

    #[test]
    fn an_aligned_uptrend_emits_a_continuation_event() {
        use crate::readings::{PivotSeries, ReadingSeries};

        let len = 20;
        let candles = (0..len)
            .map(|index| candle(index, 100.0 + index as f64, 1.0))
            .collect::<Vec<_>>();
        let series = |values: Vec<Option<f64>>| ReadingSeries::new(values);
        let readings = Readings {
            ema20: series((0..len).map(|_| Some(3.0)).collect()),
            ema50: series((0..len).map(|_| Some(2.0)).collect()),
            ema200: series((0..len).map(|_| Some(1.0)).collect()),
            rsi14: series(vec![None; len]),
            atr14: series(vec![None; len]),
            rvol20: series(vec![None; len]),
            pivots: PivotSeries::compute(&candles),
        };
        let macd = MacdSeries {
            line: series((0..len).map(|_| Some(1.0)).collect()),
            signal: series((0..len).map(|_| Some(0.0)).collect()),
            histogram: series((0..len).map(|_| Some(1.0)).collect()),
        };
        let adx = AdxSeries {
            adx: series(
                (0..len)
                    .map(|bar| Some(if bar < 10 { 20.0 } else { 30.0 }))
                    .collect(),
            ),
            di_plus: series((0..len).map(|_| Some(30.0)).collect()),
            di_minus: series((0..len).map(|_| Some(10.0)).collect()),
        };
        let events = TrendContinuationSeries::compute(&readings, &macd, &adx, &config());

        assert_eq!(events.len(), 1);
        let first = events.events().first().expect("an event");
        assert_eq!(first.direction_code, 1);
        assert_eq!(first.occurred_bar_index, 10);
        assert_eq!(first.available_bar_index, 10);
        assert!(first.primary_value.expect("ADX value") >= 25.0);
    }

    #[test]
    fn a_range_emits_no_continuation_event() {
        let candles = (0..300)
            .map(|index| {
                let close = if index % 2 == 0 { 100.0 } else { 100.5 };
                candle(index, close, 0.0)
            })
            .collect::<Vec<_>>();
        let readings = Readings::compute(&candles);
        let macd = MacdSeries::compute(&candles, 12, 26, 9);
        let adx = AdxSeries::compute(&candles, 14);
        let events = TrendContinuationSeries::compute(&readings, &macd, &adx, &config());

        assert!(events.is_empty());
    }

    #[test]
    fn the_event_is_causal() {
        let candles = trend_after_range(300, 0.8);
        let full_readings = Readings::compute(&candles);
        let full_macd = MacdSeries::compute(&candles, 12, 26, 9);
        let full_adx = AdxSeries::compute(&candles, 14);
        let full =
            TrendContinuationSeries::compute(&full_readings, &full_macd, &full_adx, &config());

        let prefix = &candles[..250];
        let prefix_readings = Readings::compute(prefix);
        let prefix_macd = MacdSeries::compute(prefix, 12, 26, 9);
        let prefix_adx = AdxSeries::compute(prefix, 14);
        let prefix_events = TrendContinuationSeries::compute(
            &prefix_readings,
            &prefix_macd,
            &prefix_adx,
            &config(),
        );

        let full_prefix: Vec<_> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < 250)
            .copied()
            .collect();
        assert_eq!(full_prefix, prefix_events.events());
    }

    #[test]
    fn a_disagreement_emits_no_event() {
        let candles = trend_after_range(300, 1.0);
        let readings = Readings::compute(&candles);
        let macd = MacdSeries::compute(&candles, 12, 26, 9);
        let adx = AdxSeries::compute(&candles, 14);
        let config = TrendContinuationConfig::new(
            NonZeroU64::new(300).expect("non-zero timeframe"),
            101.0,
            3,
        );
        let events = TrendContinuationSeries::compute(&readings, &macd, &adx, &config);
        assert!(events.is_empty());
    }
}
