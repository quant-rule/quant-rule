//! Extended OHLCV readings: VWAP and rolling volume profile.
//!
//! These readings are computed from closed candles only. They are pure: no I/O,
//! no clock, no randomness. A missing denominator publishes `None`, never zero.
//!
//! The v4 readings remain the contract surface; this module adds the first batch
//! of the indicator-completeness programme (epic #405).

use quantrule_data::Candle;

use super::{ReadingSeries, Readings};

/// Configuration for the extended readings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtendedConfig {
    /// Rolling VWAP window in bars.
    pub vwap_window: usize,
    /// VWAP band width in mean absolute deviations.
    pub vwap_band_mad: f64,
    /// Volume-profile window in bars.
    pub profile_window: usize,
    /// Number of price bins in the profile.
    pub profile_bins: usize,
    /// Fraction of volume included in the value area.
    pub profile_value_area: f64,
}

impl ExtendedConfig {
    /// The default configuration: VWAP 24, bands 2 MAD, profile 96 bars / 24
    /// bins / 70% value area.
    pub const DEFAULT: Self = Self {
        vwap_window: 24,
        vwap_band_mad: 2.0,
        profile_window: 96,
        profile_bins: 24,
        profile_value_area: 0.7,
    };
}

/// The first extended-reading batch.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtendedReadings {
    /// Rolling VWAP.
    pub vwap: ReadingSeries,
    /// `(close - VWAP) / ATR`.
    pub vwap_distance_atr: ReadingSeries,
    /// `VWAP - VWAP[n]`.
    pub vwap_slope: ReadingSeries,
    /// `VWAP + k * MAD`.
    pub vwap_band_upper: ReadingSeries,
    /// `VWAP - k * MAD`.
    pub vwap_band_lower: ReadingSeries,
    /// Point of control of the rolling profile.
    pub poc: ReadingSeries,
    /// Value-area high.
    pub vah: ReadingSeries,
    /// Value-area low.
    pub val: ReadingSeries,
    /// `(close - POC) / ATR`.
    pub poc_distance_atr: ReadingSeries,
    /// `(close - VAL) / (VAH - VAL)`.
    pub va_position: ReadingSeries,
    /// `(volume above POC - volume below POC) / total volume`.
    pub profile_skew: ReadingSeries,
    /// `max bin volume / total volume`.
    pub profile_concentration: ReadingSeries,
    /// `POC - POC[n]`.
    pub poc_migration: ReadingSeries,
}

impl ExtendedReadings {
    /// An all-null batch of the given length.
    #[must_use]
    pub fn empty(len: usize) -> Self {
        let null = || ReadingSeries::new(vec![None; len]);
        Self {
            vwap: null(),
            vwap_distance_atr: null(),
            vwap_slope: null(),
            vwap_band_upper: null(),
            vwap_band_lower: null(),
            poc: null(),
            vah: null(),
            val: null(),
            poc_distance_atr: null(),
            va_position: null(),
            profile_skew: null(),
            profile_concentration: null(),
            poc_migration: null(),
        }
    }

    /// Computes the extended readings over the candles.
    ///
    /// `readings.atr14` supplies the ATR for the distance outputs. A missing
    /// ATR makes the distance `None`; the VWAP and profile still publish.
    #[must_use]
    pub fn compute(candles: &[Candle], readings: &Readings, config: &ExtendedConfig) -> Self {
        let len = candles.len();
        let (vwap, mad) = vwap_series(candles, config.vwap_window);
        let mut distance = vec![None; len];
        let mut slope = vec![None; len];
        let mut upper = vec![None; len];
        let mut lower = vec![None; len];
        for (bar, value) in vwap.iter().enumerate() {
            let Some(center) = value else { continue };
            let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
                continue;
            };
            distance[bar] = Some((candles[bar].close() - center) / atr);
            if let Some(deviation) = mad[bar] {
                upper[bar] = Some(center + config.vwap_band_mad * deviation);
                lower[bar] = Some(center - config.vwap_band_mad * deviation);
            }
            if bar + 1 >= config.vwap_window {
                let prior = bar + 1 - config.vwap_window;
                if let Some(previous) = vwap[prior] {
                    slope[bar] = Some(center - previous);
                }
            }
        }

        let profile = ProfileSeries::compute(candles, readings, config);
        Self {
            vwap: ReadingSeries::new(vwap),
            vwap_distance_atr: ReadingSeries::new(distance),
            vwap_slope: ReadingSeries::new(slope),
            vwap_band_upper: ReadingSeries::new(upper),
            vwap_band_lower: ReadingSeries::new(lower),
            poc: ReadingSeries::new(profile.poc.clone()),
            vah: ReadingSeries::new(profile.vah.clone()),
            val: ReadingSeries::new(profile.val.clone()),
            poc_distance_atr: ReadingSeries::new(profile.poc_distance_atr),
            va_position: ReadingSeries::new(profile.va_position),
            profile_skew: ReadingSeries::new(profile.skew),
            profile_concentration: ReadingSeries::new(profile.concentration),
            poc_migration: ReadingSeries::new(profile.poc_migration),
        }
    }
}

fn typical_price(candle: &Candle) -> f64 {
    (candle.high() + candle.low() + candle.close()) / 3.0
}

fn volume(candle: &Candle) -> f64 {
    candle.volume().unwrap_or(0.0)
}

fn vwap_series(candles: &[Candle], window: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
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

struct ProfileSeries {
    poc: Vec<Option<f64>>,
    vah: Vec<Option<f64>>,
    val: Vec<Option<f64>>,
    poc_distance_atr: Vec<Option<f64>>,
    va_position: Vec<Option<f64>>,
    skew: Vec<Option<f64>>,
    concentration: Vec<Option<f64>>,
    poc_migration: Vec<Option<f64>>,
}

impl ProfileSeries {
    fn compute(candles: &[Candle], readings: &Readings, config: &ExtendedConfig) -> Self {
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

#[cfg(test)]
mod tests {
    use super::{ExtendedConfig, ExtendedReadings};
    use crate::readings::{Readings, ReadingsConfig};
    use quantrule_data::Candle;

    fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
        Candle::new(index, index as i64, open, high, low, close, Some(volume))
            .expect("valid candle")
    }

    fn readings(candles: &[Candle]) -> Readings {
        Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT)
    }

    #[test]
    fn vwap_is_the_volume_weighted_typical_price() {
        let candles = [
            candle(0, 10.0, 10.0, 10.0, 10.0, 1.0),
            candle(1, 20.0, 20.0, 20.0, 20.0, 1.0),
            candle(2, 30.0, 30.0, 30.0, 30.0, 1.0),
        ];
        let config = ExtendedConfig {
            vwap_window: 3,
            ..ExtendedConfig::DEFAULT
        };
        let readings = readings(&candles);
        let extended = ExtendedReadings::compute(&candles, &readings, &config);
        assert_eq!(extended.vwap.get(2), Some(20.0));
        assert_eq!(extended.vwap.get(1), None);
    }

    #[test]
    fn vwap_is_null_without_volume() {
        let candles = [
            candle(0, 10.0, 10.0, 10.0, 10.0, 0.0),
            candle(1, 10.0, 10.0, 10.0, 10.0, 0.0),
        ];
        let config = ExtendedConfig {
            vwap_window: 2,
            ..ExtendedConfig::DEFAULT
        };
        let readings = readings(&candles);
        let extended = ExtendedReadings::compute(&candles, &readings, &config);
        assert_eq!(extended.vwap.get(1), None);
    }

    #[test]
    fn profile_poc_is_the_highest_volume_bin() {
        let candles = [
            candle(0, 1.0, 1.0, 1.0, 1.0, 1.0),
            candle(1, 1.0, 1.0, 1.0, 1.0, 1.0),
            candle(2, 3.0, 3.0, 3.0, 3.0, 4.0),
            candle(3, 3.0, 3.0, 3.0, 3.0, 4.0),
        ];
        let config = ExtendedConfig {
            profile_window: 4,
            profile_bins: 2,
            ..ExtendedConfig::DEFAULT
        };
        let readings = readings(&candles);
        let extended = ExtendedReadings::compute(&candles, &readings, &config);
        let poc = extended.poc.get(3).expect("poc");
        assert!(poc > 2.0 && poc < 3.0);
        assert_eq!(extended.profile_concentration.get(3), Some(0.8));
    }

    #[test]
    fn profile_is_null_on_a_flat_range() {
        let candles = [
            candle(0, 1.0, 1.0, 1.0, 1.0, 1.0),
            candle(1, 1.0, 1.0, 1.0, 1.0, 1.0),
        ];
        let config = ExtendedConfig {
            profile_window: 2,
            profile_bins: 2,
            ..ExtendedConfig::DEFAULT
        };
        let readings = readings(&candles);
        let extended = ExtendedReadings::compute(&candles, &readings, &config);
        assert_eq!(extended.poc.get(1), None);
    }

    #[test]
    fn appending_future_bars_does_not_change_past_values() {
        let candles: Vec<Candle> = (0..10)
            .map(|index| {
                let base = 10.0 + index as f64;
                candle(
                    index,
                    base,
                    base + 1.0,
                    base - 1.0,
                    base + 0.5,
                    1.0 + index as f64,
                )
            })
            .collect();
        let config = ExtendedConfig {
            vwap_window: 3,
            profile_window: 4,
            profile_bins: 4,
            ..ExtendedConfig::DEFAULT
        };
        let full = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let prefix = ExtendedReadings::compute(&candles[..7], &readings(&candles[..7]), &config);
        for bar in 0..7 {
            assert_eq!(full.vwap.get(bar), prefix.vwap.get(bar), "vwap {bar}");
            assert_eq!(full.poc.get(bar), prefix.poc.get(bar), "poc {bar}");
            assert_eq!(full.vah.get(bar), prefix.vah.get(bar), "vah {bar}");
            assert_eq!(full.val.get(bar), prefix.val.get(bar), "val {bar}");
        }
    }

    #[test]
    fn repeated_computation_is_identical() {
        let candles: Vec<Candle> = (0..8)
            .map(|index| {
                let base = 10.0 + index as f64;
                candle(
                    index,
                    base,
                    base + 1.0,
                    base - 1.0,
                    base + 0.5,
                    1.0 + index as f64,
                )
            })
            .collect();
        let config = ExtendedConfig {
            vwap_window: 3,
            profile_window: 4,
            profile_bins: 4,
            ..ExtendedConfig::DEFAULT
        };
        let readings = readings(&candles);
        let first = ExtendedReadings::compute(&candles, &readings, &config);
        let second = ExtendedReadings::compute(&candles, &readings, &config);
        assert_eq!(first, second);
    }
}
