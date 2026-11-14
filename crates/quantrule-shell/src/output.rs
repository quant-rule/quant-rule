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
    StageStatus, TimeframeInspection,
};
use serde_json::Value;

use crate::error::RunError;

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

/// Render a run detail.
pub(crate) fn run_detail(writer: &mut dyn Write, detail: &RunDetail) -> io::Result<()> {
    writeln!(writer, "run_id: {}", detail.run_id)?;
    writeln!(writer, "status: {}", detail.status.as_str())?;
    writeln!(writer, "owner: {}", detail.owner)?;
    writeln!(writer, "data_version: {}", detail.provenance.data_version)?;
    writeln!(writer, "config_hash: {}", detail.provenance.config_hash)?;
    writeln!(writer, "model_version: {}", detail.provenance.model_version)?;
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
