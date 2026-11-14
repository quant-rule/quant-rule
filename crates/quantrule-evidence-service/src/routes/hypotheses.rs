//! The research-memory surface.
//!
//! A hypothesis is sealed once and read by fingerprint. The record is scoped by
//! the profile the gateway injects; evidence owns the `research` schema.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::ErrorCode;
use quantrule_research::{Hypothesis, SealedHypothesis};
use quantrule_runtime::clock;

use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::EvidenceState;

pub(crate) async fn create_hypothesis(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    payload: Result<Json<Hypothesis>, JsonRejection>,
) -> Result<(StatusCode, Json<SealedHypothesis>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(draft) = payload
        .map_err(|_| failure(ErrorCode::HypothesisInvalid, "the body is not a hypothesis"))?;
    let store = state
        .hypotheses
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    let sealed = draft
        .seal(clock::now_utc())
        .map_err(|error| failure(ErrorCode::HypothesisInvalid, error.to_string()))?;
    store
        .insert(profile, sealed.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(sealed)))
}

pub(crate) async fn get_hypothesis(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(fingerprint): Path<String>,
) -> Result<Json<SealedHypothesis>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = state
        .hypotheses
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    match store.load(profile, fingerprint.clone()).await {
        Ok(Some(sealed)) => Ok(Json(sealed)),
        Ok(None) => Err(failure(
            ErrorCode::HypothesisNotFound,
            format!("hypothesis {fingerprint} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}
