//! Deterministic ordering for event records.
//!
//! The aggregate's latest event and the report's latest event and event order
//! share one key. The model's batch is already in availability order, but a
//! consumer sorts explicitly so a future detector or retention change cannot
//! make the published order depend on insertion order. The two optional numbers
//! use [`f64::total_cmp`], with `None` ordered before `Some`.

use std::cmp::Ordering;

use crate::events::EventRecord;

/// Orders two events by the shared fixed key.
pub(crate) fn event_cmp(left: &EventRecord, right: &EventRecord) -> Ordering {
    left.available_bar_index
        .cmp(&right.available_bar_index)
        .then_with(|| left.occurred_bar_index.cmp(&right.occurred_bar_index))
        .then_with(|| left.event_type.code().cmp(&right.event_type.code()))
        .then_with(|| left.event_subtype_code.cmp(&right.event_subtype_code))
        .then_with(|| left.source_code.cmp(&right.source_code))
        .then_with(|| left.reference_code.cmp(&right.reference_code))
        .then_with(|| optional_value_cmp(left.primary_value, right.primary_value))
        .then_with(|| left.direction_code.cmp(&right.direction_code))
        .then_with(|| optional_value_cmp(left.reference_value, right.reference_value))
}

/// Orders two optional numbers with `None` before `Some`.
fn optional_value_cmp(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => left.total_cmp(&right),
    }
}
