//! The worker's internal stage lifecycle.
//!
//! Claim, read, computed, and failed are the four hops the backtest stage
//! worker calls. They carry the shared `quantrule.internal/v1` records and no
//! profile: a stage row is shared, and the worker's authorization is the stage
//! run id. The public surface remains behind the gateway's profile validation.
//!
//! The computed hop verifies the stage manifest and every blob before it records
//! success. A missing or changed artifact leaves the row running, so a killed
//! worker exposes no visible artifact.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use quantrule_api::ErrorCode;
use quantrule_service_api::{
    StageClaim, StageClaimOutcome, StageComputed, StageFailed, StageRecord,
};

use crate::routes::{ApiFailure, failure};
use crate::stage::record_computed;
use crate::state::EvidenceState;

pub(crate) async fn get_stage(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
) -> Result<Json<StageRecord>, ApiFailure> {
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    match stages.get_any(id.clone()).await {
        Ok(Some(record)) => Ok(Json(record)),
        Ok(None) => Err(failure(
            ErrorCode::StageNotFound,
            format!("stage {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn claim_stage(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<StageClaim>, JsonRejection>,
) -> Result<Json<StageClaimOutcome>, ApiFailure> {
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::StageInvalid, "the body is not a stage claim"))?;
    if request.stage_run_id.to_string() != id {
        return Err(failure(
            ErrorCode::StageInvalid,
            "the claim names a different stage than the path",
        ));
    }
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    stages
        .claim(id, request.redelivered)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}

pub(crate) async fn computed_stage(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<StageComputed>, JsonRejection>,
) -> Result<Json<StageRecord>, ApiFailure> {
    let Json(computed) = payload
        .map_err(|_| failure(ErrorCode::StageInvalid, "the body is not a computed stage"))?;
    if computed.stage_run_id.to_string() != id {
        return Err(failure(
            ErrorCode::StageInvalid,
            "the computed stage names a different stage than the path",
        ));
    }
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    record_computed(stages.as_ref(), objects.as_ref(), &id, computed.artifact)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::StageInvalid, error.to_string()))
}

pub(crate) async fn failed_stage(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<StageFailed>, JsonRejection>,
) -> Result<Json<StageRecord>, ApiFailure> {
    let Json(failed) =
        payload.map_err(|_| failure(ErrorCode::StageInvalid, "the body is not a failed stage"))?;
    if failed.stage_run_id.to_string() != id {
        return Err(failure(
            ErrorCode::StageInvalid,
            "the failed stage names a different stage than the path",
        ));
    }
    let stages = state
        .stages
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no stage store is configured"))?;
    stages
        .fail(id, failed.error)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}
