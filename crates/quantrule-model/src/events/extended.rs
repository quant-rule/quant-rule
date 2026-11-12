//! Extended event detection by reading family.
//!
//! The batch covers VWAP, profile, participation, flow, Keltner/Donchian and
//! level strength, pivot sequence, Fibonacci, horizontal bands, ATR/range/MAR,
//! Aroon, candle anatomy, patterns, gaps, path, extremes, drawdown, and range
//! position. The detector reads only closed candles and the published readings.
//! It emits Level, Divergence, and Signal records with the contract's numeric
//! codes, and it is pure: no I/O, clock, randomness, or statistics.

pub use crate::events::source_codes::{
    ABSORPTION_SOURCE_CODE, AD_SOURCE_CODE, AROON_SOURCE_CODE,
    ATR_TERM_SOURCE_CODE as ATR_SOURCE_CODE, BOS_CHOCH_SOURCE_CODE, CANDLE_SOURCE_CODE,
    CLIMAX_SOURCE_CODE, CMF_SOURCE_CODE, DIRECTION_SOURCE_CODE, DONCHIAN_SOURCE_CODE,
    DRAWDOWN_SOURCE_CODE, EXTREME_SOURCE_CODE, GAP_SOURCE_CODE, KELTNER_SOURCE_CODE,
    MAR_SOURCE_CODE, MFI_SOURCE_CODE, OBV_SOURCE_CODE, PATH_SOURCE_CODE, PATTERN_SOURCE_CODE,
    PROFILE_SOURCE_CODE, RANGE_POSITION_SOURCE_CODE, RANGE_SOURCE_CODE, RVOL_SOURCE_CODE,
    STRENGTH_SOURCE_CODE, SWING_SOURCE_CODE, VOLUME_OSCILLATOR_SOURCE_CODE, VWAP_SOURCE_CODE,
};
use std::collections::BTreeSet;
use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::events::{EventRecord, EventType, codes};
use crate::readings::{ExtendedReadings, PivotKind, PivotScale, ReadingSeries, Readings};

/// The configuration of the extended event run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtendedEventConfig {
    timeframe_secs: NonZeroU64,
    reclaim_bars: usize,
    level_atr: f64,
    path_window: usize,
    fib_test_atr: f64,
    band_atr: f64,
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
            fib_test_atr: 0.2,
            band_atr: 0.25,
        }
    }

    /// Uses the same path window as the extended readings.
    #[must_use]
    pub const fn with_path_window(mut self, path_window: usize) -> Self {
        self.path_window = path_window;
        self
    }

    /// Uses the configured Fibonacci test tolerance.
    #[must_use]
    pub const fn with_fib_test_atr(mut self, fib_test_atr: f64) -> Self {
        self.fib_test_atr = fib_test_atr;
        self
    }

    /// Uses the configured horizontal-band tolerance.
    #[must_use]
    pub const fn with_band_atr(mut self, band_atr: f64) -> Self {
        self.band_atr = band_atr;
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
            structure_sequence_events(readings, extended, config, bar, &mut events);
            fibonacci_events(candles, readings, extended, config, bar, &mut events);
            band_events(candles, readings, extended, config, bar, &mut events);
        }
        divergence_events(readings, extended, config, &mut events);
        participation_divergences(readings, extended, config, &mut events);
        flow_divergences(readings, extended, config, &mut events);
        atr_divergences(readings, extended, config, &mut events);
        signals::compute(candles, readings, extended, config, &mut events);
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

mod band;
mod divergence;
mod fibonacci;
mod flow;
mod levels;
mod participation;
mod profile;
mod shared;
mod signals;
mod structure;
mod vwap;

#[cfg(test)]
mod row_signal_tests;
#[cfg(test)]
mod row_tests;
#[cfg(test)]
mod tests;

use self::band::band_events;
use self::divergence::{atr_divergences, divergence_events};
use self::fibonacci::fibonacci_events;
use self::flow::{flow_divergences, flow_events};
use self::levels::level_extended_events;
use self::participation::{participation_divergences, participation_events};
use self::profile::profile_events;
use self::structure::structure_sequence_events;
use self::vwap::vwap_events;

pub(crate) use divergence::divergence_subtype;
pub(crate) use shared::{
    cross_down_value, cross_up_value, push, push_confirmed, push_with_reference,
};
