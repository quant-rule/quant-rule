//! NDJSON rendering for the retained candle window.
//!
//! The shell maps the engine's numeric fields to the command's output schema.
//! It does not recompute a candle or an indicator, and it prints the window in
//! the order the engine retains it: oldest first.

use std::io::{self, Write};

use quantrule_data::{Candle, CandleWindow, format_utc_timestamp};

/// Writes one NDJSON line per retained candle, oldest first.
///
/// The first retained candle always has `window_index` 0. The stable
/// `series_index` is printed unchanged.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn write_ndjson(window: &CandleWindow, writer: &mut impl Write) -> io::Result<()> {
    let Some(first) = window.first() else {
        return Ok(());
    };
    let first_series_index = first.series_index();

    for candle in window.iter() {
        write_candle(candle, candle.series_index() - first_series_index, writer)?;
    }

    Ok(())
}

fn write_candle(candle: &Candle, window_index: u64, writer: &mut impl Write) -> io::Result<()> {
    let datetime = format_utc_timestamp(candle.timestamp());

    match candle.volume() {
        Some(volume) => writeln!(
            writer,
            "{{\"series_index\":{},\"window_index\":{},\"datetime\":\"{}\",\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"volume\":{}}}",
            candle.series_index(),
            window_index,
            datetime,
            candle.open(),
            candle.high(),
            candle.low(),
            candle.close(),
            volume
        ),
        None => writeln!(
            writer,
            "{{\"series_index\":{},\"window_index\":{},\"datetime\":\"{}\",\"open\":{},\"high\":{},\"low\":{},\"close\":{},\"volume\":null}}",
            candle.series_index(),
            window_index,
            datetime,
            candle.open(),
            candle.high(),
            candle.low(),
            candle.close()
        ),
    }
}
