//! Family-level edge cases for the extended, panel, and session readings.
//!
//! The suite proves every reading is finite or null on an empty series, a
//! single candle, a short series, a flat range, zero volume, and equal highs
//! and lows. It does not replace a per-indicator formula test; it proves the
//! shared edge behavior and the absence of NaN or infinity.

use quantrule_data::Candle;
use quantrule_model::panel::{PanelConfig, PanelInstrument, PanelReadings};
use quantrule_model::readings::{ExtendedConfig, ExtendedReadings, ReadingSeries, Readings};
use quantrule_model::sessions::{SessionConfig, SessionReadings};

fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle::new(index, index as i64, open, high, low, close, Some(volume))
        .expect("the generated candle is valid")
}

fn flat_series(count: u64, volume: f64) -> Vec<Candle> {
    (0..count)
        .map(|index| candle(index, 100.0, 100.0, 100.0, 100.0, volume))
        .collect()
}

fn assert_safe(series: &ReadingSeries) {
    for value in series.values().iter().flatten() {
        assert!(value.is_finite(), "a reading is not finite: {value}");
    }
}

fn assert_empty(series: &ReadingSeries) {
    assert!(series.values().is_empty(), "an empty input published a bar");
}

fn assert_extended_safe(extended: &ExtendedReadings) {
    assert_safe(&extended.vwap);
    assert_safe(&extended.vwap_distance_atr);
    assert_safe(&extended.vwap_slope);
    assert_safe(&extended.vwap_band_upper);
    assert_safe(&extended.vwap_band_lower);
    assert_safe(&extended.poc);
    assert_safe(&extended.vah);
    assert_safe(&extended.val);
    assert_safe(&extended.poc_distance_atr);
    assert_safe(&extended.va_position);
    assert_safe(&extended.profile_skew);
    assert_safe(&extended.profile_concentration);
    assert_safe(&extended.poc_migration);
    assert_safe(&extended.rvol_fast);
    assert_safe(&extended.rvol_slow);
    assert_safe(&extended.rvol_ratio);
    assert_safe(&extended.volume_oscillator);
    assert_safe(&extended.climax_flag);
    assert_safe(&extended.climax_direction);
    assert_safe(&extended.climax_strength);
    assert_safe(&extended.absorption_flag);
    assert_safe(&extended.absorption_direction);
    assert_safe(&extended.absorption_strength);
    assert_safe(&extended.obv);
    assert_safe(&extended.obv_ma);
    assert_safe(&extended.obv_slope);
    assert_safe(&extended.cmf);
    assert_safe(&extended.cmf_slope);
    assert_safe(&extended.mfi);
    assert_safe(&extended.ad_line);
    assert_safe(&extended.ad_ma);
    assert_safe(&extended.atr_ratio);
    assert_safe(&extended.atr_slope);
    assert_safe(&extended.vol_regime);
    assert_safe(&extended.range_expansion);
    assert_safe(&extended.mar_fast);
    assert_safe(&extended.mar_slow);
    assert_safe(&extended.mar_ratio);
    assert_safe(&extended.keltner_upper);
    assert_safe(&extended.keltner_lower);
    assert_safe(&extended.keltner_position);
    assert_safe(&extended.donchian_upper);
    assert_safe(&extended.donchian_lower);
    assert_safe(&extended.donchian_position);
    assert_safe(&extended.structure_state);
    assert_safe(&extended.lower_high_count);
    assert_safe(&extended.lower_low_count);
    assert_safe(&extended.higher_high_count);
    assert_safe(&extended.higher_low_count);
    assert_safe(&extended.fib_direction);
    assert_safe(&extended.fib_low);
    assert_safe(&extended.fib_high);
    assert_safe(&extended.fib_236);
    assert_safe(&extended.fib_382);
    assert_safe(&extended.fib_500);
    assert_safe(&extended.fib_618);
    assert_safe(&extended.fib_786);
    assert_safe(&extended.fib_position);
    assert_safe(&extended.fib_nearest_ratio);
    assert_safe(&extended.fib_nearest_atr);
    assert_safe(&extended.band_upper);
    assert_safe(&extended.band_lower);
    assert_safe(&extended.band_center);
    assert_safe(&extended.band_touches);
    assert_safe(&extended.band_age);
    assert_safe(&extended.band_width_atr);
    assert_safe(&extended.band_position);
    assert_safe(&extended.last_pivot_high);
    assert_safe(&extended.last_pivot_low);
    assert_safe(&extended.level_touches);
    assert_safe(&extended.level_age);
    assert_safe(&extended.aroon_up);
    assert_safe(&extended.aroon_down);
    assert_safe(&extended.aroon_osc);
    assert_safe(&extended.body_ratio);
    assert_safe(&extended.upper_wick_ratio);
    assert_safe(&extended.lower_wick_ratio);
    assert_safe(&extended.close_position);
    assert_safe(&extended.pattern_code);
    assert_safe(&extended.pattern_direction);
    assert_safe(&extended.gap_atr);
    assert_safe(&extended.gap_filled);
    assert_safe(&extended.path_efficiency);
    assert_safe(&extended.direction_changes);
    assert_safe(&extended.bars_since_high);
    assert_safe(&extended.bars_since_low);
    assert_safe(&extended.drawdown_atr);
    assert_safe(&extended.range_position);
}

fn assert_extended_empty(extended: &ExtendedReadings) {
    assert_empty(&extended.vwap);
    assert_empty(&extended.vwap_distance_atr);
    assert_empty(&extended.vwap_slope);
    assert_empty(&extended.vwap_band_upper);
    assert_empty(&extended.vwap_band_lower);
    assert_empty(&extended.poc);
    assert_empty(&extended.vah);
    assert_empty(&extended.val);
    assert_empty(&extended.poc_distance_atr);
    assert_empty(&extended.va_position);
    assert_empty(&extended.profile_skew);
    assert_empty(&extended.profile_concentration);
    assert_empty(&extended.poc_migration);
    assert_empty(&extended.rvol_fast);
    assert_empty(&extended.rvol_slow);
    assert_empty(&extended.rvol_ratio);
    assert_empty(&extended.volume_oscillator);
    assert_empty(&extended.climax_flag);
    assert_empty(&extended.climax_direction);
    assert_empty(&extended.climax_strength);
    assert_empty(&extended.absorption_flag);
    assert_empty(&extended.absorption_direction);
    assert_empty(&extended.absorption_strength);
    assert_empty(&extended.obv);
    assert_empty(&extended.obv_ma);
    assert_empty(&extended.obv_slope);
    assert_empty(&extended.cmf);
    assert_empty(&extended.cmf_slope);
    assert_empty(&extended.mfi);
    assert_empty(&extended.ad_line);
    assert_empty(&extended.ad_ma);
    assert_empty(&extended.atr_ratio);
    assert_empty(&extended.atr_slope);
    assert_empty(&extended.vol_regime);
    assert_empty(&extended.range_expansion);
    assert_empty(&extended.mar_fast);
    assert_empty(&extended.mar_slow);
    assert_empty(&extended.mar_ratio);
    assert_empty(&extended.keltner_upper);
    assert_empty(&extended.keltner_lower);
    assert_empty(&extended.keltner_position);
    assert_empty(&extended.donchian_upper);
    assert_empty(&extended.donchian_lower);
    assert_empty(&extended.donchian_position);
    assert_empty(&extended.structure_state);
    assert_empty(&extended.lower_high_count);
    assert_empty(&extended.lower_low_count);
    assert_empty(&extended.higher_high_count);
    assert_empty(&extended.higher_low_count);
    assert_empty(&extended.fib_direction);
    assert_empty(&extended.fib_low);
    assert_empty(&extended.fib_high);
    assert_empty(&extended.fib_236);
    assert_empty(&extended.fib_382);
    assert_empty(&extended.fib_500);
    assert_empty(&extended.fib_618);
    assert_empty(&extended.fib_786);
    assert_empty(&extended.fib_position);
    assert_empty(&extended.fib_nearest_ratio);
    assert_empty(&extended.fib_nearest_atr);
    assert_empty(&extended.band_upper);
    assert_empty(&extended.band_lower);
    assert_empty(&extended.band_center);
    assert_empty(&extended.band_touches);
    assert_empty(&extended.band_age);
    assert_empty(&extended.band_width_atr);
    assert_empty(&extended.band_position);
    assert_empty(&extended.last_pivot_high);
    assert_empty(&extended.last_pivot_low);
    assert_empty(&extended.level_touches);
    assert_empty(&extended.level_age);
    assert_empty(&extended.aroon_up);
    assert_empty(&extended.aroon_down);
    assert_empty(&extended.aroon_osc);
    assert_empty(&extended.body_ratio);
    assert_empty(&extended.upper_wick_ratio);
    assert_empty(&extended.lower_wick_ratio);
    assert_empty(&extended.close_position);
    assert_empty(&extended.pattern_code);
    assert_empty(&extended.pattern_direction);
    assert_empty(&extended.gap_atr);
    assert_empty(&extended.gap_filled);
    assert_empty(&extended.path_efficiency);
    assert_empty(&extended.direction_changes);
    assert_empty(&extended.bars_since_high);
    assert_empty(&extended.bars_since_low);
    assert_empty(&extended.drawdown_atr);
    assert_empty(&extended.range_position);
}

fn assert_session_safe(sessions: &SessionReadings) {
    assert_safe(&sessions.session_code);
    assert_safe(&sessions.session_age);
    assert_safe(&sessions.opening_range_high);
    assert_safe(&sessions.opening_range_low);
    assert_safe(&sessions.opening_range_atr);
    assert_safe(&sessions.session_range_atr);
    assert_safe(&sessions.session_vwap);
    assert_safe(&sessions.overnight_return);
    assert_safe(&sessions.intraday_return);
    assert_safe(&sessions.calendar_bucket);
}

fn assert_session_empty(sessions: &SessionReadings) {
    assert_empty(&sessions.session_code);
    assert_empty(&sessions.session_age);
    assert_empty(&sessions.opening_range_high);
    assert_empty(&sessions.opening_range_low);
    assert_empty(&sessions.opening_range_atr);
    assert_empty(&sessions.session_range_atr);
    assert_empty(&sessions.session_vwap);
    assert_empty(&sessions.overnight_return);
    assert_empty(&sessions.intraday_return);
    assert_empty(&sessions.calendar_bucket);
}

#[test]
fn an_empty_series_publishes_no_readings() {
    let readings = Readings::compute(&[]);
    let extended = ExtendedReadings::compute(&[], &readings, &ExtendedConfig::DEFAULT);
    assert_extended_empty(&extended);
    let sessions = SessionReadings::compute(&[], &readings, &SessionConfig::london());
    assert_session_empty(&sessions);
    let panel = PanelReadings::compute(&[], &PanelConfig::DEFAULT);
    assert!(panel.timestamps.is_empty());
    assert!(panel.instruments.is_empty());
}

#[test]
fn a_single_candle_publishes_finite_or_null_readings() {
    let candles = [candle(0, 100.0, 101.0, 99.0, 100.0, 10.0)];
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    assert_extended_safe(&extended);
    let sessions = SessionReadings::compute(&candles, &readings, &SessionConfig::london());
    assert_session_safe(&sessions);
}

#[test]
fn a_short_series_keeps_unavailable_readings_null() {
    let candles = flat_series(5, 10.0);
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    assert_extended_safe(&extended);
    assert!(extended.vwap.get(4).is_none());
    assert!(extended.atr_ratio.get(4).is_none());
    assert!(extended.poc.get(4).is_none());
}

#[test]
fn a_flat_series_does_not_divide_by_zero() {
    let candles = flat_series(40, 10.0);
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    assert_extended_safe(&extended);
    assert_eq!(extended.vwap_distance_atr.get(39), None);
    assert_eq!(extended.range_expansion.get(39), None);
    assert_eq!(extended.keltner_position.get(39), None);
    assert_eq!(extended.poc.get(39), None);
}

#[test]
fn zero_volume_publishes_no_volume_weighted_readings() {
    let candles: Vec<Candle> = (0..40)
        .map(|index| {
            let close = 100.0 + index as f64 * 0.1;
            candle(index, close, close + 0.5, close - 0.5, close, 0.0)
        })
        .collect();
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    assert_extended_safe(&extended);
    assert_eq!(extended.vwap.get(39), None);
    assert_eq!(extended.poc.get(39), None);
    assert_eq!(extended.rvol_ratio.get(39), None);
}

#[test]
fn equal_highs_and_lows_are_handled() {
    let candles = flat_series(40, 10.0);
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    assert_extended_safe(&extended);
    assert_eq!(extended.range_position.get(39), None);
    assert_eq!(extended.profile_concentration.get(39), None);
}

#[test]
fn a_one_bar_panel_and_session_are_safe() {
    let left = [candle(0, 100.0, 101.0, 99.0, 100.0, 10.0)];
    let right = [candle(0, 100.0, 101.0, 99.0, 100.0, 10.0)];
    let panel = [
        PanelInstrument {
            name: "A",
            candles: &left,
        },
        PanelInstrument {
            name: "B",
            candles: &right,
        },
    ];
    let readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
    assert_eq!(readings.timestamps.len(), 1);
    for series in readings.instruments.values() {
        for value in &series.relative_strength {
            assert!(value.is_none_or(f64::is_finite));
        }
    }
    let session_readings = Readings::compute(&left);
    let sessions = SessionReadings::compute(&left, &session_readings, &SessionConfig::london());
    assert_session_safe(&sessions);
}
