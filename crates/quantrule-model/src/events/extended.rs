//! Extended events for the first indicator batch: VWAP and volume profile.
//!
//! The detector reads only closed candles and the published readings. It emits
//! Level, Divergence, and Signal records with the contract's numeric codes. The
//! module is pure: no I/O, clock, randomness, or statistics.

use std::collections::BTreeSet;
use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::events::{EventRecord, EventType, codes};
use crate::readings::{ExtendedReadings, PivotKind, PivotScale, ReadingSeries, Readings};

/// Source code for the rolling VWAP.
pub const VWAP_SOURCE_CODE: u16 = 19;
/// Source code for the rolling volume profile.
pub const PROFILE_SOURCE_CODE: u16 = 20;
/// Source code for RVOL, used by the profile breakout confirmation.
pub const RVOL_SOURCE_CODE: u16 = 6;
/// Source code for the volume climax.
pub const CLIMAX_SOURCE_CODE: u16 = 21;
/// Source code for absorption.
pub const ABSORPTION_SOURCE_CODE: u16 = 22;
/// Source code for the volume oscillator.
pub const VOLUME_OSCILLATOR_SOURCE_CODE: u16 = 23;
/// Source code for on-balance volume.
pub const OBV_SOURCE_CODE: u16 = 24;
/// Source code for Chaikin money flow.
pub const CMF_SOURCE_CODE: u16 = 25;
/// Source code for the money flow index.
pub const MFI_SOURCE_CODE: u16 = 26;
/// Source code for the accumulation/distribution line.
pub const AD_SOURCE_CODE: u16 = 27;
/// Source code for Keltner.
pub const KELTNER_SOURCE_CODE: u16 = 31;
/// Source code for Donchian.
pub const DONCHIAN_SOURCE_CODE: u16 = 32;
/// Source code for swing structure.
pub const SWING_SOURCE_CODE: u16 = 33;
/// Source code for level strength.
pub const STRENGTH_SOURCE_CODE: u16 = 34;
/// Source code for break of structure and change of character.
pub const BOS_CHOCH_SOURCE_CODE: u16 = 36;
/// Source code for candle anatomy.
pub const CANDLE_SOURCE_CODE: u16 = 37;
/// Source code for gaps.
pub const GAP_SOURCE_CODE: u16 = 39;
/// Source code for the ATR term structure.
pub const ATR_SOURCE_CODE: u16 = 28;

/// Source code for range expansion.
pub const RANGE_SOURCE_CODE: u16 = 29;
/// Source code for mean absolute return.
pub const MAR_SOURCE_CODE: u16 = 30;
/// Source code for Aroon.
pub const AROON_SOURCE_CODE: u16 = 35;
/// Source code for candle patterns.
pub const PATTERN_SOURCE_CODE: u16 = 38;
/// Source code for path efficiency.
pub const PATH_SOURCE_CODE: u16 = 40;
/// Source code for direction changes.
pub const DIRECTION_SOURCE_CODE: u16 = 41;
/// Source code for bars since extreme.
pub const EXTREME_SOURCE_CODE: u16 = 42;
/// Source code for drawdown.
pub const DRAWDOWN_SOURCE_CODE: u16 = 43;
/// Source code for range position.
pub const RANGE_POSITION_SOURCE_CODE: u16 = 44;

/// The configuration of the extended event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtendedEventConfig {
    timeframe_secs: NonZeroU64,
    reclaim_bars: usize,
    level_atr: f64,
    path_window: usize,
}

impl ExtendedEventConfig {
    /// Builds a configuration from a non-zero timeframe.
    #[must_use]
    pub const fn new(timeframe_secs: NonZeroU64) -> Self {
        Self {
            timeframe_secs,
            reclaim_bars: 6,
            level_atr: 0.2,
            path_window: 20,
        }
    }

    /// Uses the same path window as the extended readings.
    #[must_use]
    pub const fn with_path_window(mut self, path_window: usize) -> Self {
        self.path_window = path_window;
        self
    }

    /// Builds a configuration from a timeframe in seconds.
    #[must_use]
    pub fn from_secs(timeframe_secs: u64) -> Option<Self> {
        NonZeroU64::new(timeframe_secs).map(Self::new)
    }

    /// The configured timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(self) -> u64 {
        self.timeframe_secs.get()
    }

    /// The path window used by the extreme-stale event.
    #[must_use]
    pub const fn path_window(self) -> usize {
        self.path_window
    }
}

/// The extended events detected over one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtendedEventSeries {
    events: Vec<EventRecord>,
}

/// Removes duplicate occurrences while preserving distinct measured values.
///
/// The event code already identifies the type, subtype, and source. The key
/// therefore adds the bar, direction, reference, and reference value. The
/// reference value matters: one MFI threshold event code can fire for 20, 50,
/// and 80 on the same bar, and each crossing is a distinct occurrence.
fn deduplicate_occurrences(events: &mut Vec<EventRecord>) {
    events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));
    let mut seen = BTreeSet::new();
    events.retain(|event| {
        seen.insert((
            event.timeframe_secs,
            event.event_code,
            event.occurred_bar_index,
            event.direction_code,
            event.reference_code,
            event.reference_value.map(f64::to_bits),
        ))
    });
}

impl ExtendedEventSeries {
    /// Computes every extended event over the candles and readings.
    #[must_use]
    pub fn compute(
        candles: &[Candle],
        readings: &Readings,
        extended: &ExtendedReadings,
        config: &ExtendedEventConfig,
    ) -> Self {
        let mut events = Vec::new();
        for bar in 0..candles.len() {
            let candle = candles[bar];
            let atr = readings.atr14.get(bar).filter(|atr| *atr > 0.0);
            vwap_events(candles, extended, config, bar, candle, atr, &mut events);
            profile_events(candles, readings, extended, config, bar, &mut events);
            participation_events(candles, readings, extended, config, bar, &mut events);
            flow_events(candles, extended, config, bar, &mut events);
            level_extended_events(candles, readings, extended, config, bar, &mut events);
        }
        divergence_events(readings, extended, config, &mut events);
        participation_divergences(readings, extended, config, &mut events);
        flow_divergences(readings, extended, config, &mut events);
        atr_divergences(readings, extended, config, &mut events);
        crate::events::signals::compute(candles, readings, extended, config, &mut events);
        deduplicate_occurrences(&mut events);
        Self { events }
    }

    /// The number of detected events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Whether no event was detected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Every detected event.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        &self.events
    }
}

mod divergence;
mod flow;
mod levels;
mod participation;
mod profile;
mod shared;
mod vwap;

#[cfg(test)]
mod row_tests;
#[cfg(test)]
mod tests;

use self::divergence::{atr_divergences, divergence_events};
use self::flow::{flow_divergences, flow_events};
use self::levels::level_extended_events;
use self::participation::{participation_divergences, participation_events};
use self::profile::profile_events;
use self::vwap::vwap_events;

pub(crate) use divergence::divergence_subtype;
pub(crate) use shared::{cross_down_value, cross_up_value, push};
