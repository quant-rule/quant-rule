//! Row-level contracts for the compression events.
//!
//! Each test names one event and checks its record, availability, uniqueness,
//! determinism, and prefix causality on a deterministic synthetic series.

use quantrule_data::Candle;
use quantrule_model::context::ContextSeries;
use quantrule_model::events::codes;
use quantrule_model::events::compression::{
    CompressionBreakoutConfig, CompressionBreakoutSeries, CompressionRangeConfig,
    CompressionRangeOnsetSeries, CompressionRangeSeries,
};
use quantrule_model::events::{EventRecord, EventType};
use quantrule_model::readings::{PivotSeries, ReadingSeries, Readings};
use std::num::NonZeroU64;

const TIMEFRAME_SECS: u64 = 300;

fn timeframe() -> NonZeroU64 {
    NonZeroU64::new(TIMEFRAME_SECS).expect("a non-zero timeframe")
}

fn candle(index: u64, high: f64, low: f64, close: f64) -> Candle {
    Candle::new(index, index as i64, close, high, low, close, Some(10.0))
        .expect("the test candle is valid")
}

fn series(len: usize, value: impl Fn(usize) -> Option<f64>) -> ReadingSeries {
    ReadingSeries::new((0..len).map(value).collect())
}

fn fixture(len: usize, rvol_at_25: f64) -> (Vec<Candle>, Readings, ContextSeries) {
    let candles: Vec<Candle> = (0..len)
        .map(|index| {
            let close = if index == 25 { 101.0 } else { 100.0 };
            let high = if index == 25 { 101.2 } else { 100.5 };
            let low = if index == 25 { 100.8 } else { 99.5 };
            candle(index as u64, high, low, close)
        })
        .collect();
    let readings = Readings {
        ema20: series(len, |_| Some(100.0)),
        ema50: series(len, |_| Some(100.0)),
        ema200: series(len, |_| Some(100.0)),
        rsi14: series(len, |_| Some(50.0)),
        atr14: series(len, |index| Some(if index < 20 { 2.0 } else { 1.0 })),
        rvol20: series(len, |index| {
            Some(if index == 25 { rvol_at_25 } else { 0.5 })
        }),
        pivots: PivotSeries::compute(&[]),
    };
    let context = ContextSeries::compute(&readings);
    (candles, readings, context)
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
fn signal_compression_breakout_row_contract() {
    assert_contract(
        "compression breakout",
        codes::SIGNAL_COMPRESSION_BREAKOUT,
        30,
        &[24, 25, 26],
        Expected {
            event_type: EventType::Signal,
            subtype: 7,
            source: 5,
            direction: 1,
            occurred: 25,
            available: 25,
            primary: 101.0,
            reference: 100.5,
        },
        |len| {
            let (candles, readings, context) = fixture(len, 1.5);
            CompressionBreakoutSeries::compute(
                &candles,
                &readings,
                &context,
                &CompressionBreakoutConfig::new(timeframe()),
            )
            .events()
            .to_vec()
        },
    );
}

#[test]
fn signal_compression_range_row_contract() {
    assert_contract(
        "compression range",
        codes::SIGNAL_COMPRESSION_RANGE,
        30,
        &[23, 24, 25],
        Expected {
            event_type: EventType::Signal,
            subtype: 8,
            source: 5,
            direction: 0,
            occurred: 24,
            available: 24,
            primary: 100.5,
            reference: 99.5,
        },
        |len| {
            let (candles, _, context) = fixture(len, 0.5);
            CompressionRangeSeries::compute(
                &candles,
                &context,
                &CompressionRangeConfig::new(timeframe()),
            )
            .events()
            .to_vec()
        },
    );
}

#[test]
fn signal_compression_range_onset_row_contract() {
    assert_contract(
        "compression range onset",
        codes::SIGNAL_COMPRESSION_RANGE_ONSET,
        30,
        &[23, 24, 25],
        Expected {
            event_type: EventType::Signal,
            subtype: 9,
            source: 5,
            direction: 0,
            occurred: 24,
            available: 24,
            primary: 100.5,
            reference: 99.5,
        },
        |len| {
            let (candles, _, context) = fixture(len, 0.5);
            CompressionRangeOnsetSeries::compute(
                &candles,
                &context,
                &CompressionRangeConfig::new(timeframe()),
            )
            .events()
            .to_vec()
        },
    );
}
