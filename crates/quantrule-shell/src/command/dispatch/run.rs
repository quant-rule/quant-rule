//! The run command handlers.

use std::fs;
use std::io::Write;
use std::path::Path;

use quantrule_api::{Digest, RunDetail, RunState, RunSummary, SubmitRunRequest, SubmitRunResponse};
use serde_json::json;

use super::path::{safe_segments, with_query};
use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn run_submit(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    data_version: &str,
    config_id: &str,
    model_artifact_id: Option<&str>,
) -> Result<(), RunError> {
    let digest = Digest::parse(data_version)
        .map_err(|error| RunError::Usage(format!("data_version {error}")))?;
    let model_artifact_id = model_artifact_id
        .map(Digest::parse)
        .transpose()
        .map_err(|error| RunError::Usage(format!("model_artifact_id {error}")))?;
    let request = SubmitRunRequest {
        data_version: digest,
        config_id: config_id.to_owned(),
        model_artifact_id,
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

pub(super) fn run_status(
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

pub(super) fn run_list(
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

pub(super) fn run_fetch(
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

pub(super) fn run_cancel(
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
