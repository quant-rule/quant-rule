//! Horizontal support/resistance band test events.
//!
//! A test is a bar-range intersection with the selected multi-touch band. The
//! direction is descriptive: support when the close is at or above the center,
//! resistance when it is below.

use crate::events::codes;
pub use crate::events::source_codes::BAND_SOURCE_CODE;
use crate::events::{EventRecord, EventType};
use crate::readings::{ExtendedReadings, Readings};
use quantrule_data::Candle;

use super::{ExtendedEventConfig, push};

pub(super) fn band_events(
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
    let (Some(lower), Some(upper), Some(center)) = (
        extended.band_lower.get(bar),
        extended.band_upper.get(bar),
        extended.band_center.get(bar),
    ) else {
        return;
    };
    let tolerance = config.band_atr * atr;
    let candle = candles[bar];
    if candle.high() >= lower - tolerance && candle.low() <= upper + tolerance {
        push(
            events,
            codes::LEVEL_BAND_TEST,
            EventType::Level,
            1,
            BAND_SOURCE_CODE,
            config.timeframe_secs(),
            bar,
            if candle.close() >= center { 1 } else { -1 },
            Some(candle.close()),
            Some(center),
        );
    }
}
