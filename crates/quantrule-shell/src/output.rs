//! Stable printed output.
//!
//! The shell prints numbers and codes, never a label the gateway did not send.
//! Lists are sorted where the gateway's order is not part of the contract, so
//! the same response prints the same bytes. `--json` prints the gateway value
//! unchanged (canonical JSON) for the parity tests and for agents.

use std::io::{self, Write};

use quantrule_api::{
    ConfigOrigin, ConfigSummary, DatasetRecord, DatasetSummary, HealthReport, ProfileSummary,
    RunDetail, RunState, RunSummary, ServiceState, SimulationStatus, SimulationSummary,
    StageRunDetail, StageRunSummary, StageStatus, TimeframeInspection,
};
use serde_json::Value;

use crate::error::RunError;

/// One filtered model-stage event batch and the report coverage.
///
/// The type is presentation, not measurement: it carries the published CSV
/// columns and values and the four report coverage fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StageEventsView {
    /// The stage run the artifact belongs to.
    pub(crate) stage_run_id: String,
    /// The timeframe short code the caller requested.
    pub(crate) timeframe: String,
    /// The CSV header, in artifact order.
    pub(crate) columns: Vec<String>,
    /// The event rows for the requested timeframe, in artifact order.
    pub(crate) events: Vec<std::collections::BTreeMap<String, String>>,
    /// The per-timeframe coverage from `report.json`.
    pub(crate) coverage: Vec<Value>,
}

impl StageEventsView {
    /// Build the canonical JSON value for `--json` and the parity fixture.
    pub(crate) fn to_value(&self) -> Value {
        serde_json::json!({
            "stage_run_id": self.stage_run_id,
            "timeframe": self.timeframe,
            "columns": self.columns,
            "events": self.events,
            "coverage": self.coverage,
        })
    }
}

/// Print one command result as text, or as JSON when `--json` is set.
///
/// # Errors
///
/// Returns [`RunError::Output`] when the writer fails, and
/// [`RunError::Decode`] when the local JSON view cannot be encoded.
pub(crate) fn emit(
    json: bool,
    value: Value,
    writer: &mut dyn Write,
    text: impl FnOnce(&mut dyn Write) -> io::Result<()>,
) -> Result<(), RunError> {
    if json {
        write_json_line(writer, &value)
    } else {
        text(writer).map_err(RunError::Output)?;
        writer.flush().map_err(RunError::Output)
    }
}

/// Print one command result whose text renderer already returns [`RunError`].
///
/// # Errors
///
/// Returns the text renderer's failure, or [`RunError::Output`] when the writer
/// fails.
pub(crate) fn emit_raw(
    json: bool,
    value: Value,
    writer: &mut dyn Write,
    text: impl FnOnce(&mut dyn Write) -> Result<(), RunError>,
) -> Result<(), RunError> {
    if json {
        write_json_line(writer, &value)
    } else {
        text(writer)?;
        writer.flush().map_err(RunError::Output)
    }
}

/// Print one JSON value as a single canonical line.
///
/// # Errors
///
/// Returns [`RunError::Output`] when the writer fails.
pub(crate) fn write_json_line(writer: &mut dyn Write, value: &Value) -> Result<(), RunError> {
    let text = serde_json::to_string(value).map_err(|error| RunError::Decode {
        path: String::from("the local JSON view"),
        message: error.to_string(),
    })?;
    writeln!(writer, "{text}").map_err(RunError::Output)
}

/// Print an array as one JSON line per element, or one object as one line.
///
/// A response that wraps its records in an `events` or `marks` array is
/// unwrapped so a caller reads the records, not the envelope.
///
/// # Errors
///
/// Returns the same failures as [`write_json_line`].
pub(crate) fn write_records(writer: &mut dyn Write, value: &Value) -> Result<(), RunError> {
    match value {
        Value::Array(items) => {
            for item in items {
                write_json_line(writer, item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            let records = map
                .get("events")
                .or_else(|| map.get("marks"))
                .and_then(Value::as_array);
            if let Some(items) = records {
                for item in items {
                    write_json_line(writer, item)?;
                }
                Ok(())
            } else {
                write_json_line(writer, value)
            }
        }
        _ => write_json_line(writer, value),
    }
}

/// Render the aggregate health report.
pub(crate) fn health(writer: &mut dyn Write, report: &HealthReport) -> io::Result<()> {
    writeln!(writer, "api: {}", report.v)?;
    writeln!(writer, "state: {}", service_state(report.state))?;
    writeln!(writer, "queue_depth: {}", report.queue_depth)?;
    writeln!(writer, "workers: {}", report.workers)?;
    writeln!(
        writer,
        "image_digest: {}",
        report
            .image_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    let mut services = report.services.clone();
    services.sort_by(|left, right| left.name.cmp(&right.name));
    for service in services {
        match service.detail {
            Some(detail) => writeln!(
                writer,
                "service {}: {} ({detail})",
                service.name,
                service_state(service.state)
            )?,
            None => writeln!(
                writer,
                "service {}: {}",
                service.name,
                service_state(service.state)
            )?,
        }
    }
    Ok(())
}

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

/// Render the configuration table.
pub(crate) fn config_list(writer: &mut dyn Write, configs: &[ConfigSummary]) -> io::Result<()> {
    let mut rows = configs.to_vec();
    rows.sort_by(|left, right| left.config_id.cmp(&right.config_id));
    writeln!(writer, "configs: {}", rows.len())?;
    for config in rows {
        writeln!(
            writer,
            "{} name={} origin={} valid={} message={} hash={}",
            config.config_id,
            config.name,
            config_origin(config.origin),
            config.valid,
            config.message.as_deref().unwrap_or("none"),
            config
                .config_hash
                .as_ref()
                .map_or_else(|| String::from("none"), ToString::to_string)
        )?;
    }
    Ok(())
}

/// Render one configuration summary.
pub(crate) fn config_record(writer: &mut dyn Write, config: &ConfigSummary) -> io::Result<()> {
    writeln!(writer, "config_id: {}", config.config_id)?;
    writeln!(writer, "name: {}", config.name)?;
    writeln!(writer, "origin: {}", config_origin(config.origin))?;
    writeln!(writer, "valid: {}", config.valid)?;
    writeln!(
        writer,
        "message: {}",
        config.message.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "config_hash: {}",
        config
            .config_hash
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )
}

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

fn value_text(value: &Value) -> String {
    match value {
        Value::Null => String::from("none"),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
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

/// Render the profile table.
pub(crate) fn profiles(
    writer: &mut dyn Write,
    profiles: &[ProfileSummary],
    active: &str,
) -> io::Result<()> {
    let mut rows = profiles.to_vec();
    rows.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    writeln!(writer, "active: {active}")?;
    writeln!(writer, "profiles: {}", rows.len())?;
    for profile in rows {
        writeln!(
            writer,
            "{} name={} datasets={} runs={} simulations={}",
            profile.profile_id, profile.name, profile.datasets, profile.runs, profile.simulations
        )?;
    }
    Ok(())
}

/// Render one profile summary.
pub(crate) fn profile(writer: &mut dyn Write, profile: &ProfileSummary) -> io::Result<()> {
    writeln!(writer, "profile_id: {}", profile.profile_id)?;
    writeln!(writer, "name: {}", profile.name)?;
    writeln!(writer, "datasets: {}", profile.datasets)?;
    writeln!(writer, "runs: {}", profile.runs)?;
    writeln!(writer, "simulations: {}", profile.simulations)
}

/// Render the simulation table.
pub(crate) fn simulation_list(
    writer: &mut dyn Write,
    simulations: &[SimulationSummary],
) -> io::Result<()> {
    let mut rows = simulations.to_vec();
    rows.sort_by(|left, right| left.sim_id.cmp(&right.sim_id));
    writeln!(writer, "simulations: {}", rows.len())?;
    for simulation in rows {
        writeln!(
            writer,
            "{} status={} windows={} marks={} premise={}",
            simulation.sim_id,
            simulation_status(simulation.status),
            simulation.windows,
            simulation.marks,
            simulation.premise
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

/// Render the active profile after `profile use`.
pub(crate) fn active_profile(writer: &mut dyn Write, profile: &str) -> io::Result<()> {
    writeln!(writer, "active: {profile}")
}

/// Render a digest record.
pub(crate) fn digest(writer: &mut dyn Write, run_id: &str, digest: &str) -> io::Result<()> {
    writeln!(writer, "run_id: {run_id}")?;
    writeln!(writer, "digest: {digest}")
}

fn service_state(state: ServiceState) -> &'static str {
    match state {
        ServiceState::Healthy => "healthy",
        ServiceState::Degraded => "degraded",
        ServiceState::Down => "down",
    }
}

fn stage_status(status: StageStatus) -> &'static str {
    status.as_str()
}

fn config_origin(origin: ConfigOrigin) -> &'static str {
    match origin {
        ConfigOrigin::Repository => "repository",
        ConfigOrigin::Upload => "upload",
    }
}

fn simulation_status(status: SimulationStatus) -> &'static str {
    match status {
        SimulationStatus::Draft => "draft",
        SimulationStatus::Marking => "marking",
        SimulationStatus::Reviewed => "reviewed",
        SimulationStatus::Checked => "checked",
        SimulationStatus::Decided => "decided",
    }
}

fn elapsed(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("-"), |value| format!("{value}ms"))
}

fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| String::from("-"), |value| value.to_string())
}

/// Render one improvement run detail.
pub(crate) fn improvement_detail(
    writer: &mut dyn Write,
    detail: &quantrule_api::ImprovementRunDetail,
) -> io::Result<()> {
    let manifest = &detail.manifest;
    writeln!(writer, "improvement_id: {}", detail.improvement_id)?;
    writeln!(writer, "tier: {}", detail.tier.as_str())?;
    writeln!(writer, "state: {}", detail.state.as_str())?;
    writeln!(
        writer,
        "decision: {}",
        detail
            .decision
            .map_or("none", quantrule_api::ImprovementDecision::as_str)
    )?;
    writeln!(
        writer,
        "decision_note: {}",
        detail.decision_note.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "created_at: {}", detail.created_at)?;
    writeln!(
        writer,
        "submitted_at: {}",
        detail.submitted_at.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "decided_at: {}",
        detail.decided_at.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "pipeline_run_id: {}",
        detail
            .pipeline_run_id
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "canonical_digest: {}",
        detail
            .canonical_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(writer, "manifest_schema: {}", manifest.schema)?;
    writeln!(writer, "hypothesis: {}", manifest.hypothesis)?;
    writeln!(writer, "created_utc: {}", manifest.created_utc)?;
    writeln!(writer, "changed: {}", manifest.changed.as_str())?;
    writeln!(
        writer,
        "frozen: {}",
        manifest
            .frozen
            .iter()
            .map(|tier| tier.as_str())
            .collect::<Vec<_>>()
            .join(",")
    )?;
    writeln!(writer, "partition: {}", manifest.partition.as_str())?;
    writeln!(writer, "data_version: {}", manifest.data_version)?;
    writeln!(writer, "config_id: {}", manifest.config_id)?;
    writeln!(
        writer,
        "criteria_id: {}",
        manifest.criteria_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "green_criteria: {}",
        serde_json::to_string(&manifest.green_criteria).map_err(io::Error::other)?
    )?;
    writeln!(
        writer,
        "holdout_access: {}",
        manifest.holdout_access.as_str()
    )?;
    writeln!(
        writer,
        "baseline: {}",
        manifest.baseline.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "code_commit: {}", manifest.code_commit)?;
    writeln!(writer, "seed: {}", manifest.seed)?;
    writeln!(
        writer,
        "command: {}",
        manifest.command.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "analytics: {}",
        manifest
            .analytics
            .map_or_else(|| String::from("none"), |value| value.to_string())
    )
}

/// Render the improvement-run table.
pub(crate) fn improvement_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::ImprovementRunSummary],
) -> io::Result<()> {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.improvement_id.cmp(&right.improvement_id))
    });
    writeln!(writer, "research_runs: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} tier={} state={} decision={} created={} submitted={} decided={}",
            row.improvement_id,
            row.tier.as_str(),
            row.state.as_str(),
            row.decision
                .map_or("none", quantrule_api::ImprovementDecision::as_str),
            row.created_at,
            row.submitted_at.as_deref().unwrap_or("none"),
            row.decided_at.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

/// Render one published-number comparison.
pub(crate) fn improvement_comparison(
    writer: &mut dyn Write,
    comparison: &quantrule_api::ImprovementComparison,
) -> io::Result<()> {
    writeln!(writer, "improvement_id: {}", comparison.improvement_id)?;
    writeln!(writer, "tier: {}", comparison.tier.as_str())?;
    writeln!(writer, "candidate_run_id: {}", comparison.candidate_run_id)?;
    writeln!(
        writer,
        "baseline_id: {}",
        comparison.baseline_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "candidate_digest: {}",
        comparison
            .candidate_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "baseline_digest: {}",
        comparison
            .baseline_digest
            .as_ref()
            .map_or_else(|| String::from("none"), ToString::to_string)
    )?;
    writeln!(
        writer,
        "note: {}",
        comparison.note.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "metric candidate baseline delta")?;
    for row in &comparison.rows {
        writeln!(
            writer,
            "{} {} {} {}",
            row.metric,
            number_text(row.candidate),
            number_text(row.baseline),
            number_text(row.delta)
        )?;
    }
    Ok(())
}

/// Render the accepted-baseline table.
pub(crate) fn baseline_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::BaselineRecord],
) -> io::Result<()> {
    writeln!(writer, "baselines: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} tier={} run={} improvement={} accepted_by={} accepted_on={} scope={} supersedes={}",
            row.baseline_id,
            row.tier.as_str(),
            row.run_id.as_deref().unwrap_or("none"),
            row.improvement_id.as_deref().unwrap_or("none"),
            row.accepted_by,
            row.accepted_on,
            row.scope,
            row.supersedes.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

/// Render one accepted baseline.
pub(crate) fn baseline_detail(
    writer: &mut dyn Write,
    row: &quantrule_api::BaselineRecord,
) -> io::Result<()> {
    writeln!(writer, "baseline_id: {}", row.baseline_id)?;
    writeln!(writer, "tier: {}", row.tier.as_str())?;
    writeln!(
        writer,
        "run_id: {}",
        row.run_id.as_deref().unwrap_or("none")
    )?;
    writeln!(
        writer,
        "improvement_id: {}",
        row.improvement_id.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "accepted_by: {}", row.accepted_by)?;
    writeln!(writer, "accepted_on: {}", row.accepted_on)?;
    writeln!(writer, "scope: {}", row.scope)?;
    writeln!(
        writer,
        "supersedes: {}",
        row.supersedes.as_deref().unwrap_or("none")
    )?;
    writeln!(writer, "created_at: {}", row.created_at)
}

/// Render the invalidation tombstone table.
pub(crate) fn invalidation_list(
    writer: &mut dyn Write,
    rows: &[quantrule_api::InvalidationRecord],
) -> io::Result<()> {
    writeln!(writer, "invalidations: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} date={} commit={} removed={} reason={} superseded_by={}",
            row.invalidation_id,
            row.date,
            row.commit,
            row.removed,
            row.reason,
            row.superseded_by.as_deref().unwrap_or("none")
        )?;
    }
    Ok(())
}

fn number_text(value: Option<f64>) -> String {
    value.map_or_else(
        || String::from("none"),
        |value| {
            let text = format!("{value:.6}");
            let trimmed = text.trim_end_matches('0').trim_end_matches('.');
            if trimmed.is_empty() || trimmed == "-0" {
                String::from("0")
            } else {
                trimmed.to_owned()
            }
        },
    )
}

/// Render the simulation check table and verdict.
///
/// The check payload is a research diagnostic; the shell prints its numbers
/// and the side verdicts without adding a label.
pub(crate) fn simulation_check(writer: &mut dyn Write, value: &Value) -> io::Result<()> {
    let field = |name: &str| {
        value
            .get(name)
            .map_or_else(|| String::from("-"), Value::to_string)
    };
    writeln!(
        writer,
        "check: {}",
        value
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )?;
    writeln!(writer, "events: {}", field("events"))?;
    writeln!(writer, "mean_net: {}", field("mean_net"))?;
    writeln!(writer, "hit_rate: {}", field("hit_rate"))?;
    writeln!(writer, "control: {}", field("control"))?;
    writeln!(writer, "control_mean_net: {}", field("control_mean_net"))?;
    writeln!(writer, "control_hit_rate: {}", field("control_hit_rate"))?;
    writeln!(
        writer,
        "positive_half_years: {}",
        field("positive_half_years")
    )?;
    writeln!(writer, "passed: {}", field("passed"))?;
    for failure in value
        .get("failures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writeln!(writer, "failure: {}", failure.as_str().unwrap_or("-"))?;
    }
    for side in value
        .get("sides")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writeln!(
            writer,
            "side {}: {}",
            side.get("side").and_then(Value::as_str).unwrap_or("-"),
            side.get("verdict").and_then(Value::as_str).unwrap_or("-")
        )?;
    }
    Ok(())
}
