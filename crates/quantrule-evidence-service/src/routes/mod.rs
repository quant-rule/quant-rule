//! The evidence service's HTTP surface.
//!
//! The public routes are the ones the gateway proxies: runs and research
//! memory. The service trusts the profile header the gateway injects; it owns no
//! profile store. The internal routes expose the worker lifecycle: claim, read,
//! computed, and failed. No internal response carries an artifact payload.

pub mod hypotheses;
pub mod internal;
pub mod runs;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use quantrule_api::{
    ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, VersionInfo,
};
use quantrule_runtime::client::ClientError;

use crate::state::EvidenceState;

/// A failed request: a status and the shared envelope.
pub(crate) type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

pub(crate) fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Map a data-service client failure onto the public envelope.
///
/// A transport failure is a 503 `service.unavailable`; an undecodable body is a
/// 500 `internal`. A 4xx the caller did not expect is an internal defect,
/// because the gateway already validated the request.
pub(crate) fn data_failure(error: ClientError) -> ApiFailure {
    match &error {
        ClientError::InvalidResponse { .. } | ClientError::InvalidBase { .. } => {
            failure(ErrorCode::Internal, error.to_string())
        }
        ClientError::Unreachable { .. }
        | ClientError::Timeout { .. }
        | ClientError::Unconfigured { .. }
        | ClientError::Status { .. } => failure(ErrorCode::ServiceUnavailable, error.to_string()),
    }
}

/// The routes before state is attached.
fn routes() -> Router<Arc<EvidenceState>> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/runs", get(runs::list_runs).post(runs::submit_run))
        .route("/runs/{id}", get(runs::get_run))
        .route("/runs/{id}/cancel", post(runs::cancel_run))
        .route("/runs/{id}/artifacts/{name}", get(runs::get_run_artifact))
        .route("/runs/{id}/digest", get(runs::get_run_digest))
        .route("/hypotheses", post(hypotheses::create_hypothesis))
        .route("/hypotheses/{fingerprint}", get(hypotheses::get_hypothesis))
        .route("/internal/v1/runs/{id}", get(internal::get_run))
        .route("/internal/v1/runs/{id}/claim", post(internal::claim_run))
        .route(
            "/internal/v1/runs/{id}/computed",
            post(internal::computed_run),
        )
        .route("/internal/v1/runs/{id}/failed", post(internal::failed_run))
}

/// Build the service router.
///
/// The gateway forwards the client path unchanged, so the same routes answer at
/// the root and under `/api/v1`. `/internal/v1` is service-to-service only.
pub fn router(state: Arc<EvidenceState>) -> Router {
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .with_state(state)
}

async fn health(State(state): State<Arc<EvidenceState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<EvidenceState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the injected profile header.
///
/// The gateway validates that the profile exists and injects the normalized
/// value. Evidence trusts that value; a direct caller cannot reach it on the
/// published surface.
pub(crate) fn request_profile(headers: &HeaderMap) -> Result<ProfileId, ApiFailure> {
    let Some(value) = headers.get(PROFILE_HEADER) else {
        return Err(failure(
            ErrorCode::ProfileNotFound,
            format!("{PROFILE_HEADER} is required"),
        ));
    };
    let text = value
        .to_str()
        .map_err(|_| failure(ErrorCode::ProfileInvalid, "the profile header is not text"))?;
    ProfileId::parse(text).map_err(|error| failure(ErrorCode::ProfileInvalid, error.to_string()))
}
