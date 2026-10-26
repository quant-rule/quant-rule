//! View-only Structure evidence.
//!
//! The Structure axis publishes a swing-relation position and a
//! swing-alignment magnitude size. The dashboard adds two pieces of evidence
//! that must never feed back into either: the current reference prices and the
//! density of Structure-mapped events in the last few closed bars.
//!
//! Event activity is deliberately a detail. It is not a confidence score and
//! it does not move the dot; the same event is counted once here and is not a
//! second vote on the axis. The module is pure: it reads the timeframe's
//! published levels, events, and readings up to the view bar.

use crate::events::EventType;
use crate::events::signal::{
    DIRECTION_SOURCE_CODE, MOMENTUM_SOURCE_CODE, STRUCTURE_SOURCE_CODE, SignalSubtype,
};
use crate::readings::PivotKind;
use crate::readings::PivotScale;
use crate::readings::roles::AxisCode;
use crate::run::{ModelBar, TimeframeModel};

use super::super::LevelReading;

/// The fixed activity window from the dashboard contract.
const ACTIVITY_WINDOW_BARS: usize = 5;

/// The reference-kind codes, in the contract's fixed order.
const EMA20_REFERENCE_KIND: u8 = 1;
const EMA50_REFERENCE_KIND: u8 = 2;
const EMA200_REFERENCE_KIND: u8 = 3;
const MINOR_HIGH_REFERENCE_KIND: u8 = 4;
const MINOR_LOW_REFERENCE_KIND: u8 = 5;
const MAJOR_HIGH_REFERENCE_KIND: u8 = 6;
const MAJOR_LOW_REFERENCE_KIND: u8 = 7;

/// The Structure activity at one view bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct StructureActivity {
    /// Structure-mapped events available in the window.
    pub(super) count: u32,
    /// The window length in bars, at most five.
    pub(super) window: usize,
    /// `count / window`; a zero count is a measured zero.
    pub(super) per_bar: Option<f64>,
}

/// Maps one event record to the axis that owns it.
///
/// The mapping is a display assignment, not a vote. Level interactions and
/// divergences belong to Structure; RSI threshold crossings belong to Momentum;
/// EMA/EMA crossovers belong to Structure; a compact-state change belongs to
/// the axis named by its source code. An unknown code is not assigned, which
/// keeps a future event family from silently inflating an axis count.
#[must_use]
pub(super) fn event_axis(event: &crate::events::EventRecord) -> Option<AxisCode> {
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

/// Counts Structure-mapped events that became available in the recent window.
///
/// The window is the last `min(5, bar + 1)` bars, inclusive of the view bar. An
/// event is counted by its availability bar, never by its occurrence bar, so a
/// pivot that becomes knowable later cannot appear early.
#[must_use]
pub(super) fn structure_activity(timeframe: &TimeframeModel, bar: usize) -> StructureActivity {
    activity_from_events(timeframe.events(), bar)
}

/// Counts Structure-mapped events in the window for one event list.
///
/// Kept separate from the timeframe so the availability rule can be tested
/// with synthetic events, without constructing a whole model.
fn activity_from_events(events: &[crate::events::EventRecord], bar: usize) -> StructureActivity {
    let window = bar.saturating_add(1).min(ACTIVITY_WINDOW_BARS);
    let start = bar.saturating_add(1).saturating_sub(window);
    let count = events
        .iter()
        .filter(|event| {
            event.available_bar_index >= start
                && event.available_bar_index <= bar
                && event_axis(event) == Some(AxisCode::Structure)
        })
        .count();

    StructureActivity {
        count: u32::try_from(count).unwrap_or(u32::MAX),
        window,
        per_bar: Some(count as f64 / window as f64),
    }
}

/// Builds the current reference prices in the contract's fixed order.
///
/// EMAs are current at the view bar. Swing references use the latest confirmed
/// level that was knowable by the view bar; their `available_bar` is the bar the
/// level became knowable, not the later view bar that reads it. Missing
/// references are omitted rather than filled.
#[must_use]
pub(super) fn level_references(
    timeframe: &TimeframeModel,
    model_bar: &ModelBar,
) -> Vec<LevelReading> {
    let mut references = Vec::with_capacity(7);
    let bar = model_bar.bar_index;
    let mut push_ema = |kind: u8, price: Option<f64>| {
        if let Some(price) = price.filter(|price| price.is_finite()) {
            references.push(LevelReading {
                reference_kind_code: kind,
                price,
                available_bar: bar,
            });
        }
    };

    push_ema(EMA20_REFERENCE_KIND, model_bar.readings.ema20);
    push_ema(EMA50_REFERENCE_KIND, model_bar.readings.ema50);
    push_ema(EMA200_REFERENCE_KIND, model_bar.readings.ema200);

    for (scale, kind, reference_kind_code) in [
        (
            PivotScale::Minor,
            PivotKind::High,
            MINOR_HIGH_REFERENCE_KIND,
        ),
        (PivotScale::Minor, PivotKind::Low, MINOR_LOW_REFERENCE_KIND),
        (
            PivotScale::Major,
            PivotKind::High,
            MAJOR_HIGH_REFERENCE_KIND,
        ),
        (PivotScale::Major, PivotKind::Low, MAJOR_LOW_REFERENCE_KIND),
    ] {
        let level =
            timeframe.levels().iter().rev().find(|level| {
                level.available_at <= bar && level.scale == scale && level.kind == kind
            });
        if let Some(level) = level {
            references.push(LevelReading {
                reference_kind_code,
                price: level.price,
                available_bar: level.available_at,
            });
        }
    }

    references
}

#[cfg(test)]
mod tests;
