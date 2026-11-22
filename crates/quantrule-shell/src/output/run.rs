//! Run submission, detail, table, and artifact rendering.

use std::io::{self, Write};

use quantrule_api::{RunDetail, RunLineage, RunState, RunSummary};

use super::stage::stage_status;

/// Render a run submission.
pub(crate) fn run_submit(
    writer: &mut dyn Write,
    run_id: &str,
    status: RunState,
    created: bool,
) -> io::Result<()> {
    writeln!(writer, "run_id: {run_id}")?;
    writeln!(writer, "status: {}", status.as_str())?;
    writeln!(writer, "created: {created}")
}
/// Render a run detail.
pub(crate) fn run_detail(writer: &mut dyn Write, detail: &RunDetail) -> io::Result<()> {
    writeln!(writer, "run_id: {}", detail.run_id)?;
    writeln!(writer, "status: {}", detail.status.as_str())?;
    writeln!(writer, "owner: {}", detail.owner)?;
    writeln!(writer, "data_version: {}", detail.provenance.data_version)?;
    writeln!(writer, "config_hash: {}", detail.provenance.config_hash)?;
    writeln!(writer, "model_version: {}", detail.provenance.model_version)?;
    writeln!(
        writer,
        "model_artifact_id: {}",
        detail
            .model_artifact_id
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(writer, "image_digest: {}", detail.provenance.image_digest)?;
    writeln!(writer, "stages: {}", detail.stages.len())?;
    for stage in &detail.stages {
        writeln!(
            writer,
            "stage {}: {} {}",
            stage.name.as_str(),
            stage_status(stage.state),
            elapsed(stage.elapsed_ms)
        )?;
    }
    writeln!(writer, "artifacts: {}", detail.artifacts.len())?;
    for artifact in &detail.artifacts {
        writeln!(
            writer,
            "artifact {}: bytes={} sha256={}",
            artifact.name, artifact.bytes, artifact.sha256
        )?;
    }
    writeln!(
        writer,
        "digest: {}",
        detail
            .digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "error: {}",
        detail.error.as_deref().unwrap_or("none")
    )
}

/// Render a run's recorded research artifact lineage.
pub(crate) fn run_lineage(writer: &mut dyn Write, lineage: &RunLineage) -> io::Result<()> {
    writeln!(writer, "run_id: {}", lineage.run_id)?;
    writeln!(writer, "recorded_at: {}", lineage.recorded_at)?;
    writeln!(writer, "references: {}", lineage.references.len())?;
    for (index, reference) in lineage.references.iter().enumerate() {
        write!(
            writer,
            "reference {index}: kind={} id={} hash={}",
            reference.kind, reference.id, reference.hash
        )?;
        if let Some(pack) = &reference.pack {
            write!(
                writer,
                " pack_kind={} pack_name={} pack_version={} pack_owner={} pack_state={}",
                pack.kind, pack.name, pack.version, pack.owner, pack.state
            )?;
        }
        writeln!(writer)?;
    }
    Ok(())
}

/// Render the run table.
pub(crate) fn run_list(writer: &mut dyn Write, runs: &[RunSummary]) -> io::Result<()> {
    let mut rows = runs.to_vec();
    rows.sort_by(|left, right| {
        right
            .submitted_at
            .cmp(&left.submitted_at)
            .then_with(|| left.run_id.cmp(&right.run_id))
    });
    writeln!(writer, "runs: {}", rows.len())?;
    for run in rows {
        writeln!(
            writer,
            "{} status={} config={} data={} submitted={} elapsed={}",
            run.run_id,
            run.status.as_str(),
            run.config_id,
            run.data_version,
            run.submitted_at,
            elapsed(run.elapsed_ms)
        )?;
    }
    Ok(())
}
/// Render a downloaded artifact set.
pub(crate) fn fetched(
    writer: &mut dyn Write,
    run_id: &str,
    directory: &str,
    artifacts: &[(String, u64, String, String)],
) -> io::Result<()> {
    writeln!(writer, "run_id: {run_id}")?;
    writeln!(writer, "directory: {directory}")?;
    for (name, bytes, digest, path) in artifacts {
        writeln!(
            writer,
            "artifact {name}: bytes={bytes} sha256={digest} written={path}"
        )?;
    }
    Ok(())
}
fn elapsed(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value}ms"))
}
