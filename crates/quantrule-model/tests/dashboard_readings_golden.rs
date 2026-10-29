//! Golden-fixture test for the dashboard dated readings.
//!
//! The expected file is an independent Python replay of the approved readings
//! contract over the checked-in Level, Divergence, Signal, and Progression
//! fixtures. It never runs the Rust code under test. The Progression rows
//! (axis_code 4) are produced by the checked-in oracle at
//! `tools/oracle/progression_replay.py`; the other axes remain from their
//! earlier independent replay.
//!
//! Each row is one retained reading: `rank` pins its position in the axis list,
//! `kind_code` selects an Event or a Transition, and an empty optional cell is
//! an absent value rather than a zero.

use std::collections::BTreeMap;

use quantrule_data::Candle;
use quantrule_model::dashboard::{DatedReading, TimeframeState, TransitionKind};
use quantrule_model::readings::roles::AxisCode;
use quantrule_model::run::TimeframeModel;
use quantrule_model::state::{MarketStateConfig, TimeframeInput};

const READINGS_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const DASHBOARD_FIXTURE: &str = include_str!("fixtures/dashboard_readings_golden.csv");

const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const DASHBOARD_HEADER: &str = "view_bar,axis_code,rank,kind_code,event_type_code,event_subtype_code,source_code,occurred_bar,available_bar,direction_code,reference_code,primary_value,reference_value,transition_kind_code,from_code,to_code";

const VIEW_BARS: [usize; 9] = [5, 20, 34, 37, 57, 58, 100, 200, 259];
const TOLERANCE: f64 = 1e-9;

/// One expected reading from the independent fixture.
#[derive(Debug, Clone, Copy)]
enum ExpectedReading {
    Event {
        event_type_code: u8,
        event_subtype_code: u8,
        source_code: u16,
        occurred_bar: usize,
        available_bar: usize,
        direction_code: i8,
        reference_code: Option<u32>,
        primary_value: Option<f64>,
        reference_value: Option<f64>,
    },
    Transition {
        kind: TransitionKind,
        from_code: Option<i8>,
        to_code: Option<i8>,
        occurred_bar: usize,
        available_bar: usize,
    },
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

fn integer<T>(field: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    field
        .parse::<T>()
        .unwrap_or_else(|error| panic!("invalid integer {field:?}: {error}"))
}

fn optional_integer<T>(field: &str) -> Option<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    if field.is_empty() {
        None
    } else {
        Some(integer(field))
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
                .expect("a valid fixture candle"),
        );
    }
    candles
}

fn expected_readings() -> BTreeMap<(usize, u8), Vec<ExpectedReading>> {
    let mut lines = DASHBOARD_FIXTURE
        .lines()
        .skip_while(|line| line.starts_with('#'));
    assert_eq!(lines.next(), Some(DASHBOARD_HEADER));

    let mut expected: BTreeMap<(usize, u8), Vec<ExpectedReading>> = BTreeMap::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 16);

        let view_bar = integer::<usize>(fields[0]);
        let axis_code = integer::<u8>(fields[1]);
        let rank = integer::<usize>(fields[2]);
        assert!(
            AxisCode::ALL.iter().any(|axis| axis.code() == axis_code),
            "a valid axis code"
        );

        let key = (view_bar, axis_code);
        let readings = expected.entry(key).or_default();
        assert_eq!(rank, readings.len(), "ranks are contiguous from zero");

        let reading = match integer::<u8>(fields[3]) {
            0 => {
                assert!(fields[13].is_empty(), "an event has no transition kind");
                assert!(fields[14].is_empty(), "an event has no from code");
                assert!(fields[15].is_empty(), "an event has no to code");
                ExpectedReading::Event {
                    event_type_code: integer(fields[4]),
                    event_subtype_code: integer(fields[5]),
                    source_code: integer(fields[6]),
                    occurred_bar: integer(fields[7]),
                    available_bar: integer(fields[8]),
                    direction_code: integer(fields[9]),
                    reference_code: optional_integer(fields[10]),
                    primary_value: number(fields[11]),
                    reference_value: number(fields[12]),
                }
            }
            1 => {
                assert!(fields[4].is_empty(), "a transition has no event type");
                assert!(fields[5].is_empty(), "a transition has no event subtype");
                assert!(fields[6].is_empty(), "a transition has no source");
                assert!(fields[9].is_empty(), "a transition has no direction");
                assert!(fields[10].is_empty(), "a transition has no reference code");
                assert!(fields[11].is_empty(), "a transition has no primary value");
                assert!(fields[12].is_empty(), "a transition has no reference value");
                let kind = match integer::<u8>(fields[13]) {
                    1 => TransitionKind::CompactState,
                    2 => TransitionKind::ProgressionStage,
                    code => panic!("unknown transition kind {code}"),
                };
                ExpectedReading::Transition {
                    kind,
                    from_code: optional_integer(fields[14]),
                    to_code: optional_integer(fields[15]),
                    occurred_bar: integer(fields[7]),
                    available_bar: integer(fields[8]),
                }
            }
            code => panic!("unknown reading kind {code}"),
        };
        readings.push(reading);
    }
    expected
}

fn assert_optional_value(actual: Option<f64>, expected: Option<f64>, context: &str) {
    match (actual, expected) {
        (Some(actual), Some(expected)) => assert!(
            (actual - expected).abs() < TOLERANCE,
            "{context}: expected {expected}, got {actual}"
        ),
        (None, None) => {}
        (actual, expected) => {
            panic!("{context}: value presence differs, actual {actual:?}, expected {expected:?}")
        }
    }
}

fn assert_reading(
    actual: &DatedReading,
    expected: &ExpectedReading,
    view_bar: usize,
    axis_code: u8,
    rank: usize,
) {
    let context = format!("view {view_bar}, axis {axis_code}, rank {rank}");
    match (actual, expected) {
        (
            DatedReading::Event(actual),
            ExpectedReading::Event {
                event_type_code,
                event_subtype_code,
                source_code,
                occurred_bar,
                available_bar,
                direction_code,
                reference_code,
                primary_value,
                reference_value,
            },
        ) => {
            assert_eq!(
                actual.event_type_code, *event_type_code,
                "{context}: event type"
            );
            assert_eq!(
                actual.event_subtype_code, *event_subtype_code,
                "{context}: event subtype"
            );
            assert_eq!(actual.source_code, *source_code, "{context}: source");
            assert_eq!(actual.occurred_bar, *occurred_bar, "{context}: occurred");
            assert_eq!(actual.available_bar, *available_bar, "{context}: available");
            assert_eq!(
                actual.direction_code, *direction_code,
                "{context}: direction"
            );
            assert_eq!(
                actual.reference_code, *reference_code,
                "{context}: reference code"
            );
            assert_optional_value(actual.primary_value, *primary_value, &context);
            assert_optional_value(actual.reference_value, *reference_value, &context);
        }
        (
            DatedReading::Transition(actual),
            ExpectedReading::Transition {
                kind,
                from_code,
                to_code,
                occurred_bar,
                available_bar,
            },
        ) => {
            assert_eq!(actual.axis.code(), axis_code, "{context}: axis");
            assert_eq!(actual.kind, *kind, "{context}: transition kind");
            assert_eq!(actual.from_code, *from_code, "{context}: from code");
            assert_eq!(actual.to_code, *to_code, "{context}: to code");
            assert_eq!(actual.occurred_bar, *occurred_bar, "{context}: occurred");
            assert_eq!(actual.available_bar, *available_bar, "{context}: available");
        }
        (actual, expected) => {
            panic!("{context}: reading kind differs, actual {actual:?}, expected {expected:?}")
        }
    }
}

#[test]
fn the_dashboard_readings_match_the_independent_golden() {
    let candles = fixture_candles();
    let timeframe = TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: 300,
            candles: &candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture is non-empty");
    let expected = expected_readings();

    for view_bar in VIEW_BARS {
        let state = TimeframeState::at(&timeframe, view_bar).expect("a view state");
        for axis_code in AxisCode::ALL {
            let axis = state
                .axes
                .iter()
                .find(|axis| axis.axis == axis_code)
                .expect("one state per axis");
            let empty = Vec::new();
            let expected_axis = expected
                .get(&(view_bar, axis_code.code()))
                .unwrap_or(&empty);

            assert_eq!(
                axis.readings.len(),
                expected_axis.len(),
                "view {view_bar}, axis {}: reading count",
                axis_code.code()
            );
            for (rank, (actual, expected)) in axis.readings.iter().zip(expected_axis).enumerate() {
                assert_reading(actual, expected, view_bar, axis_code.code(), rank);
            }
        }
    }
}

#[test]
fn the_golden_covers_the_readings_contract() {
    let expected = expected_readings();

    assert!(
        expected
            .keys()
            .all(|(_, axis_code)| *axis_code != AxisCode::Direction.code()),
        "the real fixture has no Direction compact event"
    );

    assert_eq!(
        expected
            .get(&(259, AxisCode::Momentum.code()))
            .map(Vec::len),
        Some(50),
        "the Momentum history is capped at 50"
    );
    assert_eq!(
        expected
            .get(&(259, AxisCode::Structure.code()))
            .map(Vec::len),
        Some(50),
        "the Structure history is capped at 50"
    );
    assert_eq!(
        expected
            .get(&(259, AxisCode::Progression.code()))
            .map(Vec::len),
        Some(50),
        "the Progression history is capped at 50"
    );
    assert_eq!(
        expected
            .get(&(57, AxisCode::Progression.code()))
            .map(Vec::len),
        Some(25),
        "the confirmed leg has a stage history by bar 57"
    );

    assert!(
        expected.values().flatten().any(|reading| matches!(
            reading,
            ExpectedReading::Event {
                available_bar,
                occurred_bar,
                ..
            } if available_bar > occurred_bar
        )),
        "the fixture exercises an event that became knowable after it occurred"
    );
    assert!(
        expected.values().flatten().any(|reading| matches!(
            reading,
            ExpectedReading::Transition {
                kind: TransitionKind::CompactState,
                ..
            }
        )),
        "the fixture exercises a compact-state transition"
    );
    assert!(
        expected.values().flatten().any(|reading| matches!(
            reading,
            ExpectedReading::Transition {
                kind: TransitionKind::ProgressionStage,
                ..
            }
        )),
        "the fixture exercises a progression-stage transition"
    );

    let before = expected
        .get(&(34, AxisCode::Structure.code()))
        .expect("Structure readings at view 34");
    assert!(
        !before.iter().any(|reading| matches!(
            reading,
            ExpectedReading::Event {
                event_type_code: 2,
                ..
            }
        )),
        "the divergence is not available at view 34"
    );
    let after = expected
        .get(&(37, AxisCode::Structure.code()))
        .expect("Structure readings at view 37");
    assert!(
        after.iter().any(|reading| matches!(
            reading,
            ExpectedReading::Event {
                event_type_code: 2,
                occurred_bar: 32,
                available_bar: 37,
                ..
            }
        )),
        "the divergence is available at view 37"
    );
}
