//! The stage command executors.
//!
//! These handlers build one gateway call, decode the contract type, and hand it
//! to the renderer. They hold no model logic: `stage events` filters and formats
//! the published `events.csv` and reads coverage from `report.json`.

use std::fs;
use std::io::Write;
use std::path::Path;

use quantrule_api::{
    ArtifactRef, Digest, StageInput, StageKind, StageRunDetail, StageRunRequest, StageRunSummary,
    StageState, StageSubmitResponse,
};
use serde_json::{Value, json};

use crate::client::{Client, encode};
use crate::command::stage::stage_events;
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

macro_rules! decode {
    ($ty:ty, $response:expr, $path:expr) => {
        serde_json::from_slice::<$ty>(&$response.body)
            .map_err(|error| RunError::decode($path, error))
    };
}

macro_rules! json_view {
    ($value:expr) => {
        serde_json::to_value($value).map_err(|error| RunError::decode("the local JSON view", error))
    };
}

pub(crate) fn stage_run_model(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
    config_id: &str,
) -> Result<(), RunError> {
    let digest = Digest::parse(data_version)
        .map_err(|error| RunError::Usage(format!("data_version {error}")))?;
    let request = StageRunRequest {
        kind: StageKind::Model,
        config_id: config_id.to_owned(),
        input: StageInput::DataVersion {
            data_version: digest,
        },
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the stage submission", error))?;
    let response = client.post_json("/stages/model/runs", &body)?;
    let submitted: StageSubmitResponse =
        decode!(StageSubmitResponse, response, "/stages/model/runs")?;
    output::emit(globals.json, json_view!(&submitted)?, stdout, |writer| {
        output::stage_submitted(
            writer,
            &submitted.stage_run_id.to_string(),
            submitted.state.as_str(),
            submitted.created,
        )
    })
}

pub(crate) fn stage_run_strategy(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    model_artifact_id: &str,
    config_id: &str,
) -> Result<(), RunError> {
    let artifact = Digest::parse(model_artifact_id)
        .map_err(|error| RunError::Usage(format!("model_artifact_id {error}")))?;
    let request = StageRunRequest {
        kind: StageKind::Strategy,
        config_id: config_id.to_owned(),
        input: StageInput::ModelArtifact {
            model_artifact_id: artifact,
        },
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the stage submission", error))?;
    let response = client.post_json("/stages/strategy/runs", &body)?;
    let submitted: StageSubmitResponse =
        decode!(StageSubmitResponse, response, "/stages/strategy/runs")?;
    output::emit(globals.json, json_view!(&submitted)?, stdout, |writer| {
        output::stage_submitted(
            writer,
            &submitted.stage_run_id.to_string(),
            submitted.state.as_str(),
            submitted.created,
        )
    })
}

pub(crate) fn stage_explain(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
    bar: usize,
) -> Result<(), RunError> {
    let detail = stage_detail(client, stage_run_id)?;
    let artifact = detail.artifact.as_ref().ok_or_else(|| RunError::Api {
        code: String::from("stage.conflict"),
        message: format!("stage {stage_run_id} has not produced an artifact"),
    })?;
    let decisions = artifact
        .files
        .iter()
        .find(|file| file.name == "decisions.ndjson")
        .ok_or_else(|| RunError::Api {
            code: String::from("stage.not_found"),
            message: format!("stage {stage_run_id} has no decisions.ndjson artifact"),
        })?;
    let bytes = stage_artifact_bytes(client, stage_run_id, decisions)?;
    let text = std::str::from_utf8(&bytes).map_err(|error| RunError::Decode {
        path: String::from("decisions.ndjson"),
        message: error.to_string(),
    })?;
    let mut record: Option<Value> = None;
    for (index, line) in text.lines().filter(|line| !line.is_empty()).enumerate() {
        let value: Value = serde_json::from_str(line).map_err(|error| RunError::Decode {
            path: String::from("decisions.ndjson"),
            message: format!("line {}: {error}", index + 1),
        })?;
        if value.get("bar_index").and_then(Value::as_u64) == Some(bar as u64) {
            record = Some(value);
            break;
        }
    }
    let record = record.ok_or_else(|| RunError::Api {
        code: String::from("stage.not_found"),
        message: format!("stage {stage_run_id} has no decision for bar {bar}"),
    })?;
    output::emit(globals.json, record.clone(), stdout, |writer| {
        output::stage_explain(writer, stage_run_id, bar, &record)
    })
}

pub(crate) fn stage_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    kind: Option<StageKind>,
    status: Option<StageState>,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(kind) = kind {
        query.push(format!("kind={}", kind.as_str()));
    }
    if let Some(status) = status {
        query.push(format!("status={}", status.as_str()));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let path = with_query("/stages/runs", &query);
    let response = client.get(&path)?;
    let rows: Vec<StageRunSummary> = decode!(Vec<StageRunSummary>, response, &path)?;
    output::emit(globals.json, json_view!(&rows)?, stdout, |writer| {
        output::stage_list(writer, &rows)
    })
}

pub(crate) fn stage_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/stages/runs/{}", encode(stage_run_id));
    let response = client.get(&path)?;
    let detail: StageRunDetail = decode!(StageRunDetail, response, &path)?;
    output::emit(globals.json, json_view!(&detail)?, stdout, |writer| {
        output::stage_detail(writer, &detail)
    })
}

pub(crate) fn stage_events_command(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
    timeframe: &str,
) -> Result<(), RunError> {
    let detail = stage_detail(client, stage_run_id)?;
    let artifact = detail.artifact.as_ref().ok_or_else(|| RunError::Api {
        code: String::from("stage.conflict"),
        message: format!("stage {stage_run_id} has not produced an artifact"),
    })?;
    let events = artifact
        .files
        .iter()
        .find(|file| file.name == "events.csv")
        .ok_or_else(|| RunError::Api {
            code: String::from("stage.not_found"),
            message: format!("stage {stage_run_id} has no events.csv artifact"),
        })?;
    let events_bytes = stage_artifact_bytes(client, stage_run_id, events)?;
    let report_bytes = match artifact
        .files
        .iter()
        .find(|file| file.name == "report.json")
    {
        Some(report) => Some(stage_artifact_bytes(client, stage_run_id, report)?),
        None => None,
    };
    let view = stage_events(
        stage_run_id,
        timeframe,
        &events_bytes,
        report_bytes.as_deref(),
    )?;
    let value = view.to_value();
    output::emit(globals.json, value, stdout, |writer| {
        output::stage_events(writer, &view)
    })
}

pub(crate) fn stage_fetch(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
    directory: &Path,
) -> Result<(), RunError> {
    let detail = stage_detail(client, stage_run_id)?;
    let artifact = detail.artifact.as_ref().ok_or_else(|| RunError::Api {
        code: String::from("stage.conflict"),
        message: format!("stage {stage_run_id} has not produced an artifact"),
    })?;
    fs::create_dir_all(directory).map_err(|error| RunError::file(directory.display(), error))?;
    let mut files = artifact.files.clone();
    files.sort_by(|left, right| left.name.cmp(&right.name));
    let mut written = Vec::with_capacity(files.len());
    for file in &files {
        let segments = safe_segments(&file.name)?;
        let target = segments
            .iter()
            .fold(directory.to_path_buf(), |path, segment| path.join(segment));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| RunError::file(parent.display(), error))?;
        }
        let bytes = stage_artifact_bytes(client, stage_run_id, file)?;
        fs::write(&target, &bytes).map_err(|error| RunError::file(target.display(), error))?;
        written.push((
            file.name.clone(),
            file.bytes,
            file.sha256.to_string(),
            target.display().to_string(),
        ));
    }
    let view = json!({
        "stage_run_id": detail.stage_run_id.to_string(),
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
        output::stage_fetched(
            writer,
            &detail.stage_run_id.to_string(),
            &directory.display().to_string(),
            &written,
        )
    })
}

pub(crate) fn stage_cancel(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/stages/runs/{}/cancel", encode(stage_run_id));
    let response = client.post_empty(&path)?;
    let detail: StageRunDetail = decode!(StageRunDetail, response, &path)?;
    output::emit(globals.json, json_view!(&detail)?, stdout, |writer| {
        output::stage_detail(writer, &detail)
    })
}

pub(crate) fn stage_delete(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    stage_run_id: &str,
) -> Result<(), RunError> {
    let path = format!("/stages/runs/{}", encode(stage_run_id));
    client.delete_empty(&path)?;
    let view = json!({ "stage_run_id": stage_run_id });
    output::emit(globals.json, view, stdout, |writer| {
        output::stage_deleted(writer, stage_run_id)
    })
}

fn stage_detail(client: &Client, stage_run_id: &str) -> Result<StageRunDetail, RunError> {
    let path = format!("/stages/runs/{}", encode(stage_run_id));
    let response = client.get(&path)?;
    decode!(StageRunDetail, response, &path)
}

fn stage_artifact_bytes(
    client: &Client,
    stage_run_id: &str,
    artifact: &ArtifactRef,
) -> Result<Vec<u8>, RunError> {
    let segments = safe_segments(&artifact.name)?;
    let encoded: Vec<String> = segments.iter().map(|segment| encode(segment)).collect();
    let path = format!(
        "/stages/runs/{}/artifacts/{}",
        encode(stage_run_id),
        encoded.join("/")
    );
    let response = client.get(&path)?;
    if response.body.len() as u64 != artifact.bytes
        || Digest::sha256(&response.body) != artifact.sha256
    {
        return Err(RunError::Api {
            code: String::from("internal"),
            message: format!(
                "artifact {} does not match the stage record; the download was refused",
                artifact.name
            ),
        });
    }
    Ok(response.body)
}

fn with_query(path: &str, query: &[String]) -> String {
    if query.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", query.join("&"))
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
        path: String::from("a stage artifact"),
        message: format!("the artifact name '{name}' is not a safe relative path"),
    }
}
