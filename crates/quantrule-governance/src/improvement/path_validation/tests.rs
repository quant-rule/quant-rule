//! Tests for the path-validation primitive arithmetic.

use super::model::{counts, path_hit};
use super::*;

fn reading(close: f64, high: f64, low: f64) -> Reading {
    Reading {
        bar: 0,
        high,
        low,
        close,
        atr: Some(1.0),
        atr_ratio: Some(1.0),
        rsi: Some(50.0),
        adx_regime: Some(0.0),
        adx_slope: Some(0.0),
        di_state: Some(0.0),
        ema_stack: Some(0.0),
        macd_state: Some(0.0),
        ema_fast: Some(close),
        ema_mid: Some(close),
        ema_slow: Some(close),
    }
}

#[test]
fn stop_wins_the_same_bar_tie() {
    let rows = vec![
        reading(100.0, 100.5, 99.5),
        reading(99.0, 101.5, 98.5),
        reading(100.0, 100.5, 99.5),
    ];
    assert_eq!(path_hit(&rows, 0, 1, 2, 1.0, 1.0), Some(false));
}

#[test]
fn a_target_before_the_stop_is_a_hit() {
    let rows = vec![
        reading(100.0, 100.5, 99.5),
        reading(101.0, 101.5, 99.9),
        reading(100.0, 100.5, 99.5),
    ];
    assert_eq!(path_hit(&rows, 0, 1, 2, 1.0, 1.0), Some(true));
}

#[test]
fn a_short_history_is_unavailable() {
    let rows = vec![reading(100.0, 100.5, 99.5)];
    assert_eq!(path_hit(&rows, 0, 1, 2, 1.0, 1.0), None);
}

#[test]
fn counts_ignore_unavailable_bars() {
    assert_eq!(counts(&[Some(true), None, Some(false)]), (1, 2));
}
