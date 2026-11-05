//! Extended OHLCV readings: VWAP and rolling volume profile.
//!
//! These readings are computed from closed candles only. They are pure: no I/O,
//! no clock, no randomness. A missing denominator publishes `None`, never zero.
//!
//! The v4 readings remain the contract surface; this module adds the first batch
//! of the indicator-completeness programme (epic #405).

use quantrule_data::Candle;

use super::{PivotKind, PivotScale, ReadingSeries, Readings, atr, ema};
use crate::axes::structure::registry::ConfirmedLevelRegistry;

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

struct AnatomyReadings {
    body_ratio: Vec<Option<f64>>,
    upper_wick_ratio: Vec<Option<f64>>,
    lower_wick_ratio: Vec<Option<f64>>,
    close_position: Vec<Option<f64>>,
    pattern_code: Vec<Option<f64>>,
    pattern_direction: Vec<Option<f64>>,
    gap_atr: Vec<Option<f64>>,
    gap_filled: Vec<Option<f64>>,
    path_efficiency: Vec<Option<f64>>,
    direction_changes: Vec<Option<f64>>,
    bars_since_high: Vec<Option<f64>>,
    bars_since_low: Vec<Option<f64>>,
    drawdown_atr: Vec<Option<f64>>,
    range_position: Vec<Option<f64>>,
}

fn anatomy_series(
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

struct StructureReadings {
    atr_ratio: Vec<Option<f64>>,
    atr_slope: Vec<Option<f64>>,
    vol_regime: Vec<Option<f64>>,
    range_expansion: Vec<Option<f64>>,
    mar_fast: Vec<Option<f64>>,
    mar_slow: Vec<Option<f64>>,
    mar_ratio: Vec<Option<f64>>,
    keltner_upper: Vec<Option<f64>>,
    keltner_lower: Vec<Option<f64>>,
    keltner_position: Vec<Option<f64>>,
    donchian_upper: Vec<Option<f64>>,
    donchian_lower: Vec<Option<f64>>,
    donchian_position: Vec<Option<f64>>,
    structure_state: Vec<Option<f64>>,
    last_pivot_high: Vec<Option<f64>>,
    last_pivot_low: Vec<Option<f64>>,
    level_touches: Vec<Option<f64>>,
    level_age: Vec<Option<f64>>,
    aroon_up: Vec<Option<f64>>,
    aroon_down: Vec<Option<f64>>,
    aroon_osc: Vec<Option<f64>>,
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

fn structure_series(
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

struct FlowSeries {
    obv: Vec<Option<f64>>,
    obv_ma: Vec<Option<f64>>,
    obv_slope: Vec<Option<f64>>,
    cmf: Vec<Option<f64>>,
    cmf_slope: Vec<Option<f64>>,
    mfi: Vec<Option<f64>>,
    ad_line: Vec<Option<f64>>,
    ad_ma: Vec<Option<f64>>,
}

fn money_flow_volume(candle: &Candle) -> f64 {
    let span = candle.high() - candle.low();
    if span <= 0.0 {
        return 0.0;
    }
    (((candle.close() - candle.low()) - (candle.high() - candle.close())) / span) * volume(candle)
}

fn flow_series(candles: &[Candle], config: &ExtendedConfig) -> FlowSeries {
    let len = candles.len();
    let mut result = FlowSeries {
        obv: vec![None; len],
        obv_ma: vec![None; len],
        obv_slope: vec![None; len],
        cmf: vec![None; len],
        cmf_slope: vec![None; len],
        mfi: vec![None; len],
        ad_line: vec![None; len],
        ad_ma: vec![None; len],
    };
    if len == 0 {
        return result;
    }
    let mut obv = 0.0;
    let mut ad = 0.0;
    result.obv[0] = Some(0.0);
    result.ad_line[0] = Some(0.0);
    for bar in 1..len {
        let change = candles[bar].close() - candles[bar - 1].close();
        obv += change.signum() * volume(&candles[bar]);
        ad += money_flow_volume(&candles[bar]);
        result.obv[bar] = Some(obv);
        result.ad_line[bar] = Some(ad);
    }
    let window = config.flow_window;
    if window > 0 {
        for bar in (window - 1)..len {
            let start = bar + 1 - window;
            let obv_values: Vec<f64> = result.obv[start..=bar]
                .iter()
                .filter_map(|value| *value)
                .collect();
            if obv_values.len() == window {
                result.obv_ma[bar] = Some(obv_values.iter().sum::<f64>() / window as f64);
            }
            let ad_values: Vec<f64> = result.ad_line[start..=bar]
                .iter()
                .filter_map(|value| *value)
                .collect();
            if ad_values.len() == window {
                result.ad_ma[bar] = Some(ad_values.iter().sum::<f64>() / window as f64);
            }
            let mut flow = 0.0;
            let mut total_volume = 0.0;
            for candle in &candles[start..=bar] {
                flow += money_flow_volume(candle);
                total_volume += volume(candle);
            }
            if total_volume > 0.0 {
                result.cmf[bar] = Some(flow / total_volume);
            }
            if bar >= 2 * window - 1 {
                let prior = bar - window;
                if let Some(previous) = result.cmf[prior] {
                    result.cmf_slope[bar] = result.cmf[bar].map(|value| value - previous);
                }
            }
            if bar + 1 >= 2 * window {
                let prior = bar - window;
                if let Some(previous) = result.obv[prior] {
                    result.obv_slope[bar] = result.obv[bar].map(|value| value - previous);
                }
            }
        }
    }
    let mfi_window = config.mfi_window;
    if mfi_window > 0 {
        for bar in mfi_window..len {
            let start = bar + 1 - mfi_window;
            let mut positive = 0.0;
            let mut negative = 0.0;
            for index in start..=bar {
                let typical =
                    (candles[index].high() + candles[index].low() + candles[index].close()) / 3.0;
                let previous = if index > 0 {
                    (candles[index - 1].high()
                        + candles[index - 1].low()
                        + candles[index - 1].close())
                        / 3.0
                } else {
                    typical
                };
                let flow = typical * volume(&candles[index]);
                if typical > previous {
                    positive += flow;
                } else if typical < previous {
                    negative += flow;
                }
            }
            if positive + negative > 0.0 {
                result.mfi[bar] = Some(if negative > 0.0 {
                    100.0 - 100.0 / (1.0 + positive / negative)
                } else {
                    100.0
                });
            }
        }
    }
    result
}

#[allow(clippy::type_complexity)]
fn participation_series(
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

fn mean_volume(candles: &[Candle], bar: usize, window: usize) -> Option<f64> {
    if window == 0 || bar + 1 < window {
        return None;
    }
    let start = bar + 1 - window;
    let total: f64 = candles[start..=bar].iter().map(volume).sum();
    Some(total / window as f64)
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

    #[test]
    fn rvol_reads_the_volume_ratio() {
        let candles: Vec<Candle> = (0..7)
            .map(|index| {
                let volume = if index == 6 { 20.0 } else { 10.0 };
                candle(index, 10.0, 10.5, 9.5, 10.0, volume)
            })
            .collect();
        let config = ExtendedConfig {
            rvol_fast: 3,
            rvol_slow: 6,
            volume_oscillator: 6,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let value = extended.rvol_fast.get(6).expect("fast rvol");
        assert!((value - 1.5).abs() < 1e-9, "fast rvol {value}");
    }

    #[test]
    fn climax_flags_a_high_volume_wide_bar() {
        let mut candles: Vec<Candle> = (0..30)
            .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
            .collect();
        candles[29] = candle(29, 100.0, 103.0, 99.0, 102.0, 100.0);
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.climax_flag.get(29), Some(1.0));
        assert_eq!(extended.climax_direction.get(29), Some(1.0));
        assert!(extended.climax_strength.get(29).is_some());
    }

    #[test]
    fn absorption_flags_a_small_range_near_vwap() {
        let mut candles: Vec<Candle> = (0..20)
            .map(|index| candle(index, 100.0, 100.5, 99.5, 100.0, 10.0))
            .collect();
        candles[19] = candle(19, 100.0, 100.1, 99.9, 100.05, 30.0);
        let config = ExtendedConfig {
            vwap_window: 5,
            profile_window: 5,
            profile_bins: 4,
            rvol_fast: 5,
            rvol_slow: 5,
            volume_oscillator: 5,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        assert_eq!(extended.absorption_flag.get(19), Some(1.0));
        assert!(extended.absorption_strength.get(19).is_some());
    }

    #[test]
    fn participation_is_causal() {
        let candles: Vec<Candle> = (0..20)
            .map(|index| {
                let volume = 10.0 + index as f64;
                candle(index, 100.0, 101.0, 99.0, 100.0, volume)
            })
            .collect();
        let config = ExtendedConfig {
            rvol_fast: 3,
            rvol_slow: 6,
            volume_oscillator: 6,
            ..ExtendedConfig::DEFAULT
        };
        let full = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let prefix = ExtendedReadings::compute(&candles[..15], &readings(&candles[..15]), &config);
        for bar in 0..15 {
            assert_eq!(full.rvol_fast.get(bar), prefix.rvol_fast.get(bar));
            assert_eq!(full.rvol_slow.get(bar), prefix.rvol_slow.get(bar));
            assert_eq!(full.climax_flag.get(bar), prefix.climax_flag.get(bar));
        }
    }

    #[test]
    fn obv_accumulates_signed_volume() {
        let candles = [
            candle(0, 10.0, 10.0, 10.0, 10.0, 1.0),
            candle(1, 11.0, 11.0, 11.0, 11.0, 2.0),
            candle(2, 10.0, 10.0, 10.0, 10.0, 3.0),
        ];
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.obv.get(0), Some(0.0));
        assert_eq!(extended.obv.get(1), Some(2.0));
        assert_eq!(extended.obv.get(2), Some(-1.0));
    }

    #[test]
    fn cmf_is_one_when_every_close_is_the_high() {
        let candles: Vec<Candle> = (0..6)
            .map(|index| candle(index, 10.0, 11.0, 10.0, 11.0, 10.0))
            .collect();
        let config = ExtendedConfig {
            flow_window: 5,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let value = extended.cmf.get(5).expect("cmf");
        assert!((value - 1.0).abs() < 1e-9, "cmf {value}");
    }

    #[test]
    fn mfi_is_one_hundred_when_every_bar_rises() {
        let candles: Vec<Candle> = (0..6)
            .map(|index| {
                let price = 10.0 + index as f64;
                candle(index, price, price + 0.5, price - 0.5, price, 10.0)
            })
            .collect();
        let config = ExtendedConfig {
            mfi_window: 3,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        assert_eq!(extended.mfi.get(5), Some(100.0));
    }

    #[test]
    fn flow_readings_are_causal() {
        let candles: Vec<Candle> = (0..12)
            .map(|index| {
                let price = 10.0 + (index as f64 * 0.4).sin();
                candle(index, price, price + 0.5, price - 0.5, price, 10.0)
            })
            .collect();
        let config = ExtendedConfig {
            flow_window: 5,
            mfi_window: 3,
            ..ExtendedConfig::DEFAULT
        };
        let full = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let prefix = ExtendedReadings::compute(&candles[..8], &readings(&candles[..8]), &config);
        for bar in 0..8 {
            assert_eq!(full.obv.get(bar), prefix.obv.get(bar));
            assert_eq!(full.cmf.get(bar), prefix.cmf.get(bar));
            assert_eq!(full.mfi.get(bar), prefix.mfi.get(bar));
        }
    }

    #[test]
    fn atr_term_is_available_after_the_slow_window() {
        let candles: Vec<Candle> = (0..120)
            .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.atr_ratio.get(99), None);
        let ratio = extended.atr_ratio.get(119).expect("atr ratio");
        assert!((ratio - 1.0).abs() < 1e-9, "ratio {ratio}");
    }

    #[test]
    fn range_and_mar_read_the_window() {
        let candles: Vec<Candle> = (0..25)
            .map(|index| {
                let close = 100.0 + index as f64;
                candle(index, close, close + 1.0, close - 1.0, close, 10.0)
            })
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        let range = extended.range_expansion.get(24).expect("range");
        assert!((range - 1.0).abs() < 1e-9, "range {range}");
        assert!(extended.mar_fast.get(24).is_some());
        assert!(extended.mar_ratio.get(24).is_some());
    }

    #[test]
    fn keltner_and_donchian_publish_bands() {
        let candles: Vec<Candle> = (0..30)
            .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        let upper = extended.keltner_upper.get(29).expect("keltner upper");
        let lower = extended.keltner_lower.get(29).expect("keltner lower");
        assert!(upper > lower);
        assert!(extended.keltner_position.get(29).is_some());
        assert_eq!(extended.donchian_upper.get(29), Some(101.0));
        assert_eq!(extended.donchian_lower.get(29), Some(99.0));
    }

    #[test]
    fn aroon_reads_a_rising_series() {
        let candles: Vec<Candle> = (0..30)
            .map(|index| {
                let close = 100.0 + index as f64;
                candle(index, close, close + 1.0, close - 1.0, close, 10.0)
            })
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        let up = extended.aroon_up.get(29).expect("aroon up");
        assert!(up > 90.0, "up {up}");
        assert!(extended.aroon_osc.get(29).is_some());
    }

    #[test]
    fn structure_and_levels_publish_when_history_exists() {
        let candles: Vec<Candle> = (0..80)
            .map(|index| {
                let close = 100.0 + (index as f64 * 0.3).sin() * 5.0;
                candle(index, close, close + 1.0, close - 1.0, close, 10.0)
            })
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert!(extended.last_pivot_high.get(79).is_some());
        assert!(extended.last_pivot_low.get(79).is_some());
        assert!(extended.structure_state.get(79).is_some());
        assert!(extended.level_touches.get(79).is_some());
    }

    #[test]
    fn candle_anatomy_reads_the_bar() {
        let candles = [candle(0, 100.0, 101.0, 99.0, 100.8, 10.0)];
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert!((extended.body_ratio.get(0).expect("body") - 0.4).abs() < 1e-9);
        assert!((extended.upper_wick_ratio.get(0).expect("upper") - 0.1).abs() < 1e-9);
        assert!((extended.lower_wick_ratio.get(0).expect("lower") - 0.5).abs() < 1e-9);
        assert!((extended.close_position.get(0).expect("position") - 0.9).abs() < 1e-9);
    }

    #[test]
    fn candle_pattern_flags_an_engulfing_bar() {
        let candles = [
            candle(0, 100.5, 100.6, 99.9, 100.0, 10.0),
            candle(1, 99.9, 101.1, 99.8, 101.0, 10.0),
        ];
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.pattern_code.get(1), Some(1.0));
        assert_eq!(extended.pattern_direction.get(1), Some(1.0));
    }

    #[test]
    fn gap_reads_the_open_jump() {
        let candles = [
            candle(0, 100.0, 100.5, 99.5, 100.0, 10.0),
            candle(1, 101.0, 101.2, 99.8, 99.9, 10.0),
        ];
        let readings = readings(&candles);
        assert!(readings.atr14.get(1).is_none());
        let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
        assert!(extended.gap_atr.get(1).is_none());
        let readings = Readings::compute_with_config(
            &candles,
            &ReadingsConfig {
                atr: 1,
                ..ReadingsConfig::DEFAULT
            },
        );
        extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
        assert!(extended.gap_atr.get(1).is_some());
    }

    #[test]
    fn path_reads_efficiency_and_extremes() {
        let candles: Vec<Candle> = (0..20)
            .map(|index| {
                let close = 100.0 + index as f64;
                candle(index, close, close + 1.0, close - 1.0, close, 10.0)
            })
            .collect();
        let config = ExtendedConfig {
            path_window: 10,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let efficiency = extended.path_efficiency.get(19).expect("efficiency");
        assert!((efficiency - 1.0).abs() < 1e-9, "efficiency {efficiency}");
        assert_eq!(extended.bars_since_high.get(19), Some(0.0));
        assert!(extended.range_position.get(19).expect("position") > 0.9);
        assert!(extended.drawdown_atr.get(19).expect("drawdown") <= 0.0);
    }

    #[test]
    fn volume_oscillator_reads_the_fast_slow_gap() {
        let candles: Vec<Candle> = (0..30)
            .map(|index| {
                let volume = if index == 29 { 100.0 } else { 10.0 };
                candle(index, 100.0, 101.0, 99.0, 100.0, volume)
            })
            .collect();
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.volume_oscillator.get(19), Some(0.0));
        let value = extended
            .volume_oscillator
            .get(29)
            .expect("the burst is measurable");
        assert!(value > 0.5, "oscillator {value}");
    }

    #[test]
    fn ad_line_accumulates_money_flow() {
        let candles = [
            candle(0, 1.0, 1.0, 1.0, 1.0, 10.0),
            candle(1, 1.0, 2.0, 1.0, 2.0, 10.0),
            candle(2, 2.0, 3.0, 2.0, 3.0, 5.0),
        ];
        let extended =
            ExtendedReadings::compute(&candles, &readings(&candles), &ExtendedConfig::DEFAULT);
        assert_eq!(extended.ad_line.get(0), Some(0.0));
        assert_eq!(extended.ad_line.get(1), Some(10.0));
        assert_eq!(extended.ad_line.get(2), Some(15.0));
    }

    #[test]
    fn direction_changes_counts_sign_flips() {
        let candles: Vec<Candle> = (0..20)
            .map(|index| {
                let close = 100.0 + (index % 2) as f64;
                candle(index, close, close + 1.0, close - 1.0, close, 10.0)
            })
            .collect();
        let config = ExtendedConfig {
            path_window: 20,
            ..ExtendedConfig::DEFAULT
        };
        let extended = ExtendedReadings::compute(&candles, &readings(&candles), &config);
        let value = extended
            .direction_changes
            .get(19)
            .expect("the window is measurable");
        assert!((value - 0.9).abs() < 1e-9, "direction changes {value}");
    }
}
