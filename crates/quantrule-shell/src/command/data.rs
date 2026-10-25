//! Candle and timeframe command execution.
//!
//! The commands delegate validation and timeframe construction to the data
//! engine and only render what it returns.

use std::io::Write;
use std::path::Path;

use super::types::RunError;
use crate::csv;
use crate::render;

/// Loads a CSV source strictly and writes its retained window as NDJSON.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or the writer fails.
pub(crate) fn run_candles(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let window = csv::load_window(path).map_err(RunError::Source)?;
    render::write_ndjson(&window, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Loads a CSV source tolerantly, writes the retained window as NDJSON, and
/// writes the quality report to a second writer.
///
/// The NDJSON is flushed before the report is written. A source-level failure
/// leaves stdout untouched because the window is built before rendering.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or either writer fails.
pub(crate) fn run_candles_tolerant(
    path: &Path,
    output: &mut impl Write,
    report: &mut impl Write,
) -> Result<(), RunError> {
    let loaded = csv::load_window_tolerant(path).map_err(RunError::Source)?;
    render::write_ndjson(&loaded.window, output).map_err(RunError::Output)?;
    output.flush().map_err(RunError::Output)?;
    loaded.report.write(report).map_err(RunError::Output)?;
    report.flush().map_err(RunError::Output)
}

/// Builds M5, M15, and H1 from an M1 CSV and writes the series and report.
///
/// The NDJSON is flushed before the report is written. A source-level failure
/// leaves stdout untouched because the timeframe set is built before rendering.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or either writer fails.
pub(crate) fn run_timeframes(
    path: &Path,
    output: &mut impl Write,
    report: &mut impl Write,
) -> Result<(), RunError> {
    let set = csv::load_timeframes(path).map_err(RunError::Source)?;
    render::write_timeframes(&set, output).map_err(RunError::Output)?;
    output.flush().map_err(RunError::Output)?;
    render::write_timeframe_report(&set, report).map_err(RunError::Output)?;
    report.flush().map_err(RunError::Output)
}
