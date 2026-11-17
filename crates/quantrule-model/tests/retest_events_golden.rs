//! Golden fixture for the breakout-retest-hold event.
//!
//! The fixture covers a held long retest, a held short retest, and an
//! invalidated break that emits nothing. The test also checks that appending
//! bars does not change an earlier retest.

use quantrule_data::Candle;
use quantrule_model::events::EventRecord;
use quantrule_model::events::EventType;
use quantrule_model::events::codes::SIGNAL_COMPRESSION_BREAKOUT;
use quantrule_model::events::retest::BreakoutRetestHoldSeries;
use quantrule_model::events::signal::SignalSubtype;
use quantrule_model::readings::{PivotSeries, ReadingSeries, Readings};

const HEADER: &str = "row,event_code,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";
const TIMEFRAME_SECS: u64 = 300;
const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy)]
struct ExpectedEvent {
    event_code: u16,
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    timeframe_secs: u64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
    reference_code: Option<u32>,
    primary_value: f64,
    reference_value: f64,
}

fn candle(index: u64, high: f64, low: f64, close: f64) -> Candle {
    Candle::new(
        index,
        index as i64 * TIMEFRAME_SECS as i64,
        close,
        high,
        low,
        close,
        Some(1.0),
    )
    .expect("the test candle is valid")
}

fn candles() -> Vec<Candle> {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.5, 99.5, 100.0))
        .collect();
    // The break at bar 20 closes back through the boundary and emits nothing.
    candles[21] = candle(21, 100.9, 100.2, 100.4);
    // The long break at bar 25 retests and holds at bar 26.
    candles[26] = candle(26, 101.1, 100.6, 101.0);
    // The short break at bar 27 retests and holds at bar 28.
    candles[27] = candle(27, 99.7, 99.2, 99.3);
    candles[28] = candle(28, 99.6, 99.1, 99.2);
    candles
}

fn readings(len: usize) -> Readings {
    let series = |value: f64| ReadingSeries::new(vec![Some(value); len]);
    Readings {
        ema20: series(100.0),
        ema50: series(100.0),
        ema200: series(100.0),
        rsi14: series(50.0),
        atr14: series(1.0),
        rvol20: series(1.0),
        pivots: PivotSeries::compute(&[]),
    }
}

fn breakout(direction: i8, boundary: f64, bar: usize) -> EventRecord {
    EventRecord {
        event_code: SIGNAL_COMPRESSION_BREAKOUT,
        event_type: EventType::Signal,
        event_subtype_code: 7,
        source_code: 5,
        timeframe_secs: TIMEFRAME_SECS,
        occurred_bar_index: bar,
        available_bar_index: bar,
        direction_code: direction,
        reference_code: None,
        primary_value: Some(if direction > 0 {
            boundary + 0.5
        } else {
            boundary - 0.5
        }),
        reference_value: Some(boundary),
    }
}

fn compute(candles: &[Candle]) -> BreakoutRetestHoldSeries {
    let breakouts = [
        breakout(1, 100.5, 20),
        breakout(1, 100.5, 25),
        breakout(-1, 99.5, 27),
    ];
    BreakoutRetestHoldSeries::from_breakouts(candles, &readings(candles.len()), &breakouts)
}

fn load_events() -> Vec<ExpectedEvent> {
    let mut lines = include_str!("fixtures/retest_events_golden.csv").lines();
    assert_eq!(
        lines.next(),
        Some(HEADER),
        "the retest fixture header changed"
    );
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 12, "wrong column count in {line:?}");
            ExpectedEvent {
                event_code: fields[1].parse().expect("the event code"),
                event_type_code: fields[2].parse().expect("the event type"),
                event_subtype_code: fields[3].parse().expect("the subtype"),
                source_code: fields[4].parse().expect("the source"),
                timeframe_secs: fields[5].parse().expect("the timeframe"),
                occurred_bar_index: fields[6].parse().expect("the occurrence"),
                available_bar_index: fields[7].parse().expect("the availability"),
                direction_code: fields[8].parse().expect("the direction"),
                reference_code: if fields[9].is_empty() {
                    None
                } else {
                    Some(fields[9].parse().expect("the reference code"))
                },
                primary_value: fields[10].parse().expect("the primary value"),
                reference_value: fields[11].parse().expect("the reference value"),
            }
        })
        .collect()
}

fn assert_close(left: f64, right: f64, label: &str) {
    assert!(
        (left - right).abs() <= TOLERANCE,
        "{label}: {left} != {right}"
    );
}

#[test]
fn the_retest_events_match_the_golden_fixture() {
    let events = compute(&candles());
    let expected = load_events();
    assert_eq!(events.events().len(), expected.len());
    for (event, expected) in events.events().iter().zip(expected) {
        assert_eq!(event.event_code, expected.event_code);
        assert_eq!(event.event_type.code(), expected.event_type_code);
        assert_eq!(event.event_subtype_code, expected.event_subtype_code);
        assert_eq!(
            event.event_subtype_code,
            SignalSubtype::BreakoutRetestHold.code()
        );
        assert_eq!(event.source_code, expected.source_code);
        assert_eq!(event.timeframe_secs, expected.timeframe_secs);
        assert_eq!(event.occurred_bar_index, expected.occurred_bar_index);
        assert_eq!(event.available_bar_index, expected.available_bar_index);
        assert_eq!(event.direction_code, expected.direction_code);
        assert_eq!(event.reference_code, expected.reference_code);
        assert_close(
            event.primary_value.expect("the primary value"),
            expected.primary_value,
            "primary",
        );
        assert_close(
            event.reference_value.expect("the reference value"),
            expected.reference_value,
            "reference",
        );
    }
}

#[test]
fn appending_bars_does_not_change_an_earlier_retest() {
    let candles = candles();
    let full = compute(&candles);
    for cutoff in [21, 22, 24, 27, 28, 29] {
        let prefix = compute(&candles[..cutoff]);
        let expected: Vec<EventRecord> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < cutoff)
            .copied()
            .collect();
        assert_eq!(
            prefix.events(),
            expected.as_slice(),
            "the prefix of length {cutoff} changed an earlier retest"
        );
    }
}

fn render(events: &[EventRecord]) -> String {
    let mut text = String::from(HEADER);
    text.push('\n');
    for (index, event) in events.iter().enumerate() {
        let reference = event
            .reference_code
            .map(|code| code.to_string())
            .unwrap_or_default();
        let primary = event
            .primary_value
            .map(|value| value.to_string())
            .unwrap_or_default();
        let reference_value = event
            .reference_value
            .map(|value| value.to_string())
            .unwrap_or_default();
        text.push_str(&format!(
            "{index},{},{},{},{},{},{},{},{},{reference},{primary},{reference_value}\n",
            event.event_code,
            event.event_type.code(),
            event.event_subtype_code,
            event.source_code,
            event.timeframe_secs,
            event.occurred_bar_index,
            event.available_bar_index,
            event.direction_code,
        ));
    }
    text
}

#[test]
#[ignore = "regenerates the retest fixture deliberately"]
fn regenerate_retest_fixture() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/retest_events_golden.csv"
    );
    std::fs::write(path, render(compute(&candles()).events()))
        .expect("the retest fixture is writable");
}
