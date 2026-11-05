//! Event validation and precedence over the checked-in 260-bar fixture.
//!
//! The suite proves the invariants the model promises for every event batch:
//! an event never becomes available before it occurred, a batch is in
//! availability order, one underlying occurrence is recorded once, the level
//! reference order is fixed, and appending bars never changes a past record.
//!
//! The extended and session fixtures are regression locks generated from the
//! implementation, not independent oracles. The v4 level, divergence, and
//! signal fixtures keep their independent oracles.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_data::Candle;
use quantrule_model::axes::structure::registry::ConfirmedLevelRegistry;
use quantrule_model::events::EventRecord;
use quantrule_model::events::EventType;
use quantrule_model::events::panel::PanelEventSeries;
use quantrule_model::events::session::SessionEventSeries;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::panel::{PanelConfig, PanelInstrument, PanelReadings};
use quantrule_model::readings::{PivotKind, PivotScale, Readings, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionReadings};
use quantrule_model::timeframe::TimeframeModel;

const CANDLES_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const EXTENDED_FIXTURE: &str = include_str!("fixtures/extended_events_golden.csv");
const SESSION_FIXTURE: &str = include_str!("fixtures/session_events_golden.csv");
const TIMEFRAME_SECS: u64 = 300;
const CUTOFFS: [usize; 3] = [61, 131, 201];

const EVENTS_HEADER: &str = "bar,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExpectedEvent {
    bar: usize,
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    timeframe_secs: u64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
    reference_code: Option<u32>,
    primary_value: Option<f64>,
    reference_value: Option<f64>,
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
    let mut lines = CANDLES_FIXTURE.lines();
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
        candles.push(
            Candle::new(
                series_index,
                timestamp,
                number(fields[2]).expect("the open is present"),
                number(fields[3]).expect("the high is present"),
                number(fields[4]).expect("the low is present"),
                number(fields[5]).expect("the close is present"),
                number(fields[6]),
            )
            .expect("the fixture candle is valid"),
        );
    }
    candles
}

fn load_events(fixture: &str) -> Vec<ExpectedEvent> {
    let mut lines = fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    assert_eq!(
        lines.next(),
        Some(EVENTS_HEADER),
        "the event fixture header changed"
    );
    let mut events = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 11, "wrong column count in {line:?}");
        events.push(ExpectedEvent {
            bar: fields[0].parse().expect("the row index is a usize"),
            event_type_code: fields[1].parse().expect("the event type is a u8"),
            event_subtype_code: fields[2].parse().expect("the subtype is a u8"),
            source_code: fields[3].parse().expect("the source is a u16"),
            timeframe_secs: fields[4].parse().expect("the timeframe is a u64"),
            occurred_bar_index: fields[5].parse().expect("the occurrence is a usize"),
            available_bar_index: fields[6].parse().expect("the availability is a usize"),
            direction_code: fields[7].parse().expect("the direction is an i8"),
            reference_code: if fields[8].is_empty() {
                None
            } else {
                Some(fields[8].parse().expect("the reference is a u32"))
            },
            primary_value: number(fields[9]),
            reference_value: number(fields[10]),
        });
    }
    events
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: TIMEFRAME_SECS,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture produces a model")
}

fn session_events(candles: &[Candle]) -> Vec<EventRecord> {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let sessions = SessionReadings::compute(candles, &readings, &SessionConfig::london());
    SessionEventSeries::compute(
        candles,
        &readings,
        &sessions,
        &SessionConfig::london(),
        TIMEFRAME_SECS,
    )
    .events()
    .to_vec()
}

fn assert_batch(events: &[EventRecord], bars: usize) {
    let mut previous: Option<(usize, usize)> = None;
    for event in events {
        assert!(
            event.occurred_bar_index < bars,
            "an event occurred on bar {} but the series has {bars} bars",
            event.occurred_bar_index
        );
        assert!(
            event.available_bar_index >= event.occurred_bar_index,
            "an event became available before it occurred: {event:?}"
        );
        assert!(
            event.available_bar_index < bars,
            "an event became available on bar {} but the series has {bars} bars",
            event.available_bar_index
        );
        assert_eq!(event.timeframe_secs, TIMEFRAME_SECS);
        assert!(
            (-1..=1).contains(&event.direction_code),
            "the direction code is outside -1..=1: {event:?}"
        );
        if let Some(value) = event.primary_value {
            assert!(
                value.is_finite(),
                "the primary value is not finite: {event:?}"
            );
        }
        if let Some(value) = event.reference_value {
            assert!(
                value.is_finite(),
                "the reference value is not finite: {event:?}"
            );
        }
        let key = (event.available_bar_index, event.occurred_bar_index);
        if let Some(previous) = previous {
            assert!(
                previous <= key,
                "the batch is not in availability order at {event:?}"
            );
        }
        previous = Some(key);
    }
}

fn optional_bits(value: Option<f64>) -> u64 {
    value.map_or(u64::MAX, f64::to_bits)
}

type RecordKey = (u8, u8, u16, u64, usize, usize, i8, Option<u32>, u64, u64);

fn record_key(event: &EventRecord) -> RecordKey {
    (
        event.event_type.code(),
        event.event_subtype_code,
        event.source_code,
        event.timeframe_secs,
        event.occurred_bar_index,
        event.available_bar_index,
        event.direction_code,
        event.reference_code,
        optional_bits(event.primary_value),
        optional_bits(event.reference_value),
    )
}

fn assert_no_identical_records(events: &[EventRecord]) {
    let mut seen = BTreeSet::new();
    for event in events {
        assert!(
            seen.insert(record_key(event)),
            "the same record was published twice: {event:?}"
        );
    }
}

type ExtendedKey = (u64, u8, u8, u16, usize, Option<u32>, Option<u64>);

fn assert_extended_occurrences_are_unique(events: &[EventRecord]) {
    let mut seen = BTreeSet::new();
    for event in events {
        let threshold = if event.event_type == EventType::Signal && event.event_subtype_code == 1 {
            event.reference_value.map(f64::to_bits)
        } else {
            None
        };
        let key: ExtendedKey = (
            event.timeframe_secs,
            event.event_type.code(),
            event.event_subtype_code,
            event.source_code,
            event.occurred_bar_index,
            event.reference_code,
            threshold,
        );
        assert!(
            seen.insert(key),
            "one occurrence was published twice: {event:?}"
        );
    }
}

fn level_rank(registry: &ConfirmedLevelRegistry, code: u32) -> u8 {
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
            let scale = match level.scale {
                PivotScale::Minor => 0,
                PivotScale::Major => 1,
            };
            let kind = match level.kind {
                PivotKind::High => 0,
                PivotKind::Low => 1,
            };
            3 + scale * 2 + kind
        }
    }
}

fn assert_level_precedence(candles: &[Candle], events: &[EventRecord]) {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
    let mut per_bar: BTreeMap<(usize, usize), Vec<&EventRecord>> = BTreeMap::new();
    for event in events {
        if event.event_type == EventType::Level {
            per_bar
                .entry((event.available_bar_index, event.occurred_bar_index))
                .or_default()
                .push(event);
        }
    }
    for ((available, occurred), group) in per_bar {
        let mut references = BTreeSet::new();
        let mut ranks = Vec::new();
        for event in &group {
            let code = event
                .reference_code
                .expect("a level event carries a reference code");
            assert!(
                references.insert(code),
                "the same reference produced two level events on bar {occurred} available at {available}"
            );
            ranks.push(level_rank(&registry, code));
        }
        assert!(
            ranks.windows(2).all(|pair| pair[0] <= pair[1]),
            "the level reference order changed on bar {occurred}: {ranks:?}"
        );
    }
}

fn assert_golden(actual: &[EventRecord], expected: &[ExpectedEvent]) {
    assert_eq!(actual.len(), expected.len(), "the event count changed");
    for (index, (event, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(expected.bar, index, "the fixture row index changed");
        assert_eq!(event.event_type.code(), expected.event_type_code);
        assert_eq!(event.event_subtype_code, expected.event_subtype_code);
        assert_eq!(event.source_code, expected.source_code);
        assert_eq!(event.timeframe_secs, expected.timeframe_secs);
        assert_eq!(event.occurred_bar_index, expected.occurred_bar_index);
        assert_eq!(event.available_bar_index, expected.available_bar_index);
        assert_eq!(event.direction_code, expected.direction_code);
        assert_eq!(event.reference_code, expected.reference_code);
        assert_eq!(event.primary_value, expected.primary_value);
        assert_eq!(event.reference_value, expected.reference_value);
    }
}

#[test]
fn every_batch_respects_availability_and_order() {
    let candles = load_candles();
    let model = model(&candles);
    let extended = model.extended_events().events();
    let session = session_events(&candles);

    assert_batch(model.events(), candles.len());
    assert_batch(extended, candles.len());
    assert_batch(&session, candles.len());

    assert!(!model.events().is_empty());
    assert!(!extended.is_empty());
    assert!(!session.is_empty());
}

#[test]
fn one_occurrence_is_recorded_once() {
    let candles = load_candles();
    let model = model(&candles);
    let extended = model.extended_events().events();
    let session = session_events(&candles);

    assert_no_identical_records(model.events());
    assert_no_identical_records(extended);
    assert_no_identical_records(&session);
    assert_extended_occurrences_are_unique(extended);

    let v4: BTreeSet<RecordKey> = model.events().iter().map(record_key).collect();
    let additive: BTreeSet<RecordKey> = extended.iter().map(record_key).collect();
    assert!(
        v4.is_disjoint(&additive),
        "the extended batch repeated a v4 record"
    );
}

#[test]
fn level_events_follow_the_reference_order() {
    let candles = load_candles();
    let model = model(&candles);
    assert_level_precedence(&candles, model.events());
}

#[test]
fn extended_and_session_events_match_their_fixtures() {
    let candles = load_candles();
    let model = model(&candles);
    assert_golden(
        model.extended_events().events(),
        &load_events(EXTENDED_FIXTURE),
    );
    assert_golden(&session_events(&candles), &load_events(SESSION_FIXTURE));
}

#[test]
fn appending_bars_does_not_change_past_events() {
    let candles = load_candles();
    let full = model(&candles);
    let full_extended = full.extended_events().events().to_vec();
    let full_session = session_events(&candles);

    for cutoff in CUTOFFS {
        let prefix = model(&candles[..cutoff]);
        let prefix_extended = prefix.extended_events().events();
        let prefix_session = session_events(&candles[..cutoff]);

        let expected_v4: Vec<EventRecord> = full
            .events()
            .iter()
            .filter(|event| event.available_bar_index < cutoff)
            .copied()
            .collect();
        assert_eq!(
            prefix.events(),
            expected_v4,
            "the v4 batch changed at {cutoff}"
        );

        let expected_extended: Vec<EventRecord> = full_extended
            .iter()
            .filter(|event| event.available_bar_index < cutoff)
            .copied()
            .collect();
        assert_eq!(
            prefix_extended, expected_extended,
            "the extended batch changed at {cutoff}"
        );

        let expected_session: Vec<EventRecord> = full_session
            .iter()
            .filter(|event| event.available_bar_index < cutoff)
            .copied()
            .collect();
        assert_eq!(
            prefix_session, expected_session,
            "the session batch changed at {cutoff}"
        );
    }
}

#[test]
fn repeated_model_runs_publish_the_same_events() {
    let candles = load_candles();
    let first = model(&candles);
    let second = model(&candles);
    assert_eq!(first.events(), second.events());
    assert_eq!(
        first.extended_events().events(),
        second.extended_events().events()
    );
    assert_eq!(session_events(&candles), session_events(&candles));
}

fn panel_candle(index: usize, close: f64) -> Candle {
    Candle::new(
        index as u64,
        1_700_000_000 + index as i64 * 3_600,
        close,
        close + 0.5,
        close - 0.5,
        close,
        Some(10.0),
    )
    .expect("the generated panel candle is valid")
}

#[test]
fn panel_events_respect_availability_and_one_occurrence() {
    let btc: Vec<Candle> = (0..80)
        .map(|index| panel_candle(index, 100.0 + (index as f64 * 0.2).sin() * 5.0))
        .collect();
    let eth: Vec<Candle> = (0..80)
        .map(|index| panel_candle(index, 100.0 + (index as f64 * 0.25).sin() * 6.0))
        .collect();
    let xau: Vec<Candle> = (0..80)
        .map(|index| panel_candle(index, 100.0 + (index as f64 * 0.15).sin() * 3.0))
        .collect();
    let panel = [
        PanelInstrument {
            name: "BTC",
            candles: &btc,
        },
        PanelInstrument {
            name: "ETH",
            candles: &eth,
        },
        PanelInstrument {
            name: "XAU",
            candles: &xau,
        },
    ];
    let readings = PanelReadings::compute(&panel, &PanelConfig::DEFAULT);
    let events = PanelEventSeries::compute(&panel, &readings, 3_600);
    let mut total = 0;
    for instrument in events.instruments() {
        let series = events.events(instrument);
        total += series.len();
        for event in series {
            assert!(event.occurred_bar_index < btc.len());
            assert!(event.available_bar_index >= event.occurred_bar_index);
            assert_eq!(event.timeframe_secs, 3_600);
        }
        let mut seen = BTreeSet::new();
        for event in series {
            assert!(
                seen.insert((
                    event.event_type.code(),
                    event.event_subtype_code,
                    event.source_code,
                    event.occurred_bar_index,
                )),
                "the panel published one occurrence twice: {event:?}"
            );
        }
        assert!(
            series
                .windows(2)
                .all(|pair| pair[0].available_bar_index <= pair[1].available_bar_index)
        );
    }
    assert!(total > 0, "the panel fixture produced no event");
}

fn format_events(events: &[EventRecord], label: &str) -> String {
    let mut text = format!(
        "# {label} regression fixture for the 260-bar readings fixture.\n\
         # Generated from the implementation; it is a regression lock, not an independent oracle.\n\
         # Regenerate with `cargo test -p quantrule-model --test event_validation -- --ignored regenerate_event_fixtures`.\n\
         {EVENTS_HEADER}\n"
    );
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
            "{index},{},{},{},{},{},{},{},{reference},{primary},{reference_value}\n",
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
#[ignore = "regenerates the extended and session event fixtures deliberately"]
fn regenerate_event_fixtures() {
    let candles = load_candles();
    let model = model(&candles);
    let base = env!("CARGO_MANIFEST_DIR");
    std::fs::write(
        format!("{base}/tests/fixtures/extended_events_golden.csv"),
        format_events(model.extended_events().events(), "Extended event"),
    )
    .expect("the extended fixture is writable");
    std::fs::write(
        format!("{base}/tests/fixtures/session_events_golden.csv"),
        format_events(&session_events(&candles), "Session event (London calendar)"),
    )
    .expect("the session fixture is writable");
}
