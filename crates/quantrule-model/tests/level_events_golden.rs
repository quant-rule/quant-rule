//! Golden-fixture and causality tests for Level events.
//!
//! The fixture is a checked-in CSV of 260 candles and the expected Level events
//! for a five-minute timeframe. It was produced by an independent Python
//! implementation of contract §7.1 and §7.2 from the checked-in readings
//! fixture, not by the Rust code under test.
//!
//! Every event is compared field by field. An empty cell is an absent optional
//! value; a numeric cell is a measured value.

use quantrule_data::Candle;
use quantrule_model::events::level::{
    EMA20_SOURCE_CODE, EMA50_SOURCE_CODE, EMA200_SOURCE_CODE, LevelEventConfig, LevelEventSeries,
    MAJOR_PIVOT_SOURCE_CODE, MINOR_PIVOT_SOURCE_CODE,
};
use quantrule_model::readings::Readings;

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const EVENTS_FIXTURE: &str = include_str!("fixtures/level_events_golden.csv");
const TOLERANCE: f64 = 1e-9;
const TIMEFRAME_SECS: u64 = 300;

const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const EVENTS_HEADER: &str = "bar,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";

#[derive(Debug, Clone, Copy)]
struct ExpectedEvent {
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    timeframe_secs: u64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
    reference_code: u32,
    primary_value: f64,
    reference_value: f64,
}

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("invalid number {field:?}: {error}")),
        )
    }
}

fn integer(field: &str) -> usize {
    field
        .parse::<usize>()
        .unwrap_or_else(|error| panic!("invalid integer {field:?}: {error}"))
}

fn load_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(READINGS_HEADER),
        "the readings fixture header changed"
    );

    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23, "wrong column count in {line:?}");
        let series_index = fields[0].parse::<u64>().expect("the bar index is a u64");
        let timestamp = fields[1].parse::<i64>().expect("the timestamp is an i64");
        let open = number(fields[2]).expect("the open is present");
        let high = number(fields[3]).expect("the high is present");
        let low = number(fields[4]).expect("the low is present");
        let close = number(fields[5]).expect("the close is present");
        let volume = number(fields[6]);
        candles.push(
            Candle::new(series_index, timestamp, open, high, low, close, volume)
                .expect("the fixture candle is valid"),
        );
    }
    candles
}

fn load_events() -> Vec<ExpectedEvent> {
    let mut lines = EVENTS_FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(EVENTS_HEADER),
        "the level-events fixture header changed"
    );

    let mut events = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 11, "wrong column count in {line:?}");
        events.push(ExpectedEvent {
            event_type_code: fields[1].parse().expect("the event type is a u8"),
            event_subtype_code: fields[2].parse().expect("the subtype is a u8"),
            source_code: fields[3].parse().expect("the source is a u16"),
            timeframe_secs: fields[4].parse().expect("the timeframe is a u64"),
            occurred_bar_index: integer(fields[5]),
            available_bar_index: integer(fields[6]),
            direction_code: fields[7].parse().expect("the direction is an i8"),
            reference_code: fields[8].parse().expect("the reference is a u32"),
            primary_value: number(fields[9]).expect("the primary value is present"),
            reference_value: number(fields[10]).expect("the reference value is present"),
        });
    }
    events
}

fn config() -> LevelEventConfig {
    LevelEventConfig::from_secs(TIMEFRAME_SECS).expect("the timeframe is non-zero")
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{context}: actual {actual}, expected {expected}"
    );
}

#[test]
fn the_level_events_match_the_golden_fixture() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let series = LevelEventSeries::compute(&candles, &readings, &config());
    let expected = load_events();

    assert_eq!(
        series.len(),
        expected.len(),
        "the event count changed; first actual {:?}, first expected {:?}",
        series.events().first(),
        expected.first()
    );

    for (index, (actual, expected)) in series.events().iter().zip(expected.iter()).enumerate() {
        let context = format!("event {index} at bar {}", expected.occurred_bar_index);
        assert_eq!(
            actual.event_type.code(),
            expected.event_type_code,
            "{context}: type"
        );
        assert_eq!(
            actual.event_subtype_code, expected.event_subtype_code,
            "{context}: subtype"
        );
        assert_eq!(
            actual.source_code, expected.source_code,
            "{context}: source"
        );
        assert_eq!(
            actual.timeframe_secs, expected.timeframe_secs,
            "{context}: timeframe"
        );
        assert_eq!(
            actual.occurred_bar_index, expected.occurred_bar_index,
            "{context}: occurrence bar"
        );
        assert_eq!(
            actual.available_bar_index, expected.available_bar_index,
            "{context}: availability bar"
        );
        assert_eq!(
            actual.direction_code, expected.direction_code,
            "{context}: direction"
        );
        assert_eq!(
            actual.reference_code,
            Some(expected.reference_code),
            "{context}: reference"
        );
        assert_close(
            actual.primary_value.expect("a primary value"),
            expected.primary_value,
            &format!("{context}: primary value"),
        );
        assert_close(
            actual.reference_value.expect("a reference value"),
            expected.reference_value,
            &format!("{context}: reference value"),
        );
    }
}

#[test]
fn every_event_uses_a_known_source_code() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let series = LevelEventSeries::compute(&candles, &readings, &config());

    for event in series.events() {
        assert!(
            matches!(
                event.source_code,
                EMA20_SOURCE_CODE
                    | EMA50_SOURCE_CODE
                    | EMA200_SOURCE_CODE
                    | MINOR_PIVOT_SOURCE_CODE
                    | MAJOR_PIVOT_SOURCE_CODE
            ),
            "unknown source code {}",
            event.source_code
        );
    }
}

#[test]
fn every_event_occurs_on_or_after_its_reference_availability_bar() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let series = LevelEventSeries::compute(&candles, &readings, &config());

    for event in series.events() {
        assert_eq!(
            event.occurred_bar_index, event.available_bar_index,
            "a Level event is knowable on its occurrence bar"
        );
        assert!(
            event.available_bar_index < candles.len(),
            "an event points past the input"
        );
    }
}

#[test]
fn appending_or_removing_later_candles_does_not_change_earlier_events() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let full = LevelEventSeries::compute(&candles, &readings, &config());

    for length in [0, 1, 2, 13, 14, 15, 50, 100, 200, 259] {
        let prefix = &candles[..length];
        let prefix_readings = Readings::compute(prefix);
        let partial = LevelEventSeries::compute(prefix, &prefix_readings, &config());
        let expected: Vec<_> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < length)
            .copied()
            .collect();
        assert_eq!(
            partial.events(),
            expected.as_slice(),
            "the prefix of length {length} changed an earlier event"
        );
    }
}

#[test]
fn the_level_detector_is_deterministic() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let first = LevelEventSeries::compute(&candles, &readings, &config());
    let second = LevelEventSeries::compute(&candles, &readings, &config());
    assert_eq!(first, second);
}
