//! The flat per-timeframe indicator/event report.
//!
//! [`FlatReport`] projects a completed [`crate::run::ModelRun`] onto readings,
//! discrete states, confirmed levels, events, progression, and context. It
//! carries no composite value, no consensus, no dashboard view, and no
//! cross-timeframe aggregate. The report is a pure projection; it never
//! recomputes a reading or an event.

mod flat;

pub use flat::{
    FlatContext, FlatEvent, FlatEventSeries, FlatLatest, FlatLeg, FlatLevel, FlatProgression,
    FlatReadings, FlatReport, FlatStates, FlatSubtypeCounts, FlatTimeframe,
};

/// The maximum number of recent events kept in the JSON report.
pub const RECENT_EVENT_LIMIT: usize = flat::RECENT_EVENT_LIMIT;
