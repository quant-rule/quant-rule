//! Shared golden and precedence helpers for the base-event row tests.
//!
//! The fixtures are checked-in expected records. The precedence helper reads the
//! confirmed-level registry and verifies the base level reference order.

use quantrule_data::Candle;
use quantrule_model::events::{EventRecord, EventType};
use quantrule_model::levels::ConfirmedLevelRegistry;
use quantrule_model::readings::{PivotKind, PivotScale, Readings};
use std::collections::{BTreeMap, BTreeSet};

const LEVEL_GOLDEN: &str = include_str!("../fixtures/level_events_golden.csv");
const LEVEL_GOLDEN_HEADER: &str = "bar,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";
const DIVERGENCE_GOLDEN: &str = include_str!("../fixtures/divergence_events_golden.csv");
const DIVERGENCE_GOLDEN_HEADER: &str = "scale_code,bar,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";

#[derive(Debug, Clone, Copy)]
pub struct ExpectedRecord {
    pub event_type: EventType,
    pub subtype: u8,
    pub source: u16,
    pub timeframe: u64,
    pub occurred: usize,
    pub available: usize,
    pub direction: i8,
    pub reference_code: Option<u32>,
    pub primary: Option<f64>,
    pub reference: Option<f64>,
}

pub fn fixture_number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(field.parse().expect("a fixture number"))
    }
}

pub fn fixture_u32(field: &str) -> Option<u32> {
    if field.is_empty() {
        None
    } else {
        Some(field.parse().expect("a fixture reference"))
    }
}

pub fn load_level_golden() -> Vec<ExpectedRecord> {
    let mut lines = LEVEL_GOLDEN.lines();
    assert_eq!(lines.next(), Some(LEVEL_GOLDEN_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 11);
            ExpectedRecord {
                event_type: EventType::Level,
                subtype: fields[2].parse().expect("subtype"),
                source: fields[3].parse().expect("source"),
                timeframe: fields[4].parse().expect("timeframe"),
                occurred: fields[5].parse().expect("occurred"),
                available: fields[6].parse().expect("available"),
                direction: fields[7].parse().expect("direction"),
                reference_code: fixture_u32(fields[8]),
                primary: fixture_number(fields[9]),
                reference: fixture_number(fields[10]),
            }
        })
        .collect()
}

pub fn load_divergence_golden() -> Vec<ExpectedRecord> {
    let mut lines = DIVERGENCE_GOLDEN.lines();
    assert_eq!(lines.next(), Some(DIVERGENCE_GOLDEN_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 12);
            ExpectedRecord {
                event_type: EventType::Divergence,
                subtype: fields[3].parse().expect("subtype"),
                source: fields[4].parse().expect("source"),
                timeframe: fields[5].parse().expect("timeframe"),
                occurred: fields[6].parse().expect("occurred"),
                available: fields[7].parse().expect("available"),
                direction: fields[8].parse().expect("direction"),
                reference_code: fixture_u32(fields[9]),
                primary: fixture_number(fields[10]),
                reference: fixture_number(fields[11]),
            }
        })
        .collect()
}

pub fn assert_record(actual: &EventRecord, expected: &ExpectedRecord, label: &str) {
    assert_eq!(actual.event_type, expected.event_type, "{label} type");
    assert_eq!(
        actual.event_subtype_code, expected.subtype,
        "{label} subtype"
    );
    assert_eq!(actual.source_code, expected.source, "{label} source");
    assert_eq!(
        actual.timeframe_secs, expected.timeframe,
        "{label} timeframe"
    );
    assert_eq!(
        actual.occurred_bar_index, expected.occurred,
        "{label} occurred"
    );
    assert_eq!(
        actual.available_bar_index, expected.available,
        "{label} available"
    );
    assert_eq!(
        actual.direction_code, expected.direction,
        "{label} direction"
    );
    assert_eq!(
        actual.reference_code, expected.reference_code,
        "{label} reference"
    );
    assert_eq!(
        actual.primary_value.map(f64::to_bits),
        expected.primary.map(f64::to_bits),
        "{label} primary"
    );
    assert_eq!(
        actual.reference_value.map(f64::to_bits),
        expected.reference.map(f64::to_bits),
        "{label} reference value"
    );
}

type RecordKey = (
    usize,
    usize,
    u16,
    u8,
    i8,
    Option<u32>,
    Option<u64>,
    Option<u64>,
);

pub fn record_key(event: &EventRecord) -> RecordKey {
    (
        event.available_bar_index,
        event.occurred_bar_index,
        event.source_code,
        event.event_subtype_code,
        event.direction_code,
        event.reference_code,
        event.primary_value.map(f64::to_bits),
        event.reference_value.map(f64::to_bits),
    )
}

pub fn expected_key(event: &ExpectedRecord) -> RecordKey {
    (
        event.available,
        event.occurred,
        event.source,
        event.subtype,
        event.direction,
        event.reference_code,
        event.primary.map(f64::to_bits),
        event.reference.map(f64::to_bits),
    )
}

pub fn assert_records(actual: &[EventRecord], expected: &[ExpectedRecord], label: &str) {
    assert_eq!(actual.len(), expected.len(), "{label} count");
    let mut actual = actual.to_vec();
    let mut expected = expected.to_vec();
    actual.sort_by_key(record_key);
    expected.sort_by_key(expected_key);
    for (actual, expected) in actual.iter().zip(&expected) {
        assert_record(actual, expected, label);
    }
}

pub fn assert_level_golden(events: &[EventRecord], code: u16, subtype: u8) {
    let expected: Vec<ExpectedRecord> = load_level_golden()
        .into_iter()
        .filter(|event| event.subtype == subtype)
        .collect();
    let actual: Vec<EventRecord> = events
        .iter()
        .filter(|event| event.event_code == code)
        .copied()
        .collect();
    assert_records(&actual, &expected, &format!("level subtype {subtype}"));
}

pub fn assert_divergence_golden(events: &[EventRecord], code: u16, subtype: u8) {
    let expected: Vec<ExpectedRecord> = load_divergence_golden()
        .into_iter()
        .filter(|event| event.subtype == subtype)
        .collect();
    let actual: Vec<EventRecord> = events
        .iter()
        .filter(|event| event.event_code == code)
        .copied()
        .collect();
    assert_records(&actual, &expected, &format!("divergence subtype {subtype}"));
}

pub fn level_rank(registry: &ConfirmedLevelRegistry, code: u32) -> u8 {
    match code {
        1 => 0,
        2 => 1,
        3 => 2,
        _ => {
            let level = registry
                .levels()
                .iter()
                .find(|level| level.reference_code == code)
                .unwrap_or_else(|| panic!("the level reference {code} is not in the registry"));
            match (level.scale, level.kind) {
                (PivotScale::Minor, PivotKind::High) => 3,
                (PivotScale::Minor, PivotKind::Low) => 4,
                (PivotScale::Major, PivotKind::High) => 5,
                (PivotScale::Major, PivotKind::Low) => 6,
            }
        }
    }
}

pub fn assert_level_precedence(candles: &[Candle], events: &[EventRecord], code: u16) {
    let readings = Readings::compute(candles);
    let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
    let mut per_bar: BTreeMap<(usize, usize), Vec<&EventRecord>> = BTreeMap::new();
    for event in events
        .iter()
        .filter(|event| event.event_type == EventType::Level)
    {
        per_bar
            .entry((event.available_bar_index, event.occurred_bar_index))
            .or_default()
            .push(event);
    }
    for ((available, occurred), group) in per_bar {
        if !group.iter().any(|event| event.event_code == code) {
            continue;
        }
        let mut references = BTreeSet::new();
        let mut ranks = Vec::new();
        for event in group {
            let reference = event
                .reference_code
                .expect("a base level event carries a reference code");
            assert!(
                references.insert(reference),
                "the same reference produced two level events on bar {occurred} available at {available}"
            );
            ranks.push(level_rank(&registry, reference));
        }
        assert!(
            ranks.windows(2).all(|pair| pair[0] <= pair[1]),
            "the level reference order changed on bar {occurred}: {ranks:?}"
        );
    }
}
