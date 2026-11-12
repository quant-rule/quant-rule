//! Causal Fibonacci retracement readings.
//!
//! The leg is the latest confirmed major pivot and the most recent confirmed
//! major pivot of the opposite kind before it. A candidate pivot never changes a
//! level, and the confirmation delay is inherited from the pivot series.

use super::{Candle, PivotKind, PivotScale, Readings};
const RATIOS: [f64; 5] = [0.236, 0.382, 0.5, 0.618, 0.786];

pub(super) struct FibonacciReadings {
    pub(super) direction: Vec<Option<f64>>,
    pub(super) low: Vec<Option<f64>>,
    pub(super) high: Vec<Option<f64>>,
    pub(super) level_236: Vec<Option<f64>>,
    pub(super) level_382: Vec<Option<f64>>,
    pub(super) level_500: Vec<Option<f64>>,
    pub(super) level_618: Vec<Option<f64>>,
    pub(super) level_786: Vec<Option<f64>>,
    pub(super) position: Vec<Option<f64>>,
    pub(super) nearest_ratio: Vec<Option<f64>>,
    pub(super) nearest_atr: Vec<Option<f64>>,
}

pub(super) fn fibonacci_series(candles: &[Candle], readings: &Readings) -> FibonacciReadings {
    let len = candles.len();
    let null = || vec![None; len];
    let mut result = FibonacciReadings {
        direction: null(),
        low: null(),
        high: null(),
        level_236: null(),
        level_382: null(),
        level_500: null(),
        level_618: null(),
        level_786: null(),
        position: null(),
        nearest_ratio: null(),
        nearest_atr: null(),
    };
    if len == 0 {
        return result;
    }
    let mut last_high: Option<f64> = None;
    let mut last_low: Option<f64> = None;
    let mut leg_low: Option<f64> = None;
    let mut leg_high: Option<f64> = None;
    let mut leg_direction: Option<f64> = None;
    for (bar, candle) in candles.iter().enumerate() {
        if let Some(pivot) = readings.pivots.confirmed_at(PivotScale::Major, bar) {
            match pivot.kind {
                PivotKind::High => {
                    last_high = Some(pivot.price);
                    if let Some(low) = last_low {
                        leg_low = Some(low);
                        leg_high = Some(pivot.price);
                        leg_direction = Some(1.0);
                    }
                }
                PivotKind::Low => {
                    last_low = Some(pivot.price);
                    if let Some(high) = last_high {
                        leg_low = Some(pivot.price);
                        leg_high = Some(high);
                        leg_direction = Some(-1.0);
                    }
                }
            }
        }
        let (Some(low), Some(high), Some(direction)) = (leg_low, leg_high, leg_direction) else {
            continue;
        };
        if high <= low {
            continue;
        }
        let span = high - low;
        let close = candle.close();
        let levels = if direction > 0.0 {
            [
                high - RATIOS[0] * span,
                high - RATIOS[1] * span,
                high - RATIOS[2] * span,
                high - RATIOS[3] * span,
                high - RATIOS[4] * span,
            ]
        } else {
            [
                low + RATIOS[0] * span,
                low + RATIOS[1] * span,
                low + RATIOS[2] * span,
                low + RATIOS[3] * span,
                low + RATIOS[4] * span,
            ]
        };
        result.direction[bar] = Some(direction);
        result.low[bar] = Some(low);
        result.high[bar] = Some(high);
        result.level_236[bar] = Some(levels[0]);
        result.level_382[bar] = Some(levels[1]);
        result.level_500[bar] = Some(levels[2]);
        result.level_618[bar] = Some(levels[3]);
        result.level_786[bar] = Some(levels[4]);
        result.position[bar] = Some(if direction > 0.0 {
            (close - low) / span
        } else {
            (high - close) / span
        });
        let mut nearest = (f64::INFINITY, RATIOS[0]);
        for (ratio, level) in RATIOS.iter().zip(levels) {
            let distance = (close - level).abs();
            if distance < nearest.0 {
                nearest = (distance, *ratio);
            }
        }
        result.nearest_ratio[bar] = Some(nearest.1);
        if let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) {
            result.nearest_atr[bar] = Some(nearest.0 / atr);
        }
    }
    result
}
