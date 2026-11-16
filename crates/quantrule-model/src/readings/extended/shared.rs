//! Shared reading helpers.

use super::Candle;
pub(super) fn mean_volume(candles: &[Candle], bar: usize, window: usize) -> Option<f64> {
    if window == 0 || bar + 1 < window {
        return None;
    }
    let start = bar + 1 - window;
    let mut total = 0.0;
    for candle in &candles[start..=bar] {
        let volume = candle.volume()?;
        total += volume;
    }
    Some(total / window as f64)
}

pub(super) fn typical_price(candle: &Candle) -> f64 {
    (candle.high() + candle.low() + candle.close()) / 3.0
}

/// The candle volume, or `None` when the source did not supply one.
///
/// A missing volume is not a zero volume: callers must leave the reading
/// unavailable rather than substituting a value the market never produced.
pub(super) fn volume(candle: &Candle) -> Option<f64> {
    candle.volume()
}
