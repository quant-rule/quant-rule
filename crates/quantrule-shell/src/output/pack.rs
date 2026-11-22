//! Pack registry rendering.

use std::io::{self, Write};

use quantrule_api::{PackRecord, PackSummary};

fn optional(value: Option<&String>) -> &str {
    value.map_or("none", String::as_str)
}

/// Render one pack record and its lifecycle history.
pub(crate) fn pack_record(writer: &mut dyn Write, record: &PackRecord) -> io::Result<()> {
    let summary = &record.summary;
    writeln!(writer, "pack_id: {}", summary.pack_id)?;
    writeln!(writer, "kind: {}", summary.kind)?;
    writeln!(writer, "state: {}", summary.state)?;
    writeln!(writer, "name: {}", summary.name)?;
    writeln!(writer, "version: {}", summary.version)?;
    writeln!(writer, "owner: {}", summary.owner)?;
    writeln!(writer, "bytes: {}", summary.bytes)?;
    writeln!(writer, "created_at: {}", summary.created_at)?;
    writeln!(
        writer,
        "validated_at: {}",
        optional(summary.validated_at.as_ref())
    )?;
    writeln!(
        writer,
        "frozen_at: {}",
        optional(summary.frozen_at.as_ref())
    )?;
    writeln!(
        writer,
        "retired_at: {}",
        optional(summary.retired_at.as_ref())
    )?;
    writeln!(writer, "object_key: {}", record.object_key)?;
    writeln!(writer, "sha256: {}", record.sha256)?;
    writeln!(writer, "validations: {}", record.validations.len())?;
    for (index, validation) in record.validations.iter().enumerate() {
        writeln!(
            writer,
            "validation {index}: validator={} valid={} message={} created_at={}",
            validation.validator,
            validation.valid,
            validation.message.as_deref().unwrap_or("none"),
            validation.created_at
        )?;
    }
    writeln!(writer, "events: {}", record.events.len())?;
    for (index, event) in record.events.iter().enumerate() {
        writeln!(
            writer,
            "event {index}: from={} to={} action={} reason={} created_at={}",
            event
                .from
                .map_or_else(|| String::from("none"), |state| state.to_string()),
            event.to,
            event.action,
            event.reason.as_deref().unwrap_or("none"),
            event.created_at
        )?;
    }
    Ok(())
}

/// Render the pack table.
pub(crate) fn pack_list(writer: &mut dyn Write, packs: &[PackSummary]) -> io::Result<()> {
    let mut rows = packs.to_vec();
    rows.sort_by(|left, right| left.pack_id.cmp(&right.pack_id));
    writeln!(writer, "packs: {}", rows.len())?;
    for pack in rows {
        writeln!(
            writer,
            "{} kind={} state={} name={} version={} owner={} bytes={} created={}",
            pack.pack_id,
            pack.kind,
            pack.state,
            pack.name,
            pack.version,
            pack.owner,
            pack.bytes,
            pack.created_at
        )?;
    }
    Ok(())
}

/// Render a verified pack download.
pub(crate) fn pack_fetched(
    writer: &mut dyn Write,
    pack_id: &str,
    file: &str,
    bytes: u64,
    sha256: &str,
) -> io::Result<()> {
    writeln!(writer, "pack_id: {pack_id}")?;
    writeln!(writer, "file: {file}")?;
    writeln!(writer, "bytes: {bytes}")?;
    writeln!(writer, "sha256: {sha256}")
}
