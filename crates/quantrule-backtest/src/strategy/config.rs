//! The strategy configuration error.
//!
//! The generic bracket and event engines validate the values they are built
//! with. The multi-timeframe strategy context is S3.x work; only the errors the
//! two generic engines can return remain.

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
    /// The event subtype code is zero; a registered subtype is required.
    EventSubtype,
    /// The event source code is outside 1..57.
    EventSource,
    /// The event identity is not registered.
    EventNotRegistered,
    /// The event is registered as diagnostic and is not readable by a strategy.
    EventDiagnostic,
    /// The event direction is outside -1..1.
    EventDirection,
    /// A state value is outside its field's range.
    EventStateValue,
    /// The compression-bracket event is not the registered range event.
    CompressionEventNotRange,
    /// The compression-bracket time stop is zero.
    CompressionTimeStop,
    /// The compression-bracket entry window is zero.
    CompressionEntryWindow,
    /// The giveback threshold or arm is outside its valid range.
    GivebackAtr,
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
                write!(formatter, "the event subtype code must be non-zero")
            }
            Self::EventSource => write!(formatter, "the event source code must be in 1..=57"),
            Self::EventNotRegistered => {
                write!(formatter, "the event identity is not registered")
            }
            Self::EventDiagnostic => {
                write!(
                    formatter,
                    "a diagnostic event is not readable by a strategy"
                )
            }
            Self::EventDirection => write!(formatter, "the event direction must be in -1..=1"),
            Self::EventStateValue => {
                write!(formatter, "a state value is outside its field's range")
            }
            Self::CompressionEventNotRange => write!(
                formatter,
                "the compression bracket must use the registered compression-range event"
            ),
            Self::CompressionTimeStop => {
                write!(formatter, "the compression time stop must be non-zero")
            }
            Self::CompressionEntryWindow => {
                write!(formatter, "the compression entry window must be one bar")
            }
            Self::GivebackAtr => write!(
                formatter,
                "giveback_atr must be positive and giveback_arm_atr must be non-negative"
            ),
        }
    }
}

impl std::error::Error for StrategyConfigError {}
