//! Row-level contracts for extended participation, profile, and divergence rows.
//!
//! These tests live in the detector module so they can drive the per-bar
//! detectors with explicit readings. Each row checks its record, availability,
//! uniqueness, determinism, and prefix causality.

use super::ExtendedEventConfig;
use super::divergence::{atr_divergences, divergence_events};
use super::flow::flow_divergences;
use super::participation::participation_events;
use super::profile::profile_events;
use crate::events::{EventRecord, EventType, codes};
use crate::readings::{ExtendedConfig, ExtendedReadings, PivotKind, ReadingSeries, Readings};
use quantrule_data::Candle;

struct Expected {
    event_type: EventType,
    subtype: u8,
    source: u16,
    direction: i8,
    occurred: usize,
    available: usize,
    primary: f64,
    reference: f64,
}

fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle::new(index, index as i64, open, high, low, close, Some(volume)).expect("valid candle")
}

fn series(len: usize, value: impl Fn(usize) -> Option<f64>) -> ReadingSeries {
    ReadingSeries::new((0..len).map(value).collect())
}

fn series_at(len: usize, values: &[(usize, f64)]) -> ReadingSeries {
    let mut data = vec![None; len];
    for (bar, value) in values {
        if *bar < len {
            data[*bar] = Some(*value);
        }
    }
    ReadingSeries::new(data)
}

fn flat_candles(len: usize) -> Vec<Candle> {
    (0..len)
        .map(|index| candle(index as u64, 100.0, 101.0, 99.0, 100.5, 10.0))
        .collect()
}

fn pivot_candles(kind: PivotKind, first: f64, second: f64, len: usize) -> Vec<Candle> {
    (0..len)
        .map(|index| match kind {
            PivotKind::Low => {
                let low = if index == 5 {
                    first
                } else if index == 15 {
                    second
                } else {
                    101.0
                };
                candle(index as u64, 101.0, 101.5, low, 101.0, 10.0)
            }
            PivotKind::High => {
                let high = if index == 5 {
                    first
                } else if index == 15 {
                    second
                } else {
                    100.0
                };
                candle(index as u64, 100.0, high, 99.5, 100.0, 10.0)
            }
        })
        .collect()
}

fn two_pivot_values(len: usize, first: f64, second: f64) -> ReadingSeries {
    series_at(len, &[(5, first), (15, second)])
}

fn event_config() -> ExtendedEventConfig {
    ExtendedEventConfig::from_secs(3600).expect("timeframe")
}

fn assert_contract(
    label: &str,
    code: u16,
    full_len: usize,
    cutoffs: &[usize],
    expected: Expected,
    compute: impl Fn(usize) -> Vec<EventRecord>,
) {
    let full = compute(full_len);
    let events: Vec<&EventRecord> = full
        .iter()
        .filter(|event| event.event_code == code)
        .collect();
    assert!(!events.is_empty(), "{label} is missing");
    let event = events[0];
    assert_eq!(event.event_type, expected.event_type);
    assert_eq!(event.event_subtype_code, expected.subtype);
    assert_eq!(event.source_code, expected.source);
    assert_eq!(event.direction_code, expected.direction);
    assert_eq!(event.occurred_bar_index, expected.occurred);
    assert_eq!(event.available_bar_index, expected.available);
    assert_eq!(event.reference_code, None);
    assert_eq!(
        event.primary_value.map(f64::to_bits),
        Some(expected.primary.to_bits())
    );
    assert_eq!(
        event.reference_value.map(f64::to_bits),
        Some(expected.reference.to_bits())
    );
    for (index, event) in events.iter().enumerate() {
        assert!(
            event.available_bar_index >= event.occurred_bar_index,
            "{label} became available before it occurred"
        );
        assert!(
            event.available_bar_index < full_len,
            "{label} became available after the series"
        );
        assert!(
            !events[..index].contains(event),
            "{label} published the same record twice"
        );
    }
    assert_eq!(full, compute(full_len), "{label} is not deterministic");
    for cutoff in cutoffs {
        let prefix: Vec<EventRecord> = compute(*cutoff)
            .into_iter()
            .filter(|event| event.event_code == code)
            .collect();
        let expected: Vec<EventRecord> = full
            .iter()
            .filter(|event| event.event_code == code && event.available_bar_index < *cutoff)
            .copied()
            .collect();
        assert_eq!(prefix, expected, "{label} changed at prefix {cutoff}");
    }
}

fn vwap_divergence_events(len: usize, hidden: bool) -> Vec<EventRecord> {
    let (first, second, first_value, second_value) = if hidden {
        (99.0, 100.0, 1.5, 0.5)
    } else {
        (99.0, 98.0, 0.5, 1.5)
    };
    let candles = pivot_candles(PivotKind::Low, first, second, len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    extended.vwap_distance_atr = two_pivot_values(len, first_value, second_value);
    let mut events = Vec::new();
    divergence_events(&readings, &extended, &event_config(), &mut events);
    events
}

#[test]
fn divergence_vwap_regular_bullish_row_contract() {
    assert_contract(
        "vwap regular bullish",
        codes::DIVERGENCE_VWAP_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 1,
            source: 19,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| vwap_divergence_events(len, false),
    );
}

#[test]
fn divergence_vwap_hidden_bullish_row_contract() {
    assert_contract(
        "vwap hidden bullish",
        codes::DIVERGENCE_VWAP_HIDDEN_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 3,
            source: 19,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 100.0,
            reference: 0.5,
        },
        |len| vwap_divergence_events(len, true),
    );
}

fn profile_concentration_events(len: usize) -> Vec<EventRecord> {
    let candles = flat_candles(len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    if len > 29 {
        extended.profile_concentration = series_at(len, &[(28, 0.4), (29, 0.6)]);
        extended.poc = series_at(len, &[(29, 100.0)]);
    }
    let mut events = Vec::new();
    for bar in 1..len {
        profile_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            bar,
            &mut events,
        );
    }
    events
}

#[test]
fn signal_profile_concentration_row_contract() {
    assert_contract(
        "profile concentration",
        codes::SIGNAL_PROFILE_CONCENTRATION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 13,
            source: 20,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 0.6,
            reference: 0.5,
        },
        profile_concentration_events,
    );
}

fn climax_events(len: usize, close: f64) -> Vec<EventRecord> {
    let mut candles = flat_candles(len);
    if len > 29 {
        candles[29] = candle(29, 100.0, 103.0, 99.0, close, 100.0);
    }
    let readings = Readings::compute(&candles);
    let extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    let mut events = Vec::new();
    for bar in 1..len {
        participation_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            bar,
            &mut events,
        );
    }
    events
}

#[test]
fn signal_climax_reversal_row_contract() {
    assert_contract(
        "climax reversal",
        codes::SIGNAL_CLIMAX_REVERSAL,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 11,
            source: 21,
            direction: -1,
            occurred: 29,
            available: 29,
            primary: 6.666666666666667,
            reference: 101.0,
        },
        |len| climax_events(len, 100.5),
    );
}

#[test]
fn signal_climax_continuation_row_contract() {
    assert_contract(
        "climax continuation",
        codes::SIGNAL_CLIMAX_CONTINUATION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 11,
            source: 21,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 6.666666666666667,
            reference: 0.9499999999999993,
        },
        |len| climax_events(len, 102.8),
    );
}

fn absorption_test_events(len: usize) -> Vec<EventRecord> {
    let candles = flat_candles(len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    if len > 29 {
        extended.absorption_flag = series_at(len, &[(29, 1.0)]);
        extended.absorption_strength = series_at(len, &[(29, 1.0)]);
        extended.vwap = series_at(len, &[(29, 100.0)]);
    }
    let mut events = Vec::new();
    for bar in 1..len {
        participation_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            bar,
            &mut events,
        );
    }
    events
}

#[test]
fn level_absorption_test_row_contract() {
    assert_contract(
        "absorption test",
        codes::LEVEL_ABSORPTION_TEST,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Level,
            subtype: 1,
            source: 22,
            direction: 0,
            occurred: 29,
            available: 29,
            primary: 1.0,
            reference: 100.0,
        },
        absorption_test_events,
    );
}

fn absorption_boundary_events(len: usize, close: f64) -> Vec<EventRecord> {
    let mut candles = flat_candles(len);
    if len > 29 {
        candles[29] = candle(29, 100.0, 102.5, 100.0, close, 10.0);
    }
    let mut readings = Readings::compute(&candles);
    if len > 29 {
        readings.atr14 = series(len, |_| Some(1.0));
    }
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    if len > 28 {
        extended.absorption_flag = series_at(len, &[(28, 1.0)]);
        extended.absorption_direction = series_at(len, &[(28, 1.0)]);
    }
    let mut events = Vec::new();
    for bar in 1..len {
        participation_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            bar,
            &mut events,
        );
    }
    events
}

#[test]
fn signal_absorption_break_row_contract() {
    assert_contract(
        "absorption break",
        codes::SIGNAL_ABSORPTION_BREAK,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 12,
            source: 22,
            direction: 1,
            occurred: 29,
            available: 29,
            primary: 102.0,
            reference: 101.0,
        },
        |len| absorption_boundary_events(len, 102.0),
    );
}

#[test]
fn signal_absorption_rejection_row_contract() {
    assert_contract(
        "absorption rejection",
        codes::SIGNAL_ABSORPTION_REJECTION,
        30,
        &[28, 29, 30],
        Expected {
            event_type: EventType::Signal,
            subtype: 12,
            source: 22,
            direction: -1,
            occurred: 29,
            available: 29,
            primary: 100.0,
            reference: 101.0,
        },
        |len| absorption_boundary_events(len, 100.0),
    );
}

fn flow_divergence_events(
    len: usize,
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<EventRecord> {
    let candles = pivot_candles(kind, first, second, len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    for series in [
        &mut extended.obv,
        &mut extended.cmf,
        &mut extended.mfi,
        &mut extended.ad_line,
    ] {
        *series = two_pivot_values(len, first_value, second_value);
    }
    let mut events = Vec::new();
    flow_divergences(&readings, &extended, &event_config(), &mut events);
    events
}

fn atr_divergence_events(
    len: usize,
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<EventRecord> {
    let candles = pivot_candles(kind, first, second, len);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    extended.atr_ratio = two_pivot_values(len, first_value, second_value);
    let mut events = Vec::new();
    atr_divergences(&readings, &extended, &event_config(), &mut events);
    events
}

#[test]
fn divergence_obv_regular_bullish_row_contract() {
    assert_contract(
        "obv regular bullish",
        codes::DIVERGENCE_OBV_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 1,
            source: 24,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| flow_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_obv_hidden_bearish_row_contract() {
    assert_contract(
        "obv hidden bearish",
        codes::DIVERGENCE_OBV_HIDDEN_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 4,
            source: 24,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 100.5,
            reference: 1.5,
        },
        |len| flow_divergence_events(len, PivotKind::High, 101.0, 100.5, 0.5, 1.5),
    );
}

#[test]
fn divergence_cmf_hidden_bullish_row_contract() {
    assert_contract(
        "cmf hidden bullish",
        codes::DIVERGENCE_CMF_HIDDEN_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 3,
            source: 25,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 100.0,
            reference: 0.5,
        },
        |len| flow_divergence_events(len, PivotKind::Low, 99.0, 100.0, 1.5, 0.5),
    );
}

#[test]
fn divergence_mfi_regular_bullish_row_contract() {
    assert_contract(
        "mfi regular bullish",
        codes::DIVERGENCE_MFI_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 1,
            source: 26,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| flow_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_mfi_regular_bearish_row_contract() {
    assert_contract(
        "mfi regular bearish",
        codes::DIVERGENCE_MFI_REGULAR_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 2,
            source: 26,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 102.0,
            reference: 0.5,
        },
        |len| flow_divergence_events(len, PivotKind::High, 101.0, 102.0, 1.5, 0.5),
    );
}

#[test]
fn divergence_mfi_hidden_bullish_row_contract() {
    assert_contract(
        "mfi hidden bullish",
        codes::DIVERGENCE_MFI_HIDDEN_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 3,
            source: 26,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 100.0,
            reference: 0.5,
        },
        |len| flow_divergence_events(len, PivotKind::Low, 99.0, 100.0, 1.5, 0.5),
    );
}

#[test]
fn divergence_ad_regular_bullish_row_contract() {
    assert_contract(
        "ad regular bullish",
        codes::DIVERGENCE_AD_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 1,
            source: 27,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| flow_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_atr_bearish_row_contract() {
    assert_contract(
        "atr bearish",
        codes::DIVERGENCE_ATR_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            event_type: EventType::Divergence,
            subtype: 2,
            source: 28,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 102.0,
            reference: 0.5,
        },
        |len| atr_divergence_events(len, PivotKind::High, 101.0, 102.0, 1.5, 0.5),
    );
}
