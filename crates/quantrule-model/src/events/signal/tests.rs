use super::{EMA20_SOURCE_CODE, RSI_SOURCE_CODE, SignalConfig, SignalEventSeries, SignalSubtype};
use crate::input::SignalSettings;
use crate::readings::{PivotSeries, ReadingSeries, Readings};
use quantrule_data::Candle;
use std::num::NonZeroU64;

fn candle(index: u64, mid: f64) -> Candle {
    Candle::new(
        index,
        index as i64,
        mid,
        mid + 0.5,
        mid - 0.5,
        mid,
        Some(10.0),
    )
    .expect("the test candle is valid")
}

fn candles(mids: &[f64]) -> Vec<Candle> {
    mids.iter()
        .enumerate()
        .map(|(index, mid)| candle(index as u64, *mid))
        .collect()
}

fn series(values: Vec<Option<f64>>) -> ReadingSeries {
    ReadingSeries::new(values)
}

fn flat(len: usize, value: f64) -> Vec<Option<f64>> {
    vec![Some(value); len]
}

fn none(len: usize) -> Vec<Option<f64>> {
    vec![None; len]
}

#[allow(clippy::too_many_arguments)]
fn readings_with(
    candles: &[Candle],
    ema20: Vec<Option<f64>>,
    ema50: Vec<Option<f64>>,
    ema200: Vec<Option<f64>>,
    rsi: Vec<Option<f64>>,
    atr: Vec<Option<f64>>,
) -> Readings {
    let len = candles.len();
    assert_eq!(ema20.len(), len);
    assert_eq!(ema50.len(), len);
    assert_eq!(ema200.len(), len);
    assert_eq!(rsi.len(), len);
    assert_eq!(atr.len(), len);
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

fn config() -> SignalConfig {
    SignalConfig::new(NonZeroU64::new(300).expect("a non-zero timeframe"))
}

fn signals(_candles: &[Candle], readings: &Readings) -> SignalEventSeries {
    SignalEventSeries::compute(
        readings,
        &crate::readings::MacdSeries::empty(readings.len()),
        &config(),
    )
}

fn events_of(
    series: &SignalEventSeries,
    subtype: SignalSubtype,
    source_code: u16,
) -> Vec<&crate::events::EventRecord> {
    series
        .events()
        .iter()
        .filter(|event| {
            event.event_subtype_code == subtype.code() && event.source_code == source_code
        })
        .collect()
}

#[test]
fn macd_cross_zero_cross_and_histogram_flip_are_distinct() {
    use crate::readings::{MacdSeries, ReadingSeries};

    let candles = candles(&[100.0; 5]);
    let readings = readings_with(
        &candles,
        flat(5, 100.0),
        flat(5, 100.0),
        flat(5, 100.0),
        flat(5, 50.0),
        flat(5, 10.0),
    );
    let macd = MacdSeries {
        line: ReadingSeries::new(vec![None, Some(-0.2), Some(-0.1), Some(0.1), Some(0.15)]),
        signal: ReadingSeries::new(vec![None, Some(-0.1), Some(-0.1), Some(0.0), Some(0.1)]),
        histogram: ReadingSeries::new(vec![None, Some(-0.1), Some(0.0), Some(0.1), Some(0.05)]),
    };
    let series = SignalEventSeries::compute(&readings, &macd, &config());

    assert!(
        series.events().iter().any(|event| {
            event.event_subtype_code == SignalSubtype::SeriesCrossover.code()
                && event.source_code == super::MACD_LINE_SOURCE_CODE
        }),
        "the line/signal cross must be emitted"
    );
    let zero: Vec<_> = series
        .events()
        .iter()
        .filter(|event| event.event_subtype_code == SignalSubtype::ZeroCross.code())
        .collect();
    assert_eq!(zero.len(), 1);
    let flips: Vec<_> = series
        .events()
        .iter()
        .filter(|event| event.event_subtype_code == SignalSubtype::HistogramSlopeFlip.code())
        .collect();
    assert_eq!(flips.len(), 1);
    assert_eq!(flips[0].direction_code, -1);
}

#[test]
fn an_rsi_upward_crossing_emits_a_threshold_event() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(29.0), Some(31.0)],
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    let event = events[0];
    assert_eq!(event.source_code, RSI_SOURCE_CODE);
    assert_eq!(event.direction_code, 1);
    assert_eq!(event.reference_value, Some(30.0));
    assert_eq!(event.primary_value, Some(31.0));
    assert_eq!(event.reference_code, None);
    assert_eq!(event.occurred_bar_index, 1);
    assert_eq!(event.available_bar_index, 1);
}

#[test]
fn an_rsi_downward_crossing_emits_a_threshold_event() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(31.0), Some(29.0)],
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].direction_code, -1);
    assert_eq!(events[0].reference_value, Some(30.0));
}

#[test]
fn a_cross_from_equality_is_a_threshold_crossing() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(30.0), Some(31.0)],
        none(2),
    );
    let series = signals(&candles, &readings);
    let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reference_value, Some(30.0));
    assert_eq!(events[0].direction_code, 1);
}

#[test]
fn a_close_exactly_on_a_threshold_is_not_a_crossing() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(29.0), Some(30.0)],
        none(2),
    );
    let series = signals(&candles, &readings);
    assert!(events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE).is_empty());
}

#[test]
fn two_thresholds_can_cross_on_one_bar() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(29.0), Some(51.0)],
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].reference_value, Some(30.0));
    assert_eq!(events[1].reference_value, Some(50.0));
}

#[test]
fn custom_rsi_levels_replace_the_default_thresholds() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        none(2),
        none(2),
        none(2),
        vec![Some(39.0), Some(41.0)],
        none(2),
    );
    let settings = SignalSettings {
        rsi_levels: [40.0, 50.0, 60.0],
        ..SignalSettings::DEFAULT
    };
    let config = SignalConfig::with_settings(
        NonZeroU64::new(300).expect("a non-zero timeframe"),
        settings,
    );
    let series = SignalEventSeries::compute(
        &readings,
        &crate::readings::MacdSeries::empty(readings.len()),
        &config,
    );
    let events = events_of(&series, SignalSubtype::ThresholdCrossing, RSI_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reference_value, Some(40.0));
}

#[test]
fn custom_ema_periods_map_pairs_to_the_configured_roles() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(102.0)],
        vec![Some(100.0), Some(101.0)],
        vec![Some(101.0), Some(100.0)],
        none(2),
        none(2),
    );
    let settings = SignalSettings {
        ema_pairs: [(9, 21), (21, 55), (9, 55)],
        ..SignalSettings::DEFAULT
    };
    let config = SignalConfig::with_settings(
        NonZeroU64::new(300).expect("a non-zero timeframe"),
        settings,
    )
    .with_ema_periods([9, 21, 55]);
    let series = SignalEventSeries::compute(
        &readings,
        &crate::readings::MacdSeries::empty(readings.len()),
        &config,
    );
    let events: Vec<_> = series
        .events()
        .iter()
        .filter(|event| event.event_subtype_code == SignalSubtype::SeriesCrossover.code())
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].source_code, EMA20_SOURCE_CODE);
    assert_eq!(events[1].source_code, super::EMA50_SOURCE_CODE);
    assert_eq!(events[2].source_code, EMA20_SOURCE_CODE);
}

#[test]
fn an_ema_crossing_emits_a_series_event() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(101.0)],
        flat(2, 100.0),
        flat(2, 90.0),
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    let event = events[0];
    assert_eq!(event.reference_code, Some(super::EMA50_REFERENCE_CODE));
    assert_eq!(event.primary_value, Some(101.0));
    assert_eq!(event.reference_value, Some(100.0));
    assert_eq!(event.direction_code, 1);
}

#[test]
fn an_ema_downward_crossing_emits_a_series_event() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(101.0), Some(99.0)],
        flat(2, 100.0),
        flat(2, 90.0),
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].direction_code, -1);
}

#[test]
fn a_cross_from_equality_is_emitted() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(100.0), Some(101.0)],
        flat(2, 100.0),
        flat(2, 90.0),
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    let events = events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].direction_code, 1);
}

#[test]
fn current_equality_is_not_a_crossing() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(100.0)],
        flat(2, 100.0),
        flat(2, 90.0),
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    assert!(events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE).is_empty());
}

#[test]
fn all_three_ema_pairs_can_cross_on_one_bar() {
    let candles = candles(&[100.0, 100.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(102.0)],
        vec![Some(100.0), Some(101.0)],
        vec![Some(101.0), Some(100.0)],
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    let events: Vec<_> = series
        .events()
        .iter()
        .filter(|event| event.event_subtype_code == SignalSubtype::SeriesCrossover.code())
        .collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].source_code, EMA20_SOURCE_CODE);
    assert_eq!(events[1].source_code, super::EMA50_SOURCE_CODE);
    assert_eq!(events[2].source_code, EMA20_SOURCE_CODE);
}

#[test]
fn a_price_ema_crossing_is_not_a_signal() {
    let candles = candles(&[99.0, 101.0]);
    let readings = readings_with(
        &candles,
        flat(2, 100.0),
        flat(2, 100.0),
        flat(2, 100.0),
        none(2),
        none(2),
    );
    let series = signals(&candles, &readings);

    assert!(events_of(&series, SignalSubtype::SeriesCrossover, EMA20_SOURCE_CODE).is_empty());
    assert!(
        events_of(
            &series,
            SignalSubtype::SeriesCrossover,
            super::EMA50_SOURCE_CODE
        )
        .is_empty()
    );
    assert!(
        events_of(
            &series,
            SignalSubtype::SeriesCrossover,
            super::EMA200_SOURCE_CODE
        )
        .is_empty()
    );
}

#[test]
fn an_empty_series_has_no_events() {
    let candles = Vec::new();
    let readings = readings_with(&candles, vec![], vec![], vec![], vec![], vec![]);
    let series = signals(&candles, &readings);
    assert!(series.is_empty());
}

#[test]
fn a_single_candle_has_no_events() {
    let candles = candles(&[100.0]);
    let readings = Readings::compute(&candles);
    let series = signals(&candles, &readings);
    assert!(series.is_empty());
}

#[test]
fn later_candles_do_not_change_earlier_signals() {
    let candles = candles(&[100.0, 101.0, 102.0, 103.0, 104.0]);
    let ema20 = vec![
        Some(99.0),
        Some(101.0),
        Some(102.0),
        Some(103.0),
        Some(104.0),
    ];
    let ema50 = flat(5, 100.0);
    let ema200 = flat(5, 90.0);
    let rsi = vec![Some(29.0), Some(31.0), Some(51.0), Some(49.0), Some(71.0)];
    let atr = flat(5, 10.0);
    let full_readings = readings_with(
        &candles,
        ema20.clone(),
        ema50.clone(),
        ema200.clone(),
        rsi.clone(),
        atr.clone(),
    );
    let full = signals(&candles, &full_readings);

    for length in 0..=candles.len() {
        let prefix_candles = &candles[..length];
        let prefix_readings = readings_with(
            prefix_candles,
            ema20[..length].to_vec(),
            ema50[..length].to_vec(),
            ema200[..length].to_vec(),
            rsi[..length].to_vec(),
            atr[..length].to_vec(),
        );
        let partial = signals(prefix_candles, &prefix_readings);
        let expected: Vec<_> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < length)
            .copied()
            .collect();
        assert_eq!(
            partial.events(),
            expected.as_slice(),
            "the prefix of length {length} changed an earlier signal"
        );
    }
}

#[test]
fn the_detector_is_deterministic() {
    let candles = candles(&[100.0, 101.0, 102.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(101.0), Some(102.0)],
        flat(3, 100.0),
        flat(3, 90.0),
        vec![Some(29.0), Some(31.0), Some(51.0)],
        flat(3, 10.0),
    );
    let first = signals(&candles, &readings);
    let second = signals(&candles, &readings);
    assert_eq!(first, second);
}

#[test]
fn the_detector_does_not_change_the_readings() {
    let candles = candles(&[100.0, 101.0, 102.0]);
    let readings = readings_with(
        &candles,
        vec![Some(99.0), Some(101.0), Some(102.0)],
        flat(3, 100.0),
        flat(3, 90.0),
        vec![Some(29.0), Some(31.0), Some(51.0)],
        flat(3, 10.0),
    );
    let before = readings.clone();

    let events = SignalEventSeries::compute(
        &readings,
        &crate::readings::MacdSeries::empty(readings.len()),
        &config(),
    );
    assert!(!events.is_empty(), "the fixture must produce an event");
    assert_eq!(readings, before);
}

#[test]
fn the_signal_codes_are_the_contract_codes() {
    assert_eq!(SignalSubtype::ThresholdCrossing.code(), 1);
    assert_eq!(SignalSubtype::SeriesCrossover.code(), 2);
    assert_eq!(SignalSubtype::CompactStateChange.code(), 3);
    assert_eq!(SignalSubtype::TrendContinuation.code(), 6);
    assert_eq!(super::RSI_THRESHOLDS, [30.0, 50.0, 70.0]);
    assert_eq!(super::EMA_PAIRS.len(), 3);
}

#[test]
fn ema_stack_and_rsi_zone_changes_are_published() {
    let candles = candles(&[100.0; 3]);
    let readings = readings_with(
        &candles,
        vec![Some(100.0), Some(100.0), Some(101.0)],
        vec![Some(100.0), Some(100.0), Some(100.0)],
        vec![Some(100.0), Some(100.0), Some(99.0)],
        vec![Some(50.0), Some(50.0), Some(75.0)],
        flat(3, 10.0),
    );
    let series = signals(&candles, &readings);
    assert!(series.events().iter().any(|event| {
        event.event_code == crate::events::codes::SIGNAL_EMA_STACK_CHANGE
            && event.occurred_bar_index == 2
    }));
    assert!(series.events().iter().any(|event| {
        event.event_code == crate::events::codes::SIGNAL_RSI_ZONE_CHANGE
            && event.occurred_bar_index == 2
    }));
}
