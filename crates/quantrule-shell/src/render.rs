//! NDJSON rendering for the retained candle windows.
//!
//! The shell maps the engine's numeric fields to the command's output schema.
//! It does not recompute a candle, aggregate a timeframe, or detect a gap. The
//! engine supplies the series, gaps, and partial-bucket records; this module
//! only prints them in a fixed order.

use std::io::{self, Write};

use quantrule_data::{Candle, CandleWindow, Timeframe, TimeframeSet, format_utc_timestamp};

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
        write_candle(
            None,
            candle,
            candle.series_index() - first_series_index,
            writer,
        )?;
    }

    Ok(())
}

/// Writes every retained series as NDJSON, grouped by timeframe.
///
/// The order is M1, M5, M15, H1, H4; within a series the oldest candle is
/// first. Each line carries the timeframe code so one stream can hold all five
/// series.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn write_timeframes(set: &TimeframeSet, writer: &mut impl Write) -> io::Result<()> {
    for series in set.series_iter() {
        let Some(first) = series.first() else {
            continue;
        };
        let first_series_index = first.series_index();

        for candle in series.iter() {
            write_candle(
                Some(series.timeframe()),
                candle,
                candle.series_index() - first_series_index,
                writer,
            )?;
        }
    }

    Ok(())
}

/// Writes the per-timeframe partial-bucket and gap report.
///
/// The summary for each timeframe is followed by its partial-bucket records and
/// then its gap records. The engine owns both kinds of record; this function
/// only formats them.
///
/// # Errors
///
/// Returns the writer's error when a line cannot be written.
pub(crate) fn write_timeframe_report(
    set: &TimeframeSet,
    writer: &mut impl Write,
) -> io::Result<()> {
    for series in set.series_iter() {
        let timeframe = series.timeframe();
        writeln!(
            writer,
            "aggregation timeframe={} retained={} partial={} gaps={}",
            timeframe.code(),
            series.len(),
            series.partial_buckets().len(),
            series.gaps().len()
        )?;

        for partial in series.partial_buckets() {
            writeln!(
                writer,
                "partial timeframe={} bucket={} present={} expected={}",
                timeframe.code(),
                format_utc_timestamp(partial.bucket_start),
                partial.present,
                partial.expected
            )?;
        }

        for gap in series.gaps() {
            writeln!(
                writer,
                "gap timeframe={} after={} before={} missing={}",
                timeframe.code(),
                format_utc_timestamp(gap.after),
                format_utc_timestamp(gap.before),
                gap.missing
            )?;
        }
    }

    Ok(())
}

fn write_candle(
    timeframe: Option<Timeframe>,
    candle: &Candle,
    window_index: u64,
    writer: &mut impl Write,
) -> io::Result<()> {
    write!(writer, "{{")?;
    if let Some(timeframe) = timeframe {
        write!(writer, "\"timeframe\":\"{}\",", timeframe.code())?;
    }
    write!(
        writer,
        "\"series_index\":{},\"window_index\":{},\"datetime\":\"{}\",\"open\":{},\"high\":{},\"low\":{},\"close\":{}",
        candle.series_index(),
        window_index,
        format_utc_timestamp(candle.timestamp()),
        candle.open(),
        candle.high(),
        candle.low(),
        candle.close()
    )?;

    match candle.volume() {
        Some(volume) => write!(writer, ",\"volume\":{volume}")?,
        None => write!(writer, ",\"volume\":null")?,
    }
    writeln!(writer, "}}")
}
