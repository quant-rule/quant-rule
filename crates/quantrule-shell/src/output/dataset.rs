//! Dataset summary, record, and table rendering.

use std::io::{self, Write};

use quantrule_api::{DatasetRecord, DatasetSummary};

/// Render one dataset summary.
pub(crate) fn dataset_summary(writer: &mut dyn Write, dataset: &DatasetSummary) -> io::Result<()> {
    writeln!(writer, "data_version: {}", dataset.data_version)?;
    writeln!(writer, "name: {}", dataset.name)?;
    writeln!(writer, "candles: {}", dataset.candles)?;
    writeln!(writer, "first_bar: {}", dataset.first_bar)?;
    writeln!(writer, "last_bar: {}", dataset.last_bar)?;
    writeln!(writer, "bytes: {}", dataset.bytes)
}

/// Render one dataset record.
pub(crate) fn dataset_record(writer: &mut dyn Write, record: &DatasetRecord) -> io::Result<()> {
    dataset_summary(writer, &record.summary)?;
    writeln!(writer, "object_key: {}", record.object_key)?;
    writeln!(writer, "sha256: {}", record.sha256)?;
    match &record.quality {
        Some(quality) => {
            writeln!(
                writer,
                "quality: rows={} accepted={} rejected={} duplicates={} out_of_order={} invalid={}",
                quality.rows_read,
                quality.accepted,
                quality.rejected,
                quality.duplicates,
                quality.out_of_order,
                quality.invalid
            )?;
            for row in &quality.skipped {
                writeln!(writer, "skipped {} {}", row.line, row.reason)?;
            }
        }
        None => writeln!(writer, "quality: none")?,
    }
    writeln!(writer, "created_at: {}", record.created_at)
}

/// Render the dataset table.
pub(crate) fn dataset_list(writer: &mut dyn Write, datasets: &[DatasetSummary]) -> io::Result<()> {
    let mut rows = datasets.to_vec();
    rows.sort_by(|left, right| left.data_version.cmp(&right.data_version));
    writeln!(writer, "datasets: {}", rows.len())?;
    for dataset in rows {
        writeln!(
            writer,
            "{} name={} candles={} first_bar={} last_bar={} bytes={}",
            dataset.data_version,
            dataset.name,
            dataset.candles,
            dataset.first_bar,
            dataset.last_bar,
            dataset.bytes
        )?;
    }
    Ok(())
}
