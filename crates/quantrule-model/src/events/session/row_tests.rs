//! Row-level contracts for the session signal events.

use super::SessionEventSeries;
use crate::events::{EventRecord, EventType, codes};
use crate::readings::{Readings, ReadingsConfig};
use crate::sessions::{SessionConfig, SessionReadings, days_from_civil};
use quantrule_data::Candle;

fn timestamp(year: i64, month: u32, day: u32, hour: u32) -> i64 {
    days_from_civil(year, month, day) * 86_400 + i64::from(hour) * 3_600
}

fn series(days: usize) -> Vec<Candle> {
    let start = timestamp(2024, 1, 15, 0);
    let mut candles = Vec::new();
    let mut previous: f64 = 100.0;
    for index in 0..days * 24 {
        let close = 100.0 + index as f64 * 0.5;
        let open = previous;
        let high = open.max(close) + 0.5;
        let low = open.min(close) - 0.5;
        candles.push(
            Candle::new(
                index as u64,
                start + index as i64 * 3_600,
                open,
                high,
                low,
                close,
                Some(10.0),
            )
            .expect("valid candle"),
        );
        previous = close;
    }
    candles
}

fn events(candles: &[Candle]) -> SessionEventSeries {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let sessions = SessionReadings::compute(candles, &readings, &SessionConfig::london());
    SessionEventSeries::compute(
        candles,
        &readings,
        &sessions,
        &SessionConfig::london(),
        3_600,
    )
}

fn fade_candles() -> Vec<Candle> {
    let mut candles = series(2);
    let opening_low = candles[8].low().min(candles[9].low());
    let timestamp = candles[11].timestamp();
    candles[11] = Candle::new(
        11,
        timestamp,
        opening_low + 0.5,
        opening_low + 1.0,
        opening_low - 2.0,
        opening_low - 1.0,
        Some(10.0),
    )
    .expect("the fade candle is valid");
    let previous_close = candles[31].close();
    let timestamp = candles[32].timestamp();
    candles[32] = Candle::new(
        32,
        timestamp,
        previous_close + 5.0,
        previous_close + 6.0,
        previous_close + 1.0,
        previous_close + 2.0,
        Some(10.0),
    )
    .expect("the overnight-fade candle is valid");
    candles
}

fn sweep_candles(close: f64) -> Vec<Candle> {
    let mut candles = series(2);
    let timestamp = candles[14].timestamp();
    candles[14] = Candle::new(14, timestamp, 104.5, 105.0, 104.0, 104.5, Some(10.0))
        .expect("the inside candle is valid");
    let timestamp = candles[15].timestamp();
    candles[15] = Candle::new(15, timestamp, 104.5, 106.0, 104.0, close, Some(10.0))
        .expect("the sweep candle is valid");
    candles
}

fn contraction_candles() -> Vec<Candle> {
    let mut candles = fade_candles();
    let previous_close = candles[31].close();
    let timestamp = candles[32].timestamp();
    candles[32] = Candle::new(
        32,
        timestamp,
        previous_close + 5.0,
        previous_close + 6.0,
        previous_close + 1.0,
        previous_close + 2.0,
        Some(10.0),
    )
    .expect("the overnight-fade candle is valid");
    let base = candles[32].close();
    for (bar, candle) in candles.iter_mut().enumerate().take(34).skip(32) {
        let timestamp = candle.timestamp();
        *candle = Candle::new(
            bar as u64,
            timestamp,
            base,
            base + 0.2,
            base - 0.2,
            base,
            Some(10.0),
        )
        .expect("the narrow candle is valid");
    }
    let timestamp = candles[34].timestamp();
    candles[34] = Candle::new(
        34,
        timestamp,
        base,
        base + 10.0,
        base - 10.0,
        base,
        Some(10.0),
    )
    .expect("the wide candle is valid");
    candles
}

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

#[test]
fn signal_opening_range_breakout_row_contract() {
    let candles = series(1);
    assert_contract(
        "opening range breakout",
        codes::SIGNAL_OPENING_RANGE_BREAKOUT,
        candles.len(),
        &[10, 11, 12, 24],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 51,
            direction: 1,
            occurred: 11,
            available: 11,
            primary: 105.5,
            reference: 105.0,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}

#[test]
fn signal_opening_range_sweep_row_contract() {
    let candles = sweep_candles(104.5);
    assert_contract(
        "opening range sweep",
        codes::SIGNAL_OPENING_RANGE_SWEEP,
        candles.len(),
        &[14, 15, 16, 24],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 51,
            direction: -1,
            occurred: 15,
            available: 15,
            primary: 104.5,
            reference: 105.0,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}

#[test]
fn signal_opening_range_sweep_control_row_contract() {
    let candles = sweep_candles(105.5);
    assert_contract(
        "opening range sweep control",
        codes::SIGNAL_OPENING_RANGE_SWEEP_CONTROL,
        candles.len(),
        &[14, 15, 16, 24],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 51,
            direction: 1,
            occurred: 15,
            available: 15,
            primary: 105.5,
            reference: 105.0,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}

#[test]
fn signal_overnight_continuation_row_contract() {
    let candles = series(2);
    assert_contract(
        "overnight continuation",
        codes::SIGNAL_OVERNIGHT_CONTINUATION,
        candles.len(),
        &[31, 32, 33, 48],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 52,
            direction: 1,
            occurred: 32,
            available: 32,
            primary: 5.333333333333333,
            reference: 0.3333333333333333,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}

#[test]
fn signal_overnight_fade_row_contract() {
    let candles = fade_candles();
    assert_contract(
        "overnight fade",
        codes::SIGNAL_OVERNIGHT_FADE,
        candles.len(),
        &[31, 32, 33, 48],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 52,
            direction: -1,
            occurred: 32,
            available: 32,
            primary: 6.7535224269655005,
            reference: -1.558505175453577,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}

#[test]
fn signal_session_range_contraction_row_contract() {
    let candles = contraction_candles();
    assert_contract(
        "session range contraction",
        codes::SIGNAL_SESSION_RANGE_CONTRACTION,
        candles.len(),
        &[31, 32, 33, 48],
        Expected {
            event_type: EventType::Signal,
            subtype: 23,
            source: 53,
            direction: -1,
            occurred: 32,
            available: 32,
            primary: 0.2419121435168931,
            reference: 0.7,
        },
        |len| events(&candles[..len]).events().to_vec(),
    );
}
