//! Fibonacci retracement test events.
//!
//! A test is a bar-range intersection with a published retracement level within
//! the configured ATR tolerance. The event direction follows the leg direction,
//! not the bar direction.

use super::{
    Candle, EventRecord, EventType, ExtendedEventConfig, ExtendedReadings, Readings, codes,
    push_with_reference,
};
pub use crate::events::source_codes::FIBONACCI_SOURCE_CODE;

pub(super) fn fibonacci_events(
    candles: &[Candle],
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
        return;
    };
    let Some(direction) = extended.fib_direction.get(bar) else {
        return;
    };
    let tolerance = config.fib_test_atr * atr;
    let candle = candles[bar];
    let ratios = [236_u32, 382, 500, 618, 786];
    let levels = [
        extended.fib_236.get(bar),
        extended.fib_382.get(bar),
        extended.fib_500.get(bar),
        extended.fib_618.get(bar),
        extended.fib_786.get(bar),
    ];
    let event_direction = if direction > 0.0 { 1 } else { -1 };
    for (reference_code, level) in ratios
        .into_iter()
        .zip(levels)
        .filter_map(|(code, level)| level.map(|level| (code, level)))
    {
        // A test is a bar-range intersection with the level tolerance band.
        // Testing only the high and low misses a level the bar traded through
        // and closed on; the band test uses the same intersection definition.
        if candle.high() >= level - tolerance && candle.low() <= level + tolerance {
            push_with_reference(
                events,
                codes::LEVEL_FIB_TEST,
                EventType::Level,
                1,
                FIBONACCI_SOURCE_CODE,
                config.timeframe_secs(),
                bar,
                event_direction,
                Some(reference_code),
                Some(candle.close()),
                Some(level),
            );
        }
    }
}
