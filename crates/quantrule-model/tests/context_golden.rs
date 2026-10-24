//! Golden-fixture and causality tests for the context diagnostic.
//!
//! The fixture is a checked-in CSV of 260 bars and the expected ATR ratio,
//! RVOL ratio, and context code, produced by an independent Python
//! implementation of contract §12 from the checked-in readings fixture.

use quantrule_data::Candle;
use quantrule_model::context::ContextSeries;
use quantrule_model::readings::Readings;

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const CONTEXT_FIXTURE: &str = include_str!("fixtures/context_golden.csv");
const TOLERANCE: f64 = 1e-9;

const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const CONTEXT_HEADER: &str = "bar,atr_ratio,rvol_ratio,context_code";

#[derive(Debug, Clone, Copy)]
struct ExpectedContext {
    atr_ratio: Option<f64>,
    rvol_ratio: Option<f64>,
    context_code: Option<u8>,
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

fn load_context() -> Vec<ExpectedContext> {
    let mut lines = CONTEXT_FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(CONTEXT_HEADER),
        "the context fixture header changed"
    );

    let mut rows = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 4, "wrong column count in {line:?}");
        rows.push(ExpectedContext {
            atr_ratio: number(fields[1]),
            rvol_ratio: number(fields[2]),
            context_code: if fields[3].is_empty() {
                None
            } else {
                Some(fields[3].parse().expect("the context code is a u8"))
            },
        });
    }
    rows
}

fn assert_close(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{context}: actual {actual}, expected {expected}"
    );
}

#[test]
fn the_context_matches_the_golden_fixture() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let context = ContextSeries::compute(&readings);
    let expected = load_context();

    assert_eq!(context.len(), expected.len());
    for (bar, (actual, expected)) in context.bars().iter().zip(expected.iter()).enumerate() {
        match (actual.atr_ratio, expected.atr_ratio) {
            (Some(actual), Some(expected)) => {
                assert_close(actual, expected, &format!("bar {bar}: ATR ratio"));
            }
            (None, None) => {}
            (actual, expected) => panic!("bar {bar}: ATR ratio {actual:?}, expected {expected:?}"),
        }
        match (actual.rvol_ratio, expected.rvol_ratio) {
            (Some(actual), Some(expected)) => {
                assert_close(actual, expected, &format!("bar {bar}: RVOL ratio"));
            }
            (None, None) => {}
            (actual, expected) => panic!("bar {bar}: RVOL ratio {actual:?}, expected {expected:?}"),
        }
        assert_eq!(
            actual.context_code, expected.context_code,
            "bar {bar}: context code"
        );
    }
}

#[test]
fn the_context_length_equals_the_input_length() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let context = ContextSeries::compute(&readings);
    assert_eq!(context.len(), candles.len());
}

#[test]
fn every_available_ratio_retains_a_value_when_the_code_is_null() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let context = ContextSeries::compute(&readings);

    for bar in context.bars() {
        if bar.context_code.is_none() {
            assert!(
                bar.atr_ratio.is_none() || bar.rvol_ratio.is_none(),
                "a code is null only when a ratio is unavailable"
            );
        }
    }
}

#[test]
fn later_candles_do_not_change_earlier_context() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let full = ContextSeries::compute(&readings);

    for length in [0, 1, 2, 19, 20, 33, 50, 100, 200, 259] {
        let prefix = &candles[..length];
        let prefix_readings = Readings::compute(prefix);
        let partial = ContextSeries::compute(&prefix_readings);
        assert_eq!(
            partial.bars(),
            &full.bars()[..length],
            "the prefix of length {length} changed an earlier context bar"
        );
    }
}

#[test]
fn the_context_is_deterministic() {
    let candles = load_candles();
    let readings = Readings::compute(&candles);
    let first = ContextSeries::compute(&readings);
    let second = ContextSeries::compute(&readings);
    assert_eq!(first, second);
}
