//! Golden-fixture and causality tests for Divergence events.
//!
//! The fixture is a checked-in CSV of 260 candles and the expected divergence
//! events at both pivot scales, produced by an independent Python implementation
//! of contract §7.3 from the checked-in readings fixture. The test recomputes the
//! readings and both divergence series and compares every field.

use quantrule_data::Candle;
use quantrule_model::events::divergence::{
    DivergenceConfig, DivergenceEventSeries, RSI_SOURCE_CODE,
};
use quantrule_model::readings::{PivotScale, Readings};

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const EVENTS_FIXTURE: &str = include_str!("fixtures/divergence_events_golden.csv");
const TOLERANCE: f64 = 1e-9;
const TIMEFRAME_SECS: u64 = 300;

const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const EVENTS_HEADER: &str = "scale_code,bar,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";

#[derive(Debug, Clone, Copy)]
struct ExpectedEvent {
    scale_code: u8,
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    timeframe_secs: u64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
    reference_code: u32,
    primary_value: Option<f64>,
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
        "the divergence-events fixture header changed"
    );

    let mut events = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 12, "wrong column count in {line:?}");
        events.push(ExpectedEvent {
            scale_code: fields[0].parse().expect("the scale is a u8"),
            event_type_code: fields[2].parse().expect("the event type is a u8"),
            event_subtype_code: fields[3].parse().expect("the subtype is a u8"),
            source_code: fields[4].parse().expect("the source is a u16"),
            timeframe_secs: fields[5].parse().expect("the timeframe is a u64"),
            occurred_bar_index: integer(fields[6]),
            available_bar_index: integer(fields[7]),
            direction_code: fields[8].parse().expect("the direction is an i8"),
            reference_code: fields[9].parse().expect("the reference is a u32"),
            primary_value: number(fields[10]),
            reference_value: number(fields[11]).expect("the RSI delta is present"),
        });
    }
    events
}

fn config(scale: PivotScale) -> DivergenceConfig {
    DivergenceConfig::from_secs(TIMEFRAME_SECS, scale).expect("the timeframe is non-zero")
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{context}: actual {actual}, expected {expected}"
    );
}

fn assert_events(actual: &DivergenceEventSeries, expected: &[ExpectedEvent], scale: PivotScale) {
    let expected: Vec<&ExpectedEvent> = expected
        .iter()
        .filter(|event| event.scale_code == scale.code())
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "the event count changed for {scale:?}"
    );

    for (index, (actual, expected)) in actual.events().iter().zip(expected.iter()).enumerate() {
        let context = format!(
            "{scale:?} event {index} at bar {}",
            expected.occurred_bar_index
        );
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
        match (actual.primary_value, expected.primary_value) {
            (Some(actual), Some(expected)) => {
                assert_close(actual, expected, &format!("{context}: primary value"));
            }
            (None, None) => {}
            (actual, expected) => {
                panic!("{context}: primary value {actual:?}, expected {expected:?}");
            }
        }
        assert_close(
            actual.reference_value.expect("the RSI delta is present"),
            expected.reference_value,
            &format!("{context}: RSI delta"),
        );
    }
}

#[test]
fn the_divergence_events_match_the_golden_fixture_at_both_scales() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let expected = load_events();

    let minor = DivergenceEventSeries::compute(&readings, &config(PivotScale::Minor));
    let major = DivergenceEventSeries::compute(&readings, &config(PivotScale::Major));

    assert_events(&minor, &expected, PivotScale::Minor);
    assert_events(&major, &expected, PivotScale::Major);
}

#[test]
fn every_divergence_uses_rsi_as_its_source() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    for scale in PivotScale::ALL {
        let events = DivergenceEventSeries::compute(&readings, &config(scale));
        for event in events.events() {
            assert_eq!(event.source_code, RSI_SOURCE_CODE);
            assert!(event.reference_code.is_some());
            assert!(event.available_bar_index >= event.occurred_bar_index);
        }
    }
}

#[test]
fn later_candles_do_not_change_earlier_divergences() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let full_minor = DivergenceEventSeries::compute(&readings, &config(PivotScale::Minor));
    let full_major = DivergenceEventSeries::compute(&readings, &config(PivotScale::Major));

    for length in [0, 1, 2, 10, 20, 50, 100, 150, 200, 259] {
        let prefix = &candles[..length];
        let prefix_readings = Readings::compute(prefix);
        let minor = DivergenceEventSeries::compute(&prefix_readings, &config(PivotScale::Minor));
        let major = DivergenceEventSeries::compute(&prefix_readings, &config(PivotScale::Major));

        for (partial, full, scale) in [
            (&minor, &full_minor, PivotScale::Minor),
            (&major, &full_major, PivotScale::Major),
        ] {
            let expected: Vec<_> = full
                .events()
                .iter()
                .filter(|event| event.available_bar_index < length)
                .copied()
                .collect();
            assert_eq!(
                partial.events(),
                expected.as_slice(),
                "the {scale:?} prefix of length {length} changed an earlier event"
            );
        }
    }
}

#[test]
fn the_divergence_detector_is_deterministic() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    for scale in PivotScale::ALL {
        let first = DivergenceEventSeries::compute(&readings, &config(scale));
        let second = DivergenceEventSeries::compute(&readings, &config(scale));
        assert_eq!(first, second);
    }
}
