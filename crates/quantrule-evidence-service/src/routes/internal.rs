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
use quantrule_service_api::{
    RunClaimOutcome, RunClaimRequest, RunComputed, RunFailed, RunHeartbeat, RunRecord,
};

use crate::routes::{ApiFailure, failure};
use crate::runs::HeartbeatOutcome;
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
    if request.holder.is_empty() || request.holder.len() > 128 {
        return Err(failure(
            ErrorCode::RunInvalid,
            "a claim holder must be between 1 and 128 characters",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    runs.claim(id, request.holder, request.continuation)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}

pub(crate) async fn heartbeat_run(
    State(state): State<Arc<EvidenceState>>,
    Path(id): Path<String>,
    payload: Result<Json<RunHeartbeat>, JsonRejection>,
) -> Result<Json<RunRecord>, ApiFailure> {
    let Json(heartbeat) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a heartbeat"))?;
    if heartbeat.run_id.to_string() != id {
        return Err(failure(
            ErrorCode::RunInvalid,
            "the heartbeat names a different run than the path",
        ));
    }
    if heartbeat.holder.is_empty() || heartbeat.holder.len() > 128 {
        return Err(failure(
            ErrorCode::RunInvalid,
            "a heartbeat holder must be between 1 and 128 characters",
        ));
    }
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs
        .heartbeat(id, heartbeat.holder, heartbeat.term)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    {
        HeartbeatOutcome::Renewed(record) => Ok(Json(*record)),
        HeartbeatOutcome::Stale => Err(failure(ErrorCode::RunConflict, "the run lease is stale")),
        HeartbeatOutcome::Missing => Err(failure(ErrorCode::RunNotFound, "the run is unknown")),
    }
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
    let record = runs
        .computed(
            id,
            computed.term,
            computed.digest,
            computed.artifacts,
            computed.stages,
        )
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    // The computed notification committed with the digest. Publishing it here
    // keeps the normal path immediate; the reconciler retries a failed drain,
    // so the response does not fail on a broker that is merely unavailable.
    if let (Some(outbox), Some(queue)) = (state.outbox.as_ref(), state.queue.as_ref())
        && let Err(error) =
            crate::outbox::drain(outbox.as_ref(), queue.as_ref(), crate::outbox::DRAIN_LIMIT).await
    {
        tracing::warn!(run_id = %record.run_id, %error, "the computed notification will be retried");
    }
    Ok(Json(record))
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
    runs.fail(id, failed.term, failed.error, failed.stages)
        .await
        .map(Json)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}
