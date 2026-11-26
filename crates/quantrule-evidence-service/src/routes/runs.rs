//! Run submission, listing, detail, cancellation, artifacts, and digest.
//!
//! The run row is authoritative. Evidence validates the referenced dataset and
//! config through the data service, writes the content-addressed row, and the
//! same transaction records the `run.requested` intent in `runs.outbox`. The
//! publisher delivers that intent; a broker that is unavailable delays the
//! message instead of losing the run. Artifact bytes are read by the content
//! key the run recorded; a name never selects a file.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use quantrule_api::{
    DatasetReferenceKind, Digest, EraseReceipt, ErrorCode, ProfileId, ReferenceKind, RunDetail,
    RunDigest, RunState, RunSummary, StageKind, SubmitRunRequest, SubmitRunResponse,
};
use quantrule_runtime::clock;
use quantrule_runtime::queue::ROUTING_CANCELLED;
use quantrule_service_api::{ConfigBundle, RunManifest};
use serde::Deserialize;

use crate::lineage::normalize;
use crate::routes::{ApiFailure, data_failure, failure, request_profile};
use crate::runs::{
    CancelOutcome, DatasetReferenceRelease, NewRun, RunArchiveOutcome, RunEraseOutcome, RunRecord,
    derive_with_model,
};
use crate::state::EvidenceState;

/// The optional filters on a run list.
#[derive(Debug, Deserialize)]
pub(crate) struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
    include_archived: Option<bool>,
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
    if !config.references.is_empty() {
        normalize(config.references.clone())
            .map_err(|error| failure(ErrorCode::RunInvalid, error.to_string()))?;
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
    let schema = config
        .files
        .get("run.json")
        .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok())
        .and_then(|value| {
            value
                .get("schema")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    if schema.as_deref() == Some("quantrule.run/v3") {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a coupled run requires quantrule.run/v4 with a strategy",
        ));
    }
    let Some(config_hash) = config.config_hash.clone() else {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a valid config carries no hash",
        ));
    };
    if !config.references.is_empty() {
        let v5_model = config
            .references
            .iter()
            .find(|reference| reference.kind == ReferenceKind::ModelArtifact)
            .map(|reference| reference.id.clone());
        match (&request.model_artifact_id, &v5_model) {
            (Some(request_model), Some(v5_model)) if request_model != v5_model => {
                return Err(failure(
                    ErrorCode::RunModelArtifactInvalid,
                    "the v5 model artifact must match the submitted model artifact id",
                ));
            }
            (None, Some(_)) => {
                return Err(failure(
                    ErrorCode::RunModelArtifactInvalid,
                    "the v5 config cites a model artifact the submission does not name",
                ));
            }
            (Some(_), None) => {
                return Err(failure(
                    ErrorCode::RunModelArtifactInvalid,
                    "the v5 config does not cite the submitted model artifact",
                ));
            }
            _ => {}
        }
    }

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
        // The full config hash must not invalidate a strategy cell: the
        // worker compares the frozen model bundle's model-input identity to
        // the submitted config before reuse.
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
    let reference_id = format!("run:{run_id}");
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let outbox = state
        .outbox
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run outbox is configured"))?;
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::QueueUnavailable, "no run queue is configured"))?;
    // The dataset reference is registered before the run row exists. A crash
    // between the two leaves a blocking reference, not an unprotected dataset,
    // and the registration is idempotent on a retry.
    let data_version = request.data_version.clone();
    let outcome = {
        let _gate = state.history_gate.lock().await;
        data.register_dataset_reference(
            profile.clone(),
            request.data_version.clone(),
            reference_id.clone(),
            DatasetReferenceKind::Run,
        )
        .await
        .map_err(data_failure)?;
        match runs
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
        {
            Ok(outcome) => outcome,
            Err(error) => {
                if let Some(releases) = state.releases.as_ref() {
                    let release = DatasetReferenceRelease {
                        profile: profile.clone(),
                        data_version: data_version.clone(),
                        reference_id: reference_id.clone(),
                        kind: DatasetReferenceKind::Run,
                    };
                    if let Err(log) = releases.record(release).await {
                        tracing::warn!(%log, "the failed submission's release was not recorded");
                    }
                }
                if let Err(release) = data
                    .release_dataset_reference(profile.clone(), data_version, reference_id)
                    .await
                {
                    tracing::warn!(%release, "run submission failed and its dataset reference was not released");
                }
                return Err(failure(ErrorCode::Internal, error.to_string()));
            }
        }
    };
    let record = outcome.record().clone();

    // The run row and its intent committed together. A publish failure leaves
    // the intent pending for the reconciler, so the run is not lost and a
    // repeated submission still finds the same run record.
    if let Err(error) =
        crate::outbox::drain(outbox.as_ref(), queue.as_ref(), crate::outbox::DRAIN_LIMIT).await
    {
        tracing::warn!(run_id = %record.run_id, %error, "the run notification will be retried");
    }

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
    let summaries = if query.include_archived.unwrap_or(false) {
        runs.list_including_archived(profile, status, limit).await
    } else {
        runs.list(profile, status, limit).await
    }
    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(summaries))
}

/// The typed confirmation required by a run erase.
#[derive(Debug, Deserialize)]
pub(crate) struct EraseRunQuery {
    confirm: Option<String>,
}

pub(crate) async fn archive_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunSummary>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let _gate = state.history_gate.lock().await;
    let archived_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    match runs
        .archive(profile, id, archived_at)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    {
        RunArchiveOutcome::Archived(summary) => Ok(Json(summary)),
        RunArchiveOutcome::Missing => Err(failure(
            ErrorCode::RunNotFound,
            "the run is unknown in this profile",
        )),
        RunArchiveOutcome::AlreadyArchived => Err(failure(
            ErrorCode::HistoryState,
            "the run is already archived",
        )),
    }
}

pub(crate) async fn erase_run(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<EraseRunQuery>, QueryRejection>,
) -> Result<Json<EraseReceipt>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_run_id(&id)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::RunInvalid,
            "the query is not an erase confirmation",
        )
    })?;
    if query.confirm.as_deref() != Some(id.as_str()) {
        return Err(failure(
            ErrorCode::HistoryState,
            "erase requires --confirm with the exact run id",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let releases = state.releases.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::Internal,
            "no reference release log is configured",
        )
    })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let _gate = state.history_gate.lock().await;
    let outcome = runs
        .erase(profile.clone(), id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let receipt = match outcome {
        RunEraseOutcome::Erased(receipt) => receipt,
        RunEraseOutcome::Missing => {
            return Err(failure(
                ErrorCode::RunNotFound,
                "the run is unknown in this profile",
            ));
        }
        RunEraseOutcome::NotArchived => {
            return Err(failure(
                ErrorCode::HistoryState,
                "archive the run before erasing it",
            ));
        }
        RunEraseOutcome::Protected(status) => {
            return Err(failure(
                ErrorCode::HistoryProtected,
                format!("a {} run is evidence and cannot be erased", status.as_str()),
            ));
        }
    };
    if receipt.removed_run {
        if let Err(error) = objects.delete(&RunManifest::key(&id)).await {
            tracing::warn!(run_id = %id, %error, "erased run left its manifest behind");
        }
        for blob in &receipt.blobs {
            if let Err(error) = objects.delete(&RunManifest::blob_key(blob)).await {
                tracing::warn!(run_id = %id, %error, "erased run left a blob behind");
            }
        }
    }
    // The store wrote the release tombstone in the erase transaction. Retrying
    // here is best effort: if the data service is unavailable, the tombstone
    // stays pending for the reconciler and the erase stands.
    if let Err(error) = crate::reconcile::release_all(releases.as_ref(), data.as_ref()).await {
        tracing::warn!(run_id = %id, %error, "the erased run's dataset reference will be retried");
    }
    Ok(Json(EraseReceipt {
        record_id: id,
        bytes: receipt.bytes,
    }))
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
