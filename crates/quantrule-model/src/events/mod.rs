//! The model's shared event records and event detectors.
//!
//! The contract defines three event types — Level, Divergence, and Signal — that
//! share one numeric record. An event is appended once, when it becomes
//! knowable, and it never alters an axis value. The record carries the type, the
//! subtype, the source, the timeframe, the occurrence and availability bars, the
//! direction, the reference, and the measured values.
//!
//! This module owns the shared record and the Level detector. Divergence and
//! Signal are added by their own work items. The detectors are pure: they read
//! candles, readings, and a configuration, and perform no I/O, read no clock, and
//! use no randomness.

pub mod divergence;
pub mod level;
pub mod signal;

/// The contract's event-type codes.
///
/// A code is stable once published; a retired code is not reused.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Code 1: price interacted with a named price reference.
    Level = 1,
    /// Code 2: price swings and an indicator's values disagree directionally.
    Divergence = 2,
    /// Code 3: a defined numerical condition changed.
    Signal = 3,
}

impl EventType {
    /// Every event type, in contract order.
    pub const ALL: [Self; 3] = [Self::Level, Self::Divergence, Self::Signal];

    /// The contract's numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// The event type for a contract code, or `None` when the code is unknown.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Level),
            2 => Some(Self::Divergence),
            3 => Some(Self::Signal),
            _ => None,
        }
    }
}

/// One event record shared by every event type.
///
/// The model publishes numbers only. `event_type` and `event_subtype_code` are
/// the contract's numeric codes; the shell maps them to words. `occurred_bar_index`
/// is the bar on which the relationship happened, and `available_bar_index` is
/// the first closed bar on which it became knowable. The latter is never earlier
/// than the former.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventRecord {
    /// The event type.
    pub event_type: EventType,
    /// The type's compact subtype code.
    pub event_subtype_code: u8,
    /// The stable source code of the reading or comparison that produced it.
    pub source_code: u16,
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
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
    /// The compared level or value, when applicable.
    pub reference_value: Option<f64>,
}
