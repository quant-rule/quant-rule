//! Row-level contracts for the panel divergence events.
//!
//! Each test drives one instrument's confirmed pivots against an explicit panel
//! reading. The scenarios are deterministic test oracles, not market fixtures.

use crate::events::{EventRecord, EventType, codes};
use crate::panel::{PanelInstrument, PanelInstrumentReadings, PanelReadings};
use crate::readings::PivotKind;
use quantrule_data::Candle;
use std::collections::BTreeMap;

struct Expected {
    subtype: u8,
    source: u16,
    direction: i8,
    occurred: usize,
    available: usize,
    primary: f64,
    reference: f64,
}

fn candle(index: u64, open: f64, high: f64, low: f64, close: f64) -> Candle {
    Candle::new(
        index,
        1_700_000_000 + index as i64 * 3_600,
        open,
        high,
        low,
        close,
        Some(10.0),
    )
    .expect("valid candle")
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
                candle(index as u64, 101.0, 101.5, low, 101.0)
            }
            PivotKind::High => {
                let high = if index == 5 {
                    first
                } else if index == 15 {
                    second
                } else {
                    100.0
                };
                candle(index as u64, 100.0, high, 99.5, 100.0)
            }
        })
        .collect()
}

fn metric(len: usize, first: f64, second: f64) -> Vec<Option<f64>> {
    let mut values = vec![None; len];
    if len > 5 {
        values[5] = Some(first);
    }
    if len > 15 {
        values[15] = Some(second);
    }
    values
}

fn panel_readings(
    candles: &[Candle],
    relative_strength: Vec<Option<f64>>,
    lead_lag: Vec<Option<f64>>,
    breadth: Vec<Option<f64>>,
) -> PanelReadings {
    let len = candles.len();
    let timestamps = candles.iter().map(Candle::timestamp).collect();
    let mut instruments = BTreeMap::new();
    instruments.insert(
        "BTC".to_string(),
        PanelInstrumentReadings {
            relative_strength,
            leader_return: vec![None; len],
            lead_lag,
            breadth,
            co_movement: vec![None; len],
            dispersion: vec![None; len],
        },
    );
    PanelReadings {
        timestamps,
        instruments,
    }
}

fn panel_divergence_events(
    len: usize,
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<EventRecord> {
    let candles = pivot_candles(kind, first, second, len);
    let readings = panel_readings(
        &candles,
        metric(len, first_value, second_value),
        metric(len, first_value, second_value),
        metric(len, first_value, second_value),
    );
    super::PanelEventSeries::compute(
        &[PanelInstrument {
            name: "BTC",
            candles: &candles,
        }],
        &readings,
        3_600,
    )
    .events("BTC")
    .to_vec()
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
    assert_eq!(event.event_type, EventType::Divergence);
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
fn divergence_rs_regular_bullish_row_contract() {
    assert_contract(
        "relative strength regular bullish",
        codes::DIVERGENCE_RS_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 1,
            source: 45,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| panel_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_rs_regular_bearish_row_contract() {
    assert_contract(
        "relative strength regular bearish",
        codes::DIVERGENCE_RS_REGULAR_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 2,
            source: 45,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 102.0,
            reference: 0.5,
        },
        |len| panel_divergence_events(len, PivotKind::High, 101.0, 102.0, 1.5, 0.5),
    );
}

#[test]
fn divergence_lead_lag_bullish_row_contract() {
    assert_contract(
        "lead-lag bullish",
        codes::DIVERGENCE_LEAD_LAG_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 1,
            source: 46,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| panel_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_lead_lag_bearish_row_contract() {
    assert_contract(
        "lead-lag bearish",
        codes::DIVERGENCE_LEAD_LAG_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 2,
            source: 46,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 102.0,
            reference: 0.5,
        },
        |len| panel_divergence_events(len, PivotKind::High, 101.0, 102.0, 1.5, 0.5),
    );
}

#[test]
fn divergence_breadth_regular_bullish_row_contract() {
    assert_contract(
        "breadth regular bullish",
        codes::DIVERGENCE_BREADTH_REGULAR_BULLISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 1,
            source: 47,
            direction: 1,
            occurred: 15,
            available: 20,
            primary: 98.0,
            reference: 1.5,
        },
        |len| panel_divergence_events(len, PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
    );
}

#[test]
fn divergence_breadth_regular_bearish_row_contract() {
    assert_contract(
        "breadth regular bearish",
        codes::DIVERGENCE_BREADTH_REGULAR_BEARISH,
        30,
        &[6, 15, 20, 21, 30],
        Expected {
            subtype: 2,
            source: 47,
            direction: -1,
            occurred: 15,
            available: 20,
            primary: 102.0,
            reference: 0.5,
        },
        |len| panel_divergence_events(len, PivotKind::High, 101.0, 102.0, 1.5, 0.5),
    );
}
