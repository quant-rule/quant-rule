//! The run-lineage surface.
//!
//! A run's reference set is recorded by the path that resolved it. The public
//! read at `/runs/{id}/lineage` serves the run detail; the internal write at
//! `/internal/v1/runs/{id}/lineage` remains the service-to-service seam. The
//! set is idempotent for the same references and refuses a different set, so a
//! retry cannot rewrite what a run cited.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use quantrule_api::{Digest, ErrorCode, ProfileId, RecordRunLineageRequest, RunId, RunLineage};
use quantrule_runtime::clock;

use crate::lineage::{LineageError, RunLineageStore};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::EvidenceState;

fn parse_run_id(raw: &str) -> Result<RunId, ApiFailure> {
    Digest::parse(raw)
        .map(RunId)
        .map_err(|error| failure(ErrorCode::RunInvalid, error.to_string()))
}

fn lineage_failure(error: LineageError) -> ApiFailure {
    match error {
        LineageError::Missing => failure(ErrorCode::LineageNotFound, "the run lineage is unknown"),
        LineageError::Conflict => failure(
            ErrorCode::RunConflict,
            "the run already records a different reference set",
        ),
        LineageError::Invalid(message) => failure(ErrorCode::RunInvalid, message),
        LineageError::Store(message) => failure(ErrorCode::Internal, message),
    }
}

fn lineage(state: &EvidenceState) -> Result<&Arc<dyn RunLineageStore>, ApiFailure> {
    state
        .lineage
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run lineage store is configured"))
}

async fn visible_run(
    state: &EvidenceState,
    profile: &ProfileId,
    run_id: &RunId,
) -> Result<(), ApiFailure> {
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let record = runs
        .get(profile.clone(), run_id.to_string())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if record.is_none() {
        return Err(failure(
            ErrorCode::RunNotFound,
            format!("run {run_id} is unknown"),
        ));
    }
    Ok(())
}

/// Record or repeat a run's research artifact lineage.
pub(crate) async fn record_lineage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<RecordRunLineageRequest>, JsonRejection>,
) -> Result<Json<RunLineage>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run_id = parse_run_id(&id)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::RunInvalid,
            "the body is not a run lineage record",
        )
    })?;
    visible_run(&state, &profile, &run_id).await?;
    let recorded_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let lineage = lineage(&state)?
        .record(profile, run_id, request.references, recorded_at)
        .await
        .map_err(lineage_failure)?;
    Ok(Json(lineage))
}

/// Read a run's research artifact lineage.
pub(crate) async fn get_lineage(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunLineage>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run_id = parse_run_id(&id)?;
    visible_run(&state, &profile, &run_id).await?;
    let lineage = lineage(&state)?
        .get(profile, run_id)
        .await
        .map_err(lineage_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::LineageNotFound,
                format!("run {id} has no recorded lineage"),
            )
        })?;
    Ok(Json(lineage))
}
