//! Run submission, listing, detail, cancellation, artifacts, and digest.
//!
//! The run row is authoritative. Evidence validates the referenced dataset and
//! config through the data service, writes the content-addressed row, and
//! publishes `run.requested`. Artifact bytes are read by the content key the run
//! recorded; a name never selects a file.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use quantrule_api::{
    Digest, ErrorCode, RunDetail, RunDigest, RunState, RunSummary, SubmitRunRequest,
    SubmitRunResponse,
};
use quantrule_runtime::clock;
use quantrule_runtime::queue::{ROUTING_CANCELLED, ROUTING_REQUESTED};
use quantrule_service_api::RunManifest;
use serde::Deserialize;

use crate::routes::{ApiFailure, data_failure, failure, request_profile};
use crate::runs::{CancelOutcome, NewRun, derive};
use crate::state::EvidenceState;

/// The optional filters on a run list.
#[derive(Debug, Deserialize)]
pub(crate) struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
}

/// The model identity recorded in a run config, or `none` when absent.
fn model_version(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("model_id")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "none".to_owned())
}

pub(crate) async fn submit_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    payload: Result<Json<SubmitRunRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SubmitRunResponse>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a run submission"))?;

    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    match data
        .dataset_object(profile.clone(), request.data_version.to_string())
        .await
        .map_err(data_failure)?
    {
        Some(_) => {}
        None => {
            return Err(failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {} is unknown", request.data_version),
            ));
        }
    }

    let config = data
        .config_bundle(profile.clone(), request.config_id.clone())
        .await
        .map_err(data_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ConfigInvalid,
                format!("config {} is unknown", request.config_id),
            )
        })?;
    if !config.valid {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            config
                .message
                .unwrap_or_else(|| "the config is invalid".to_owned()),
        ));
    }
    let Some(config_hash) = config.config_hash.clone() else {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a valid config carries no hash",
        ));
    };

    let model_version = model_version(config.files.get("run.json").map_or("", String::as_str));
    let image_digest = state
        .version
        .image_digest
        .clone()
        .unwrap_or_else(|| Digest::sha256(b"no-image-digest"));
    let run_id = derive(
        &request.data_version,
        &config_hash,
        &model_version,
        &image_digest,
    );
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let outcome = runs
        .submit(
            profile.clone(),
            NewRun {
                run_id,
                config_id: request.config_id,
                data_version: request.data_version,
                config_hash,
                model_version,
                image_digest,
                owner: profile.as_str().to_owned(),
                submitted_at: clock::now_utc(),
            },
        )
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let record = outcome.record().clone();

    // Publish for a duplicate too: a retry after a lost publish must be able to
    // recover, and the worker treats a repeated message as a lookup. The
    // message carries the run id only; the worker reads the rest from the run
    // store, so a queued message cannot disagree with the recorded provenance.
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::QueueUnavailable, "no run queue is configured"))?;
    let message = serde_json::json!({ "run_id": record.run_id.to_string() });
    let payload = serde_json::to_vec(&message)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    queue
        .publish(ROUTING_REQUESTED, payload)
        .await
        .map_err(|error| failure(ErrorCode::QueueUnavailable, error.to_string()))?;

    let response = SubmitRunResponse {
        run_id: record.run_id,
        status: record.status,
        created: outcome.created(),
    };
    let status = if outcome.created() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(response)))
}

pub(crate) async fn list_runs(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    query: Result<Query<RunListQuery>, QueryRejection>,
) -> Result<Json<Vec<RunSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::RunInvalid, "the query is not a run filter"))?;
    let status = match query.status.as_deref() {
        None => None,
        Some(value) => Some(RunState::parse(value).ok_or_else(|| {
            failure(ErrorCode::RunInvalid, format!("unknown run status {value}"))
        })?),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let summaries = runs
        .list(profile, status, limit)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(summaries))
}

pub(crate) fn parse_run_id(id: &str) -> Result<String, ApiFailure> {
    Digest::parse(id)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::RunInvalid, error.to_string()))
}

pub(crate) async fn get_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunDetail>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.get(profile, id.clone()).await {
        Ok(Some(record)) => Ok(Json(record.detail())),
        Ok(None) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn cancel_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<RunDetail>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.cancel(profile, id.clone()).await {
        Ok(CancelOutcome::Cancelled(record)) => {
            // The run row is authoritative; the message is an observation for
            // the worker's queue. A publish failure must not undo the cancel.
            if let Some(queue) = state.queue.as_ref() {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "run_id": record.run_id.to_string(),
                }))
                .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
                if let Err(error) = queue.publish(ROUTING_CANCELLED, payload).await {
                    tracing::warn!(run_id = %record.run_id, %error, "cancel message was not published");
                }
            }
            Ok((StatusCode::ACCEPTED, Json(record.detail())))
        }
        Ok(CancelOutcome::Conflict(record)) => Err(failure(
            ErrorCode::RunConflict,
            format!(
                "run {} is already {}",
                record.run_id,
                record.status.as_str()
            ),
        )),
        Ok(CancelOutcome::Missing) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

/// Refuse an artifact name that is a path rather than a bare file name.
///
/// The bytes are read by the content key the run recorded, so a name can never
/// select a file. The check makes the refusal explicit instead of a lookup miss.
fn check_artifact_name(name: &str) -> Result<(), ApiFailure> {
    let refused = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.contains('\0');
    if refused {
        return Err(failure(
            ErrorCode::RunInvalid,
            "an artifact name is a bare file name",
        ));
    }
    Ok(())
}

fn artifact_content_type(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("json") => "application/json",
        Some("csv") => "text/csv",
        Some("ndjson") => "application/x-ndjson",
        Some("txt" | "log") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// Serve one artifact by the hash the run recorded.
pub(crate) async fn get_run_artifact(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    check_artifact_name(&name)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let record = match runs.get(profile, id.clone()).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Err(failure(
                ErrorCode::RunNotFound,
                format!("run {id} is unknown"),
            ));
        }
        Err(error) => return Err(failure(ErrorCode::Internal, error.to_string())),
    };
    let artifact = record
        .artifacts
        .iter()
        .find(|artifact| artifact.name == name)
        .ok_or_else(|| {
            failure(
                ErrorCode::RunNotFound,
                format!("run {id} has no artifact {name}"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&RunManifest::blob_key(&artifact.sha256))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if bytes.len() as u64 != artifact.bytes || Digest::sha256(&bytes) != artifact.sha256 {
        return Err(failure(
            ErrorCode::Internal,
            "the stored artifact does not match the run record",
        ));
    }
    let etag = format!("\"{}\"", artifact.sha256);
    Ok((
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                artifact_content_type(&name).to_owned(),
            ),
            (header::ETAG, etag),
        ],
        bytes,
    ))
}

/// The canonical digest of one run within the profile.
pub(crate) async fn get_run_digest(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunDigest>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.get(profile, id.clone()).await {
        Ok(Some(record)) => Ok(Json(RunDigest {
            run_id: record.run_id,
            digest: record.digest,
        })),
        Ok(None) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}
