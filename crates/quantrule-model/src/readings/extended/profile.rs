//! Volume-profile readings.

use super::shared::{typical_price, volume};
use super::*;

pub(super) struct ProfileSeries {
    pub(super) poc: Vec<Option<f64>>,
    pub(super) vah: Vec<Option<f64>>,
    pub(super) val: Vec<Option<f64>>,
    pub(super) poc_distance_atr: Vec<Option<f64>>,
    pub(super) va_position: Vec<Option<f64>>,
    pub(super) skew: Vec<Option<f64>>,
    pub(super) concentration: Vec<Option<f64>>,
    pub(super) poc_migration: Vec<Option<f64>>,
}

impl ProfileSeries {
    pub(super) fn compute(
        candles: &[Candle],
        readings: &Readings,
        config: &ExtendedConfig,
    ) -> Self {
        let len = candles.len();
        let mut series = Self {
            poc: vec![None; len],
            vah: vec![None; len],
            val: vec![None; len],
            poc_distance_atr: vec![None; len],
            va_position: vec![None; len],
            skew: vec![None; len],
            concentration: vec![None; len],
            poc_migration: vec![None; len],
        };
        let window = config.profile_window;
        let bins = config.profile_bins;
        if window == 0 || bins == 0 || len < window || config.profile_value_area <= 0.0 {
            return series;
        }
        for bar in (window - 1)..len {
            let start = bar + 1 - window;
            let mut low = f64::INFINITY;
            let mut high = f64::NEG_INFINITY;
            for candle in &candles[start..=bar] {
                low = low.min(candle.low());
                high = high.max(candle.high());
            }
            if !low.is_finite() || !high.is_finite() || high <= low {
                continue;
            }
            let width = (high - low) / bins as f64;
            let mut buckets = vec![0.0; bins];
            let mut total = 0.0;
            for candle in &candles[start..=bar] {
                let volume = volume(candle);
                if volume <= 0.0 {
                    continue;
                }
                let index = (((typical_price(candle) - low) / width) as usize).min(bins - 1);
                buckets[index] += volume;
                total += volume;
            }
            if total <= 0.0 {
                continue;
            }
            let mut poc_index = 0;
            for index in 1..bins {
                if buckets[index] > buckets[poc_index] {
                    poc_index = index;
                }
            }
            let poc = low + (poc_index as f64 + 0.5) * width;
            let (lower, upper) = value_area(&buckets, poc_index, total * config.profile_value_area);
            let vah = low + (upper as f64 + 1.0) * width;
            let val = low + lower as f64 * width;
            let above: f64 = buckets[poc_index + 1..].iter().sum();
            let below: f64 = buckets[..poc_index].iter().sum();
            series.poc[bar] = Some(poc);
            series.vah[bar] = Some(vah);
            series.val[bar] = Some(val);
            series.skew[bar] = Some((above - below) / total);
            series.concentration[bar] = Some(buckets[poc_index] / total);
            if bar >= window
                && let Some(previous) = series.poc[bar - window]
            {
                series.poc_migration[bar] = Some(poc - previous);
            }
            if let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) {
                series.poc_distance_atr[bar] = Some((candles[bar].close() - poc) / atr);
            }
            if vah > val {
                series.va_position[bar] = Some((candles[bar].close() - val) / (vah - val));
            }
        }
        series
    }
}

fn value_area(buckets: &[f64], poc: usize, target: f64) -> (usize, usize) {
    let mut lower = poc;
    let mut upper = poc;
    let mut accumulated = buckets[poc];
    while accumulated < target && (lower > 0 || upper + 1 < buckets.len()) {
        let lower_volume = if lower > 0 { buckets[lower - 1] } else { -1.0 };
        let upper_volume = if upper + 1 < buckets.len() {
            buckets[upper + 1]
        } else {
            -1.0
        };
        if upper_volume > lower_volume {
            upper += 1;
            accumulated += upper_volume;
        } else if lower_volume >= 0.0 {
            lower -= 1;
            accumulated += lower_volume;
        } else {
            break;
        }
    }
    (lower, upper)
}
