//! The pivot, confirmed-level, event, and leg records.
//!
//! The layer build already computes the model's [`PivotSeries`],
//! [`ConfirmedLevelRegistry`], event series, and progression series. This module
//! turns them into the flat rows the store writes, without recomputing any of
//! them. A pivot occurrence is one row: a candidate has `confirmed_code = 0`
//! and is available on its occurrence bar; a pivot that later confirms has
//! `confirmed_code = 1` and is available on the confirmation bar.
//!
//! Rows are deterministic. Pivots are ordered by timeframe, occurrence bar,
//! scale, and kind; levels are ordered by timeframe and reference code; events
//! are ordered by timeframe, availability bar, type, subtype, source, occurrence
//! bar, reference, and reference value; legs are ordered by timeframe and
//! `leg_id`. The reference value is the final tie-break because coincident
//! Signal threshold crossings share every other column. The builders never read
//! the clock or an unordered collection.

use std::cmp::Ordering;

use quantrule_model::axes::progression::ProgressionLeg;
use quantrule_model::axes::structure::registry::ConfirmedLevel;
use quantrule_model::events::EventRecord;
use quantrule_model::readings::{Pivot, PivotKind, PivotScale, PivotSeries};

/// One stored pivot occurrence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The scale's registry code: 1 minor, 2 major.
    pub scale_code: u8,
    /// The kind's registry code: 1 swing high, 2 swing low.
    pub kind_code: u8,
    /// The swing high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_bar_index: usize,
    /// The first bar on which the pivot was knowable.
    pub available_bar_index: usize,
    /// 1 when the pivot is confirmed, 0 when it is still a candidate.
    pub confirmed_code: u8,
}

/// One stored confirmed level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The contract's `1000 + n` reference code.
    pub reference_code: u32,
    /// The scale's registry code: 1 minor, 2 major.
    pub scale_code: u8,
    /// The kind's registry code: 1 swing high, 2 swing low.
    pub kind_code: u8,
    /// The swing high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_bar_index: usize,
    /// The first bar on which the level was knowable.
    pub available_bar_index: usize,
}

/// One stored event record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The contract's event-type code: 1 Level, 2 Divergence, 3 Signal.
    pub event_type_code: u8,
    /// The compact subtype code within the type.
    pub event_subtype_code: u8,
    /// The stable source code of the reading or comparison.
    pub source_code: u16,
    /// The bar on which the underlying relationship occurred.
    pub occurred_bar_index: usize,
    /// The first closed bar on which the event became knowable.
    pub available_bar_index: usize,
    /// The event's implication: -1 bearish, 0 neutral, +1 bullish.
    pub direction_code: i8,
    /// The identified EMA or confirmed swing level, when applicable.
    pub reference_code: Option<u32>,
    /// The main measured value at the occurrence.
    pub primary_value: Option<f64>,
    /// The compared level or value; the threshold for a Signal threshold
    /// crossing, when applicable.
    pub reference_value: Option<f64>,
}

/// One stored progression leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LegRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The per-series creation-order identifier.
    pub leg_id: u64,
    /// The pivot scale: 1 minor, 2 major.
    pub scale_code: u8,
    /// The candidate high or low that became the origin.
    pub origin_price: f64,
    /// The bar the origin pivot occurred on.
    pub origin_occurred_bar_index: usize,
    /// The bar the origin became knowable, or `None` while it is forming.
    pub origin_available_bar_index: Option<usize>,
    /// 1 when the origin pivot is confirmed, 0 while it is forming.
    pub origin_confirmed_code: u8,
    /// +1 for an upward leg from a low, -1 for a downward leg from a high.
    pub leg_direction_code: i8,
    /// The selected target price, or `None` when no target is usable.
    pub target_price: Option<f64>,
    /// 1 = confirmed structural level, 2 = ATR fallback, or `None`.
    pub target_basis_code: Option<u8>,
    /// The number of target revisions since creation.
    pub target_version: u32,
    /// 1 when the target is stable, 0 when it is missing or unstable.
    pub target_stable_code: u8,
    /// `(Close - origin) / (target - origin)`, unclamped.
    pub progress: Option<f64>,
    /// `Close - origin`, or `None` when the close is unavailable.
    pub displacement_price: Option<f64>,
    /// `(Close - origin) / ATR_at_origin`, or `None` without a usable ATR.
    pub displacement_atr: Option<f64>,
    /// 0 = forming, 1 = active, 2 = completed, 3 = invalidated, 4 = superseded.
    pub lifecycle_code: u8,
    /// The ordinal stage 0..5, or `None` while it is unavailable.
    pub stage_code: Option<i8>,
    /// 1 when progress reached the target, 0 when it has not, or `None`.
    pub target_reached_code: Option<u8>,
}

/// Builds one row per pivot occurrence for `timeframe_secs`.
pub(crate) fn pivot_rows(timeframe_secs: u64, pivots: &PivotSeries) -> Vec<PivotRow> {
    let len = pivots.len();
    let mut confirmed: [Vec<Option<Pivot>>; 2] = [vec![None; len], vec![None; len]];
    for (index, scale) in PivotScale::ALL.iter().copied().enumerate() {
        for pivot in pivots.all_confirmed(scale) {
            if let Some(slot) = confirmed[index].get_mut(pivot.occurred_at) {
                *slot = Some(*pivot);
            }
        }
    }

    let mut rows = Vec::new();
    for bar in 0..len {
        for (index, scale) in PivotScale::ALL.iter().copied().enumerate() {
            let Some(candidate) = pivots.candidate(scale, bar) else {
                continue;
            };
            let confirmed_pivot = confirmed[index]
                .get(bar)
                .and_then(|pivot| *pivot)
                .filter(|pivot| pivot.kind == candidate.kind);
            rows.push(PivotRow {
                timeframe_secs,
                scale_code: scale.code(),
                kind_code: kind_code(candidate.kind),
                price: candidate.price,
                occurred_bar_index: candidate.occurred_at,
                available_bar_index: confirmed_pivot
                    .map_or(candidate.available_at, |pivot| pivot.available_at),
                confirmed_code: u8::from(confirmed_pivot.is_some()),
            });
        }
    }
    rows
}

/// Builds one row per confirmed level for `timeframe_secs`.
pub(crate) fn level_rows(timeframe_secs: u64, levels: &[ConfirmedLevel]) -> Vec<LevelRow> {
    levels
        .iter()
        .map(|level| LevelRow {
            timeframe_secs,
            reference_code: level.reference_code,
            scale_code: level.scale.code(),
            kind_code: kind_code(level.kind),
            price: level.price,
            occurred_bar_index: level.occurred_at,
            available_bar_index: level.available_at,
        })
        .collect()
}

/// Builds one row per event for `timeframe_secs`, in contract order.
pub(crate) fn event_rows(timeframe_secs: u64, events: &[EventRecord]) -> Vec<EventRow> {
    let mut rows: Vec<EventRow> = events
        .iter()
        .map(|event| EventRow {
            timeframe_secs,
            event_type_code: event.event_type.code(),
            event_subtype_code: event.event_subtype_code,
            source_code: event.source_code,
            occurred_bar_index: event.occurred_bar_index,
            available_bar_index: event.available_bar_index,
            direction_code: event.direction_code,
            reference_code: event.reference_code,
            primary_value: event.primary_value,
            reference_value: event.reference_value,
        })
        .collect();
    rows.sort_by(|left, right| {
        (
            left.available_bar_index,
            left.event_type_code,
            left.event_subtype_code,
            left.source_code,
            left.occurred_bar_index,
            left.reference_code.unwrap_or(0),
        )
            .cmp(&(
                right.available_bar_index,
                right.event_type_code,
                right.event_subtype_code,
                right.source_code,
                right.occurred_bar_index,
                right.reference_code.unwrap_or(0),
            ))
            .then_with(|| compare_optional(left.reference_value, right.reference_value))
    });
    rows
}

/// Orders two optional measured values with a missing value first.
///
/// `total_cmp` keeps the order total even when a value is a negative zero, so
/// the builder's order does not depend on the input order of equal keys.
fn compare_optional(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

/// Builds one row per retained leg for `timeframe_secs`, ordered by `leg_id`.
pub(crate) fn leg_rows(timeframe_secs: u64, legs: &[ProgressionLeg]) -> Vec<LegRow> {
    let mut rows: Vec<LegRow> = legs
        .iter()
        .map(|leg| LegRow {
            timeframe_secs,
            leg_id: leg.leg_id,
            scale_code: leg.scale_code,
            origin_price: leg.origin_price,
            origin_occurred_bar_index: leg.origin_occurred_at,
            origin_available_bar_index: leg.origin_available_at,
            origin_confirmed_code: leg.origin_confirmed_code,
            leg_direction_code: leg.direction_code,
            target_price: leg.target_price,
            target_basis_code: leg.target_basis_code,
            target_version: leg.target_version,
            target_stable_code: leg.target_stable_code,
            progress: leg.progress,
            displacement_price: leg.displacement_price,
            displacement_atr: leg.displacement_atr,
            lifecycle_code: leg.lifecycle_code,
            stage_code: leg.stage_code,
            target_reached_code: leg.target_reached_code,
        })
        .collect();
    rows.sort_by_key(|row| row.leg_id);
    rows
}

/// The research contract's pivot-kind code.
const fn kind_code(kind: PivotKind) -> u8 {
    match kind {
        PivotKind::High => 1,
        PivotKind::Low => 2,
    }
}
