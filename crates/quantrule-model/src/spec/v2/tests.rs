use super::{
    ModelSpecV2, PredictionKind, SPEC_V2_SCHEMA, SpecEventCode, SpecEventRole, SpecPrediction,
    SpecRun, SpecV2Error, predict,
};
use crate::events::EventType;
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::panel::{PanelConfig, PanelInstrument, PanelReadings};
use crate::run::ModelRun;
use crate::spec::{SpecCondition, SpecEvent, SpecExpression};
use quantrule_data::Candle;

const BASE: i64 = 1_700_000_100;

fn candle(index: usize, timestamp: i64, open: f64, close: f64) -> Candle {
    Candle::new(
        index as u64,
        timestamp,
        open,
        open.max(close) + 0.4,
        open.min(close) - 0.4,
        close,
        Some(1_000.0 + index as f64),
    )
    .expect("the generated candle is valid")
}

fn candles(count: usize) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let phase = index as f64;
            let open = 100.0 + (phase / 7.0).sin() * 3.0 + 0.02 * phase;
            let close = 100.0 + ((phase + 1.0) / 7.0).sin() * 3.0 + 0.02 * (phase + 1.0);
            candle(index, BASE + index as i64 * 300, open, close)
        })
        .collect()
}

fn event(id: &str, role: SpecEventRole, subtype: u8, source: u16) -> SpecEventCode {
    let code = match id {
        "signal.rsi_threshold" => crate::events::codes::SIGNAL_RSI_THRESHOLD,
        "signal.ema_pair_cross" => crate::events::codes::SIGNAL_EMA_PAIR_CROSS,
        "signal.session_open" => crate::events::codes::SIGNAL_SESSION_OPEN,
        "signal.relative_strength_cross" => crate::events::codes::SIGNAL_RELATIVE_STRENGTH_CROSS,
        _ => 0,
    };
    SpecEventCode {
        id: id.to_string(),
        code,
        role,
        event_type: EventType::Signal,
        subtype,
        source,
    }
}

fn spec() -> ModelSpecV2 {
    let long = event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4);
    let short = event("signal.ema_pair_cross", SpecEventRole::Signal, 2, 1);
    ModelSpecV2 {
        schema: SPEC_V2_SCHEMA.to_string(),
        id: "M2.test".to_string(),
        family: "M2".to_string(),
        instrument: "XAU".to_string(),
        timeframes: vec![300],
        evaluation_timeframe_secs: 300,
        readings: MarketStateConfig::default(),
        active_events: vec![long.clone(), short.clone()],
        prediction: SpecPrediction {
            kind: PredictionKind::Vote,
            long: vec![long],
            short: vec![short],
            events: Vec::new(),
            min_votes: 1,
        },
        conditions: Vec::new(),
        session: None,
        panel_instruments: Vec::new(),
    }
}

fn run(candles: &[Candle]) -> ModelRun {
    run_with(candles, &MarketStateConfig::default())
}

fn run_with(candles: &[Candle], config: &MarketStateConfig) -> ModelRun {
    ModelRun::compute(
        &[TimeframeInput {
            timeframe_secs: 300,
            candles,
        }],
        config,
    )
}

#[test]
fn same_tuple_events_remain_distinct_by_event_code() {
    use crate::events::EventRecord;

    let candles = candles(40);
    let run = run(&candles);
    let timeframe = run.timeframe(300).expect("the timeframe is present");
    let event = |code| SpecEventCode {
        id: format!("event-{code}"),
        code,
        role: SpecEventRole::Signal,
        event_type: EventType::Signal,
        subtype: 10,
        source: 37,
    };
    let first = event(crate::events::codes::SIGNAL_CANDLE_MOMENTUM);
    let second = event(crate::events::codes::SIGNAL_CANDLE_MARUBOZU);
    let spec = ModelSpecV2 {
        schema: SPEC_V2_SCHEMA.to_string(),
        id: "M2.distinct".to_string(),
        family: "M2".to_string(),
        instrument: "XAU".to_string(),
        timeframes: vec![300],
        evaluation_timeframe_secs: 300,
        readings: MarketStateConfig::default(),
        active_events: vec![first.clone(), second.clone()],
        prediction: SpecPrediction {
            kind: PredictionKind::Direction,
            long: Vec::new(),
            short: Vec::new(),
            events: vec![first, second],
            min_votes: 2,
        },
        conditions: Vec::new(),
        session: None,
        panel_instruments: Vec::new(),
    };
    let record = |code| EventRecord {
        event_code: code,
        event_type: EventType::Signal,
        event_subtype_code: 10,
        source_code: 37,
        timeframe_secs: 300,
        occurred_bar_index: 0,
        available_bar_index: 0,
        direction_code: 1,
        reference_code: None,
        primary_value: Some(1.0),
        reference_value: None,
    };
    let prediction = predict(
        &spec,
        timeframe,
        &[
            record(spec.prediction.events[0].code),
            record(spec.prediction.events[1].code),
        ],
    );
    assert_eq!(prediction.long_votes[0], 2);
    assert_eq!(prediction.value(0), Some(1));
}

#[test]
fn a_level_condition_matches_every_registered_source() {
    let candles = candles(400);
    let run = run(&candles);
    let timeframe = run.timeframe(300).expect("the timeframe is present");
    let non_ema20 = timeframe
        .events()
        .iter()
        .find(|event| {
            event.event_code == crate::events::codes::LEVEL_TEST && event.source_code != 1
        })
        .copied()
        .expect("the synthetic series produced a non-EMA20 level test");
    let mut spec = spec();
    spec.conditions = vec![SpecCondition {
        id: "level_test".to_string(),
        expression: SpecExpression::Event(SpecEvent {
            event_code: crate::events::codes::LEVEL_TEST,
            event_type: EventType::Level,
            subtype: 1,
            source: 1,
            timeframe_secs: 300,
            direction: None,
            within_bars: 0,
        }),
    }];
    let result = SpecRun::compute(&spec, run, &[]).expect("the level spec run");
    let evaluation = result.conditions().expect("the condition evaluation");
    assert!(
        evaluation.conditions[0].values[non_ema20.available_bar_index],
        "the level condition missed source {}",
        non_ema20.source_code
    );
}

#[test]
fn a_v2_spec_predicts_and_rejects_a_prefix_change() {
    let candles = candles(400);
    let full = SpecRun::compute(&spec(), run(&candles), &[]).expect("the full spec run");
    assert_eq!(full.bars(), candles.len());
    assert_eq!(full.prediction().len(), candles.len());
    assert!(
        full.prediction()
            .values
            .iter()
            .all(|value| (-1..=1).contains(value))
    );
    assert!(
        full.prediction().values.iter().any(|value| *value != 0),
        "the synthetic series produced no vote"
    );

    let prefix = SpecRun::compute(&spec(), run(&candles[..300]), &[]).expect("the prefix run");
    assert_eq!(
        prefix.prediction().values,
        full.prediction().values[..300],
        "a past prediction changed when later bars were appended"
    );
}

#[test]
fn a_v2_spec_is_deterministic() {
    let candles = candles(400);
    let first = SpecRun::compute(&spec(), run(&candles), &[]).expect("the first run");
    let second = SpecRun::compute(&spec(), run(&candles), &[]).expect("the second run");
    assert_eq!(first, second);
}

#[test]
fn the_minimum_vote_suppresses_a_single_vote() {
    let candles = candles(400);
    let mut spec = spec();
    spec.prediction.min_votes = 2;
    let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the threshold run");
    for bar in 0..result.prediction().len() {
        let total = i32::from(result.prediction().long_votes[bar])
            - i32::from(result.prediction().short_votes[bar]);
        let expected = if total.unsigned_abs() as usize >= 2 {
            total.signum() as i8
        } else {
            0
        };
        assert_eq!(result.prediction().value(bar), Some(expected));
    }
}

#[test]
fn active_events_may_share_a_runtime_code() {
    let mut spec = spec();
    // The registry has several IDs per numeric code; the active set is
    // unique by ID and the prediction still counts the code once per bar.
    spec.active_events
        .push(event("signal.rsi_zone", SpecEventRole::Context, 1, 4));
    spec.validate()
        .expect("two active IDs may share one runtime code");
}

#[test]
fn a_reading_override_reaches_the_extended_readings() {
    let candles = candles(60);
    let mut spec = spec();
    let mut readings = spec.readings;
    let mut extended = readings.extended();
    extended.vwap_window = 5;
    readings.set_extended(extended);
    spec.readings = readings;
    let result =
        SpecRun::compute(&spec, run_with(&candles, &spec.readings), &[]).expect("the override run");
    let timeframe = result
        .base()
        .timeframe(300)
        .expect("the evaluation timeframe");
    assert!(
        timeframe.extended().vwap.get(4).is_some(),
        "the five-bar VWAP was not available at bar 4"
    );
    assert!(
        timeframe.extended().vwap.get(3).is_none(),
        "the VWAP was available before its window"
    );
}

#[test]
fn conditions_see_the_session_batch() {
    let candles = candles(400);
    let mut spec = spec();
    spec.session = Some(crate::sessions::SessionConfig::london());
    spec.active_events
        .push(event("signal.session_open", SpecEventRole::Context, 23, 50));
    spec.conditions = vec![SpecCondition {
        id: "session_open".to_string(),
        expression: SpecExpression::Event(SpecEvent {
            event_code: crate::events::codes::SIGNAL_SESSION_OPEN,
            event_type: EventType::Signal,
            subtype: 23,
            source: 50,
            timeframe_secs: 300,
            direction: Some(0),
            within_bars: 0,
        }),
    }];
    let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the session run");
    let evaluation = result.conditions().expect("the condition evaluation");
    assert_eq!(evaluation.conditions[0].values.len(), candles.len());
    assert!(
        evaluation.conditions[0].true_count > 0,
        "the synthetic series had no session open"
    );
}

#[test]
fn conditions_see_the_panel_batch() {
    let candles = candles(400);
    let mut other = Vec::with_capacity(candles.len());
    for (index, candle) in candles.iter().enumerate() {
        let close = 50.0 + (index as f64 * 0.11).sin() * 4.0;
        other.push(candle_with_close(index, candle.timestamp(), close));
    }
    let panel = [
        PanelInstrument {
            name: "XAU",
            candles: &candles,
        },
        PanelInstrument {
            name: "BTC",
            candles: &other,
        },
    ];
    let mut spec = spec();
    spec.panel_instruments = vec!["XAU".to_string(), "BTC".to_string()];
    spec.active_events.push(event(
        "signal.relative_strength_cross",
        SpecEventRole::Context,
        22,
        45,
    ));
    spec.conditions = vec![SpecCondition {
        id: "relative_strength".to_string(),
        expression: SpecExpression::Event(SpecEvent {
            event_code: crate::events::codes::SIGNAL_RELATIVE_STRENGTH_CROSS,
            event_type: EventType::Signal,
            subtype: 22,
            source: 45,
            timeframe_secs: 300,
            direction: None,
            within_bars: 0,
        }),
    }];
    let result = SpecRun::compute(&spec, run(&candles), &panel).expect("the panel run");
    let expected = crate::events::panel::PanelEventSeries::compute(
        &panel,
        &PanelReadings::compute(&panel, &PanelConfig::DEFAULT),
        300,
    )
    .events("XAU")
    .to_vec();
    assert_eq!(result.panel_events(), expected.as_slice());
    let evaluation = result.conditions().expect("the condition evaluation");
    assert_eq!(evaluation.conditions[0].values.len(), candles.len());
}

#[test]
fn a_direction_prediction_votes_from_event_directions() {
    let candles = candles(400);
    let mut spec = spec();
    spec.prediction = SpecPrediction {
        kind: PredictionKind::Direction,
        long: Vec::new(),
        short: Vec::new(),
        events: vec![
            event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4),
            event("signal.ema_pair_cross", SpecEventRole::Signal, 2, 1),
        ],
        min_votes: 1,
    };
    let result = SpecRun::compute(&spec, run(&candles), &[]).expect("the direction run");
    assert_eq!(result.prediction().len(), candles.len());
    assert!(
        result.prediction().values.iter().any(|value| *value != 0),
        "the synthetic series produced no direction vote"
    );
}

#[test]
fn a_prediction_kind_rejects_the_other_kind_fields() {
    let first = event("signal.rsi_threshold", SpecEventRole::Signal, 1, 4);
    let mut vote_with_events = spec();
    vote_with_events.prediction.events = vec![first.clone()];
    assert!(matches!(
        vote_with_events.validate(),
        Err(SpecV2Error::InvalidPredictionShape(_))
    ));
    let mut direction_with_sides = spec();
    direction_with_sides.prediction.kind = PredictionKind::Direction;
    direction_with_sides.prediction.events = vec![first];
    assert!(matches!(
        direction_with_sides.validate(),
        Err(SpecV2Error::InvalidPredictionShape(_))
    ));
}

fn candle_with_close(index: usize, timestamp: i64, close: f64) -> Candle {
    Candle::new(
        index as u64,
        timestamp,
        close,
        close + 0.5,
        close - 0.5,
        close,
        Some(1_000.0 + index as f64),
    )
    .expect("the generated candle is valid")
}

#[test]
fn extended_fixed_tolerances_are_range_checked() {
    use super::extended_is_valid;
    use crate::readings::ExtendedConfig;

    let mut config = ExtendedConfig::DEFAULT;
    assert!(extended_is_valid(config));
    config.fib_test_atr = -0.1;
    assert!(!extended_is_valid(config));
    config.fib_test_atr = 0.2;
    config.band_atr = 2.1;
    assert!(!extended_is_valid(config));
}
