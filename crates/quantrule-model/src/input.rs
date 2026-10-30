//! The model's input contract.
//!
//! The configuration and the per-timeframe candle slice are consumed by both
//! the run orchestration and the assembled state. They live here so neither
//! module has to import the other, and so the input shape has one owner.

use quantrule_data::Candle;

use crate::readings::PivotScale;

/// The configuration of a market-state run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketStateConfig {
    /// The pivot scale used by Structure and Divergence.
    pub structure_scale: PivotScale,
}

impl MarketStateConfig {
    /// Builds the configuration for a structure scale.
    #[must_use]
    pub const fn new(structure_scale: PivotScale) -> Self {
        Self { structure_scale }
    }
}

impl Default for MarketStateConfig {
    fn default() -> Self {
        Self {
            structure_scale: PivotScale::Major,
        }
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
