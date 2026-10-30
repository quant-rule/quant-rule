//! Shared layer construction for the command families.
//!
//! Both the research and chart commands need the same M1/M5/M15/H1 build. The
//! helper keeps that construction in one place without giving either command
//! family ownership of the other.

use std::path::Path;

use quantrule_analytics::layers::{LayerConfig, LayerSet, build_layers};
use quantrule_data::{Candle, Timeframe};

use super::types::{RunError, WindowMode};
use crate::pipeline::{self, ModelInputs};

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
    let set = pipeline::load_set(path, window).map_err(RunError::Source)?;
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let inputs = ModelInputs::from_set(&set);

    let layers =
        build_layers(&inputs.as_inputs(), &LayerConfig::default()).map_err(RunError::Layers)?;

    Ok(LayerBuild { layers, m1 })
}
