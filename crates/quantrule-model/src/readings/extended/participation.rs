//! Participation readings.

use super::shared::{mean_volume, volume};
use super::{Candle, ExtendedConfig, Readings};

#[allow(clippy::type_complexity)]
pub(super) fn participation_series(
    candles: &[Candle],
    readings: &Readings,
    config: &ExtendedConfig,
    vwap: &[Option<f64>],
    poc: &[Option<f64>],
) -> (
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
    Vec<Option<f64>>,
) {
    let len = candles.len();
    let mut fast = vec![None; len];
    let mut slow = vec![None; len];
    let mut ratio = vec![None; len];
    let mut oscillator = vec![None; len];
    let mut climax_flag = vec![None; len];
    let mut climax_direction = vec![None; len];
    let mut climax_strength = vec![None; len];
    let mut absorption_flag = vec![None; len];
    let mut absorption_direction = vec![None; len];
    let mut absorption_strength = vec![None; len];

    for bar in 0..len {
        let Some(fast_mean) = mean_volume(candles, bar, config.rvol_fast) else {
            continue;
        };
        let volume = volume(&candles[bar]);
        if fast_mean <= 0.0 {
            continue;
        }
        let fast_value = volume / fast_mean;
        fast[bar] = Some(fast_value);
        if let Some(slow_mean) = mean_volume(candles, bar, config.rvol_slow)
            && slow_mean > 0.0
        {
            let slow_value = volume / slow_mean;
            slow[bar] = Some(slow_value);
            ratio[bar] = Some(fast_value / slow_value);
        }
        if let Some(long_mean) = mean_volume(candles, bar, config.volume_oscillator)
            && long_mean > 0.0
        {
            oscillator[bar] = Some((fast_mean - long_mean) / long_mean);
        }

        let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
            continue;
        };
        let range_atr = (candles[bar].high() - candles[bar].low()) / atr;
        if fast_value >= config.climax_volume && range_atr >= config.climax_range_atr {
            climax_flag[bar] = Some(1.0);
            let direction = (candles[bar].close() - candles[bar].open()).signum();
            climax_direction[bar] = Some(direction);
            climax_strength[bar] = Some(fast_value * range_atr);
        }
        if fast_value >= config.absorption_volume && range_atr <= config.absorption_range_atr {
            let mut nearest: Option<(f64, f64)> = None;
            for level in [vwap[bar], poc[bar]].into_iter().flatten() {
                let distance = (candles[bar].close() - level).abs();
                if distance <= config.absorption_level_atr * atr
                    && nearest.is_none_or(|(best, _)| distance < best)
                {
                    nearest = Some((distance, level));
                }
            }
            if let Some((_, level)) = nearest {
                absorption_flag[bar] = Some(1.0);
                absorption_direction[bar] = Some((candles[bar].close() - level).signum());
                if range_atr > 0.0 {
                    absorption_strength[bar] = Some(fast_value / range_atr);
                }
            }
        }
    }
    (
        fast,
        slow,
        ratio,
        oscillator,
        climax_flag,
        climax_direction,
        climax_strength,
        absorption_flag,
        absorption_direction,
        absorption_strength,
    )
}
