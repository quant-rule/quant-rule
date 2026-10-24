//! Event aggregates across timeframes.
//!
//! Per timeframe, the aggregator counts the events that became knowable in the
//! most recent five closed bars, by type and subtype, and records the latest
//! event and its age. Across timeframes it sums those counts and reports the
//! divergence direction agreement. Event codes are counted, never averaged.

use super::TimeframeSnapshot;
use crate::events::EventRecord;

/// The number of closed bars in the published event window.
pub const EVENT_WINDOW_BARS: usize = 5;

/// Counts by subtype, kept separate because the subtype vocabularies differ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventSubtypeCounts {
    /// Level subtypes 1..3.
    pub level: [u32; 3],
    /// Divergence subtypes 1..4.
    pub divergence: [u32; 4],
    /// Signal subtypes 1..3.
    pub signal: [u32; 3],
}

/// One timeframe's event summary.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeEventSummary {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// Counts over the five-bar window, indexed by type code - 1.
    pub counts_by_type: [u32; 3],
    /// Counts over the five-bar window, by subtype.
    pub counts_by_subtype: EventSubtypeCounts,
    /// The latest event's type code, when any event exists.
    pub latest_event_type_code: Option<u8>,
    /// The latest event's subtype code, when any event exists.
    pub latest_event_subtype_code: Option<u8>,
    /// The latest event's occurrence bar, when any event exists.
    pub latest_occurred_bar_index: Option<usize>,
    /// The latest event's availability bar, when any event exists.
    pub latest_available_bar_index: Option<usize>,
    /// Closed bars since the latest event became knowable.
    pub latest_event_age_bars: Option<u64>,
}

/// Event aggregates across the ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct EventAggregates {
    /// The ladder timeframes.
    pub timeframes: Vec<u64>,
    /// Per-timeframe summaries, in ladder order.
    pub per_timeframe: Vec<TimeframeEventSummary>,
    /// Summed counts by type over the five-bar windows.
    pub event_count_by_type: [u32; 3],
    /// Summed counts by subtype over the five-bar windows.
    pub event_count_by_subtype: EventSubtypeCounts,
    /// Divergence counts by subtype.
    pub divergence_count_by_subtype: [u32; 4],
    /// Timeframes with at least one divergence in the window.
    pub divergence_timeframe_count: u32,
    /// Largest share of eligible timeframes whose latest divergence has the same direction.
    pub divergence_direction_agreement_share: Option<f64>,
    /// Timeframes with at least one Level event in the window.
    pub level_event_timeframe_count: u32,
    /// Timeframes with at least one Signal event in the window.
    pub signal_event_timeframe_count: u32,
}

/// Builds the event aggregates.
pub(crate) fn aggregate(ordered: &[&TimeframeSnapshot]) -> EventAggregates {
    let mut per_timeframe = Vec::with_capacity(ordered.len());
    let mut totals_by_type = [0_u32; 3];
    let mut totals_by_subtype = EventSubtypeCounts::default();
    let mut divergence_timeframe_count = 0_u32;
    let mut level_event_timeframe_count = 0_u32;
    let mut signal_event_timeframe_count = 0_u32;
    let mut divergence_directions = [0_u32; 3];

    for snapshot in ordered {
        let summary = summarize(snapshot);
        for (index, count) in summary.counts_by_type.iter().enumerate() {
            totals_by_type[index] += count;
        }
        add_counts(&mut totals_by_subtype, &summary.counts_by_subtype);

        if summary
            .counts_by_subtype
            .divergence
            .iter()
            .any(|count| *count > 0)
        {
            divergence_timeframe_count += 1;
            if let Some(direction) = latest_divergence_direction(snapshot) {
                divergence_directions[(direction + 1) as usize] += 1;
            }
        }
        if summary.counts_by_type[0] > 0 {
            level_event_timeframe_count += 1;
        }
        if summary.counts_by_type[2] > 0 {
            signal_event_timeframe_count += 1;
        }
        per_timeframe.push(summary);
    }

    let divergence_total: u32 = divergence_directions.iter().sum();
    let divergence_direction_agreement_share = if divergence_total == 0 {
        None
    } else {
        let maximum = divergence_directions.iter().copied().max().unwrap_or(0);
        Some(f64::from(maximum) / f64::from(divergence_total))
    };

    EventAggregates {
        timeframes: ordered
            .iter()
            .map(|snapshot| snapshot.timeframe_secs)
            .collect(),
        per_timeframe,
        event_count_by_type: totals_by_type,
        event_count_by_subtype: totals_by_subtype,
        divergence_count_by_subtype: totals_by_subtype.divergence,
        divergence_timeframe_count,
        divergence_direction_agreement_share,
        level_event_timeframe_count,
        signal_event_timeframe_count,
    }
}

fn summarize(snapshot: &TimeframeSnapshot) -> TimeframeEventSummary {
    let latest = snapshot.latest_bar_index;
    let window_start = latest.saturating_sub(EVENT_WINDOW_BARS - 1);
    let mut counts_by_type = [0_u32; 3];
    let mut counts_by_subtype = EventSubtypeCounts::default();

    for event in &snapshot.events {
        if event.available_bar_index < window_start || event.available_bar_index > latest {
            continue;
        }
        count_event(event, &mut counts_by_type, &mut counts_by_subtype);
    }

    let latest_event = snapshot
        .events
        .iter()
        .filter(|event| event.available_bar_index <= latest)
        .max_by_key(|event| (event.available_bar_index, event.occurred_bar_index));

    TimeframeEventSummary {
        timeframe_secs: snapshot.timeframe_secs,
        counts_by_type,
        counts_by_subtype,
        latest_event_type_code: latest_event.map(|event| event.event_type.code()),
        latest_event_subtype_code: latest_event.map(|event| event.event_subtype_code),
        latest_occurred_bar_index: latest_event.map(|event| event.occurred_bar_index),
        latest_available_bar_index: latest_event.map(|event| event.available_bar_index),
        latest_event_age_bars: latest_event
            .map(|event| latest.saturating_sub(event.available_bar_index) as u64),
    }
}

fn count_event(
    event: &EventRecord,
    counts_by_type: &mut [u32; 3],
    counts_by_subtype: &mut EventSubtypeCounts,
) {
    let type_index = event.event_type.code().saturating_sub(1) as usize;
    if type_index >= counts_by_type.len() {
        return;
    }
    counts_by_type[type_index] += 1;

    match event.event_type.code() {
        1 => {
            if let Some(slot) = counts_by_subtype
                .level
                .get_mut(event.event_subtype_code.saturating_sub(1) as usize)
            {
                *slot += 1;
            }
        }
        2 => {
            if let Some(slot) = counts_by_subtype
                .divergence
                .get_mut(event.event_subtype_code.saturating_sub(1) as usize)
            {
                *slot += 1;
            }
        }
        3 => {
            if let Some(slot) = counts_by_subtype
                .signal
                .get_mut(event.event_subtype_code.saturating_sub(1) as usize)
            {
                *slot += 1;
            }
        }
        _ => {}
    }
}

fn add_counts(total: &mut EventSubtypeCounts, counts: &EventSubtypeCounts) {
    for (target, value) in total.level.iter_mut().zip(counts.level.iter()) {
        *target += value;
    }
    for (target, value) in total.divergence.iter_mut().zip(counts.divergence.iter()) {
        *target += value;
    }
    for (target, value) in total.signal.iter_mut().zip(counts.signal.iter()) {
        *target += value;
    }
}

fn latest_divergence_direction(snapshot: &TimeframeSnapshot) -> Option<i8> {
    let latest = snapshot.latest_bar_index;
    let window_start = latest.saturating_sub(EVENT_WINDOW_BARS - 1);
    snapshot
        .events
        .iter()
        .filter(|event| {
            event.event_type == crate::events::EventType::Divergence
                && event.available_bar_index >= window_start
                && event.available_bar_index <= latest
        })
        .max_by_key(|event| (event.available_bar_index, event.occurred_bar_index))
        .map(|event| event.direction_code)
}
