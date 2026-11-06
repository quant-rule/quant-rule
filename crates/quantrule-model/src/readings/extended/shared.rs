//! Shared reading helpers.

use super::*;

pub(super) fn mean_volume(candles: &[Candle], bar: usize, window: usize) -> Option<f64> {
    if window == 0 || bar + 1 < window {
        return None;
    }
    let start = bar + 1 - window;
    let total: f64 = candles[start..=bar].iter().map(volume).sum();
    Some(total / window as f64)
}

pub(super) fn typical_price(candle: &Candle) -> f64 {
    (candle.high() + candle.low() + candle.close()) / 3.0
}

pub(super) fn volume(candle: &Candle) -> f64 {
    candle.volume().unwrap_or(0.0)
}
