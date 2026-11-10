//! Shared helpers for the extended event detectors.

use super::*;

// A private constructor that mirrors the EventRecord fields one-for-one.
// Grouping them into synthetic structs would obscure the mapping, so the
// argument count is intentional.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push(
    events: &mut Vec<EventRecord>,
    event_code: u16,
    event_type: EventType,
    subtype: u8,
    source: u16,
    timeframe: u64,
    bar: usize,
    direction: i8,
    primary: Option<f64>,
    reference: Option<f64>,
) {
    push_with_reference(
        events, event_code, event_type, subtype, source, timeframe, bar, direction, None, primary,
        reference,
    );
}

// A private constructor that mirrors the EventRecord fields one-for-one.
// Grouping them into synthetic structs would obscure the mapping, so the
// argument count is intentional.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_with_reference(
    events: &mut Vec<EventRecord>,
    event_code: u16,
    event_type: EventType,
    subtype: u8,
    source: u16,
    timeframe: u64,
    bar: usize,
    direction: i8,
    reference_code: Option<u32>,
    primary: Option<f64>,
    reference: Option<f64>,
) {
    push_record(
        events,
        event_code,
        event_type,
        subtype,
        source,
        timeframe,
        bar,
        bar,
        direction,
        reference_code,
        primary,
        reference,
    );
}

/// Appends an event whose occurrence and availability bars differ.
///
/// Confirmed-pivot measurements use this constructor: the pivot extreme
/// occurred earlier, but the event is not knowable until the pivot confirms.
#[allow(clippy::too_many_arguments)]
pub(crate) fn push_confirmed(
    events: &mut Vec<EventRecord>,
    event_code: u16,
    event_type: EventType,
    subtype: u8,
    source: u16,
    timeframe: u64,
    occurred: usize,
    available: usize,
    direction: i8,
    primary: Option<f64>,
    reference: Option<f64>,
) {
    push_record(
        events, event_code, event_type, subtype, source, timeframe, occurred, available, direction,
        None, primary, reference,
    );
}

// The single EventRecord constructor. Keeping the field mapping in one place
// prevents the occurrence and availability bars from drifting apart again.
#[allow(clippy::too_many_arguments)]
fn push_record(
    events: &mut Vec<EventRecord>,
    event_code: u16,
    event_type: EventType,
    subtype: u8,
    source: u16,
    timeframe: u64,
    occurred: usize,
    available: usize,
    direction: i8,
    reference_code: Option<u32>,
    primary: Option<f64>,
    reference: Option<f64>,
) {
    events.push(EventRecord {
        event_code,
        event_type,
        event_subtype_code: subtype,
        source_code: source,
        timeframe_secs: timeframe,
        occurred_bar_index: occurred,
        available_bar_index: available,
        direction_code: direction,
        reference_code,
        primary_value: primary,
        reference_value: reference,
    });
}

pub(crate) fn cross_up_value(series: &ReadingSeries, bar: usize, level: f64) -> bool {
    if bar == 0 {
        return false;
    }
    match (series.get(bar), series.get(bar - 1)) {
        (Some(now), Some(previous)) => now > level && previous <= level,
        _ => false,
    }
}

pub(crate) fn cross_down_value(series: &ReadingSeries, bar: usize, level: f64) -> bool {
    if bar == 0 {
        return false;
    }
    match (series.get(bar), series.get(bar - 1)) {
        (Some(now), Some(previous)) => now < level && previous >= level,
        _ => false,
    }
}

pub(super) fn cross_up_series(a: &ReadingSeries, b: &ReadingSeries, bar: usize) -> bool {
    if bar == 0 {
        return false;
    }
    match (a.get(bar), b.get(bar), a.get(bar - 1), b.get(bar - 1)) {
        (Some(now), Some(level), Some(previous), Some(previous_level)) => {
            now > level && previous <= previous_level
        }
        _ => false,
    }
}

pub(super) fn cross_down_series(a: &ReadingSeries, b: &ReadingSeries, bar: usize) -> bool {
    if bar == 0 {
        return false;
    }
    match (a.get(bar), b.get(bar), a.get(bar - 1), b.get(bar - 1)) {
        (Some(now), Some(level), Some(previous), Some(previous_level)) => {
            now < level && previous >= previous_level
        }
        _ => false,
    }
}

pub(super) fn mfi_zone(value: f64) -> i8 {
    if value < 20.0 {
        -1
    } else if value > 80.0 {
        1
    } else {
        0
    }
}
