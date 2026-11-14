//! The worker's internal run lifecycle.
//!
//! Claim, read, computed, and failed are the four hops the backtest worker
//! calls. They carry the shared `quantrule.internal/v1` records and no profile:
//! the run row is shared, and the worker's authorization is the run id. The
//! public surface remains behind the gateway's profile validation.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use quantrule_api::ErrorCode;
use quantrule_service_api::{RunClaimOutcome, RunClaimRequest, RunComputed, RunFailed, RunRecord};

use crate::routes::{ApiFailure, failure};
use crate::state::EvidenceState;

pub(crate) async fn get_run(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
) -> Result<Json<RunRecord>, ApiFailure> {
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.get_any(id.clone()).await {
        Ok(Some(record)) => Ok(Json(record)),
        Ok(None) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

pub(crate) async fn claim_run(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<RunClaimRequest>, JsonRejection>,
) -> Result<Json<RunClaimOutcome>, ApiFailure> {
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a claim"))?;
    if request.run_id.to_string() != id {
        return Err(failure(
            ErrorCode::RunInvalid,
            "the claim names a different run than the path",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    runs.claim(id, request.redelivered)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}

pub(crate) async fn computed_run(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<RunComputed>, JsonRejection>,
) -> Result<Json<RunRecord>, ApiFailure> {
    let Json(computed) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a computed run"))?;
    if computed.run_id.to_string() != id {
        return Err(failure(
            ErrorCode::RunInvalid,
            "the computed run names a different run than the path",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    runs.computed(id, computed.digest, computed.artifacts, computed.stages)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}

pub(crate) async fn failed_run(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<RunFailed>, JsonRejection>,
) -> Result<Json<RunRecord>, ApiFailure> {
    let Json(failed) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a failed run"))?;
    if failed.run_id.to_string() != id {
        return Err(failure(
            ErrorCode::RunInvalid,
            "the failed run names a different run than the path",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    runs.fail(id, failed.error, failed.stages)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}
