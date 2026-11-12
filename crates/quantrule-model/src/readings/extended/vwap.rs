//! VWAP readings.

use super::Candle;
use super::shared::{typical_price, volume};

pub(super) fn vwap_series(
    candles: &[Candle],
    window: usize,
) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let len = candles.len();
    let mut vwap = vec![None; len];
    let mut mad = vec![None; len];
    if window == 0 || len < window {
        return (vwap, mad);
    }
    for bar in (window - 1)..len {
        let start = bar + 1 - window;
        let mut weighted = 0.0;
        let mut total = 0.0;
        for candle in &candles[start..=bar] {
            let price = typical_price(candle);
            let volume = volume(candle);
            weighted += price * volume;
            total += volume;
        }
        if total <= 0.0 {
            continue;
        }
        let center = weighted / total;
        vwap[bar] = Some(center);
        let mut deviation = 0.0;
        for candle in &candles[start..=bar] {
            deviation += (typical_price(candle) - center).abs();
        }
        mad[bar] = Some(deviation / window as f64);
    }
    (vwap, mad)
}
