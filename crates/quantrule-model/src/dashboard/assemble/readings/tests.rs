use super::*;
use crate::axes::progression::ProgressionBar;
use crate::events::{EventRecord, EventType};
use crate::kernel::{Readiness, UnavailableReason};

fn record(
    event_type: EventType,
    subtype: u8,
    source: u16,
    occurred: usize,
    available: usize,
) -> EventRecord {
    EventRecord {
        event_type,
        event_subtype_code: subtype,
        source_code: source,
        timeframe_secs: 60,
        occurred_bar_index: occurred,
        available_bar_index: available,
        direction_code: 1,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }
}

fn with_values(
    mut event: EventRecord,
    primary: Option<f64>,
    reference: Option<f64>,
) -> EventRecord {
    event.primary_value = primary;
    event.reference_value = reference;
    event
}

fn stage(stage_code: Option<i8>) -> ProgressionBar {
    ProgressionBar {
        progress: None,
        stage_code,
        leg_direction: None,
        leg_scale_code: None,
        published_leg: None,
        minor_leg: None,
        major_leg: None,
        size_percentile: None,
        readiness: Readiness::Unavailable,
        unavailable_reason: Some(UnavailableReason::InsufficientBars),
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn list(readings: &[Vec<DatedReading>; 4], axis: AxisCode) -> &[DatedReading] {
    &readings[axis_index(axis)]
}

#[test]
fn every_event_type_maps_to_exactly_one_axis() {
    let events = [
        record(EventType::Level, 1, 1, 3, 3),
        record(EventType::Divergence, 1, 4, 3, 4),
        record(EventType::Signal, 1, 4, 3, 3),
        record(EventType::Signal, 2, 1, 3, 3),
        record(EventType::Signal, 3, DIRECTION_SOURCE_CODE, 3, 3),
        record(EventType::Signal, 3, STRUCTURE_SOURCE_CODE, 3, 3),
        record(EventType::Signal, 3, MOMENTUM_SOURCE_CODE, 3, 3),
    ];
    let readings = axis_readings(&events, &[], 10);

    assert_eq!(list(&readings, AxisCode::Direction).len(), 1);
    assert_eq!(list(&readings, AxisCode::Momentum).len(), 2);
    assert_eq!(list(&readings, AxisCode::Structure).len(), 4);
    assert_eq!(list(&readings, AxisCode::Progression).len(), 0);
    let total: usize = readings.iter().map(Vec::len).sum();
    assert_eq!(
        total,
        events.len(),
        "every event is assigned to exactly one axis"
    );
}

#[test]
fn compact_state_signal_becomes_a_transition_with_its_codes() {
    let event = with_values(
        record(EventType::Signal, 3, DIRECTION_SOURCE_CODE, 7, 7),
        Some(1.0),
        Some(-1.0),
    );
    let readings = axis_readings(&[event], &[], 7);
    let direction = list(&readings, AxisCode::Direction);
    assert_eq!(direction.len(), 1);
    let DatedReading::Transition(transition) = direction[0] else {
        panic!("a compact state change is a transition");
    };
    assert_eq!(transition.axis, AxisCode::Direction);
    assert_eq!(transition.kind, TransitionKind::CompactState);
    assert_eq!(transition.from_code, Some(-1));
    assert_eq!(transition.to_code, Some(1));
    assert_eq!(transition.occurred_bar, 7);
    assert_eq!(transition.available_bar, 7);
}

#[test]
fn a_non_integral_compact_code_is_typed_absence() {
    let event = with_values(
        record(EventType::Signal, 3, MOMENTUM_SOURCE_CODE, 2, 2),
        Some(0.5),
        Some(0.0),
    );
    let readings = axis_readings(&[event], &[], 2);
    let momentum = list(&readings, AxisCode::Momentum);
    let DatedReading::Transition(transition) = momentum[0] else {
        panic!("a compact state change is a transition");
    };
    assert_eq!(transition.from_code, Some(0));
    assert_eq!(
        transition.to_code, None,
        "a fractional code is absence, not a truncated code"
    );

    let out_of_range = with_values(
        record(EventType::Signal, 3, MOMENTUM_SOURCE_CODE, 3, 3),
        Some(300.0),
        Some(0.0),
    );
    let readings = axis_readings(&[out_of_range], &[], 3);
    let momentum = list(&readings, AxisCode::Momentum);
    let DatedReading::Transition(transition) = momentum[0] else {
        panic!("a compact state change is a transition");
    };
    assert_eq!(transition.to_code, None);
}

#[test]
fn progression_stage_changes_are_transitions() {
    let bars = [
        stage(None),
        stage(Some(0)),
        stage(Some(0)),
        stage(Some(1)),
        stage(Some(1)),
        stage(None),
        stage(Some(5)),
    ];
    let readings = axis_readings(&[], &bars, 6);
    let progression = list(&readings, AxisCode::Progression);
    assert_eq!(progression.len(), 4);

    let expected = [
        (1, None, Some(0)),
        (3, Some(0), Some(1)),
        (5, Some(1), None),
        (6, None, Some(5)),
    ];
    for (reading, (bar, from, to)) in progression.iter().zip(expected) {
        let DatedReading::Transition(transition) = reading else {
            panic!("a stage change is a transition");
        };
        assert_eq!(transition.axis, AxisCode::Progression);
        assert_eq!(transition.kind, TransitionKind::ProgressionStage);
        assert_eq!(transition.from_code, from);
        assert_eq!(transition.to_code, to);
        assert_eq!(transition.occurred_bar, bar);
        assert_eq!(transition.available_bar, bar);
    }
}

#[test]
fn the_first_bar_never_changes_stage() {
    let bars = [stage(Some(2))];
    let readings = axis_readings(&[], &bars, 0);
    assert!(list(&readings, AxisCode::Progression).is_empty());
}

#[test]
fn the_view_bar_limits_events_and_stages() {
    let events = [
        record(EventType::Level, 1, 1, 1, 1),
        record(EventType::Level, 1, 2, 2, 2),
    ];
    let bars = [stage(Some(0)), stage(Some(1)), stage(Some(2))];
    let readings = axis_readings(&events, &bars, 1);

    let structure = list(&readings, AxisCode::Structure);
    assert_eq!(structure.len(), 1);
    let DatedReading::Event(event) = structure[0] else {
        panic!("a level event stays an event");
    };
    assert_eq!(event.occurred_bar, 1);
    assert_eq!(list(&readings, AxisCode::Progression).len(), 1);
}

#[test]
fn the_history_keeps_the_last_fifty_readings() {
    let events: Vec<_> = (0..60)
        .map(|bar| record(EventType::Signal, 1, 4, bar, bar))
        .collect();
    let readings = axis_readings(&events, &[], 59);
    let momentum = list(&readings, AxisCode::Momentum);

    assert_eq!(momentum.len(), 50);
    assert_eq!(reading_occurred(&momentum[0]), 10);
    assert_eq!(reading_occurred(&momentum[49]), 59);
}

#[test]
fn the_cap_is_per_axis() {
    let mut events = Vec::new();
    for bar in 0..60 {
        events.push(record(EventType::Level, 1, 1, bar, bar));
        events.push(record(EventType::Signal, 1, 4, bar, bar));
    }
    let readings = axis_readings(&events, &[], 59);
    assert_eq!(list(&readings, AxisCode::Structure).len(), 50);
    assert_eq!(list(&readings, AxisCode::Momentum).len(), 50);
}

#[test]
fn events_precede_a_transition_on_the_same_bar() {
    let events = [
        record(EventType::Signal, 1, 4, 2, 2),
        with_values(
            record(EventType::Signal, 3, MOMENTUM_SOURCE_CODE, 2, 2),
            Some(1.0),
            Some(0.0),
        ),
    ];
    let readings = axis_readings(&events, &[], 2);
    let momentum = list(&readings, AxisCode::Momentum);

    assert_eq!(momentum.len(), 2);
    assert!(matches!(momentum[0], DatedReading::Event(_)));
    assert!(matches!(momentum[1], DatedReading::Transition(_)));
}

#[test]
fn the_event_order_follows_the_contract_key() {
    let events = [
        with_values(record(EventType::Signal, 2, 1, 4, 4), Some(0.9), None),
        with_values(record(EventType::Level, 1, 1, 4, 4), Some(0.9), None),
        with_values(record(EventType::Level, 1, 1, 4, 4), Some(0.1), None),
        record(EventType::Level, 1, 1, 4, 4),
    ];
    let readings = axis_readings(&events, &[], 4);
    let structure = list(&readings, AxisCode::Structure);

    assert_eq!(structure.len(), 4);
    let expected = [(1, None), (1, Some(0.1)), (1, Some(0.9)), (3, Some(0.9))];
    for (reading, (event_type, primary)) in structure.iter().zip(expected) {
        let DatedReading::Event(event) = reading else {
            panic!("these are event readings");
        };
        assert_eq!(event.event_type_code, event_type);
        assert_eq!(event.primary_value, primary);
    }
}

#[test]
fn the_replay_is_causal() {
    let events = [record(EventType::Level, 1, 1, 1, 1)];
    let bars = [stage(Some(0)), stage(Some(1))];
    let before = axis_readings(&events, &bars, 1);

    let mut later_events = events.to_vec();
    later_events.push(record(EventType::Level, 1, 2, 5, 5));
    let mut later_bars = bars.to_vec();
    later_bars.extend([stage(Some(2)), stage(Some(3)), stage(Some(4))]);
    let after = axis_readings(&later_events, &later_bars, 1);

    assert_eq!(before, after, "a later bar cannot change an earlier view");
}

#[test]
fn the_replay_is_deterministic_across_event_order() {
    let events = vec![
        record(EventType::Level, 1, 1, 1, 1),
        record(EventType::Level, 2, 2, 2, 2),
        record(EventType::Divergence, 1, 4, 3, 4),
        record(EventType::Signal, 1, 4, 4, 4),
        record(EventType::Signal, 2, 1, 5, 5),
        with_values(
            record(EventType::Signal, 3, STRUCTURE_SOURCE_CODE, 6, 6),
            Some(1.0),
            Some(0.0),
        ),
    ];
    let mut reversed = events.clone();
    reversed.reverse();

    assert_eq!(
        axis_readings(&events, &[], 10),
        axis_readings(&reversed, &[], 10)
    );
}

#[test]
fn an_empty_series_is_empty() {
    let readings = axis_readings(&[], &[], 0);
    assert!(readings.iter().all(Vec::is_empty));
}

#[test]
fn a_zero_value_is_kept() {
    let event = with_values(record(EventType::Signal, 1, 4, 1, 1), Some(0.0), Some(0.0));
    let readings = axis_readings(&[event], &[], 1);
    let momentum = list(&readings, AxisCode::Momentum);
    let DatedReading::Event(reading) = momentum[0] else {
        panic!("a threshold crossing stays an event");
    };
    assert_eq!(reading.primary_value, Some(0.0));
    assert_eq!(reading.reference_value, Some(0.0));
}

#[test]
fn an_unknown_signal_source_is_ignored() {
    let event = record(EventType::Signal, 3, 99, 1, 1);
    let readings = axis_readings(&[event], &[], 1);
    assert!(readings.iter().all(Vec::is_empty));
}
