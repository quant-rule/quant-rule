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
    Digest, ErrorCode, ProfileId, RunDetail, RunDigest, RunState, RunSummary, StageKind,
    SubmitRunRequest, SubmitRunResponse,
};
use quantrule_runtime::clock;
use quantrule_runtime::queue::{ROUTING_CANCELLED, ROUTING_REQUESTED};
use quantrule_service_api::{ConfigBundle, RunManifest};
use serde::Deserialize;

use crate::routes::{ApiFailure, data_failure, failure, request_profile};
use crate::runs::{CancelOutcome, NewRun, RunRecord, derive_with_model};
use crate::state::EvidenceState;

/// The optional filters on a run list.
#[derive(Debug, Deserialize)]
pub(crate) struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
}

/// The model identity recorded in a run config, or `none` when absent.
pub(crate) fn model_version(body: &str) -> String {
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

/// Validate the dataset and configuration referenced by a run submission.
///
/// A missing dataset, an invalid bundle, or a bundle with no hash is the
/// caller's error. The route that owns the command maps it to its own stable
/// code; the run submission path uses the run codes directly.
pub(crate) async fn validated_config(
    state: &EvidenceState,
    profile: &ProfileId,
    request: &SubmitRunRequest,
) -> Result<ConfigBundle, ApiFailure> {
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
    if config.config_hash.is_none() {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a valid config carries no hash",
        ));
    }
    Ok(config)
}

/// Submit a pipeline run whose dataset and configuration are already validated.
///
/// The research route validates the coupled config once, then calls this helper;
/// the public run route validates and calls it in one step.
pub(crate) async fn submit_validated_pipeline(
    state: &EvidenceState,
    profile: &ProfileId,
    request: SubmitRunRequest,
    config: ConfigBundle,
) -> Result<(SubmitRunResponse, RunRecord), ApiFailure> {
    let Some(config_hash) = config.config_hash.clone() else {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a valid config carries no hash",
        ));
    };

    let model_version = model_version(config.files.get("run.json").map_or("", String::as_str));
    // A run identity names the build that executed it. If the server has no
    // image digest, refuse the submission instead of recording a synthetic
    // identity that two different builds would share.
    let Some(image_digest) = state.version.image_digest.clone() else {
        return Err(failure(
            ErrorCode::Internal,
            "the server has no image digest; set QR_IMAGE_DIGEST to the image digest",
        ));
    };
    let model_artifact_id = request.model_artifact_id.clone();
    if let Some(artifact_id) = &model_artifact_id {
        let stages = state.stages.as_ref().ok_or_else(|| {
            failure(
                ErrorCode::Internal,
                "no stage store is configured for a promoted run",
            )
        })?;
        let stage = stages
            .find_by_artifact(profile.clone(), StageKind::Model, artifact_id.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
            .ok_or_else(|| {
                failure(
                    ErrorCode::RunModelArtifactInvalid,
                    format!(
                        "no succeeded model stage publishes artifact {artifact_id} in this profile"
                    ),
                )
            })?;
        if stage.provenance.data_version != request.data_version {
            return Err(failure(
                ErrorCode::RunModelArtifactInvalid,
                "the model artifact belongs to another dataset",
            ));
        }
        if stage.provenance.config_hash != config_hash {
            return Err(failure(
                ErrorCode::RunModelArtifactInvalid,
                "the model artifact belongs to another config hash",
            ));
        }
        if stage.provenance.model_version != model_version {
            return Err(failure(
                ErrorCode::RunModelArtifactInvalid,
                "the model artifact belongs to another model version",
            ));
        }
    }
    let run_id = derive_with_model(
        &request.data_version,
        &config_hash,
        &model_version,
        &image_digest,
        model_artifact_id.as_ref(),
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
                model_artifact_id,
                owner: profile.as_str().to_owned(),
                submitted_at: clock::now_utc()
                    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
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
        run_id: record.run_id.clone(),
        status: record.status,
        created: outcome.created(),
    };
    Ok((response, record))
}

/// Validate and submit a coupled pipeline run for a profile.
pub(crate) async fn submit_pipeline_run(
    state: &EvidenceState,
    profile: &ProfileId,
    request: SubmitRunRequest,
) -> Result<(SubmitRunResponse, RunRecord), ApiFailure> {
    let config = validated_config(state, profile, &request).await?;
    submit_validated_pipeline(state, profile, request, config).await
}

pub(crate) async fn submit_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    payload: Result<Json<SubmitRunRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SubmitRunResponse>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a run submission"))?;
    let (response, _) = submit_pipeline_run(&state, &profile, request).await?;
    let status = if response.created {
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

/// Refuse an artifact name that is not a safe relative path.
///
/// The name is a lookup key, not a filesystem path, but a nested artifact name
/// such as `execution/fills.csv` is a valid key. Empty segments, `.`, `..`, a
/// leading slash, a backslash, and NUL are refused so a name can never escape
/// the artifact set or select a file outside it.
pub(crate) fn check_artifact_name(name: &str) -> Result<(), &'static str> {
    let refused = name.is_empty()
        || name.starts_with('/')
        || name.contains('\\')
        || name.contains('\0')
        || name
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..");
    if refused {
        return Err(
            "an artifact name is a relative path without empty, parent, or absolute segments",
        );
    }
    Ok(())
}

pub(crate) fn artifact_content_type(name: &str) -> &'static str {
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
    check_artifact_name(&name).map_err(|message| failure(ErrorCode::RunInvalid, message))?;
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
