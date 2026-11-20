//! Inspection rendering.

use std::io::{self, Write};

use quantrule_api::TimeframeInspection;

/// Render a timeframe inspection.
pub(crate) fn timeframes(
    writer: &mut dyn Write,
    inspection: &TimeframeInspection,
) -> io::Result<()> {
    writeln!(writer, "data_version: {}", inspection.data_version)?;
    writeln!(writer, "timeframes: {}", inspection.timeframes.len())?;
    for timeframe in &inspection.timeframes {
        writeln!(
            writer,
            "{}: bars={} partial_buckets={} first_bar={} last_bar={}",
            timeframe.timeframe,
            timeframe.bars,
            timeframe.partial_buckets,
            optional_number(timeframe.first_bar),
            optional_number(timeframe.last_bar)
        )?;
    }
    Ok(())
}
fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}
