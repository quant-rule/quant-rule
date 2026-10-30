//! The model's input contract.
//!
//! The configuration and the per-timeframe candle slice are consumed by both
//! the run orchestration and the assembled state. They live here so neither
//! module has to import the other, and so the input shape has one owner.

use quantrule_data::{Candle, Timeframe};

use crate::readings::{PivotScale, ReadingsConfig};

/// The configuration of a market-state run.
///
/// The readings are indexed by [`Timeframe::ALL`] order. A timeframe without an
/// explicit override uses the default reading configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketStateConfig {
    /// The pivot scale used by Structure and Divergence.
    pub structure_scale: PivotScale,
    readings: [ReadingsConfig; Timeframe::ALL.len()],
    overridden: [bool; Timeframe::ALL.len()],
}

impl MarketStateConfig {
    /// Builds the configuration with the default readings on every timeframe.
    #[must_use]
    pub const fn new(structure_scale: PivotScale) -> Self {
        Self {
            structure_scale,
            readings: [ReadingsConfig::DEFAULT; Timeframe::ALL.len()],
            overridden: [false; Timeframe::ALL.len()],
        }
    }

    /// The reading configuration for one timeframe.
    #[must_use]
    pub fn readings_for(&self, timeframe_secs: u64) -> ReadingsConfig {
        Timeframe::ALL
            .iter()
            .position(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
            .map_or_else(ReadingsConfig::default, |index| self.readings[index])
    }

    /// Sets one timeframe's readings and marks them as an explicit override.
    ///
    /// A timeframe outside [`Timeframe::ALL`] is ignored; the config loader
    /// rejects it before this point.
    pub fn set_readings(&mut self, timeframe: Timeframe, readings: ReadingsConfig) {
        if let Some(index) = Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
        {
            self.readings[index] = readings;
            self.overridden[index] = true;
        }
    }

    /// Whether the timeframe's readings were explicitly overridden.
    #[must_use]
    pub fn is_overridden(&self, timeframe: Timeframe) -> bool {
        Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
            .is_some_and(|index| self.overridden[index])
    }
}

impl Default for MarketStateConfig {
    fn default() -> Self {
        Self::new(PivotScale::Major)
    }
}

/// One timeframe's candles, paired with its timeframe in seconds.
#[derive(Debug, Clone, Copy)]
pub struct TimeframeInput<'a> {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The closed candles for this timeframe.
    pub candles: &'a [Candle],
}
