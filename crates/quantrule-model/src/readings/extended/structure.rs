//! Volatility and structure readings.

use super::*;

pub(super) struct StructureReadings {
    pub(super) atr_ratio: Vec<Option<f64>>,
    pub(super) atr_slope: Vec<Option<f64>>,
    pub(super) vol_regime: Vec<Option<f64>>,
    pub(super) range_expansion: Vec<Option<f64>>,
    pub(super) mar_fast: Vec<Option<f64>>,
    pub(super) mar_slow: Vec<Option<f64>>,
    pub(super) mar_ratio: Vec<Option<f64>>,
    pub(super) keltner_upper: Vec<Option<f64>>,
    pub(super) keltner_lower: Vec<Option<f64>>,
    pub(super) keltner_position: Vec<Option<f64>>,
    pub(super) donchian_upper: Vec<Option<f64>>,
    pub(super) donchian_lower: Vec<Option<f64>>,
    pub(super) donchian_position: Vec<Option<f64>>,
    pub(super) structure_state: Vec<Option<f64>>,
    pub(super) last_pivot_high: Vec<Option<f64>>,
    pub(super) last_pivot_low: Vec<Option<f64>>,
    pub(super) level_touches: Vec<Option<f64>>,
    pub(super) level_age: Vec<Option<f64>>,
    pub(super) aroon_up: Vec<Option<f64>>,
    pub(super) aroon_down: Vec<Option<f64>>,
    pub(super) aroon_osc: Vec<Option<f64>>,
}

fn mean_range(candles: &[Candle], bar: usize, window: usize) -> Option<f64> {
    if window == 0 || bar + 1 < window {
        return None;
    }
    let start = bar + 1 - window;
    let total: f64 = candles[start..=bar]
        .iter()
        .map(|candle| candle.high() - candle.low())
        .sum();
    Some(total / window as f64)
}

fn mean_absolute_return(candles: &[Candle], bar: usize, window: usize) -> Option<f64> {
    if window == 0 || bar < window || bar == 0 {
        return None;
    }
    let start = bar + 1 - window;
    let mut total = 0.0;
    for index in start..=bar {
        let previous = candles[index - 1].close();
        if previous == 0.0 {
            return None;
        }
        total += ((candles[index].close() - previous) / previous).abs();
    }
    Some(total / window as f64)
}

pub(super) fn structure_series(
    candles: &[Candle],
    readings: &Readings,
    config: &ExtendedConfig,
) -> StructureReadings {
    let len = candles.len();
    let null = || vec![None; len];
    let mut result = StructureReadings {
        atr_ratio: null(),
        atr_slope: null(),
        vol_regime: null(),
        range_expansion: null(),
        mar_fast: null(),
        mar_slow: null(),
        mar_ratio: null(),
        keltner_upper: null(),
        keltner_lower: null(),
        keltner_position: null(),
        donchian_upper: null(),
        donchian_lower: null(),
        donchian_position: null(),
        structure_state: null(),
        last_pivot_high: null(),
        last_pivot_low: null(),
        level_touches: null(),
        level_age: null(),
        aroon_up: null(),
        aroon_down: null(),
        aroon_osc: null(),
    };
    if len == 0 {
        return result;
    }
    let atr_slow = atr(candles, config.atr_slow_window);
    let ema_close = ema(candles, config.keltner_window);
    let levels = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
    let mut last_high: Option<f64> = None;
    let mut last_low: Option<f64> = None;
    let mut previous_high: Option<f64> = None;
    let mut previous_low: Option<f64> = None;

    for bar in 0..len {
        let atr_fast = readings.atr14.get(bar);
        if let (Some(fast), Some(slow)) = (atr_fast, atr_slow.get(bar))
            && slow > 0.0
        {
            let ratio = fast / slow;
            result.atr_ratio[bar] = Some(ratio);
            result.vol_regime[bar] = Some(if ratio <= 0.7 {
                0.0
            } else if ratio < 1.3 {
                1.0
            } else {
                2.0
            });
            if bar >= config.atr_slope_window
                && let Some(previous) = result.atr_ratio[bar - config.atr_slope_window]
            {
                result.atr_slope[bar] = Some(ratio - previous);
            }
        }
        if let (Some(mean), Some(atr)) = (mean_range(candles, bar, config.range_window), atr_fast)
            && mean > 0.0
            && atr > 0.0
        {
            result.range_expansion[bar] = Some((candles[bar].high() - candles[bar].low()) / mean);
        }
        let fast_mar = mean_absolute_return(candles, bar, config.mar_fast);
        let slow_mar = mean_absolute_return(candles, bar, config.mar_slow);
        result.mar_fast[bar] = fast_mar;
        result.mar_slow[bar] = slow_mar;
        if let (Some(fast), Some(slow)) = (fast_mar, slow_mar)
            && slow > 0.0
        {
            result.mar_ratio[bar] = Some(fast / slow);
        }
        if let (Some(center), Some(atr)) = (ema_close.get(bar), atr_fast) {
            let upper = center + config.keltner_atr * atr;
            let lower = center - config.keltner_atr * atr;
            result.keltner_upper[bar] = Some(upper);
            result.keltner_lower[bar] = Some(lower);
            if upper > lower {
                result.keltner_position[bar] =
                    Some((candles[bar].close() - lower) / (upper - lower));
            }
        }
        if bar + 1 >= config.donchian_window && config.donchian_window > 0 {
            let start = bar + 1 - config.donchian_window;
            let mut upper = f64::NEG_INFINITY;
            let mut lower = f64::INFINITY;
            for candle in &candles[start..=bar] {
                upper = upper.max(candle.high());
                lower = lower.min(candle.low());
            }
            result.donchian_upper[bar] = Some(upper);
            result.donchian_lower[bar] = Some(lower);
            if upper > lower {
                result.donchian_position[bar] =
                    Some((candles[bar].close() - lower) / (upper - lower));
            }
        }
        if let Some(pivot) = readings.pivots.confirmed_at(PivotScale::Major, bar) {
            match pivot.kind {
                PivotKind::High => {
                    previous_high = last_high;
                    last_high = Some(pivot.price);
                }
                PivotKind::Low => {
                    previous_low = last_low;
                    last_low = Some(pivot.price);
                }
            }
        }
        result.last_pivot_high[bar] = last_high;
        result.last_pivot_low[bar] = last_low;
        if let (Some(high), Some(prior_high), Some(low), Some(prior_low)) =
            (last_high, previous_high, last_low, previous_low)
        {
            let high_state = if high > prior_high {
                1
            } else if high < prior_high {
                -1
            } else {
                0
            };
            let low_state = if low > prior_low {
                1
            } else if low < prior_low {
                -1
            } else {
                0
            };
            let state = if high_state == 1 && low_state == 1 {
                1.0
            } else if high_state == -1 && low_state == -1 {
                -1.0
            } else {
                0.0
            };
            result.structure_state[bar] = Some(state);
        }
        if let Some(atr) = atr_fast {
            let tolerance = config.level_atr * atr;
            let mut touches = 0_u32;
            let mut youngest: Option<usize> = None;
            for level in levels.levels() {
                if level.scale != PivotScale::Major
                    || level.available_at > bar
                    || bar - level.available_at > config.level_lookback
                {
                    continue;
                }
                if (candles[bar].high() - level.price).abs() <= tolerance
                    || (candles[bar].low() - level.price).abs() <= tolerance
                {
                    touches = touches.saturating_add(1);
                    let age = bar - level.available_at;
                    youngest = Some(youngest.map_or(age, |current| current.min(age)));
                }
            }
            result.level_touches[bar] = Some(f64::from(touches));
            result.level_age[bar] = youngest.map(|age| age as f64);
        }
        if bar + 1 >= config.aroon_window && config.aroon_window > 0 {
            let start = bar + 1 - config.aroon_window;
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
            let window = config.aroon_window as f64;
            let up = 100.0 * (window - (bar - high_index) as f64) / window;
            let down = 100.0 * (window - (bar - low_index) as f64) / window;
            result.aroon_up[bar] = Some(up);
            result.aroon_down[bar] = Some(down);
            result.aroon_osc[bar] = Some(up - down);
        }
    }
    result
}
