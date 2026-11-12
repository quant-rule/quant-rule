//! Candle anatomy and path readings.

use super::{Candle, ExtendedConfig, Readings};
pub(super) struct AnatomyReadings {
    pub(super) body_ratio: Vec<Option<f64>>,
    pub(super) upper_wick_ratio: Vec<Option<f64>>,
    pub(super) lower_wick_ratio: Vec<Option<f64>>,
    pub(super) close_position: Vec<Option<f64>>,
    pub(super) pattern_code: Vec<Option<f64>>,
    pub(super) pattern_direction: Vec<Option<f64>>,
    pub(super) gap_atr: Vec<Option<f64>>,
    pub(super) gap_filled: Vec<Option<f64>>,
    pub(super) path_efficiency: Vec<Option<f64>>,
    pub(super) direction_changes: Vec<Option<f64>>,
    pub(super) bars_since_high: Vec<Option<f64>>,
    pub(super) bars_since_low: Vec<Option<f64>>,
    pub(super) drawdown_atr: Vec<Option<f64>>,
    pub(super) range_position: Vec<Option<f64>>,
}

pub(super) fn anatomy_series(
    candles: &[Candle],
    readings: &Readings,
    config: &ExtendedConfig,
) -> AnatomyReadings {
    let len = candles.len();
    let null = || vec![None; len];
    let mut result = AnatomyReadings {
        body_ratio: null(),
        upper_wick_ratio: null(),
        lower_wick_ratio: null(),
        close_position: null(),
        pattern_code: null(),
        pattern_direction: null(),
        gap_atr: null(),
        gap_filled: null(),
        path_efficiency: null(),
        direction_changes: null(),
        bars_since_high: null(),
        bars_since_low: null(),
        drawdown_atr: null(),
        range_position: null(),
    };
    for bar in 0..len {
        let candle = candles[bar];
        let span = candle.high() - candle.low();
        if span > 0.0 {
            let body = (candle.close() - candle.open()).abs();
            let upper = candle.high() - candle.open().max(candle.close());
            let lower = candle.open().min(candle.close()) - candle.low();
            result.body_ratio[bar] = Some(body / span);
            result.upper_wick_ratio[bar] = Some(upper / span);
            result.lower_wick_ratio[bar] = Some(lower / span);
            result.close_position[bar] = Some((candle.close() - candle.low()) / span);
        }
        if bar >= 1 {
            let previous = candles[bar - 1];
            let bullish_engulf = candle.close() > candle.open()
                && candle.open() <= previous.close()
                && candle.close() >= previous.open();
            let bearish_engulf = candle.close() < candle.open()
                && candle.open() >= previous.close()
                && candle.close() <= previous.open();
            let inside = candle.high() <= previous.high() && candle.low() >= previous.low();
            let outside = candle.high() > previous.high() && candle.low() < previous.low();
            let three_soldiers = bar >= 2 && {
                let previous2 = candles[bar - 2];
                candle.close() > candle.open()
                    && previous.close() > previous.open()
                    && previous2.close() > previous2.open()
                    && candle.close() > previous.close()
                    && previous.close() > previous2.close()
            };
            let three_crows = bar >= 2 && {
                let previous2 = candles[bar - 2];
                candle.close() < candle.open()
                    && previous.close() < previous.open()
                    && previous2.close() < previous2.open()
                    && candle.close() < previous.close()
                    && previous.close() < previous2.close()
            };
            let (code, direction) = if bullish_engulf {
                (1.0, 1.0)
            } else if bearish_engulf {
                (2.0, -1.0)
            } else if three_soldiers {
                (7.0, 1.0)
            } else if three_crows {
                (8.0, -1.0)
            } else if outside {
                (4.0, 0.0)
            } else if inside {
                (3.0, 0.0)
            } else if span > 0.0 {
                let body = (candle.close() - candle.open()).abs();
                let upper = candle.high() - candle.open().max(candle.close());
                let lower = candle.open().min(candle.close()) - candle.low();
                let position = (candle.close() - candle.low()) / span;
                if lower >= 2.0 * body && upper <= 0.1 * span && position >= 0.7 {
                    (5.0, 1.0)
                } else if upper >= 2.0 * body && lower <= 0.1 * span && position <= 0.3 {
                    (6.0, -1.0)
                } else {
                    (0.0, 0.0)
                }
            } else {
                (0.0, 0.0)
            };
            result.pattern_code[bar] = Some(code);
            result.pattern_direction[bar] = Some(direction);
        }
        if bar > 0 {
            let previous_close = candles[bar - 1].close();
            if let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) {
                let gap = (candle.open() - previous_close) / atr;
                result.gap_atr[bar] = Some(gap);
                if gap.abs() >= config.gap_atr {
                    let filled = if gap > 0.0 {
                        candle.close() <= previous_close
                    } else {
                        candle.close() >= previous_close
                    };
                    result.gap_filled[bar] = Some(if filled { 1.0 } else { 0.0 });
                } else {
                    result.gap_filled[bar] = Some(0.0);
                }
            }
        }
        let window = config.path_window;
        if window > 0 && bar + 1 >= window {
            let start = bar + 1 - window;
            let mut path = 0.0;
            let mut changes = 0_u32;
            let mut previous_sign = 0_i8;
            for index in (start + 1)..=bar {
                let delta = candles[index].close() - candles[index - 1].close();
                path += delta.abs();
                let sign = delta.signum() as i8;
                if previous_sign != 0 && sign != 0 && sign != previous_sign {
                    changes += 1;
                }
                if sign != 0 {
                    previous_sign = sign;
                }
            }
            let net = (candles[bar].close() - candles[start].close()).abs();
            if path > 0.0 {
                result.path_efficiency[bar] = Some(net / path);
            }
            result.direction_changes[bar] = Some(changes as f64 / window as f64);
            let mut high_index = start;
            let mut low_index = start;
            for index in start..=bar {
                if candles[index].high() > candles[high_index].high() {
                    high_index = index;
                }
                if candles[index].low() < candles[low_index].low() {
                    low_index = index;
                }
            }
            result.bars_since_high[bar] = Some((bar - high_index) as f64);
            result.bars_since_low[bar] = Some((bar - low_index) as f64);
            let high = candles[high_index].high();
            let low = candles[low_index].low();
            if high > low {
                result.range_position[bar] = Some((candles[bar].close() - low) / (high - low));
            }
            if let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) {
                result.drawdown_atr[bar] = Some((candles[bar].close() - high) / atr);
            }
        }
    }
    result
}
