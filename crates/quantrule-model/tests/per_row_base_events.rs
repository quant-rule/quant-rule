//! Row-level contracts for the 16 base events.
//!
//! Each test triggers one registered base event and asserts its code, family,
//! availability, one-occurrence property, causality, and determinism. The level
//! and divergence rows use the checked-in scalar fixture; the signal rows use
//! explicit readings so the condition is isolated.
//!
//! These are construct contracts: a row proves its event is emitted with the
//! registered identity under a triggering condition and is never published
//! before it is knowable. Numeric golden values and the full edge matrix remain
//! family-level evidence.

use quantrule_data::Candle;
use quantrule_model::events::codes;
use quantrule_model::events::divergence::{self, DivergenceConfig, DivergenceEventSeries};
use quantrule_model::events::dmi::{self, DmiConfig, DmiEventSeries};
use quantrule_model::events::level::{self, LevelEventConfig, LevelEventSeries};
use quantrule_model::events::signal::{self, SignalConfig, SignalEventSeries};
use quantrule_model::events::trend::{TrendContinuationConfig, TrendContinuationSeries};
use quantrule_model::events::{EventRecord, EventType};
use quantrule_model::readings::{
    AdxSeries, MacdSeries, PivotScale, PivotSeries, ReadingSeries, Readings,
};
use std::collections::BTreeSet;
use std::num::NonZeroU64;

const FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const TIMEFRAME: u64 = 300;
const LEVEL_SOURCES: [u16; 5] = [
    level::EMA20_SOURCE_CODE,
    level::EMA50_SOURCE_CODE,
    level::EMA200_SOURCE_CODE,
    level::MINOR_PIVOT_SOURCE_CODE,
    level::MAJOR_PIVOT_SOURCE_CODE,
];

fn timeframe() -> NonZeroU64 {
    NonZeroU64::new(TIMEFRAME).expect("non-zero timeframe")
}

fn fixture_candles() -> Vec<Candle> {
    let mut lines = FIXTURE.lines();
    assert_eq!(lines.next(), Some(HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 23);
            Candle::new(
                fields[0].parse().expect("bar"),
                fields[1].parse().expect("timestamp"),
                fields[2].parse().expect("open"),
                fields[3].parse().expect("high"),
                fields[4].parse().expect("low"),
                fields[5].parse().expect("close"),
                if fields[6].is_empty() {
                    None
                } else {
                    Some(fields[6].parse().expect("volume"))
                },
            )
            .expect("valid fixture candle")
        })
        .collect()
}

fn series(values: Vec<Option<f64>>) -> ReadingSeries {
    ReadingSeries::new(values)
}

fn none(len: usize) -> Vec<Option<f64>> {
    vec![None; len]
}

fn readings_with(
    candles: &[Candle],
    ema20: Vec<Option<f64>>,
    ema50: Vec<Option<f64>>,
    ema200: Vec<Option<f64>>,
    rsi: Vec<Option<f64>>,
    atr: Vec<Option<f64>>,
) -> Readings {
    let len = candles.len();
    Readings {
        ema20: series(ema20),
        ema50: series(ema50),
        ema200: series(ema200),
        rsi14: series(rsi),
        atr14: series(atr),
        rvol20: series(none(len)),
        pivots: PivotSeries::compute(candles),
    }
}

fn flat(len: usize, value: f64) -> Vec<Option<f64>> {
    vec![Some(value); len]
}

fn macd_with(line: Vec<Option<f64>>, signal: Vec<Option<f64>>) -> MacdSeries {
    let histogram = line
        .iter()
        .zip(&signal)
        .map(|(line, signal)| match (line, signal) {
            (Some(line), Some(signal)) => Some(line - signal),
            _ => None,
        })
        .collect();
    MacdSeries {
        line: series(line),
        signal: series(signal),
        histogram: series(histogram),
    }
}

fn adx_with(
    adx: Vec<Option<f64>>,
    di_plus: Vec<Option<f64>>,
    di_minus: Vec<Option<f64>>,
) -> AdxSeries {
    AdxSeries {
        adx: series(adx),
        di_plus: series(di_plus),
        di_minus: series(di_minus),
    }
}

fn assert_event<'a>(
    events: &'a [EventRecord],
    code: u16,
    event_type: EventType,
    subtype: u8,
    sources: &[u16],
) -> &'a EventRecord {
    let event = events
        .iter()
        .filter(|event| event.event_code == code)
        .min_by_key(|event| event.available_bar_index)
        .unwrap_or_else(|| panic!("missing event code {code}"));
    assert_eq!(event.event_type, event_type);
    assert_eq!(event.event_subtype_code, subtype);
    assert!(
        sources.contains(&event.source_code),
        "event code {code}: source {} is not one of {sources:?}",
        event.source_code
    );
    assert!(event.available_bar_index >= event.occurred_bar_index);
    event
}

fn assert_signal_event(
    events: &[EventRecord],
    code: u16,
    subtype: u8,
    source: u16,
    occurred_bar_index: usize,
) -> &EventRecord {
    let event = assert_event(events, code, EventType::Signal, subtype, &[source]);
    assert_eq!(event.occurred_bar_index, occurred_bar_index);
    assert_eq!(event.available_bar_index, event.occurred_bar_index);
    event
}

fn assert_one_occurrence(events: &[EventRecord], code: u16) {
    let mut seen = BTreeSet::new();
    for event in events.iter().filter(|event| event.event_code == code) {
        assert!(
            seen.insert((
                event.source_code,
                event.occurred_bar_index,
                event.available_bar_index,
                event.direction_code,
                event.reference_code,
                event.primary_value.map(f64::to_bits),
                event.reference_value.map(f64::to_bits),
            )),
            "duplicate event code {code}"
        );
    }
}

fn assert_deterministic(produce: impl Fn() -> Vec<EventRecord>, label: &str) {
    assert_eq!(produce(), produce(), "{label} is not deterministic");
}

/// Compares every prefix against the full stream and requires a prefix to
/// contain exactly the events that were already knowable at that length.
fn assert_causal_prefixes(len: usize, produce: impl Fn(usize) -> Vec<EventRecord>, label: &str) {
    let full = produce(len);
    for prefix_len in 0..=len {
        let prefix = produce(prefix_len);
        let expected: Vec<EventRecord> = full
            .iter()
            .filter(|event| event.available_bar_index < prefix_len)
            .copied()
            .collect();
        assert_eq!(
            prefix, expected,
            "{label}: prefix of length {prefix_len} changed an earlier event"
        );
    }
}

fn slice(values: &ReadingSeries, len: usize) -> ReadingSeries {
    ReadingSeries::new(values.values()[..len].to_vec())
}

fn prefix_readings(readings: &Readings, candles: &[Candle], len: usize) -> Readings {
    Readings {
        ema20: slice(&readings.ema20, len),
        ema50: slice(&readings.ema50, len),
        ema200: slice(&readings.ema200, len),
        rsi14: slice(&readings.rsi14, len),
        atr14: slice(&readings.atr14, len),
        rvol20: slice(&readings.rvol20, len),
        pivots: PivotSeries::compute(&candles[..len]),
    }
}

fn prefix_macd(macd: &MacdSeries, len: usize) -> MacdSeries {
    MacdSeries {
        line: slice(&macd.line, len),
        signal: slice(&macd.signal, len),
        histogram: slice(&macd.histogram, len),
    }
}

fn prefix_adx(adx: &AdxSeries, len: usize) -> AdxSeries {
    AdxSeries {
        adx: slice(&adx.adx, len),
        di_plus: slice(&adx.di_plus, len),
        di_minus: slice(&adx.di_minus, len),
    }
}

fn level_events_for(candles: &[Candle]) -> Vec<EventRecord> {
    let readings = Readings::compute(candles);
    LevelEventSeries::compute(
        candles,
        &readings,
        &LevelEventConfig::from_secs(TIMEFRAME).expect("timeframe"),
    )
    .events()
    .to_vec()
}

fn level_events() -> Vec<EventRecord> {
    level_events_for(&fixture_candles())
}

fn divergence_events_for(candles: &[Candle]) -> Vec<EventRecord> {
    let readings = Readings::compute(candles);
    let mut events = Vec::new();
    for scale in [PivotScale::Major, PivotScale::Minor] {
        events.extend(
            DivergenceEventSeries::compute(
                &readings,
                &DivergenceConfig::from_secs(TIMEFRAME, scale).expect("timeframe"),
            )
            .events()
            .iter()
            .copied(),
        );
    }
    events
}

fn divergence_events() -> Vec<EventRecord> {
    divergence_events_for(&fixture_candles())
}

fn divergence_events_with(
    candles: &[Candle],
    rsi: &[Option<f64>],
    atr: &[Option<f64>],
) -> Vec<EventRecord> {
    let readings = readings_with(
        candles,
        none(candles.len()),
        none(candles.len()),
        none(candles.len()),
        rsi.to_vec(),
        atr.to_vec(),
    );
    DivergenceEventSeries::compute(
        &readings,
        &DivergenceConfig::from_secs(TIMEFRAME, PivotScale::Minor).expect("timeframe"),
    )
    .events()
    .to_vec()
}

#[test]
fn level_test_row_contract() {
    let candles = fixture_candles();
    let events = level_events();
    assert_event(
        &events,
        codes::LEVEL_TEST,
        EventType::Level,
        1,
        &LEVEL_SOURCES,
    );
    assert_one_occurrence(&events, codes::LEVEL_TEST);
    assert_deterministic(level_events, "level events");
    assert_causal_prefixes(
        candles.len(),
        |len| level_events_for(&candles[..len]),
        "level.test",
    );
}

#[test]
fn level_cross_row_contract() {
    let candles = fixture_candles();
    let events = level_events();
    assert_event(
        &events,
        codes::LEVEL_CROSS,
        EventType::Level,
        2,
        &LEVEL_SOURCES,
    );
    assert_one_occurrence(&events, codes::LEVEL_CROSS);
    assert_deterministic(level_events, "level events");
    assert_causal_prefixes(
        candles.len(),
        |len| level_events_for(&candles[..len]),
        "level.cross",
    );
}

#[test]
fn level_break_row_contract() {
    let candles = fixture_candles();
    let events = level_events();
    assert_event(
        &events,
        codes::LEVEL_BREAK,
        EventType::Level,
        3,
        &LEVEL_SOURCES,
    );
    assert_one_occurrence(&events, codes::LEVEL_BREAK);
    assert_deterministic(level_events, "level events");
    assert_causal_prefixes(
        candles.len(),
        |len| level_events_for(&candles[..len]),
        "level.break",
    );
}

#[test]
fn divergence_rsi_regular_bullish_row_contract() {
    let candles = fixture_candles();
    let events = divergence_events();
    assert_event(
        &events,
        codes::DIVERGENCE_RSI_REGULAR_BULLISH,
        EventType::Divergence,
        1,
        &[divergence::RSI_SOURCE_CODE],
    );
    assert_one_occurrence(&events, codes::DIVERGENCE_RSI_REGULAR_BULLISH);
    assert_deterministic(divergence_events, "divergence events");
    assert_causal_prefixes(
        candles.len(),
        |len| divergence_events_for(&candles[..len]),
        "divergence.rsi_regular_bullish",
    );
}

#[test]
fn divergence_rsi_regular_bearish_row_contract() {
    let candles = fixture_candles();
    let events = divergence_events();
    assert_event(
        &events,
        codes::DIVERGENCE_RSI_REGULAR_BEARISH,
        EventType::Divergence,
        2,
        &[divergence::RSI_SOURCE_CODE],
    );
    assert_one_occurrence(&events, codes::DIVERGENCE_RSI_REGULAR_BEARISH);
    assert_deterministic(divergence_events, "divergence events");
    assert_causal_prefixes(
        candles.len(),
        |len| divergence_events_for(&candles[..len]),
        "divergence.rsi_regular_bearish",
    );
}

#[test]
fn divergence_rsi_hidden_bullish_row_contract() {
    // The scalar fixture has no higher-low/lower-RSI-low pair, so this row
    // supplies the smallest series that produces one.
    let mids = [
        100.0, 99.0, 98.0, 99.0, 100.0, 101.0, 100.0, 99.0, 98.5, 99.0, 100.0, 101.0,
    ];
    let candles: Vec<Candle> = mids
        .iter()
        .enumerate()
        .map(|(index, mid)| {
            Candle::new(
                index as u64,
                index as i64,
                *mid,
                mid + 0.5,
                mid - 0.5,
                *mid,
                Some(10.0),
            )
            .expect("candle")
        })
        .collect();
    let rsi: Vec<Option<f64>> = [
        50.0, 45.0, 30.0, 35.0, 40.0, 50.0, 45.0, 40.0, 20.0, 25.0, 30.0, 50.0,
    ]
    .into_iter()
    .map(Some)
    .collect();
    let mut atr = none(candles.len());
    atr[8] = Some(2.0);
    let events = divergence_events_with(&candles, &rsi, &atr);
    assert_event(
        &events,
        codes::DIVERGENCE_RSI_HIDDEN_BULLISH,
        EventType::Divergence,
        3,
        &[divergence::RSI_SOURCE_CODE],
    );
    assert_one_occurrence(&events, codes::DIVERGENCE_RSI_HIDDEN_BULLISH);
    assert_deterministic(
        || divergence_events_with(&candles, &rsi, &atr),
        "hidden bullish divergence",
    );
    assert_causal_prefixes(
        candles.len(),
        |len| divergence_events_with(&candles[..len], &rsi[..len], &atr[..len]),
        "divergence.rsi_hidden_bullish",
    );
}

#[test]
fn divergence_rsi_hidden_bearish_row_contract() {
    let candles = fixture_candles();
    let events = divergence_events();
    assert_event(
        &events,
        codes::DIVERGENCE_RSI_HIDDEN_BEARISH,
        EventType::Divergence,
        4,
        &[divergence::RSI_SOURCE_CODE],
    );
    assert_one_occurrence(&events, codes::DIVERGENCE_RSI_HIDDEN_BEARISH);
    assert_deterministic(divergence_events, "divergence events");
    assert_causal_prefixes(
        candles.len(),
        |len| divergence_events_for(&candles[..len]),
        "divergence.rsi_hidden_bearish",
    );
}

fn signal_events(readings: &Readings, macd: &MacdSeries) -> Vec<EventRecord> {
    SignalEventSeries::compute(readings, macd, &SignalConfig::new(timeframe()))
        .events()
        .to_vec()
}

fn signal_causal_prefixes(candles: &[Candle], readings: &Readings, macd: &MacdSeries, label: &str) {
    assert_causal_prefixes(
        readings.len(),
        |len| {
            signal_events(
                &prefix_readings(readings, candles, len),
                &prefix_macd(macd, len),
            )
        },
        label,
    );
}

fn signal_candles(len: u64) -> Vec<Candle> {
    (0..len)
        .map(|index| {
            Candle::new(index, index as i64, 100.0, 101.0, 99.0, 100.0, None).expect("candle")
        })
        .collect()
}

#[test]
fn rsi_threshold_row_contract() {
    let candles = signal_candles(2);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(29.0), Some(31.0)],
        none(2),
    );
    let macd = MacdSeries::empty(2);
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_RSI_THRESHOLD,
        1,
        signal::RSI_SOURCE_CODE,
        1,
    );
    assert_one_occurrence(&events, codes::SIGNAL_RSI_THRESHOLD);
    assert_deterministic(|| signal_events(&readings, &macd), "rsi threshold");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.rsi_threshold");
}

#[test]
fn ema_pair_cross_row_contract() {
    let candles = signal_candles(2);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(102.0)],
        vec![Some(100.0), Some(101.0)],
        none(2),
        none(2),
        none(2),
    );
    let macd = MacdSeries::empty(2);
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_EMA_PAIR_CROSS,
        2,
        signal::EMA20_SOURCE_CODE,
        1,
    );
    assert_one_occurrence(&events, codes::SIGNAL_EMA_PAIR_CROSS);
    assert_deterministic(|| signal_events(&readings, &macd), "ema pair cross");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.ema_pair_cross");
}

#[test]
fn ema_stack_change_row_contract() {
    let candles = signal_candles(3);
    let readings = readings_with(
        &candles,
        vec![Some(100.0), Some(100.0), Some(101.0)],
        vec![Some(100.0), Some(100.0), Some(100.0)],
        vec![Some(100.0), Some(100.0), Some(99.0)],
        flat(3, 50.0),
        flat(3, 10.0),
    );
    let macd = MacdSeries::empty(3);
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_EMA_STACK_CHANGE,
        3,
        signal::EMA20_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_EMA_STACK_CHANGE);
    assert_deterministic(|| signal_events(&readings, &macd), "ema stack change");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.ema_stack_change");
}

#[test]
fn rsi_zone_change_row_contract() {
    let candles = signal_candles(3);
    let readings = readings_with(
        &candles,
        none(3),
        none(3),
        none(3),
        vec![Some(50.0), Some(50.0), Some(75.0)],
        flat(3, 10.0),
    );
    let macd = MacdSeries::empty(3);
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_RSI_ZONE_CHANGE,
        3,
        signal::RSI_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_RSI_ZONE_CHANGE);
    assert_deterministic(|| signal_events(&readings, &macd), "rsi zone change");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.rsi_zone_change");
}

#[test]
fn macd_line_signal_cross_row_contract() {
    let candles = signal_candles(3);
    let readings = readings_with(&candles, none(3), none(3), none(3), none(3), none(3));
    let macd = macd_with(
        vec![Some(1.0), Some(1.0), Some(2.0)],
        vec![Some(2.0), Some(2.0), Some(1.0)],
    );
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_MACD_LINE_SIGNAL_CROSS,
        2,
        signal::MACD_LINE_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_MACD_LINE_SIGNAL_CROSS);
    assert_deterministic(|| signal_events(&readings, &macd), "macd line/signal cross");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.macd_line_signal_cross");
}

#[test]
fn macd_histogram_slope_flip_row_contract() {
    let candles = signal_candles(3);
    let readings = readings_with(&candles, none(3), none(3), none(3), none(3), none(3));
    let macd = MacdSeries {
        line: series(flat(3, 0.0)),
        signal: series(flat(3, 0.0)),
        histogram: series(vec![Some(1.0), Some(2.0), Some(1.0)]),
    };
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_MACD_HISTOGRAM_SLOPE_FLIP,
        4,
        signal::MACD_HISTOGRAM_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_MACD_HISTOGRAM_SLOPE_FLIP);
    assert_deterministic(
        || signal_events(&readings, &macd),
        "macd histogram slope flip",
    );
    signal_causal_prefixes(
        &candles,
        &readings,
        &macd,
        "signal.macd_histogram_slope_flip",
    );
}

#[test]
fn macd_zero_cross_row_contract() {
    let candles = signal_candles(3);
    let readings = readings_with(&candles, none(3), none(3), none(3), none(3), none(3));
    let macd = macd_with(vec![Some(-1.0), Some(-1.0), Some(1.0)], flat(3, 0.0));
    let events = signal_events(&readings, &macd);
    assert_signal_event(
        &events,
        codes::SIGNAL_MACD_ZERO_CROSS,
        5,
        signal::MACD_LINE_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_MACD_ZERO_CROSS);
    assert_deterministic(|| signal_events(&readings, &macd), "macd zero cross");
    signal_causal_prefixes(&candles, &readings, &macd, "signal.macd_zero_cross");
}

#[test]
fn di_cross_row_contract() {
    let candles = signal_candles(3);
    let adx = adx_with(
        none(3),
        vec![Some(1.0), Some(1.0), Some(2.0)],
        vec![Some(2.0), Some(2.0), Some(1.0)],
    );
    let config = DmiConfig::new(timeframe());
    let events = DmiEventSeries::compute(&adx, &config).events().to_vec();
    assert_signal_event(
        &events,
        codes::SIGNAL_DI_CROSS,
        2,
        dmi::DI_PLUS_SOURCE_CODE,
        2,
    );
    assert_one_occurrence(&events, codes::SIGNAL_DI_CROSS);
    assert_deterministic(
        || DmiEventSeries::compute(&adx, &config).events().to_vec(),
        "di cross",
    );
    assert_causal_prefixes(
        candles.len(),
        |len| {
            DmiEventSeries::compute(&prefix_adx(&adx, len), &config)
                .events()
                .to_vec()
        },
        "signal.di_cross",
    );
}

#[test]
fn trend_continuation_row_contract() {
    let candles = signal_candles(4);
    let readings = readings_with(
        &candles,
        flat(4, 103.0),
        flat(4, 102.0),
        flat(4, 101.0),
        flat(4, 60.0),
        flat(4, 10.0),
    );
    let macd = macd_with(flat(4, 2.0), flat(4, 1.0));
    let adx = adx_with(
        vec![Some(10.0), Some(20.0), Some(30.0), Some(40.0)],
        flat(4, 30.0),
        flat(4, 10.0),
    );
    let config = TrendContinuationConfig::new(timeframe(), 25.0, 3);
    let events = TrendContinuationSeries::compute(&readings, &macd, &adx, &config)
        .events()
        .to_vec();
    assert_signal_event(
        &events,
        codes::SIGNAL_TREND_CONTINUATION,
        6,
        dmi::ADX_SOURCE_CODE,
        3,
    );
    assert_one_occurrence(&events, codes::SIGNAL_TREND_CONTINUATION);
    assert_deterministic(
        || {
            TrendContinuationSeries::compute(&readings, &macd, &adx, &config)
                .events()
                .to_vec()
        },
        "trend continuation",
    );
    assert_causal_prefixes(
        readings.len(),
        |len| {
            TrendContinuationSeries::compute(
                &prefix_readings(&readings, &candles, len),
                &prefix_macd(&macd, len),
                &prefix_adx(&adx, len),
                &config,
            )
            .events()
            .to_vec()
        },
        "signal.trend_continuation",
    );
}
