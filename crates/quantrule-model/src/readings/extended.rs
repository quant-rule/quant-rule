//! Extended OHLCV readings: VWAP and rolling volume profile.
//!
//! These readings are computed from closed candles only. They are pure: no I/O,
//! no clock, no randomness. A missing denominator publishes `None`, never zero.
//!
//! The v4 readings remain the contract surface; this module adds the first batch
//! of the indicator-completeness programme (epic #405).

use quantrule_data::Candle;

use super::{PivotKind, PivotScale, ReadingSeries, Readings, atr, ema};
use crate::levels::ConfirmedLevelRegistry;

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
    /// Fast relative-volume window.
    pub rvol_fast: usize,
    /// Slow relative-volume window.
    pub rvol_slow: usize,
    /// Volume-oscillator long window.
    pub volume_oscillator: usize,
    /// Climax volume multiple.
    pub climax_volume: f64,
    /// Climax range multiple.
    pub climax_range_atr: f64,
    /// Absorption volume multiple.
    pub absorption_volume: f64,
    /// Absorption range maximum.
    pub absorption_range_atr: f64,
    /// Absorption level tolerance.
    pub absorption_level_atr: f64,
    /// Flow window for OBV/CMF/AD means.
    pub flow_window: usize,
    /// MFI window.
    pub mfi_window: usize,
    /// CMF threshold.
    pub cmf_threshold: f64,
    /// Slow ATR window for the term structure.
    pub atr_slow_window: usize,
    /// ATR-ratio slope lookback.
    pub atr_slope_window: usize,
    /// Range-expansion mean window.
    pub range_window: usize,
    /// Fast mean-absolute-return window.
    pub mar_fast: usize,
    /// Slow mean-absolute-return window.
    pub mar_slow: usize,
    /// Keltner EMA window.
    pub keltner_window: usize,
    /// Keltner ATR multiple.
    pub keltner_atr: f64,
    /// Donchian window.
    pub donchian_window: usize,
    /// Confirmed-level lookback in bars.
    pub level_lookback: usize,
    /// Level touch tolerance in ATR.
    pub level_atr: f64,
    /// Aroon window.
    pub aroon_window: usize,
    /// Path window for efficiency, direction changes, extremes, drawdown, position.
    pub path_window: usize,
    /// Momentum-body ratio.
    pub candle_body: f64,
    /// Rejection-wick ratio.
    pub candle_wick: f64,
    /// Close-at-extreme ratio.
    pub candle_close: f64,
    /// Gap threshold in ATR.
    pub gap_atr: f64,
    /// Fibonacci level test tolerance in ATR.
    pub fib_test_atr: f64,
    /// Horizontal band cluster tolerance in ATR.
    pub band_atr: f64,
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
        rvol_fast: 5,
        rvol_slow: 50,
        volume_oscillator: 20,
        climax_volume: 3.0,
        climax_range_atr: 1.5,
        absorption_volume: 2.0,
        absorption_range_atr: 0.6,
        absorption_level_atr: 0.25,
        flow_window: 20,
        mfi_window: 14,
        cmf_threshold: 0.2,
        atr_slow_window: 100,
        atr_slope_window: 10,
        range_window: 20,
        mar_fast: 5,
        mar_slow: 20,
        keltner_window: 20,
        keltner_atr: 2.0,
        donchian_window: 20,
        level_lookback: 500,
        level_atr: 0.25,
        aroon_window: 25,
        path_window: 20,
        candle_body: 0.7,
        candle_wick: 0.6,
        candle_close: 0.8,
        gap_atr: 0.5,
        fib_test_atr: 0.2,
        band_atr: 0.25,
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
    /// `V / mean(V, fast)`.
    pub rvol_fast: ReadingSeries,
    /// `V / mean(V, slow)`.
    pub rvol_slow: ReadingSeries,
    /// `rvol_fast / rvol_slow`.
    pub rvol_ratio: ReadingSeries,
    /// `(mean(V, fast) - mean(V, long)) / mean(V, long)`.
    pub volume_oscillator: ReadingSeries,
    /// One when the bar is a volume climax.
    pub climax_flag: ReadingSeries,
    /// The climax bar's direction.
    pub climax_direction: ReadingSeries,
    /// `rvol_fast * range / ATR`.
    pub climax_strength: ReadingSeries,
    /// One when the bar is absorption at VWAP or POC.
    pub absorption_flag: ReadingSeries,
    /// The absorption bar's close side relative to the level.
    pub absorption_direction: ReadingSeries,
    /// `rvol_fast / (range / ATR)`.
    pub absorption_strength: ReadingSeries,
    /// On-balance volume.
    pub obv: ReadingSeries,
    /// Mean of OBV over the flow window.
    pub obv_ma: ReadingSeries,
    /// `OBV - OBV[n]`.
    pub obv_slope: ReadingSeries,
    /// Chaikin money flow.
    pub cmf: ReadingSeries,
    /// `CMF - CMF[n]`.
    pub cmf_slope: ReadingSeries,
    /// Money flow index.
    pub mfi: ReadingSeries,
    /// Accumulation/distribution line.
    pub ad_line: ReadingSeries,
    /// Mean of the A/D line over the flow window.
    pub ad_ma: ReadingSeries,
    /// `ATR fast / ATR slow`.
    pub atr_ratio: ReadingSeries,
    /// `ATR ratio[t] - ATR ratio[t-n]`.
    pub atr_slope: ReadingSeries,
    /// 0 compressed, 1 normal, 2 expanded.
    pub vol_regime: ReadingSeries,
    /// `range / mean(range, n)`.
    pub range_expansion: ReadingSeries,
    /// Fast mean absolute return.
    pub mar_fast: ReadingSeries,
    /// Slow mean absolute return.
    pub mar_slow: ReadingSeries,
    /// `mar_fast / mar_slow`.
    pub mar_ratio: ReadingSeries,
    /// Keltner upper band.
    pub keltner_upper: ReadingSeries,
    /// Keltner lower band.
    pub keltner_lower: ReadingSeries,
    /// `(close - lower) / (upper - lower)`.
    pub keltner_position: ReadingSeries,
    /// Donchian upper band.
    pub donchian_upper: ReadingSeries,
    /// Donchian lower band.
    pub donchian_lower: ReadingSeries,
    /// `(close - lower) / (upper - lower)`.
    pub donchian_position: ReadingSeries,
    /// +1 HH/HL, -1 LH/LL, 0 mixed.
    pub structure_state: ReadingSeries,
    /// Consecutive confirmed major lower highs.
    pub lower_high_count: ReadingSeries,
    /// Consecutive confirmed major lower lows.
    pub lower_low_count: ReadingSeries,
    /// Consecutive confirmed major higher highs.
    pub higher_high_count: ReadingSeries,
    /// Consecutive confirmed major higher lows.
    pub higher_low_count: ReadingSeries,
    /// Fibonacci leg direction: +1 up, -1 down.
    pub fib_direction: ReadingSeries,
    /// Fibonacci leg low.
    pub fib_low: ReadingSeries,
    /// Fibonacci leg high.
    pub fib_high: ReadingSeries,
    /// 0.236 retracement level.
    pub fib_236: ReadingSeries,
    /// 0.382 retracement level.
    pub fib_382: ReadingSeries,
    /// 0.500 retracement level.
    pub fib_500: ReadingSeries,
    /// 0.618 retracement level.
    pub fib_618: ReadingSeries,
    /// 0.786 retracement level.
    pub fib_786: ReadingSeries,
    /// Position in the leg: 0 at the start, 1 at the end.
    pub fib_position: ReadingSeries,
    /// Nearest Fibonacci ratio.
    pub fib_nearest_ratio: ReadingSeries,
    /// Distance to the nearest Fibonacci level in ATR.
    pub fib_nearest_atr: ReadingSeries,
    /// Horizontal band upper edge.
    pub band_upper: ReadingSeries,
    /// Horizontal band lower edge.
    pub band_lower: ReadingSeries,
    /// Horizontal band center.
    pub band_center: ReadingSeries,
    /// Number of confirmed levels in the band.
    pub band_touches: ReadingSeries,
    /// Age in bars of the youngest level in the band.
    pub band_age: ReadingSeries,
    /// Band width in ATR.
    pub band_width_atr: ReadingSeries,
    /// Position inside the band.
    pub band_position: ReadingSeries,
    /// The latest confirmed major pivot high.
    pub last_pivot_high: ReadingSeries,
    /// The latest confirmed major pivot low.
    pub last_pivot_low: ReadingSeries,
    /// Confirmed-level touches in the lookback.
    pub level_touches: ReadingSeries,
    /// Age of the most recently touched level.
    pub level_age: ReadingSeries,
    /// Aroon up.
    pub aroon_up: ReadingSeries,
    /// Aroon down.
    pub aroon_down: ReadingSeries,
    /// `aroon_up - aroon_down`.
    pub aroon_osc: ReadingSeries,
    /// `body / range`.
    pub body_ratio: ReadingSeries,
    /// `upper_wick / range`.
    pub upper_wick_ratio: ReadingSeries,
    /// `lower_wick / range`.
    pub lower_wick_ratio: ReadingSeries,
    /// `(close - low) / range`.
    pub close_position: ReadingSeries,
    /// Candle-pattern ordinal code.
    pub pattern_code: ReadingSeries,
    /// Candle-pattern direction.
    pub pattern_direction: ReadingSeries,
    /// `(open - previous close) / ATR`.
    pub gap_atr: ReadingSeries,
    /// One when a qualifying gap is filled on the same bar.
    pub gap_filled: ReadingSeries,
    /// Net move over the absolute path.
    pub path_efficiency: ReadingSeries,
    /// Direction-change ratio over the path window.
    pub direction_changes: ReadingSeries,
    /// Bars since the path-window high.
    pub bars_since_high: ReadingSeries,
    /// Bars since the path-window low.
    pub bars_since_low: ReadingSeries,
    /// `(close - highest high) / ATR`.
    pub drawdown_atr: ReadingSeries,
    /// Position inside the path-window range.
    pub range_position: ReadingSeries,
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
            rvol_fast: null(),
            rvol_slow: null(),
            rvol_ratio: null(),
            volume_oscillator: null(),
            climax_flag: null(),
            climax_direction: null(),
            climax_strength: null(),
            absorption_flag: null(),
            absorption_direction: null(),
            absorption_strength: null(),
            obv: null(),
            obv_ma: null(),
            obv_slope: null(),
            cmf: null(),
            cmf_slope: null(),
            mfi: null(),
            ad_line: null(),
            ad_ma: null(),
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
            lower_high_count: null(),
            lower_low_count: null(),
            higher_high_count: null(),
            higher_low_count: null(),
            fib_direction: null(),
            fib_low: null(),
            fib_high: null(),
            fib_236: null(),
            fib_382: null(),
            fib_500: null(),
            fib_618: null(),
            fib_786: null(),
            fib_position: null(),
            fib_nearest_ratio: null(),
            fib_nearest_atr: null(),
            band_upper: null(),
            band_lower: null(),
            band_center: null(),
            band_touches: null(),
            band_age: null(),
            band_width_atr: null(),
            band_position: null(),
            last_pivot_high: null(),
            last_pivot_low: null(),
            level_touches: null(),
            level_age: null(),
            aroon_up: null(),
            aroon_down: null(),
            aroon_osc: null(),
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
        let (
            rvol_fast,
            rvol_slow,
            rvol_ratio,
            volume_oscillator,
            climax_flag,
            climax_direction,
            climax_strength,
            absorption_flag,
            absorption_direction,
            absorption_strength,
        ) = participation_series(candles, readings, config, &vwap, &profile.poc);
        let flow = flow_series(candles, config);
        let structure = structure_series(candles, readings, config);
        let fibonacci = fibonacci_series(candles, readings);
        let band = band_series(candles, readings, config);
        let anatomy = anatomy_series(candles, readings, config);
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
            rvol_fast: ReadingSeries::new(rvol_fast),
            rvol_slow: ReadingSeries::new(rvol_slow),
            rvol_ratio: ReadingSeries::new(rvol_ratio),
            volume_oscillator: ReadingSeries::new(volume_oscillator),
            climax_flag: ReadingSeries::new(climax_flag),
            climax_direction: ReadingSeries::new(climax_direction),
            climax_strength: ReadingSeries::new(climax_strength),
            absorption_flag: ReadingSeries::new(absorption_flag),
            absorption_direction: ReadingSeries::new(absorption_direction),
            absorption_strength: ReadingSeries::new(absorption_strength),
            obv: ReadingSeries::new(flow.obv),
            obv_ma: ReadingSeries::new(flow.obv_ma),
            obv_slope: ReadingSeries::new(flow.obv_slope),
            cmf: ReadingSeries::new(flow.cmf),
            cmf_slope: ReadingSeries::new(flow.cmf_slope),
            mfi: ReadingSeries::new(flow.mfi),
            ad_line: ReadingSeries::new(flow.ad_line),
            ad_ma: ReadingSeries::new(flow.ad_ma),
            atr_ratio: ReadingSeries::new(structure.atr_ratio),
            atr_slope: ReadingSeries::new(structure.atr_slope),
            vol_regime: ReadingSeries::new(structure.vol_regime),
            range_expansion: ReadingSeries::new(structure.range_expansion),
            mar_fast: ReadingSeries::new(structure.mar_fast),
            mar_slow: ReadingSeries::new(structure.mar_slow),
            mar_ratio: ReadingSeries::new(structure.mar_ratio),
            keltner_upper: ReadingSeries::new(structure.keltner_upper),
            keltner_lower: ReadingSeries::new(structure.keltner_lower),
            keltner_position: ReadingSeries::new(structure.keltner_position),
            donchian_upper: ReadingSeries::new(structure.donchian_upper),
            donchian_lower: ReadingSeries::new(structure.donchian_lower),
            donchian_position: ReadingSeries::new(structure.donchian_position),
            structure_state: ReadingSeries::new(structure.structure_state),
            lower_high_count: ReadingSeries::new(structure.lower_high_count),
            lower_low_count: ReadingSeries::new(structure.lower_low_count),
            higher_high_count: ReadingSeries::new(structure.higher_high_count),
            higher_low_count: ReadingSeries::new(structure.higher_low_count),
            fib_direction: ReadingSeries::new(fibonacci.direction),
            fib_low: ReadingSeries::new(fibonacci.low),
            fib_high: ReadingSeries::new(fibonacci.high),
            fib_236: ReadingSeries::new(fibonacci.level_236),
            fib_382: ReadingSeries::new(fibonacci.level_382),
            fib_500: ReadingSeries::new(fibonacci.level_500),
            fib_618: ReadingSeries::new(fibonacci.level_618),
            fib_786: ReadingSeries::new(fibonacci.level_786),
            fib_position: ReadingSeries::new(fibonacci.position),
            fib_nearest_ratio: ReadingSeries::new(fibonacci.nearest_ratio),
            fib_nearest_atr: ReadingSeries::new(fibonacci.nearest_atr),
            band_upper: ReadingSeries::new(band.upper),
            band_lower: ReadingSeries::new(band.lower),
            band_center: ReadingSeries::new(band.center),
            band_touches: ReadingSeries::new(band.touches),
            band_age: ReadingSeries::new(band.age),
            band_width_atr: ReadingSeries::new(band.width_atr),
            band_position: ReadingSeries::new(band.position),
            last_pivot_high: ReadingSeries::new(structure.last_pivot_high),
            last_pivot_low: ReadingSeries::new(structure.last_pivot_low),
            level_touches: ReadingSeries::new(structure.level_touches),
            level_age: ReadingSeries::new(structure.level_age),
            aroon_up: ReadingSeries::new(structure.aroon_up),
            aroon_down: ReadingSeries::new(structure.aroon_down),
            aroon_osc: ReadingSeries::new(structure.aroon_osc),
            body_ratio: ReadingSeries::new(anatomy.body_ratio),
            upper_wick_ratio: ReadingSeries::new(anatomy.upper_wick_ratio),
            lower_wick_ratio: ReadingSeries::new(anatomy.lower_wick_ratio),
            close_position: ReadingSeries::new(anatomy.close_position),
            pattern_code: ReadingSeries::new(anatomy.pattern_code),
            pattern_direction: ReadingSeries::new(anatomy.pattern_direction),
            gap_atr: ReadingSeries::new(anatomy.gap_atr),
            gap_filled: ReadingSeries::new(anatomy.gap_filled),
            path_efficiency: ReadingSeries::new(anatomy.path_efficiency),
            direction_changes: ReadingSeries::new(anatomy.direction_changes),
            bars_since_high: ReadingSeries::new(anatomy.bars_since_high),
            bars_since_low: ReadingSeries::new(anatomy.bars_since_low),
            drawdown_atr: ReadingSeries::new(anatomy.drawdown_atr),
            range_position: ReadingSeries::new(anatomy.range_position),
        }
    }
}

mod anatomy;
mod band;
mod fibonacci;
mod flow;
mod participation;
mod profile;
mod shared;
mod structure;
mod vwap;

#[cfg(test)]
mod tests;

use self::anatomy::anatomy_series;
use self::band::band_series;
use self::fibonacci::fibonacci_series;
use self::flow::flow_series;
use self::participation::participation_series;
use self::profile::ProfileSeries;
use self::structure::structure_series;
use self::vwap::vwap_series;
