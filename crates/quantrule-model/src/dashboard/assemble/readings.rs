//! View-only dated readings for each axis.
//!
//! The model publishes an event batch and a per-bar Progression stage series.
//! This module assigns each recognized event to exactly one axis, turns a
//! compact-state Signal event into a transition, synthesizes Progression stage
//! transitions, filters by availability at the view bar, orders the result, and
//! keeps the last 50 readings per axis. An unmapped model code is left out
//! rather than assigned to a guessed axis. Current Level reference prices stay
//! in Structure detail; they are not repeated in the dated history.
//!
//! The module is pure: it reads only the values it is given and performs no
//! I/O, no clock read, and no randomness.

use std::cmp::Ordering;

use crate::axes::progression::ProgressionBar;
use crate::events::EventRecord;
use crate::events::EventType;
use crate::events::signal::{
    DIRECTION_SOURCE_CODE, MOMENTUM_SOURCE_CODE, STRUCTURE_SOURCE_CODE, SignalSubtype,
};
use crate::readings::roles::AxisCode;

use crate::dashboard::{DatedReading, EventReading, TransitionKind, TransitionReading};

/// The contract's maximum retained readings per axis.
const READING_HISTORY_LIMIT: usize = 50;

/// The number of axes, in [`AxisCode::ALL`] order.
const AXIS_COUNT: usize = 4;

/// Projects every axis's dated reading history for one view bar.
///
/// `events` is the timeframe's complete event batch; only events that were
/// knowable at `view_bar` are included. `progression` is the per-bar published
/// stage series; a stage change is dated at the bar it happened and is
/// available at that same closed bar. The first bar has no prior stage, so it
/// cannot be a change.
///
/// The result is indexed in [`AxisCode::ALL`] order, and each list is oldest
/// first, at most [`READING_HISTORY_LIMIT`] entries.
pub(super) fn axis_readings(
    events: &[EventRecord],
    progression: &[ProgressionBar],
    view_bar: usize,
) -> [Vec<DatedReading>; AXIS_COUNT] {
    let mut lists: [Vec<DatedReading>; AXIS_COUNT] = std::array::from_fn(|_| Vec::new());

    for event in events
        .iter()
        .filter(|event| event.available_bar_index <= view_bar)
    {
        // An unmapped code is a model defect, not a dashboard measurement. It
        // is left out rather than assigned to a guessed axis.
        let Some(axis) = event_axis(event) else {
            continue;
        };
        let reading = if is_compact_state_change(event) {
            compact_state_transition(event, axis)
        } else {
            event_reading(event)
        };
        lists[axis_index(axis)].push(reading);
    }

    let stage_end = view_bar.saturating_add(1).min(progression.len());
    let mut previous: Option<Option<i8>> = None;
    for (bar, record) in progression.iter().take(stage_end).enumerate() {
        let stage = record.stage_code;
        if previous.is_some_and(|prior| prior != stage) {
            lists[axis_index(AxisCode::Progression)].push(DatedReading::Transition(
                TransitionReading {
                    axis: AxisCode::Progression,
                    kind: TransitionKind::ProgressionStage,
                    from_code: previous.flatten(),
                    to_code: stage,
                    occurred_bar: bar,
                    available_bar: bar,
                },
            ));
        }
        previous = Some(stage);
    }

    for list in &mut lists {
        sort_readings(list);
        if list.len() > READING_HISTORY_LIMIT {
            let excess = list.len() - READING_HISTORY_LIMIT;
            list.drain(..excess);
        }
    }

    lists
}

/// The axis a published event is displayed under.
///
/// The contract assigns Level and Divergence to Structure, a threshold
/// crossing to Momentum, an EMA crossover to Structure, and a compact state
/// change to the axis named by its source code. Progression is not an event
/// axis; its transitions are synthesized from the stage series.
fn event_axis(event: &EventRecord) -> Option<AxisCode> {
    match event.event_type {
        EventType::Level | EventType::Divergence => Some(AxisCode::Structure),
        EventType::Signal => match SignalSubtype::from_code(event.event_subtype_code)? {
            SignalSubtype::ThresholdCrossing => Some(AxisCode::Momentum),
            SignalSubtype::SeriesCrossover => Some(AxisCode::Structure),
            SignalSubtype::CompactStateChange => match event.source_code {
                DIRECTION_SOURCE_CODE => Some(AxisCode::Direction),
                MOMENTUM_SOURCE_CODE => Some(AxisCode::Momentum),
                STRUCTURE_SOURCE_CODE => Some(AxisCode::Structure),
                _ => None,
            },
        },
    }
}

/// Whether a Signal event records a compact state change.
fn is_compact_state_change(event: &EventRecord) -> bool {
    event.event_type == EventType::Signal
        && SignalSubtype::from_code(event.event_subtype_code)
            == Some(SignalSubtype::CompactStateChange)
}

/// Copies an event record into the dashboard's dated event reading.
fn event_reading(event: &EventRecord) -> DatedReading {
    DatedReading::Event(EventReading {
        event_type_code: event.event_type.code(),
        event_subtype_code: event.event_subtype_code,
        source_code: event.source_code,
        occurred_bar: event.occurred_bar_index,
        available_bar: event.available_bar_index,
        direction_code: event.direction_code,
        reference_code: event.reference_code,
        primary_value: event.primary_value,
        reference_value: event.reference_value,
    })
}

/// Turns a compact-state Signal event into a dated transition.
///
/// The event carries the prior and current compact codes as measured values,
/// so the transition keeps them as typed codes instead of repeating the event
/// twice in the same axis history.
fn compact_state_transition(event: &EventRecord, axis: AxisCode) -> DatedReading {
    DatedReading::Transition(TransitionReading {
        axis,
        kind: TransitionKind::CompactState,
        from_code: code_from_value(event.reference_value),
        to_code: code_from_value(event.primary_value),
        occurred_bar: event.occurred_bar_index,
        available_bar: event.available_bar_index,
    })
}

/// Reads a compact code from an event's measured value.
///
/// The model emits whole codes. A non-finite, fractional, or out-of-range
/// value becomes a typed absence instead of a truncated code.
fn code_from_value(value: Option<f64>) -> Option<i8> {
    let value = value?;
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    if !(f64::from(i8::MIN)..=f64::from(i8::MAX)).contains(&value) {
        return None;
    }
    Some(value as i8)
}

/// The array position of an axis in [`axis_readings`].
const fn axis_index(axis: AxisCode) -> usize {
    match axis {
        AxisCode::Direction => 0,
        AxisCode::Momentum => 1,
        AxisCode::Structure => 2,
        AxisCode::Progression => 3,
    }
}

/// Orders one axis's readings by availability, occurrence, and the contract's
/// fixed event key.
///
/// Events precede transitions on the same bar, so the latest reading of a bar
/// is its state change when both exist. The event tie-breakers beyond the
/// contract key keep the order independent of the batch order.
fn sort_readings(list: &mut [DatedReading]) {
    list.sort_by(|a, b| {
        reading_available(a)
            .cmp(&reading_available(b))
            .then_with(|| reading_occurred(a).cmp(&reading_occurred(b)))
            .then_with(|| reading_kind_rank(a).cmp(&reading_kind_rank(b)))
            .then_with(|| match (a, b) {
                (DatedReading::Event(a), DatedReading::Event(b)) => compare_event_key(a, b),
                _ => Ordering::Equal,
            })
    });
}

/// The contract's fixed event key, with direction and reference value as the
/// final tie-breakers.
fn compare_event_key(a: &EventReading, b: &EventReading) -> Ordering {
    (
        a.event_type_code,
        a.event_subtype_code,
        a.source_code,
        a.reference_code,
    )
        .cmp(&(
            b.event_type_code,
            b.event_subtype_code,
            b.source_code,
            b.reference_code,
        ))
        .then_with(|| compare_optional_value(a.primary_value, b.primary_value))
        .then_with(|| a.direction_code.cmp(&b.direction_code))
        .then_with(|| compare_optional_value(a.reference_value, b.reference_value))
}

/// Orders optional measured values with absence before presence and `total_cmp`
/// for the present values, so `NaN` cannot make the order unstable.
fn compare_optional_value(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
    }
}

/// When a reading became knowable.
fn reading_available(reading: &DatedReading) -> usize {
    match reading {
        DatedReading::Event(event) => event.available_bar,
        DatedReading::Transition(transition) => transition.available_bar,
    }
}

/// The bar a reading describes.
fn reading_occurred(reading: &DatedReading) -> usize {
    match reading {
        DatedReading::Event(event) => event.occurred_bar,
        DatedReading::Transition(transition) => transition.occurred_bar,
    }
}

/// Events sort before transitions at the same availability and occurrence.
const fn reading_kind_rank(reading: &DatedReading) -> u8 {
    match reading {
        DatedReading::Event(_) => 0,
        DatedReading::Transition(_) => 1,
    }
}

#[cfg(test)]
mod tests;
