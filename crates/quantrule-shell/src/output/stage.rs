//! Diagnostic stage rendering.

use std::io::{self, Write};

use quantrule_api::{StageRunDetail, StageRunSummary, StageStatus};
use serde_json::Value;

use super::StageEventsView;
use super::json::value_text;

/// Render a stage submission.
pub(crate) fn stage_submitted(
    writer: &mut dyn Write,
    stage_run_id: &str,
    state: &str,
    created: bool,
) -> io::Result<()> {
    writeln!(writer, "stage_run_id: {stage_run_id}")?;
    writeln!(writer, "state: {state}")?;
    writeln!(writer, "created: {created}")
}

/// Render the stage table.
pub(crate) fn stage_list(writer: &mut dyn Write, stages: &[StageRunSummary]) -> io::Result<()> {
    let mut rows = stages.to_vec();
    rows.sort_by(|left, right| {
        right
            .submitted_at
            .cmp(&left.submitted_at)
            .then_with(|| left.stage_run_id.cmp(&right.stage_run_id))
    });
    writeln!(writer, "stages: {}", rows.len())?;
    for stage in rows {
        writeln!(
            writer,
            "{} kind={} state={} config={} submitted={} artifact={}",
            stage.stage_run_id,
            stage.kind.as_str(),
            stage.state.as_str(),
            stage.config_id,
            stage.submitted_at,
            stage
                .artifact_id
                .as_ref()
                .map_or_else(|| String::from("none"), ToString::to_string)
        )?;
    }
    Ok(())
}

/// Render one stage detail.
pub(crate) fn stage_detail(writer: &mut dyn Write, detail: &StageRunDetail) -> io::Result<()> {
    writeln!(writer, "stage_run_id: {}", detail.stage_run_id)?;
    writeln!(writer, "kind: {}", detail.kind.as_str())?;
    writeln!(writer, "state: {}", detail.state.as_str())?;
    writeln!(writer, "owner: {}", detail.owner)?;
    writeln!(writer, "config_id: {}", detail.config_id)?;
    writeln!(
        writer,
        "input: {} {}",
        detail.input.label(),
        detail.input.digest()
    )?;
    writeln!(writer, "data_version: {}", detail.provenance.data_version)?;
    writeln!(writer, "config_hash: {}", detail.provenance.config_hash)?;
    writeln!(writer, "model_version: {}", detail.provenance.model_version)?;
    writeln!(writer, "image_digest: {}", detail.provenance.image_digest)?;
    writeln!(writer, "attempts: {}", detail.attempts)?;
    writeln!(
        writer,
        "error: {}",
        detail.error.as_deref().unwrap_or("none")
    )?;
    match &detail.artifact {
        Some(artifact) => {
            writeln!(writer, "artifact_id: {}", artifact.artifact_id)?;
            writeln!(writer, "artifacts: {}", artifact.files.len())?;
            for file in &artifact.files {
                writeln!(
                    writer,
                    "artifact {}: bytes={} sha256={}",
                    file.name, file.bytes, file.sha256
                )?;
            }
        }
        None => {
            writeln!(writer, "artifact_id: none")?;
            writeln!(writer, "artifacts: 0")?;
        }
    }
    writeln!(writer, "evidence: {}", detail.evidence)
}

/// Render a downloaded stage artifact set.
pub(crate) fn stage_fetched(
    writer: &mut dyn Write,
    stage_run_id: &str,
    directory: &str,
    artifacts: &[(String, u64, String, String)],
) -> io::Result<()> {
    writeln!(writer, "stage_run_id: {stage_run_id}")?;
    writeln!(writer, "directory: {directory}")?;
    for (name, bytes, digest, path) in artifacts {
        writeln!(
            writer,
            "artifact {name}: bytes={bytes} sha256={digest} written={path}"
        )?;
    }
    Ok(())
}

/// Render the deleted stage identity.
pub(crate) fn stage_deleted(writer: &mut dyn Write, stage_run_id: &str) -> io::Result<()> {
    writeln!(writer, "deleted: {stage_run_id}")
}

/// Render one published strategy decision without recomputing it.
pub(crate) fn stage_explain(
    writer: &mut dyn Write,
    stage_run_id: &str,
    bar: usize,
    record: &Value,
) -> io::Result<()> {
    writeln!(writer, "stage_run_id: {stage_run_id}")?;
    writeln!(writer, "bar_index: {bar}")?;
    for field in [
        "decision",
        "side",
        "reason",
        "position_before",
        "position_after",
    ] {
        writeln!(writer, "{field}: {}", value_text(&record[field]))?;
    }
    if let Some(object) = record.as_object() {
        let shown = [
            "schema",
            "bar_index",
            "decision",
            "side",
            "reason",
            "position_before",
            "position_after",
        ];
        for (key, value) in object {
            if shown.contains(&key.as_str()) {
                continue;
            }
            writeln!(writer, "{key}: {}", value_text(value))?;
        }
    }
    Ok(())
}

/// Render one model-stage event batch and its report coverage.
pub(crate) fn stage_events(writer: &mut dyn Write, view: &StageEventsView) -> io::Result<()> {
    writeln!(writer, "stage_run_id: {}", view.stage_run_id)?;
    writeln!(writer, "timeframe: {}", view.timeframe)?;
    writeln!(writer, "events: {}", view.events.len())?;
    writeln!(writer, "{}", view.columns.join(","))?;
    for event in &view.events {
        let row: Vec<&str> = view
            .columns
            .iter()
            .map(|column| event.get(column).map_or("", String::as_str))
            .collect();
        writeln!(writer, "{}", row.join(","))?;
    }
    writeln!(writer, "coverage: {}", view.coverage.len())?;
    for entry in &view.coverage {
        writeln!(
            writer,
            "timeframe_secs={} bars={} expected_bars={} coverage_share={}",
            value_text(&entry["timeframe_secs"]),
            value_text(&entry["bars"]),
            value_text(&entry["expected_bars"]),
            value_text(&entry["coverage_share"])
        )?;
    }
    Ok(())
}
pub(super) fn stage_status(status: StageStatus) -> &'static str {
    status.as_str()
}
