//! The strategy configuration error.
//!
//! The generic bracket and event engines validate the values they are built
//! with. The axis-ladder strategy and its configuration left with the four-axis
//! cutover; only the errors those two generic engines can return remain.

use std::fmt;

/// A strategy configuration is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyConfigError {
    /// The event window is zero.
    EventWindowBars,
    /// The entry condition list is empty.
    EventEmptyEntry,
    /// A condition list exceeds the fixed bound.
    EventTooManyConditions,
    /// A condition is duplicated.
    EventDuplicateCondition,
    /// The entry has no directional event.
    EventEntryDirection,
    /// Entry events disagree on direction.
    EventDirectionConflict,
    /// The condition timeframe is zero.
    EventTimeframe,
    /// The event type code is outside 1..3.
    EventType,
    /// The event subtype code is outside its registered range.
    EventSubtype,
    /// The event source code is outside 1..54.
    EventSource,
    /// The event direction is outside -1..1.
    EventDirection,
    /// A state value is outside its field's range.
    EventStateValue,
    /// The compression-bracket time stop is zero.
    CompressionTimeStop,
    /// The compression-bracket entry window is zero.
    CompressionEntryWindow,
}

impl fmt::Display for StrategyConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventWindowBars => write!(formatter, "the event window must be at least one bar"),
            Self::EventEmptyEntry => write!(formatter, "the entry condition list is empty"),
            Self::EventTooManyConditions => write!(
                formatter,
                "an event condition list exceeds {} entries",
                super::event::MAX_CONDITIONS
            ),
            Self::EventDuplicateCondition => write!(formatter, "an event condition is duplicated"),
            Self::EventEntryDirection => {
                write!(formatter, "the entry has no directional event")
            }
            Self::EventDirectionConflict => {
                write!(formatter, "entry events disagree on direction")
            }
            Self::EventTimeframe => write!(formatter, "the condition timeframe must be non-zero"),
            Self::EventType => write!(formatter, "the event type code must be in 1..=3"),
            Self::EventSubtype => {
                write!(formatter, "the event subtype code is outside its range")
            }
            Self::EventSource => write!(formatter, "the event source code must be in 1..=54"),
            Self::EventDirection => write!(formatter, "the event direction must be in -1..=1"),
            Self::EventStateValue => {
                write!(formatter, "a state value is outside its field's range")
            }
            Self::CompressionTimeStop => {
                write!(formatter, "the compression time stop must be non-zero")
            }
            Self::CompressionEntryWindow => {
                write!(formatter, "the compression entry window must be one bar")
            }
        }
    }
}

impl std::error::Error for StrategyConfigError {}
