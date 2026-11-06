//! Extended reading tests.

use super::{ExtendedConfig, ExtendedReadings};
use crate::readings::{Readings, ReadingsConfig};
use quantrule_data::Candle;

fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle::new(index, index as i64, open, high, low, close, Some(volume)).expect("valid candle")
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
    assert_eq!(extended.absorption_direction.get(19), Some(1.0));
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
    let gap = extended.gap_atr.get(1).expect("the gap is available");
    assert!((gap - 1.0 / 1.4).abs() < 1e-9, "gap {gap}");
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
