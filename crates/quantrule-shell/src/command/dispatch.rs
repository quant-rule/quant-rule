//! The command boundary: one executor, one gateway call per command.
//!
//! The dispatcher owns the error-to-exit-code policy and the per-command path.
//! It holds no domain logic: it builds one request, decodes the contract type,
//! and hands the value to the renderer.

use std::fs;
use std::io::Write;
use std::path::Path;

use quantrule_api::{
    CreateProfileRequest, CreateSimulationRequest, DatasetRecord, DatasetSummary, Digest,
    ProfileId, ProfileSummary, RunDetail, RunState, RunSummary, SimulationSummary,
    SubmitRunRequest, SubmitRunResponse, TimeframeInspection,
};
use serde_json::{Value, json};

use super::help::run_help;
use super::types::{Command, GlobalOptions, Invocation};
use crate::client::{Client, encode};
use crate::error::RunError;
use crate::output;
use crate::session::Session;

/// Execute one invocation and return the process exit code.
///
/// The interactive loop ignores the code and keeps the session open; the
/// non-interactive entry point returns it. A failure is written once to stderr.
pub(crate) fn execute(
    invocation: Invocation,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> i32 {
    let Invocation { globals, command } = invocation;
    let result = match Session::new(&globals) {
        Ok(session) => run(&session, &globals, command, stdout),
        Err(error) => Err(error),
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(stderr, "error: {error}");
            1
        }
    }
}

fn run(
    session: &Session,
    globals: &GlobalOptions,
    command: Command,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    match &command {
        Command::Help(topic) => return run_help(topic, stdout),
        Command::Quit => return Ok(()),
        _ => {}
    }
    let client = session.client()?;
    match command {
        Command::Health => health(&client, globals, stdout),
        Command::DatasetList => dataset_list(&client, globals, stdout),
        Command::DatasetShow { data_version } => {
            dataset_show(&client, globals, stdout, &data_version)
        }
        Command::DatasetUpload {
            path,
            name,
            tolerant,
        } => dataset_upload(&client, globals, stdout, &path, name.as_deref(), tolerant),
        Command::ConfigList => config_list(&client, globals, stdout),
        Command::ConfigShow { config_id } => config_show(&client, globals, stdout, &config_id),
        Command::RunSubmit {
            data_version,
            config_id,
        } => run_submit(&client, globals, stdout, &data_version, &config_id),
        Command::RunStatus { run_id } => run_status(&client, globals, stdout, &run_id),
        Command::RunList { status, limit } => run_list(&client, globals, stdout, status, limit),
        Command::RunFetch { run_id, directory } => {
            run_fetch(&client, globals, stdout, &run_id, &directory)
        }
        Command::RunCancel { run_id } => run_cancel(&client, globals, stdout, &run_id),
        Command::InspectCandles { data_version } => {
            inspect_candles(&client, globals, stdout, &data_version)
        }
        Command::InspectTimeframes { data_version } => {
            inspect_timeframes(&client, globals, stdout, &data_version)
        }
        Command::InspectEvents {
            data_version,
            timeframe,
        } => inspect_events(&client, globals, stdout, &data_version, &timeframe),
        Command::Verify { run_id } => verify(&client, globals, stdout, &run_id),
        Command::ProfileList => profile_list(session, &client, globals, stdout),
        Command::ProfileCreate { name, id } => {
            profile_create(&client, globals, stdout, &name, id.as_deref())
        }
        Command::ProfileUse { profile_id } => {
            if globals.profile.is_some() {
                return Err(RunError::Usage(String::from(
                    "profile use cannot run with --profile; the flag is a one-off selection",
                )));
            }
            profile_use(session, &client, globals, stdout, &profile_id)
        }
        Command::SimulationList { status, limit } => {
            simulation_list(&client, globals, stdout, status, limit)
        }
        Command::SimulationCreate {
            premise,
            bundle,
            seed,
            criteria,
        } => simulation_create(&client, globals, stdout, &premise, &bundle, seed, &criteria),
        Command::SimulationWindows { sim_id } => {
            simulation_windows(&client, globals, stdout, &sim_id)
        }
        Command::SimulationMark {
            sim_id,
            chart,
            bar,
            action,
            quality,
            side,
            note,
        } => simulation_mark(
            &client,
            globals,
            stdout,
            &MarkInput {
                sim_id: &sim_id,
                chart: &chart,
                bar,
                action,
                quality,
                side,
                note: &note,
            },
        ),
        Command::SimulationMarks { sim_id, origin } => {
            simulation_marks(&client, globals, stdout, &sim_id, origin)
        }
        Command::SimulationOracle { sim_id } => {
            simulation_oracle(&client, globals, stdout, &sim_id)
        }
        Command::SimulationCheck { sim_id } => simulation_check(&client, globals, stdout, &sim_id),
        Command::SimulationDecide {
            sim_id,
            decision,
            note,
        } => simulation_decide(&client, globals, stdout, &sim_id, &decision, &note),
        Command::Help(_) | Command::Quit => Ok(()),
    }
}

/// Decode one contract type from a response body.
///
/// A macro is used because the shell's dependency set does not name `serde`
/// directly; the concrete type is enough for `serde_json`.
macro_rules! decode_json {
    ($ty:ty, $response:expr, $path:expr) => {
        serde_json::from_slice::<$ty>(&$response.body)
            .map_err(|error| RunError::decode($path, error))?
    };
}

/// Build the JSON view of a typed response for `--json`.
macro_rules! json_view {
    ($value:expr) => {
        serde_json::to_value($value)
            .map_err(|error| RunError::decode("the local JSON view", error))?
    };
}

fn health(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/health")?;
    let report: quantrule_api::HealthReport =
        decode_json!(quantrule_api::HealthReport, response, "/health");
    output::emit(globals.json, json_view!(&report), stdout, |writer| {
        output::health(writer, &report)
    })
}

fn dataset_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/catalog/datasets")?;
    let rows: Vec<DatasetSummary> =
        decode_json!(Vec<DatasetSummary>, response, "/catalog/datasets");
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::dataset_list(writer, &rows)
    })
}

fn dataset_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/catalog/datasets/{}", encode(data_version));
    let response = client.get(&path)?;
    let record: DatasetRecord = decode_json!(DatasetRecord, response, &path);
    output::emit(globals.json, json_view!(&record), stdout, |writer| {
        output::dataset_record(writer, &record)
    })
}

fn dataset_upload(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    path: &Path,
    name: Option<&str>,
    tolerant: bool,
) -> Result<(), RunError> {
    let body = fs::read(path).map_err(|error| RunError::file(path.display(), error))?;
    let label = match name {
        Some(name) => name.to_owned(),
        None => path
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| {
                RunError::Usage(format!(
                    "cannot derive a dataset name from '{}'; use --name",
                    path.display()
                ))
            })?
            .to_owned(),
    };
    let route = if tolerant {
        "/catalog/datasets/tolerant"
    } else {
        "/catalog/datasets"
    };
    let url = format!("{route}?name={}", encode(&label));
    let response = client.post_csv(&url, body)?;
    let record: DatasetRecord = decode_json!(DatasetRecord, response, &url);
    let created = response.status == 201;
    output::emit(globals.json, json_view!(&record), stdout, |writer| {
        writeln!(writer, "created: {created}")?;
        output::dataset_record(writer, &record)
    })
}

fn config_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/catalog/configs")?;
    let rows: Vec<quantrule_api::ConfigSummary> = decode_json!(
        Vec<quantrule_api::ConfigSummary>,
        response,
        "/catalog/configs"
    );
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::config_list(writer, &rows)
    })
}

fn config_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    config_id: &str,
) -> Result<(), RunError> {
    let response = client.get("/catalog/configs")?;
    let rows: Vec<quantrule_api::ConfigSummary> = decode_json!(
        Vec<quantrule_api::ConfigSummary>,
        response,
        "/catalog/configs"
    );
    let config = rows
        .iter()
        .find(|row| row.config_id == config_id)
        .ok_or_else(|| RunError::Api {
            code: String::from("config.invalid"),
            message: format!("config {config_id} is unknown in this profile"),
        })?;
    output::emit(globals.json, json_view!(config), stdout, |writer| {
        output::config_record(writer, config)
    })
}

fn run_submit(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
    config_id: &str,
) -> Result<(), RunError> {
    let digest = Digest::parse(data_version)
        .map_err(|error| RunError::Usage(format!("data_version {error}")))?;
    let request = SubmitRunRequest {
        data_version: digest,
        config_id: config_id.to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the run submission", error))?;
    let response = client.post_json("/runs", &body)?;
    let submitted: SubmitRunResponse = decode_json!(SubmitRunResponse, response, "/runs");
    output::emit(globals.json, json_view!(&submitted), stdout, |writer| {
        output::run_submit(
            writer,
            &submitted.run_id.to_string(),
            submitted.status,
            submitted.created,
        )
    })
}

fn run_status(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/runs/{}", encode(run_id));
    let response = client.get(&path)?;
    let detail: RunDetail = decode_json!(RunDetail, response, &path);
    output::emit(globals.json, json_view!(&detail), stdout, |writer| {
        output::run_detail(writer, &detail)
    })
}

fn run_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    status: Option<RunState>,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(status) = status {
        query.push(format!("status={}", status.as_str()));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let path = with_query("/runs", &query);
    let response = client.get(&path)?;
    let rows: Vec<RunSummary> = decode_json!(Vec<RunSummary>, response, &path);
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::run_list(writer, &rows)
    })
}

fn run_fetch(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
    directory: &Path,
) -> Result<(), RunError> {
    let detail_path = format!("/runs/{}", encode(run_id));
    let response = client.get(&detail_path)?;
    let detail: RunDetail = decode_json!(RunDetail, response, &detail_path);

    fs::create_dir_all(directory).map_err(|error| RunError::file(directory.display(), error))?;
    let mut written = Vec::with_capacity(detail.artifacts.len());
    for artifact in &detail.artifacts {
        let segments = safe_segments(&artifact.name)?;
        let encoded: Vec<String> = segments.iter().map(|segment| encode(segment)).collect();
        let path = format!("/runs/{}/artifacts/{}", encode(run_id), encoded.join("/"));
        let response = client.get(&path)?;
        let digest = Digest::sha256(&response.body);
        if digest != artifact.sha256 || response.body.len() as u64 != artifact.bytes {
            return Err(RunError::Api {
                code: String::from("internal"),
                message: format!(
                    "artifact {} does not match the run record; the download was refused",
                    artifact.name
                ),
            });
        }
        let target = segments
            .iter()
            .fold(directory.to_path_buf(), |path, segment| path.join(segment));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| RunError::file(parent.display(), error))?;
        }
        fs::write(&target, &response.body)
            .map_err(|error| RunError::file(target.display(), error))?;
        written.push((
            artifact.name.clone(),
            artifact.bytes,
            artifact.sha256.to_string(),
            target.display().to_string(),
        ));
    }

    let view = json!({
        "run_id": detail.run_id.to_string(),
        "directory": directory.display().to_string(),
        "artifacts": written
            .iter()
            .map(|(name, bytes, digest, path)| json!({
                "name": name,
                "bytes": bytes,
                "sha256": digest,
                "path": path,
            }))
            .collect::<Vec<_>>(),
    });
    output::emit(globals.json, view, stdout, |writer| {
        output::fetched(
            writer,
            &detail.run_id.to_string(),
            &directory.display().to_string(),
            &written,
        )
    })
}

fn run_cancel(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/runs/{}/cancel", encode(run_id));
    let response = client.post_empty(&path)?;
    let detail: RunDetail = decode_json!(RunDetail, response, &path);
    output::emit(globals.json, json_view!(&detail), stdout, |writer| {
        output::run_detail(writer, &detail)
    })
}

fn inspect_candles(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/inspect/candles?data_version={}", encode(data_version));
    let response = client.get(&path)?;
    let summary: DatasetSummary = decode_json!(DatasetSummary, response, &path);
    output::emit(globals.json, json_view!(&summary), stdout, |writer| {
        output::dataset_summary(writer, &summary)
    })
}

fn inspect_timeframes(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
) -> Result<(), RunError> {
    let path = format!("/inspect/timeframes?data_version={}", encode(data_version));
    let response = client.get(&path)?;
    let inspection: TimeframeInspection = decode_json!(TimeframeInspection, response, &path);
    output::emit(globals.json, json_view!(&inspection), stdout, |writer| {
        output::timeframes(writer, &inspection)
    })
}

fn inspect_events(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
    timeframe: &str,
) -> Result<(), RunError> {
    let path = format!(
        "/inspect/events?data_version={}&timeframe={}",
        encode(data_version),
        encode(timeframe)
    );
    let response = client.get(&path)?;
    let events: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, events.clone(), stdout, |writer| {
        output::write_records(writer, &events)
    })
}

fn verify(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/runs/{}/digest", encode(run_id));
    let response = client.get(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    let digest = digest_from(&value).ok_or_else(|| RunError::Decode {
        path: path.clone(),
        message: String::from("the response carries no digest"),
    })?;
    let recorded_run = value
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or(run_id)
        .to_owned();
    let view = json!({
        "run_id": recorded_run.clone(),
        "digest": digest.clone(),
    });
    output::emit(globals.json, view, stdout, |writer| {
        output::digest(writer, &recorded_run, &digest)
    })
}

fn digest_from(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map.get("digest").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

fn profile_list(
    session: &Session,
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/profiles")?;
    let rows: Vec<ProfileSummary> = decode_json!(Vec<ProfileSummary>, response, "/profiles");
    let active = session.profile()?.to_string();
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::profiles(writer, &rows, &active)
    })
}

fn profile_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    name: &str,
    id: Option<&str>,
) -> Result<(), RunError> {
    let profile_id = match id {
        Some(id) => {
            ProfileId::parse(id).map_err(|error| RunError::Usage(format!("profile id {error}")))?
        }
        None => ProfileId::parse(&slug(name))
            .map_err(|error| RunError::Usage(format!("cannot derive a profile id: {error}")))?,
    };
    let request = CreateProfileRequest {
        profile_id,
        name: name.trim().to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the profile request", error))?;
    let response = client.post_json("/profiles", &body)?;
    let profile: ProfileSummary = decode_json!(ProfileSummary, response, "/profiles");
    let created = response.status == 201;
    output::emit(globals.json, json_view!(&profile), stdout, |writer| {
        writeln!(writer, "created: {created}")?;
        output::profile(writer, &profile)
    })
}

fn profile_use(
    session: &Session,
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    profile_id: &str,
) -> Result<(), RunError> {
    let profile = ProfileId::parse(profile_id)
        .map_err(|error| RunError::Usage(format!("profile id {error}")))?;
    let response = client.get("/profiles")?;
    let rows: Vec<ProfileSummary> = decode_json!(Vec<ProfileSummary>, response, "/profiles");
    if !rows.iter().any(|row| row.profile_id == profile) {
        return Err(RunError::Api {
            code: String::from("profile.not_found"),
            message: format!("profile {profile} is unknown"),
        });
    }
    session.store_default(&profile)?;
    let view = json!({ "active": profile.to_string() });
    output::emit(globals.json, view, stdout, |writer| {
        output::active_profile(writer, profile.as_str())
    })
}

fn simulation_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    status: Option<quantrule_api::SimulationStatus>,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(status) = status {
        query.push(format!("status={}", simulation_status(status)));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let path = with_query("/simulation/runs", &query);
    let response = client.get(&path)?;
    let rows: Vec<SimulationSummary> = decode_json!(Vec<SimulationSummary>, response, &path);
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::simulation_list(writer, &rows)
    })
}

fn simulation_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    premise: &str,
    bundle: &Digest,
    seed: u64,
    criteria: &str,
) -> Result<(), RunError> {
    let request = CreateSimulationRequest {
        premise: premise.to_owned(),
        bundle: bundle.clone(),
        seed,
        criteria: criteria.to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the simulation request", error))?;
    let response = client.post_json("/simulation/runs", &body)?;
    let value: Value = decode_json!(Value, response, "/simulation/runs");
    let sim_id = value
        .get("sim_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RunError::Decode {
            path: String::from("/simulation/runs"),
            message: String::from("the response carries no sim_id"),
        })?
        .to_owned();
    output::emit(globals.json, value, stdout, |writer| {
        writeln!(writer, "sim_id: {sim_id}")
    })
}

fn simulation_windows(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/windows", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

struct MarkInput<'a> {
    sim_id: &'a str,
    chart: &'a str,
    bar: u64,
    action: quantrule_api::MarkAction,
    quality: quantrule_api::MarkQuality,
    side: quantrule_api::Side,
    note: &'a str,
}

fn simulation_mark(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    mark: &MarkInput<'_>,
) -> Result<(), RunError> {
    let body = json!({
        "chart": mark.chart,
        "bar": mark.bar,
        "action": mark.action,
        "quality": mark.quality,
        "side": mark.side,
        "note": mark.note,
    })
    .to_string();
    let path = format!("/simulation/runs/{}/marks", encode(mark.sim_id));
    let response = client.post_json(&path, &body)?;
    let value: Value = decode_json!(Value, response, &path);
    let mark_id = value
        .get("mark_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RunError::Decode {
            path: path.clone(),
            message: String::from("the response carries no mark_id"),
        })?
        .to_owned();
    output::emit(globals.json, value, stdout, |writer| {
        writeln!(writer, "mark_id: {mark_id}")
    })
}

fn simulation_marks(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
    origin: Option<quantrule_api::MarkOrigin>,
) -> Result<(), RunError> {
    let path = match origin {
        Some(origin) => format!(
            "/simulation/runs/{}/marks?origin={}",
            encode(sim_id),
            mark_origin(origin)
        ),
        None => format!("/simulation/runs/{}/marks", encode(sim_id)),
    };
    let response = client.get(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

fn simulation_oracle(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/oracle", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

fn simulation_check(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/check", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_json_line(writer, &value)
    })
}

fn simulation_decide(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
    decision: &str,
    note: &str,
) -> Result<(), RunError> {
    let body = json!({ "decision": decision, "note": note }).to_string();
    let path = format!("/simulation/runs/{}/decision", encode(sim_id));
    let response = client.post_json(&path, &body)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_json_line(writer, &value)
    })
}

fn with_query(path: &str, query: &[String]) -> String {
    if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", query.join("&"))
    }
}

fn simulation_status(status: quantrule_api::SimulationStatus) -> &'static str {
    match status {
        quantrule_api::SimulationStatus::Draft => "draft",
        quantrule_api::SimulationStatus::Marking => "marking",
        quantrule_api::SimulationStatus::Reviewed => "reviewed",
        quantrule_api::SimulationStatus::Checked => "checked",
        quantrule_api::SimulationStatus::Decided => "decided",
    }
}

fn mark_origin(origin: quantrule_api::MarkOrigin) -> &'static str {
    match origin {
        quantrule_api::MarkOrigin::Human => "human",
        quantrule_api::MarkOrigin::Oracle => "oracle",
    }
}

fn safe_segments(name: &str) -> Result<Vec<&str>, RunError> {
    if name.is_empty() || name.starts_with('/') || name.contains('\\') {
        return Err(unsafe_name(name));
    }
    let segments: Vec<&str> = name.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == "." || *segment == "..")
    {
        return Err(unsafe_name(name));
    }
    Ok(segments)
}

fn unsafe_name(name: &str) -> RunError {
    RunError::Decode {
        path: String::from("a run artifact"),
        message: format!("the artifact name '{name}' is not a safe relative path"),
    }
}

fn slug(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut pending_hyphen = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            pending_hyphen = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            pending_hyphen = true;
        }
    }
    if slug.len() > 64 {
        slug.truncate(64);
        while slug.ends_with('-') {
            slug.pop();
        }
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::{safe_segments, slug};

    #[test]
    fn a_slug_is_lowercase_and_hyphenated() {
        assert_eq!(slug("Research 2026"), "research-2026");
        assert_eq!(slug("  --Alpha-- "), "alpha");
        assert_eq!(slug(""), "");
    }

    #[test]
    fn a_slug_is_capped_at_sixty_four_characters() {
        let slug = slug(&"a".repeat(80));
        assert_eq!(slug.len(), 64);
    }

    #[test]
    fn a_relative_artifact_path_is_accepted() {
        assert_eq!(
            safe_segments("tables/equity.csv").expect("safe"),
            vec!["tables", "equity.csv"]
        );
    }

    #[test]
    fn a_traversal_artifact_path_is_refused() {
        for name in ["../etc/passwd", "/etc/passwd", "a//b", "a/./b", "a\\b", ""] {
            assert!(safe_segments(name).is_err(), "{name}");
        }
    }
}
