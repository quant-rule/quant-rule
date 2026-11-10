//! Confirmed pivot-sequence events.
//!
//! A sequence event is emitted only on the confirmation bar of the pivot that
//! extends the sequence. A reading that stays constant between pivots does not
//! re-emit. The detector reads confirmed major pivots only, so a candidate or a
//! future pivot can never affect an earlier event.

use super::*;

/// Source code for the confirmed pivot sequence.
pub const PIVOT_SEQUENCE_SOURCE_CODE: u16 = 55;

/// Emits the lower-high and lower-low sequence events.
pub(super) fn structure_sequence_events(
    readings: &Readings,
    extended: &ExtendedReadings,
    config: &ExtendedEventConfig,
    bar: usize,
    events: &mut Vec<EventRecord>,
) {
    let Some(pivot) = readings.pivots.confirmed_at(PivotScale::Major, bar) else {
        return;
    };
    let timeframe = config.timeframe_secs();
    match pivot.kind {
        PivotKind::High => {
            if let Some(count) = extended.lower_high_count.get(bar)
                && count >= 2.0
            {
                push(
                    events,
                    codes::SIGNAL_LOWER_HIGH_SEQUENCE,
                    EventType::Signal,
                    19,
                    PIVOT_SEQUENCE_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(count),
                    Some(2.0),
                );
            }
        }
        PivotKind::Low => {
            if let Some(count) = extended.lower_low_count.get(bar)
                && count >= 2.0
            {
                push(
                    events,
                    codes::SIGNAL_LOWER_LOW_SEQUENCE,
                    EventType::Signal,
                    19,
                    PIVOT_SEQUENCE_SOURCE_CODE,
                    timeframe,
                    bar,
                    -1,
                    Some(count),
                    Some(2.0),
                );
            }
        }
    }
}
