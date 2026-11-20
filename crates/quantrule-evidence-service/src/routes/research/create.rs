//! Research create, list, show, and submit.

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Json, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::{
    CreateImprovementRequest, DatasetReferenceKind, EraseReceipt, ErrorCode, ImprovementRunDetail,
    ImprovementRunState, ImprovementRunSummary, SubmitRunRequest,
};
use serde::Deserialize;

use super::{
    date_of, ensure_partition_window, load, manifest, next_id, now, parse_state, parse_tier,
    refresh_digest, require_coupled, store, store_failure, validation_failure,
};
use crate::research::{ResearchArchiveOutcome, ResearchEraseOutcome};
use crate::routes::runs::{submit_validated_pipeline, validated_config};
use crate::routes::{ApiFailure, data_failure, failure, request_profile};
use crate::state::EvidenceState;

/// The optional filters on a research list.
#[derive(Debug, Deserialize)]
pub(crate) struct ResearchListQuery {
    tier: Option<String>,
    state: Option<String>,
    limit: Option<usize>,
    include_archived: Option<bool>,
}

/// Create one v3 pre-registration.
pub(crate) async fn create(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    payload: Result<Json<CreateImprovementRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ImprovementRunDetail>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::ResearchInvalid,
            "the body is not an improvement-run request",
        )
    })?;
    let store = store(&state)?;
    let created_utc = now()?;
    let date = date_of(&created_utc);

    let submit = SubmitRunRequest {
        data_version: request.data_version.clone(),
        config_id: request.config_id.clone(),
        model_artifact_id: None,
    };
    let config = validated_config(&state, &profile, &submit)
        .await
        .map_err(validation_failure)?;
    require_coupled(&config).map_err(|message| failure(ErrorCode::ResearchInvalid, message))?;
    ensure_partition_window(
        &state,
        &profile,
        &config,
        request.partition,
        &request.data_version,
    )
    .await?;

    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    for _ in 0..8 {
        let improvement_id = next_id(store.as_ref(), &profile, request.tier, &date).await?;
        let manifest = manifest(&request, improvement_id.clone(), created_utc.clone());
        manifest
            .validate()
            .map_err(|error| failure(ErrorCode::ResearchInvalid, error.to_string()))?;
        let reference_id = format!("research:{improvement_id}");
        let _gate = state.history_gate.lock().await;
        data.register_dataset_reference(
            profile.clone(),
            request.data_version.clone(),
            reference_id.clone(),
            DatasetReferenceKind::Research,
        )
        .await
        .map_err(data_failure)?;
        match store
            .create(profile.clone(), manifest, created_utc.clone())
            .await
        {
            Ok(outcome) => {
                let status = if outcome.created() {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                };
                return Ok((status, Json(outcome.record().clone())));
            }
            Err(error) if error.to_string().contains("already") => {
                let _ = data
                    .release_dataset_reference(
                        profile.clone(),
                        request.data_version.clone(),
                        reference_id,
                    )
                    .await;
                continue;
            }
            Err(error) => {
                let _ = data
                    .release_dataset_reference(
                        profile.clone(),
                        request.data_version.clone(),
                        reference_id,
                    )
                    .await;
                return Err(store_failure(error));
            }
        }
    }
    Err(failure(
        ErrorCode::ResearchConflict,
        "the improvement-id sequence could not be allocated",
    ))
}

/// List research runs, newest first.
pub(crate) async fn list(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    query: Result<Query<ResearchListQuery>, QueryRejection>,
) -> Result<Json<Vec<ImprovementRunSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::ResearchInvalid,
            "the query is not an improvement-run filter",
        )
    })?;
    let tier = query.tier.as_deref().map(parse_tier).transpose()?;
    let state_filter = query.state.as_deref().map(parse_state).transpose()?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let store = store(&state)?;
    let rows = if query.include_archived.unwrap_or(false) {
        store.list_including_archived(profile, 1_000).await
    } else {
        store.list(profile, 1_000).await
    }
    .map_err(store_failure)?;
    let rows = rows
        .into_iter()
        .filter(|row| tier.is_none_or(|wanted| wanted == row.tier))
        .filter(|row| state_filter.is_none_or(|wanted| wanted == row.state))
        .take(limit)
        .collect();
    Ok(Json(rows))
}

/// Read one research run and lazily link its successful digest.
pub(crate) async fn show(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ImprovementRunDetail>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let detail = load(store.as_ref(), &profile, &id).await?;
    let detail = refresh_digest(&state, store.as_ref(), &profile, detail).await?;
    Ok(Json(detail))
}

/// Submit the coupled pipeline run for a created research record.
pub(crate) async fn submit(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<ImprovementRunDetail>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let detail = load(store.as_ref(), &profile, &id).await?;
    if detail.state != ImprovementRunState::Created {
        return Err(failure(
            ErrorCode::ResearchConflict,
            format!(
                "improvement run {} is {} and cannot be submitted again",
                detail.improvement_id,
                detail.state.as_str()
            ),
        ));
    }
    let request = SubmitRunRequest {
        data_version: detail.manifest.data_version.clone(),
        config_id: detail.manifest.config_id.clone(),
        model_artifact_id: None,
    };
    let config = validated_config(&state, &profile, &request)
        .await
        .map_err(validation_failure)?;
    require_coupled(&config).map_err(|message| failure(ErrorCode::ResearchInvalid, message))?;
    ensure_partition_window(
        &state,
        &profile,
        &config,
        detail.manifest.partition,
        &detail.manifest.data_version,
    )
    .await?;
    let (response, record) = submit_validated_pipeline(&state, &profile, request, config)
        .await
        .map_err(validation_failure)?;
    let detail = store
        .link_run(
            profile.clone(),
            detail.improvement_id.clone(),
            response.run_id,
            now()?,
        )
        .await
        .map_err(store_failure)?;
    let detail = if record.status == quantrule_api::RunState::Succeeded {
        if let Some(digest) = record.digest {
            match store
                .link_digest(profile.clone(), detail.improvement_id.clone(), digest)
                .await
            {
                Ok(updated) => updated,
                Err(error) if error.to_string().contains("already") => {
                    load(store.as_ref(), &profile, &detail.improvement_id).await?
                }
                Err(error) => return Err(store_failure(error)),
            }
        } else {
            detail
        }
    } else {
        detail
    };
    Ok((StatusCode::ACCEPTED, Json(detail)))
}

/// The typed confirmation required by a research erase.
#[derive(Debug, Deserialize)]
pub(crate) struct EraseResearchQuery {
    confirm: Option<String>,
}

pub(crate) async fn archive(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ImprovementRunSummary>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let archived_at = now()?;
    let _gate = state.history_gate.lock().await;
    match store
        .archive(profile, id, archived_at)
        .await
        .map_err(store_failure)?
    {
        ResearchArchiveOutcome::Archived(summary) => Ok(Json(summary)),
        ResearchArchiveOutcome::Missing => Err(failure(
            ErrorCode::ResearchNotFound,
            "the research record is unknown",
        )),
        ResearchArchiveOutcome::AlreadyArchived => Err(failure(
            ErrorCode::HistoryState,
            "the research record is already archived",
        )),
    }
}

pub(crate) async fn erase(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<EraseResearchQuery>, QueryRejection>,
) -> Result<Json<EraseReceipt>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::ResearchInvalid,
            "the query is not an erase confirmation",
        )
    })?;
    if query.confirm.as_deref() != Some(id.as_str()) {
        return Err(failure(
            ErrorCode::HistoryState,
            "erase requires --confirm with the exact research id",
        ));
    }
    let _gate = state.history_gate.lock().await;
    match store
        .erase(profile.clone(), id.clone())
        .await
        .map_err(store_failure)?
    {
        ResearchEraseOutcome::Erased(receipt) => {
            if let Err(error) = data
                .release_dataset_reference(
                    profile,
                    receipt.data_version,
                    format!("research:{}", receipt.improvement_id),
                )
                .await
            {
                tracing::warn!(
                    improvement_id = %receipt.improvement_id,
                    %error,
                    "erased research record left its dataset reference behind"
                );
            }
            Ok(Json(EraseReceipt {
                record_id: id,
                bytes: 0,
            }))
        }
        ResearchEraseOutcome::Missing => Err(failure(
            ErrorCode::ResearchNotFound,
            "the research record is unknown",
        )),
        ResearchEraseOutcome::NotArchived => Err(failure(
            ErrorCode::HistoryState,
            "archive the research record before erasing it",
        )),
        ResearchEraseOutcome::Protected => Err(failure(
            ErrorCode::HistoryProtected,
            "a decided research record is evidence and cannot be erased",
        )),
    }
}
