//! Row-level contracts for single-instrument signal events.
//!
//! The tests share the helper surface in `row_tests` and drive the signal
//! detector with explicit extended readings.

use super::row_tests::{Expected, assert_contract, candle, event_config, series_at};
use crate::events::{EventRecord, EventType, codes};
use crate::readings::{ExtendedConfig, ExtendedReadings, Readings};
use quantrule_data::Candle;

fn flat_series(len: usize) -> Vec<Candle> {
    (0..len)
        .map(|index| candle(index as u64, 100.0, 101.0, 99.0, 100.5, 10.0))
        .collect()
}

fn wide_series(len: usize) -> Vec<Candle> {
    let mut candles = flat_series(len);
    if len > 29 {
        candles[29] = candle(29, 101.0, 110.0, 100.0, 109.0, 10.0);
    }
    candles
}

fn rejection_series(len: usize) -> Vec<Candle> {
    let mut candles = flat_series(len);
    if len > 29 {
        candles[29] = candle(29, 105.0, 110.0, 100.0, 101.0, 10.0);
    }
    candles
}

fn gap_series(len: usize, close: f64) -> Vec<Candle> {
    let mut candles = flat_series(len);
    if len > 29 {
        candles[29] = candle(29, 102.0, 103.0, 101.0, close, 10.0);
    }
    candles
}

fn signal_events(
    len: usize,
    candles: Vec<Candle>,
    overrides: impl Fn(&mut ExtendedReadings, usize),
) -> Vec<EventRecord> {
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    overrides(&mut extended, len);
    let mut events = Vec::new();
    super::signals::compute(&candles, &readings, &extended, &event_config(), &mut events);
    events
}

#[test]
fn signal_atr_compression_row_contract() {
    assert_contract(
        "atr compression",
        codes::SIGNAL_ATR_COMPRESSION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 16,
            source: 28,
            direction: 0,
            occurred: 29,
            available: 29,
            primary: 0.6,
            reference: 0.7,
        },
        |len| {
            signal_events(len, flat_series(len), |extended, len| {
                extended.atr_ratio = series_at(len, &[(28, 0.8), (29, 0.6)]);
            })
        },
    );
}

#[test]
fn signal_atr_expansion_row_contract() {
    assert_contract(
        "atr expansion",
        codes::SIGNAL_ATR_EXPANSION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 16,
            source: 28,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 1.4,
            reference: 1.3,
        },
        |len| {
            signal_events(len, flat_series(len), |extended, len| {
                extended.atr_ratio = series_at(len, &[(28, 1.2), (29, 1.4)]);
            })
        },
    );
}

#[test]
fn signal_atr_regime_row_contract() {
    assert_contract(
        "atr regime",
        codes::SIGNAL_ATR_REGIME,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 16,
            source: 28,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 1.0,
            reference: 0.0,
        },
        |len| {
            signal_events(len, flat_series(len), |extended, len| {
                extended.vol_regime = series_at(len, &[(28, 0.0), (29, 1.0)]);
            })
        },
    );
}

#[test]
fn signal_candle_momentum_row_contract() {
    assert_contract(
        "candle momentum",
        codes::SIGNAL_CANDLE_MOMENTUM,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 37,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 0.8,
            reference: 0.9,
        },
        |len| {
            signal_events(len, wide_series(len), |extended, len| {
                extended.body_ratio = series_at(len, &[(29, 0.8)]);
                extended.close_position = series_at(len, &[(29, 0.9)]);
            })
        },
    );
}

#[test]
fn signal_candle_rejection_row_contract() {
    assert_contract(
        "candle rejection",
        codes::SIGNAL_CANDLE_REJECTION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 37,
            direction: -1,
            occurred: 29,
            available: 29,
            primary: 0.7,
            reference: 0.6,
        },
        |len| {
            signal_events(len, rejection_series(len), |extended, len| {
                extended.upper_wick_ratio = series_at(len, &[(29, 0.7)]);
                extended.lower_wick_ratio = series_at(len, &[(29, 0.2)]);
            })
        },
    );
}

#[test]
fn signal_candle_marubozu_row_contract() {
    assert_contract(
        "candle marubozu",
        codes::SIGNAL_CANDLE_MARUBOZU,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 37,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 0.95,
            reference: 0.9,
        },
        |len| {
            signal_events(len, wide_series(len), |extended, len| {
                extended.body_ratio = series_at(len, &[(29, 0.95)]);
            })
        },
    );
}

fn pattern_events(len: usize, pattern: f64) -> Vec<EventRecord> {
    signal_events(len, flat_series(len), |extended, len| {
        extended.pattern_code = series_at(len, &[(29, pattern)]);
        extended.pattern_direction = series_at(len, &[(29, 1.0)]);
    })
}

#[test]
fn signal_pattern_hammer_row_contract() {
    assert_contract(
        "pattern hammer",
        codes::SIGNAL_PATTERN_HAMMER,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 38,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 5.0,
            reference: 0.0,
        },
        |len| pattern_events(len, 5.0),
    );
}

#[test]
fn signal_pattern_star_row_contract() {
    assert_contract(
        "pattern star",
        codes::SIGNAL_PATTERN_STAR,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 38,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 6.0,
            reference: 0.0,
        },
        |len| pattern_events(len, 6.0),
    );
}

#[test]
fn signal_pattern_three_soldiers_row_contract() {
    assert_contract(
        "pattern three soldiers",
        codes::SIGNAL_PATTERN_THREE_SOLDIERS,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 38,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 7.0,
            reference: 0.0,
        },
        |len| pattern_events(len, 7.0),
    );
}

#[test]
fn signal_pattern_three_crows_row_contract() {
    assert_contract(
        "pattern three crows",
        codes::SIGNAL_PATTERN_THREE_CROWS,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 10,
            source: 38,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 8.0,
            reference: 0.0,
        },
        |len| pattern_events(len, 8.0),
    );
}

#[test]
fn signal_gap_continuation_row_contract() {
    assert_contract(
        "gap continuation",
        codes::SIGNAL_GAP_CONTINUATION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 20,
            source: 39,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 0.6,
            reference: 0.5,
        },
        |len| {
            signal_events(len, gap_series(len, 102.5), |extended, len| {
                extended.gap_atr = series_at(len, &[(29, 0.6)]);
            })
        },
    );
}

#[test]
fn signal_gap_fade_row_contract() {
    assert_contract(
        "gap fade",
        codes::SIGNAL_GAP_FADE,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 20,
            source: 39,
            direction: -1,
            occurred: 29,
            available: 29,
            primary: -0.6,
            reference: -0.5,
        },
        |len| {
            signal_events(len, gap_series(len, 101.5), |extended, len| {
                extended.gap_atr = series_at(len, &[(29, -0.6)]);
            })
        },
    );
}

fn wick_test_events(len: usize) -> Vec<EventRecord> {
    let candles = flat_series(len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    if len > 29 {
        extended.upper_wick_ratio = series_at(len, &[(29, 0.7)]);
        extended.last_pivot_high = series_at(len, &[(29, 101.0)]);
    }
    let mut events = Vec::new();
    if len > 0 {
        super::levels::level_extended_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            len - 1,
            &mut events,
        );
    }
    events
}

#[test]
fn level_wick_test_row_contract() {
    assert_contract(
        "wick test",
        codes::LEVEL_WICK_TEST,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Level,
            subtype: 1,
            source: 37,
            direction: 0,
            occurred: 29,
            available: 29,
            primary: 0.7,
            reference: 101.0,
        },
        wick_test_events,
    );
}
