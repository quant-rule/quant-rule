//! Directional Movement Index crossover events.
//!
//! A DI crossover records the bar on which `+DI` and `-DI` change order. The
//! event is a Signal event because it is a discrete change in a reading; ADX
//! itself remains context and never supplies a direction.
//!
//! The module is pure: it reads the published ADX/DI series and the timeframe it
//! is given. It performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use crate::events::signal::SignalSubtype;
use crate::events::{EventRecord, EventType};
use crate::readings::AdxSeries;

/// The contract's source code for `+DI`.
pub const DI_PLUS_SOURCE_CODE: u16 = 17;
/// The contract's source code for `-DI`.
pub const DI_MINUS_SOURCE_CODE: u16 = 18;
/// The contract's source code for ADX.
pub const ADX_SOURCE_CODE: u16 = 16;

/// The configuration of a DI-event run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DmiConfig {
    timeframe_secs: NonZeroU64,
}

impl DmiConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self { timeframe_secs }
    }

    /// Builds a configuration from a timeframe in seconds.
    #[must_use]
    pub fn from_secs(timeframe_secs: u64) -> Option<Self> {
        NonZeroU64::new(timeframe_secs).map(Self::new)
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }
}

/// The DI crossover events detected over one ADX/DI series.
#[derive(Debug, Clone, PartialEq)]
pub struct DmiEventSeries {
    events: Vec<EventRecord>,
}

impl DmiEventSeries {
    /// An empty event series.
    #[must_use]
    pub const fn empty() -> Self {
        Self { events: Vec::new() }
    }

    /// Computes every DI crossover event.
    ///
    /// The first possible event is at bar 1 because a crossover needs the prior
    /// bar. Equality on the prior bar counts as a side; equality on the current
    /// bar is not a crossover.
    #[must_use]
    pub fn compute(adx: &AdxSeries, config: &DmiConfig) -> Self {
        let mut events = Vec::new();
        for bar in 1..adx.len() {
            let (
                Some(previous_plus),
                Some(previous_minus),
                Some(current_plus),
                Some(current_minus),
            ) = (
                adx.di_plus.get(bar - 1),
                adx.di_minus.get(bar - 1),
                adx.di_plus.get(bar),
                adx.di_minus.get(bar),
            )
            else {
                continue;
            };
            let direction = if previous_plus <= previous_minus && current_plus > current_minus {
                1
            } else if previous_plus >= previous_minus && current_plus < current_minus {
                -1
            } else {
                continue;
            };
            events.push(EventRecord {
                event_type: EventType::Signal,
                event_subtype_code: SignalSubtype::SeriesCrossover.code(),
                source_code: DI_PLUS_SOURCE_CODE,
                timeframe_secs: config.timeframe_secs(),
                occurred_bar_index: bar,
                available_bar_index: bar,
                direction_code: direction,
                reference_code: Some(u32::from(DI_MINUS_SOURCE_CODE)),
                primary_value: Some(current_plus),
                reference_value: Some(current_minus),
            });
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use quantrule_data::Candle;

    use super::{DmiConfig, DmiEventSeries};
    use crate::readings::AdxSeries;

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

    fn trend(len: usize, step: f64) -> Vec<Candle> {
        (0..len)
            .map(|index| {
                let close = 100.0 + index as f64 * step;
                candle(index, close, step)
            })
            .collect()
    }

    fn config() -> DmiConfig {
        DmiConfig::new(NonZeroU64::new(300).expect("non-zero timeframe"))
    }

    #[test]
    fn a_trend_without_a_cross_has_no_event() {
        let candles = trend(60, 1.0);
        let adx = AdxSeries::compute(&candles, 14);
        let events = DmiEventSeries::compute(&adx, &config());
        assert!(events.is_empty());
    }

    #[test]
    fn a_direction_change_emits_one_cross() {
        let mut candles = trend(40, 1.0);
        candles.extend((40..80).map(|index| {
            let close = 140.0 - (index as f64 - 39.0);
            candle(index, close, 1.0)
        }));
        let adx = AdxSeries::compute(&candles, 14);
        let events = DmiEventSeries::compute(&adx, &config());
        assert!(!events.is_empty());
        let event = events.events().last().expect("an event");
        assert_eq!(event.direction_code, -1);
        assert_eq!(event.available_bar_index, event.occurred_bar_index);
        assert_eq!(event.timeframe_secs, 300);
    }

    #[test]
    fn a_flat_series_has_no_event() {
        let candles = (0..60)
            .map(|index| candle(index, 100.0, 0.0))
            .collect::<Vec<_>>();
        let adx = AdxSeries::compute(&candles, 14);
        let events = DmiEventSeries::compute(&adx, &config());
        assert!(events.is_empty());
    }

    #[test]
    fn the_series_is_causal() {
        let full = trend(100, 0.8);
        let prefix = &full[..70];
        let full_events = DmiEventSeries::compute(&AdxSeries::compute(&full, 14), &config());
        let prefix_events = DmiEventSeries::compute(&AdxSeries::compute(prefix, 14), &config());
        assert_eq!(full_events.events(), prefix_events.events());
    }
}
