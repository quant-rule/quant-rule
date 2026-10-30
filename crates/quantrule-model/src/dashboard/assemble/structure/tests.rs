use super::{activity_from_events, event_axis, level_references};
use crate::events::signal::{
    DIRECTION_SOURCE_CODE, EMA20_SOURCE_CODE, MOMENTUM_SOURCE_CODE, RSI_SOURCE_CODE,
    STRUCTURE_SOURCE_CODE, SignalSubtype,
};
use crate::events::{EventRecord, EventType};
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::readings::roles::AxisCode;
use crate::readings::{PivotKind, PivotScale};
use crate::timeframe::TimeframeModel;
use quantrule_data::Candle;

const READINGS_FIXTURE: &str = include_str!("../../../../tests/fixtures/readings_golden.csv");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

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

fn fixture_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        let series_index = fields[0].parse().expect("bar");
        let timestamp = fields[1].parse().expect("timestamp");
        let open = number(fields[2]).expect("open");
        let high = number(fields[3]).expect("high");
        let low = number(fields[4]).expect("low");
        let close = number(fields[5]).expect("close");
        let volume = number(fields[6]);
        candles.push(
            Candle::new(series_index, timestamp, open, high, low, close, volume)
                .expect("valid fixture candle"),
        );
    }
    candles
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 60,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture is non-empty")
}

fn event(event_type: EventType, subtype: u8, source: u16) -> EventRecord {
    EventRecord {
        event_type,
        event_subtype_code: subtype,
        source_code: source,
        timeframe_secs: 60,
        occurred_bar_index: 0,
        available_bar_index: 0,
        direction_code: 0,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }
}

#[test]
fn every_event_maps_to_exactly_one_axis_or_to_none() {
    assert_eq!(
        event_axis(&event(EventType::Level, 1, 1000)),
        Some(AxisCode::Structure)
    );
    assert_eq!(
        event_axis(&event(EventType::Divergence, 1, RSI_SOURCE_CODE)),
        Some(AxisCode::Structure)
    );

    let threshold = SignalSubtype::ThresholdCrossing.code();
    assert_eq!(
        event_axis(&event(EventType::Signal, threshold, RSI_SOURCE_CODE)),
        Some(AxisCode::Momentum)
    );
    assert_eq!(
        event_axis(&event(EventType::Signal, threshold, DIRECTION_SOURCE_CODE)),
        Some(AxisCode::Momentum),
        "the subtype owns the mapping for a threshold crossing"
    );

    let crossover = SignalSubtype::SeriesCrossover.code();
    assert_eq!(
        event_axis(&event(EventType::Signal, crossover, EMA20_SOURCE_CODE)),
        Some(AxisCode::Structure)
    );

    let compact = SignalSubtype::CompactStateChange.code();
    for (source, expected) in [
        (DIRECTION_SOURCE_CODE, AxisCode::Direction),
        (MOMENTUM_SOURCE_CODE, AxisCode::Momentum),
        (STRUCTURE_SOURCE_CODE, AxisCode::Structure),
    ] {
        assert_eq!(
            event_axis(&event(EventType::Signal, compact, source)),
            Some(expected)
        );
    }
    assert_eq!(
        event_axis(&event(EventType::Signal, compact, 99)),
        None,
        "an unknown compact-state source is not assigned to an axis"
    );
    assert_eq!(
        event_axis(&event(EventType::Signal, 99, DIRECTION_SOURCE_CODE)),
        None,
        "an unknown subtype does not fall back to its source"
    );
}

#[test]
fn activity_counts_by_availability_and_uses_the_recent_window() {
    let mut level = event(EventType::Level, 1, 1000);
    level.occurred_bar_index = 4;
    level.available_bar_index = 10;
    let events = [level];

    assert_eq!(
        activity_from_events(&events, 9).count,
        0,
        "an event is not counted before it is available"
    );
    assert_eq!(
        activity_from_events(&events, 10).count,
        1,
        "the event is counted on its availability bar, not its occurrence bar"
    );
    assert_eq!(
        activity_from_events(&events, 14).count,
        1,
        "the event remains in the five-bar window through bar 14"
    );
    assert_eq!(
        activity_from_events(&events, 15).count,
        0,
        "the event leaves the window after bar 14"
    );

    assert_eq!(activity_from_events(&events, 0).window, 1);
    assert_eq!(activity_from_events(&events, 3).window, 4);
    assert_eq!(activity_from_events(&events, 4).window, 5);
    assert_eq!(activity_from_events(&events, 100).window, 5);
}

#[test]
fn only_structure_mapped_events_contribute_to_activity() {
    let mut momentum = event(
        EventType::Signal,
        SignalSubtype::ThresholdCrossing.code(),
        RSI_SOURCE_CODE,
    );
    momentum.available_bar_index = 0;
    let mut direction = event(
        EventType::Signal,
        SignalSubtype::CompactStateChange.code(),
        DIRECTION_SOURCE_CODE,
    );
    direction.available_bar_index = 0;

    let activity = activity_from_events(&[momentum, direction], 0);
    assert_eq!(activity.count, 0, "non-Structure events are not counted");
    assert_eq!(activity.window, 1);
    assert_eq!(activity.per_bar, Some(0.0));
}

#[test]
fn a_zero_count_is_a_measured_zero() {
    let activity = activity_from_events(&[], 0);
    assert_eq!(activity.count, 0);
    assert_eq!(activity.window, 1);
    assert_eq!(activity.per_bar, Some(0.0));

    let later = activity_from_events(&[], 10);
    assert_eq!(later.count, 0);
    assert_eq!(later.window, 5);
    assert_eq!(later.per_bar, Some(0.0));
}

#[test]
fn activity_reports_the_exact_rate() {
    let mut first = event(EventType::Level, 1, 1000);
    first.available_bar_index = 2;
    let mut second = event(EventType::Divergence, 1, RSI_SOURCE_CODE);
    second.available_bar_index = 3;
    let events = [first, second];

    let activity = activity_from_events(&events, 3);
    assert_eq!(activity.count, 2);
    assert_eq!(activity.window, 4);
    assert_eq!(activity.per_bar, Some(0.5));
}

#[test]
fn level_references_are_ordered_missing_omitted_and_availability_aware() {
    let candles = fixture_candles();
    let timeframe = model(&candles);
    let latest = timeframe.len() - 1;
    let model_bar = timeframe.bar(latest).expect("the latest bar");
    let references = level_references(&timeframe, &model_bar);

    let kinds: Vec<u8> = references
        .iter()
        .map(|reference| reference.reference_kind_code)
        .collect();
    assert!(
        kinds.windows(2).all(|pair| pair[0] < pair[1]),
        "the fixed order is EMA20, EMA50, EMA200, minor high, minor low, major high, major low: {kinds:?}"
    );
    assert!(kinds.iter().all(|kind| (1..=7).contains(kind)));
    assert!(references.len() <= 7);
    assert!(
        references
            .iter()
            .all(|reference| reference.available_bar <= latest),
        "no reference may be knowable after the view bar"
    );

    for reference in &references {
        match reference.reference_kind_code {
            1 => assert_eq!(Some(reference.price), model_bar.readings.ema20),
            2 => assert_eq!(Some(reference.price), model_bar.readings.ema50),
            3 => assert_eq!(Some(reference.price), model_bar.readings.ema200),
            kind => {
                let (scale, pivot_kind) = match kind {
                    4 => (PivotScale::Minor, PivotKind::High),
                    5 => (PivotScale::Minor, PivotKind::Low),
                    6 => (PivotScale::Major, PivotKind::High),
                    7 => (PivotScale::Major, PivotKind::Low),
                    other => panic!("unexpected reference kind {other}"),
                };
                let expected = timeframe
                    .levels()
                    .iter()
                    .rev()
                    .find(|level| {
                        level.available_at <= latest
                            && level.scale == scale
                            && level.kind == pivot_kind
                    })
                    .expect("the reference came from a confirmed level");
                assert_eq!(reference.price, expected.price);
                assert_eq!(
                    reference.available_bar, expected.available_at,
                    "a swing reference keeps its own availability bar"
                );
            }
        }
    }

    for kind in 1..=3 {
        let present = references
            .iter()
            .any(|reference| reference.reference_kind_code == kind);
        let expected = match kind {
            1 => model_bar.readings.ema20,
            2 => model_bar.readings.ema50,
            3 => model_bar.readings.ema200,
            _ => unreachable!(),
        }
        .is_some_and(f64::is_finite);
        assert_eq!(present, expected, "EMA kind {kind} presence");
    }

    for (kind, scale, pivot_kind) in [
        (4, PivotScale::Minor, PivotKind::High),
        (5, PivotScale::Minor, PivotKind::Low),
        (6, PivotScale::Major, PivotKind::High),
        (7, PivotScale::Major, PivotKind::Low),
    ] {
        let present = references
            .iter()
            .any(|reference| reference.reference_kind_code == kind);
        let expected = timeframe.levels().iter().any(|level| {
            level.available_at <= latest && level.scale == scale && level.kind == pivot_kind
        });
        assert_eq!(present, expected, "swing kind {kind} presence");
    }

    assert!(
        references
            .iter()
            .any(|reference| reference.reference_kind_code >= 4),
        "the fixture must publish at least one confirmed swing by its last bar"
    );
}

#[test]
fn a_short_series_omits_every_missing_reference() {
    let candle = Candle::new(0, 0, 100.0, 100.5, 99.5, 100.0, Some(10.0)).expect("a valid candle");
    let timeframe = model(&[candle]);
    let model_bar = timeframe.bar(0).expect("the only bar");

    let references = level_references(&timeframe, &model_bar);
    assert!(
        references.is_empty(),
        "a missing reference is omitted, never filled with zero: {references:?}"
    );
}
