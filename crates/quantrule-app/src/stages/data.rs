//! Data inspection and event-batch use cases.
//!
//! One call owns the source-to-engine path and returns the data-engine values;
//! the adapters render them. No module here computes an indicator or an event.

use std::path::Path;

use quantrule_data::{CandleWindow, TimeframeSet};
use quantrule_model::run::ModelRun;

use crate::csv::{self, SourceError, TolerantWindow};
use crate::pipeline::{self, WindowMode};

/// Loads a CSV source strictly.
///
/// # Errors
///
/// Returns [`SourceError`] when the source or the data engine rejects a row.
pub fn inspect_candles(path: &Path) -> Result<CandleWindow, SourceError> {
    csv::load_window(path)
}

/// Loads a CSV source tolerantly.
///
/// # Errors
///
/// Returns [`SourceError`] when the source cannot be read or the file header is
/// invalid.
pub fn inspect_candles_tolerant(path: &Path) -> Result<TolerantWindow, SourceError> {
    csv::load_window_tolerant(path)
}

/// Builds the supported timeframe ladder from one M1 CSV source.
///
/// # Errors
///
/// Returns [`SourceError`] when the source or the data engine rejects a row.
pub fn inspect_timeframes(path: &Path) -> Result<TimeframeSet, SourceError> {
    csv::load_timeframes(path)
}

/// Computes the event batch over one CSV source.
///
/// # Errors
///
/// Returns [`SourceError`] when the source or the data engine fails.
pub fn compute_events(path: &Path, window: WindowMode) -> Result<ModelRun, SourceError> {
    let set = pipeline::load_set(path, window)?;
    Ok(pipeline::model_run_from_set(
        &set,
        &quantrule_model::input::MarketStateConfig::default(),
    ))
}
