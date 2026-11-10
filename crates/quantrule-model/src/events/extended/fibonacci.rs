//! Fibonacci retracement test events.
//!
//! A test is a bar-range intersection with a published retracement level within
//! the configured ATR tolerance. The event direction follows the leg direction,
//! not the bar direction.

use super::*;

/// Source code for Fibonacci retracements.
pub const FIBONACCI_SOURCE_CODE: u16 = 56;

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
    let mut tested: Option<(u32, f64, f64)> = None;
    for (reference_code, level) in ratios
        .into_iter()
        .zip(levels)
        .filter_map(|(code, level)| level.map(|level| (code, level)))
    {
        let touch = (candle.high() - level)
            .abs()
            .min((candle.low() - level).abs());
        if touch <= tolerance {
            let distance = (candle.close() - level).abs();
            if tested.is_none_or(|(_, _, best)| distance < best) {
                tested = Some((reference_code, level, distance));
            }
        }
    }
    if let Some((reference_code, level, _)) = tested {
        push_with_reference(
            events,
            codes::LEVEL_FIB_TEST,
            EventType::Level,
            1,
            FIBONACCI_SOURCE_CODE,
            config.timeframe_secs(),
            bar,
            if direction > 0.0 { 1 } else { -1 },
            Some(reference_code),
            Some(candle.close()),
            Some(level),
        );
    }
}
