//! Shared layer construction for the command families.
//!
//! Both the research and chart commands need the same M1/M5/M15/H1 build. The
//! helper keeps that construction in one place without giving either command
//! family ownership of the other.

use std::path::Path;

use quantrule_analytics::layers::{LayerConfig, LayerSet, build_layers};
use quantrule_data::{Candle, Timeframe};
use quantrule_model::state::TimeframeInput;

use super::types::{RunError, WindowMode};
use crate::csv;

/// One layer build and its accepted M1 candles.
pub(crate) struct LayerBuild {
    pub(crate) layers: LayerSet,
    pub(crate) m1: Vec<Candle>,
}

/// Loads an M1 CSV and builds the three layers.
pub(crate) fn load_layers(path: &Path) -> Result<LayerBuild, RunError> {
    load_layers_with_window(path, WindowMode::Full)
}

pub(crate) fn load_layers_with_window(
    path: &Path,
    window: WindowMode,
) -> Result<LayerBuild, RunError> {
    let set = match window {
        WindowMode::Full => csv::load_full_timeframes(path),
        WindowMode::Latest => csv::load_timeframes(path),
    }
    .map_err(RunError::Source)?;
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    let layers = build_layers(
        &[
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M1.seconds()),
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M5.seconds()),
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M15.seconds()),
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::H1.seconds()),
                candles: &h1,
            },
        ],
        &LayerConfig::default(),
    )
    .map_err(RunError::Layers)?;

    Ok(LayerBuild { layers, m1 })
}
