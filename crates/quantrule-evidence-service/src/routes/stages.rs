//! Stage submission, listing, detail, artifacts, cancellation, and cleanup.
//!
//! A stage run executes one engine outside the coupled run. The public surface
//! is profile-scoped and diagnostic: the record always projects `evidence:
//! false`, it has no canonical digest, and a stage id is never readable through
//! a run route. The route validates the referenced config and input through the
//! data service, derives the stage id from the stage contract, writes the row,
//! and publishes `stage.requested`.
//!
//! The submission carries only references. It never carries a candle, a
//! reading, a decision row, or a client path.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use quantrule_api::{
    Digest, ErrorCode, StageArtifact, StageInput, StageKind, StageProvenance, StageRunDetail,
    StageRunSummary, StageState, StageSubmitResponse,
};
use quantrule_runtime::clock;
use quantrule_runtime::queue::{ROUTING_STAGE_CANCELLED, ROUTING_STAGE_REQUESTED};
use quantrule_service_api::{NewStageRun, StageManifest};
use serde::Deserialize;

use crate::routes::runs::{artifact_content_type, check_artifact_name, model_version};
use crate::routes::{ApiFailure, data_failure, failure, request_profile};
use crate::stage::{StageCancelOutcome, StageCreateOutcome, StageStore, retention};
use crate::state::EvidenceState;

/// A stage submission: a [`StageRunRequest`](quantrule_api::StageRunRequest) and
/// the provenance fields a later stage must resolve from its frozen input.
///
/// A model stage derives `data_version` from its dataset input. A strategy or
/// portfolio stage names the dataset the frozen upstream artifact records; that
/// value cannot be read until the vertical slice lands, so the caller supplies
/// it and the route validates it through the data service. Every other
/// provenance field is derived by the server; a supplied value is accepted only
/// when it equals the derived one.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StageSubmitBody {
    kind: StageKind,
    config_id: String,
    input: StageInput,
    #[serde(default)]
    data_version: Option<Digest>,
    #[serde(default)]
    config_hash: Option<Digest>,
    #[serde(default)]
    model_version: Option<String>,
    #[serde(default)]
    image_digest: Option<Digest>,
    #[serde(default)]
    provenance: Option<StageProvenance>,
}

/// The optional filters on a stage list.
#[derive(Debug, Deserialize)]
pub(crate) struct StageListQuery {
    kind: Option<String>,
    status: Option<String>,
    limit: Option<usize>,
}

fn parse_stage_id(id: &str) -> Result<String, ApiFailure> {
    Digest::parse(id)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::StageInvalid, error.to_string()))
}

fn parse_kind(value: &str) -> Result<StageKind, ApiFailure> {
    StageKind::parse(value).ok_or_else(|| {
        failure(
            ErrorCode::StageInvalid,
            format!("unknown stage kind {value}"),
        )
    })
}

fn supplied<T: PartialEq>(value: Option<T>, expected: &T, field: &str) -> Result<(), ApiFailure> {
    if value.is_some_and(|value| &value != expected) {
        return Err(failure(
            ErrorCode::StageInvalid,
            format!("the supplied {field} does not match the one the server derived"),
        ));
    }
    Ok(())
}

fn resolve_data_version(
    body: &StageSubmitBody,
    kind: StageKind,
    provenance: Option<&StageProvenance>,
) -> Option<Digest> {
    let supplied = body
        .data_version
        .clone()
        .or_else(|| provenance.map(|value| value.data_version.clone()));
    match kind {
        StageKind::Model => None,
        StageKind::Strategy | StageKind::Portfolio => supplied,
    }
}

async fn validate_dataset(
    state: &EvidenceState,
    profile: quantrule_api::ProfileId,
    data_version: &Digest,
) -> Result<(), ApiFailure> {
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    match data
        .dataset_object(profile, data_version.to_string())
        .await
        .map_err(data_failure)?
    {
        Some(_) => Ok(()),
        None => Err(failure(
            ErrorCode::DatasetNotFound,
            format!("dataset {data_version} is unknown"),
        )),
    }
}

pub(crate) async fn submit_stage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(kind): Path<String>,
    payload: Result<Json<StageSubmitBody>, JsonRejection>,
) -> Result<(StatusCode, Json<StageSubmitResponse>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let Json(body) = payload.map_err(|_| {
        failure(
            ErrorCode::StageInvalid,
            "the body is not a stage submission",
        )
    })?;
    if body.kind != kind {
        return Err(failure(
            ErrorCode::StageInvalid,
            "the body kind does not match the path",
        ));
    }
    if !body.input.matches(kind) {
        return Err(failure(
            ErrorCode::StageInvalid,
            format!(
                "a {} stage cannot consume a {} input",
                kind.as_str(),
                body.input.label()
            ),
        ));
    }

    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let config = data
        .config_bundle(profile.clone(), body.config_id.clone())
        .await
        .map_err(data_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ConfigInvalid,
                format!("config {} is unknown", body.config_id),
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

    supplied(body.config_hash.clone(), &config_hash, "config_hash")?;
    supplied(body.model_version.clone(), &model_version, "model_version")?;
    supplied(body.image_digest.clone(), &image_digest, "image_digest")?;
    if let Some(provenance) = &body.provenance {
        supplied(
            Some(provenance.config_hash.clone()),
            &config_hash,
            "config_hash",
        )?;
        supplied(
            Some(provenance.model_version.clone()),
            &model_version,
            "model_version",
        )?;
        supplied(
            Some(provenance.image_digest.clone()),
            &image_digest,
            "image_digest",
        )?;
    }

    let data_version = match kind {
        StageKind::Model => body.input.digest().clone(),
        StageKind::Strategy | StageKind::Portfolio => {
            resolve_data_version(&body, kind, body.provenance.as_ref()).ok_or_else(|| {
                failure(
                    ErrorCode::StageInvalid,
                    "a strategy or portfolio stage must name the dataset its frozen input records",
                )
            })?
        }
    };
    if let Some(provenance) = &body.provenance
        && provenance.data_version != data_version
    {
        return Err(failure(
            ErrorCode::StageInvalid,
            "the supplied data_version disagrees with the stage input",
        ));
    }
    if kind == StageKind::Model {
        supplied(body.data_version.clone(), &data_version, "data_version")?;
        supplied(
            body.provenance
                .as_ref()
                .map(|value| value.data_version.clone()),
            &data_version,
            "data_version",
        )?;
    }
    validate_dataset(&state, profile.clone(), &data_version).await?;

    let provenance = StageProvenance {
        data_version,
        config_hash,
        model_version,
        image_digest,
    };
    let submitted_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let run = NewStageRun::new(
        kind,
        body.config_id,
        body.input,
        provenance,
        profile.as_str().to_owned(),
        submitted_at,
    );
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    let outcome = stages
        .create(profile, run)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let record = outcome.record().clone();

    // Publish for a duplicate too: a retry after a lost publish must be able to
    // recover, and the worker treats a repeated message as a lookup. The payload
    // carries the stage run id only.
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::QueueUnavailable, "no stage queue is configured"))?;
    let message = serde_json::json!({ "stage_run_id": record.stage_run_id.to_string() });
    let payload = serde_json::to_vec(&message)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    queue
        .publish(ROUTING_STAGE_REQUESTED, payload)
        .await
        .map_err(|error| failure(ErrorCode::QueueUnavailable, error.to_string()))?;

    let response = StageSubmitResponse {
        stage_run_id: record.stage_run_id,
        state: record.state,
        created: outcome.created(),
    };
    let status = match outcome {
        StageCreateOutcome::Created(_) => StatusCode::ACCEPTED,
        StageCreateOutcome::Existing(_) => StatusCode::OK,
    };
    Ok((status, Json(response)))
}

pub(crate) async fn list_stages(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    query: Result<Query<StageListQuery>, QueryRejection>,
) -> Result<Json<Vec<StageRunSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::StageInvalid, "the query is not a stage filter"))?;
    let kind = query.kind.as_deref().map(parse_kind).transpose()?;
    let status = query
        .status
        .as_deref()
        .map(|value| {
            StageState::parse(value).ok_or_else(|| {
                failure(
                    ErrorCode::StageInvalid,
                    format!("unknown stage status {value}"),
                )
            })
        })
        .transpose()?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    let summaries = stages
        .list(profile, kind, status, limit)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(summaries))
}

pub(crate) async fn get_stage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<StageRunDetail>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_stage_id(&id)?;
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    match stages.get(profile, id.clone()).await {
        Ok(Some(record)) => Ok(Json(record.detail())),
        Ok(None) => Err(failure(
            ErrorCode::StageNotFound,
            format!("stage {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn get_stage_artifacts(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<StageArtifact>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_stage_id(&id)?;
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    match stages.get(profile, id.clone()).await {
        Ok(Some(record)) => record.artifact.map(Json).ok_or_else(|| {
            failure(
                ErrorCode::StageConflict,
                format!("stage {id} has not produced an artifact"),
            )
        }),
        Ok(None) => Err(failure(
            ErrorCode::StageNotFound,
            format!("stage {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn get_stage_artifact(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path((id, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_stage_id(&id)?;
    check_artifact_name(&name).map_err(|message| failure(ErrorCode::StageInvalid, message))?;
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    let record = match stages.get(profile, id.clone()).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Err(failure(
                ErrorCode::StageNotFound,
                format!("stage {id} is unknown"),
            ));
        }
        Err(error) => return Err(failure(ErrorCode::Internal, error.to_string())),
    };
    let artifact = record.artifact.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::StageConflict,
            format!("stage {id} has not produced an artifact"),
        )
    })?;
    let file = artifact
        .files
        .iter()
        .find(|file| file.name == name)
        .ok_or_else(|| {
            failure(
                ErrorCode::StageNotFound,
                format!("stage {id} has no artifact {name}"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&StageManifest::blob_key(&file.sha256))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if bytes.len() as u64 != file.bytes || Digest::sha256(&bytes) != file.sha256 {
        return Err(failure(
            ErrorCode::Internal,
            "the stored stage artifact does not match the stage record",
        ));
    }
    let etag = format!("\"{}\"", file.sha256);
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

pub(crate) async fn cancel_stage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<StageRunDetail>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_stage_id(&id)?;
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    match stages.cancel(profile, id.clone()).await {
        Ok(StageCancelOutcome::Cancelled(record)) => {
            // The stage row is authoritative; the message is an observation for
            // the worker's queue. A publish failure must not undo the cancel.
            if let Some(queue) = state.queue.as_ref() {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "stage_run_id": record.stage_run_id.to_string(),
                }))
                .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
                if let Err(error) = queue.publish(ROUTING_STAGE_CANCELLED, payload).await {
                    tracing::warn!(stage_run_id = %record.stage_run_id, %error, "stage cancel message was not published");
                }
            }
            Ok((StatusCode::ACCEPTED, Json(record.detail())))
        }
        Ok(StageCancelOutcome::Conflict(record)) => Err(failure(
            ErrorCode::StageConflict,
            format!(
                "stage {} is already {}",
                record.stage_run_id,
                record.state.as_str()
            ),
        )),
        Ok(StageCancelOutcome::Missing) => Err(failure(
            ErrorCode::StageNotFound,
            format!("stage {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn delete_stage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_stage_id(&id)?;
    let stages: &Arc<dyn StageStore> = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    let deletion = stages
        .delete(profile, id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .ok_or_else(|| failure(ErrorCode::StageNotFound, format!("stage {id} is unknown")))?;
    if let Some(objects) = state.objects.as_ref()
        && let Err(error) =
            retention::remove_deleted(stages.as_ref(), objects.as_ref(), &deletion).await
    {
        tracing::warn!(stage_run_id = %deletion.stage_run_id, %error, "stage objects were not fully deleted");
    }
    // A cancelled publish is best effort: the row is gone, so a pending worker
    // delivery finds nothing and stops.
    if let Some(queue) = state.queue.as_ref() {
        let payload = serde_json::json!({
            "stage_run_id": deletion.stage_run_id.to_string(),
        });
        if let Ok(payload) = serde_json::to_vec(&payload)
            && let Err(error) = queue.publish(ROUTING_STAGE_CANCELLED, payload).await
        {
            tracing::warn!(stage_run_id = %deletion.stage_run_id, %error, "stage cleanup message was not published");
        }
    }
    Ok(StatusCode::NO_CONTENT)
}
