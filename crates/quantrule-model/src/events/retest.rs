//! Breakout retest-hold events.
//!
//! A compression breakout is a break confirmation. This detector adds the
//! second confirmation: price returns to the broken boundary within a fixed
//! window, touches it, and closes back on the breakout side. The event is
//! appended only at the hold bar, so it is causal and never anticipates the
//! retest.
//!
//! The break event itself is produced by [`CompressionBreakoutSeries`]. This
//! module consumes those records and emits one retest event per break. The
//! retest window and tolerance are fixed by the contract; they are not strategy
//! parameters.
//!
//! The module is pure: it reads candles, readings, and the break records. It
//! performs no I/O, reads no clock, and uses no randomness.

use quantrule_data::Candle;

use crate::events::codes::SIGNAL_BREAKOUT_RETEST_HOLD;
use crate::events::compression::CompressionBreakoutSeries;
use crate::events::signal::SignalSubtype;
use crate::events::{EventRecord, EventType};
use crate::readings::Readings;

/// The number of bars after the break in which the retest may hold.
pub const RETEST_BARS: usize = 4;

/// The retest tolerance in ATR units.
pub const RETEST_ATR: f64 = 0.2;

/// The breakout-retest-hold events over one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakoutRetestHoldSeries {
    events: Vec<EventRecord>,
}

impl BreakoutRetestHoldSeries {
    /// An empty event series.
    #[must_use]
    pub const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Computes the retest events from the compression breakouts.
    ///
    /// A break at `b0` is followed by a retest when a bar in
    /// `b0 + 1..=b0 + RETEST_BARS` touches the broken boundary within
    /// `RETEST_ATR * ATR` and closes back on the breakout side. A close through
    /// the boundary against the breakout before the hold invalidates the break.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        readings: &Readings,
        breakouts: &CompressionBreakoutSeries,
    ) -> Self {
        Self::from_breakouts(candles, readings, breakouts.events())
    }

    /// Computes the retest events from explicit break records.
    ///
    /// This is the testable seam; production callers pass the compression
    /// detector's records. A break with no reference boundary or a zero
    /// direction is ignored.
    #[must_use]
    pub fn from_breakouts(
        candles: &[Candle],
        readings: &Readings,
        breakouts: &[EventRecord],
    ) -> Self {
        let mut events = Vec::new();
        for breakout in breakouts {
            if breakout.direction_code == 0 {
                continue;
            }
            let Some(boundary) = breakout.reference_value else {
                continue;
            };
            if !boundary.is_finite() {
                continue;
            }
            let start = breakout.occurred_bar_index.saturating_add(1);
            let end = start
                .saturating_add(RETEST_BARS.saturating_sub(1))
                .min(candles.len().saturating_sub(1));
            for (bar, candle) in candles.iter().enumerate().take(end + 1).skip(start) {
                let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
                    continue;
                };
                let candle = *candle;
                let tolerance = RETEST_ATR * atr;
                let long = breakout.direction_code > 0;
                if long && candle.close() < boundary {
                    break;
                }
                if !long && candle.close() > boundary {
                    break;
                }
                let touched =
                    candle.low() <= boundary + tolerance && candle.high() >= boundary - tolerance;
                if !touched {
                    continue;
                }
                let held = if long {
                    candle.close() > boundary
                } else {
                    candle.close() < boundary
                };
                if held {
                    events.push(EventRecord {
                        event_code: SIGNAL_BREAKOUT_RETEST_HOLD,
                        event_type: EventType::Signal,
                        event_subtype_code: SignalSubtype::BreakoutRetestHold.code(),
                        source_code: breakout.source_code,
                        timeframe_secs: breakout.timeframe_secs,
                        occurred_bar_index: bar,
                        available_bar_index: bar,
                        direction_code: breakout.direction_code,
                        reference_code: None,
                        primary_value: Some(candle.close()),
                        reference_value: Some(boundary),
                    });
                    break;
                }
            }
        }
        Self { events }
    }

    /// The events, in break order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// Whether the series has no events.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use quantrule_data::Candle;

    use super::{BreakoutRetestHoldSeries, RETEST_ATR};
    use crate::context::ContextSeries;
    use crate::events::codes::SIGNAL_BREAKOUT_RETEST_HOLD;
    use crate::events::compression::{CompressionBreakoutConfig, CompressionBreakoutSeries};
    use crate::events::{EventRecord, EventType};
    use crate::readings::{PivotSeries, Readings};

    fn candle(index: u64, high: f64, low: f64, close: f64) -> Candle {
        Candle::new(
            index,
            index as i64 * 300,
            close,
            high,
            low,
            close,
            Some(1.0),
        )
        .expect("the test candle is valid")
    }

    fn series(len: usize, value: impl Fn(usize) -> Option<f64>) -> crate::readings::ReadingSeries {
        crate::readings::ReadingSeries::new((0..len).map(value).collect())
    }

    fn fixture() -> (Vec<Candle>, Readings, ContextSeries) {
        let mut candles = Vec::new();
        for index in 0..30 {
            let close = if index == 25 { 101.0 } else { 100.0 };
            let high = if index == 25 { 101.2 } else { 100.5 };
            let low = if index == 25 { 100.8 } else { 99.5 };
            candles.push(candle(index as u64, high, low, close));
        }
        let readings = Readings {
            ema20: series(30, |_| Some(100.0)),
            ema50: series(30, |_| Some(100.0)),
            ema200: series(30, |_| Some(100.0)),
            rsi14: series(30, |_| Some(50.0)),
            atr14: series(30, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
            rvol20: series(30, |index| Some(if index < 25 { 0.5 } else { 1.5 })),
            pivots: PivotSeries::compute(&[]),
        };
        let context = ContextSeries::compute(&readings);
        (candles, readings, context)
    }

    fn compression() -> CompressionBreakoutConfig {
        CompressionBreakoutConfig::with_settings(
            NonZeroU64::new(300).expect("a non-zero timeframe"),
            5,
            0.2,
            1.2,
        )
    }

    fn breakout(direction: i8, boundary: f64, bar: usize) -> EventRecord {
        EventRecord {
            event_code: crate::events::codes::SIGNAL_COMPRESSION_BREAKOUT,
            event_type: EventType::Signal,
            event_subtype_code: 7,
            source_code: 5,
            timeframe_secs: 300,
            occurred_bar_index: bar,
            available_bar_index: bar,
            direction_code: direction,
            reference_code: None,
            primary_value: Some(if direction > 0 {
                boundary + 0.5
            } else {
                boundary - 0.5
            }),
            reference_value: Some(boundary),
        }
    }

    #[test]
    fn breakout_retest_hold_row_contract() {
        let (mut candles, readings, _) = fixture();
        // A retest of the 100.5 boundary one bar after the break.
        candles[26] = candle(26, 101.1, 100.6, 101.0);
        let series = BreakoutRetestHoldSeries::from_breakouts(
            &candles,
            &readings,
            &[breakout(1, 100.5, 25)],
        );
        assert_eq!(series.events().len(), 1);
        let event = series.events()[0];
        assert_eq!(event.event_code, SIGNAL_BREAKOUT_RETEST_HOLD);
        assert_eq!(event.event_type, EventType::Signal);
        assert_eq!(event.event_subtype_code, 24);
        assert_eq!(event.source_code, 5);
        assert_eq!(event.direction_code, 1);
        assert_eq!(event.occurred_bar_index, 26);
        assert_eq!(event.available_bar_index, 26);
        assert_eq!(event.reference_value, Some(100.5));
        assert!(event.primary_value.is_some_and(|value| value > 100.5));
    }

    #[test]
    fn a_failed_retest_emits_nothing_and_a_close_through_invalidates() {
        let (mut candles, readings, _) = fixture();
        // The retest bar closes below the boundary: invalidated.
        candles[26] = candle(26, 100.9, 100.2, 100.4);
        let series = BreakoutRetestHoldSeries::from_breakouts(
            &candles,
            &readings,
            &[breakout(1, 100.5, 25)],
        );
        assert!(series.is_empty());
    }

    #[test]
    fn a_bar_that_never_touches_does_not_hold() {
        let (mut candles, readings, _) = fixture();
        candles[26] = candle(26, 101.5, 101.1, 101.3);
        let series = BreakoutRetestHoldSeries::from_breakouts(
            &candles,
            &readings,
            &[breakout(1, 100.5, 25)],
        );
        assert!(series.is_empty());
    }

    #[test]
    fn the_short_side_is_symmetric() {
        let (mut candles, readings, _) = fixture();
        candles[26] = candle(26, 100.4, 99.9, 100.0);
        let series = BreakoutRetestHoldSeries::from_breakouts(
            &candles,
            &readings,
            &[breakout(-1, 100.5, 25)],
        );
        assert_eq!(series.events().len(), 1);
        assert_eq!(series.events()[0].direction_code, -1);
    }

    #[test]
    fn a_late_retest_is_outside_the_window() {
        let (mut candles, readings, _) = fixture();
        candles[29] = candle(29, 101.1, 100.6, 101.0);
        let series = BreakoutRetestHoldSeries::from_breakouts(
            &candles,
            &readings,
            &[breakout(1, 100.5, 24)],
        );
        assert!(series.is_empty());
    }

    #[test]
    fn the_event_is_causal_and_deterministic() {
        let (mut candles, readings, _) = fixture();
        candles[26] = candle(26, 101.1, 100.6, 101.0);
        let breakouts = [breakout(1, 100.5, 25)];
        let full = BreakoutRetestHoldSeries::from_breakouts(&candles, &readings, &breakouts);
        for length in 27..=candles.len() {
            let prefix =
                BreakoutRetestHoldSeries::from_breakouts(&candles[..length], &readings, &breakouts);
            assert_eq!(prefix.events(), full.events());
        }
        let repeated = BreakoutRetestHoldSeries::from_breakouts(&candles, &readings, &breakouts);
        assert_eq!(full.events(), repeated.events());
    }

    #[test]
    fn the_tolerance_is_the_contract_value() {
        assert!((RETEST_ATR - 0.2).abs() < f64::EPSILON);
    }

    #[test]
    fn a_full_compression_breakout_feeds_the_retest() {
        let (mut candles, readings, context) = fixture();
        candles[26] = candle(26, 101.1, 100.6, 101.0);
        let breakouts =
            CompressionBreakoutSeries::compute(&candles, &readings, &context, &compression());
        let retests = BreakoutRetestHoldSeries::compute(&candles, &readings, &breakouts);
        assert_eq!(retests.events().len(), 1);
        assert_eq!(retests.events()[0].event_subtype_code, 24);
    }

    #[test]
    fn an_empty_series_emits_nothing() {
        let readings = Readings::compute(&[]);
        let series = BreakoutRetestHoldSeries::from_breakouts(&[], &readings, &[]);
        assert!(series.is_empty());
    }
}
