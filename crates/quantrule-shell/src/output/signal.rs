//! Signal artifact rendering.
//!
//! The shell renders the catalog record only. It never parses a signal row or
//! derives an action, a side, or an evaluation metric.

use std::io::{self, Write};

use quantrule_api::{ArtifactRecord, SignalResponse};

/// Render one artifact record's fields.
fn artifact_fields(writer: &mut dyn Write, record: &ArtifactRecord) -> io::Result<()> {
    writeln!(writer, "signal_id: {}", record.summary.artifact_id)?;
    writeln!(writer, "kind: {}", record.summary.kind.code())?;
    writeln!(writer, "bytes: {}", record.summary.bytes)?;
    writeln!(writer, "created_at: {}", record.summary.created_at)?;
    writeln!(writer, "object_key: {}", record.object_key)?;
    writeln!(writer, "sha256: {}", record.sha256)
}

/// Render one signal record.
pub(crate) fn signal_record(writer: &mut dyn Write, record: &ArtifactRecord) -> io::Result<()> {
    artifact_fields(writer, record)
}

/// Render one signal create response.
pub(crate) fn signal_created(writer: &mut dyn Write, response: &SignalResponse) -> io::Result<()> {
    writeln!(writer, "schema: {}", response.schema)?;
    writeln!(writer, "created: {}", response.created)?;
    artifact_fields(writer, &response.artifact)
}

/// Render the signal table.
pub(crate) fn signal_list(writer: &mut dyn Write, signals: &[ArtifactRecord]) -> io::Result<()> {
    let mut rows = signals.to_vec();
    rows.sort_by(|left, right| left.summary.artifact_id.cmp(&right.summary.artifact_id));
    writeln!(writer, "signals: {}", rows.len())?;
    for signal in rows {
        writeln!(
            writer,
            "{} kind={} bytes={} created={}",
            signal.summary.artifact_id,
            signal.summary.kind.code(),
            signal.summary.bytes,
            signal.summary.created_at
        )?;
    }
    Ok(())
}

/// Render a verified signal download.
pub(crate) fn signal_fetched(
    writer: &mut dyn Write,
    signal_id: &str,
    file: &str,
    bytes: u64,
    sha256: &str,
) -> io::Result<()> {
    writeln!(writer, "signal_id: {signal_id}")?;
    writeln!(writer, "file: {file}")?;
    writeln!(writer, "bytes: {bytes}")?;
    writeln!(writer, "sha256: {sha256}")
}
