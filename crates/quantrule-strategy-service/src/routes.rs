//! The strategy service's HTTP surface.
//!
//! The only data route is `POST /internal/v1/signals`; it is service-to-service
//! and never proxied by the gateway. `/health` and `/version` are the same probes
//! every service publishes. The service trusts the profile header the gateway
//! validates and injects.

use std::sync::Arc;

use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use quantrule_api::{
    ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, VersionInfo,
};
use quantrule_service_api::{SignalRequest, SignalResponse};

use crate::error::SignalServiceError;
use crate::resolve;
use crate::state::StrategyState;

/// A failed request: a status and the shared envelope.
type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

fn signal_failure(error: SignalServiceError) -> ApiFailure {
    match error {
        SignalServiceError::Invalid(message) => failure(ErrorCode::SignalInvalid, message),
        SignalServiceError::PackNotFound(id) => failure(
            ErrorCode::PackNotFound,
            format!("strategy pack {id} is unknown"),
        ),
        SignalServiceError::ModelNotFound {
            data_version,
            config_id,
        } => failure(
            ErrorCode::ArtifactNotFound,
            format!("model bundle for dataset {data_version} and config {config_id} is unknown"),
        ),
        SignalServiceError::Unavailable(message) => failure(ErrorCode::ServiceUnavailable, message),
        SignalServiceError::Internal(message) => failure(ErrorCode::Internal, message),
    }
}

/// The routes before state is attached.
fn routes() -> Router<Arc<StrategyState>> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/version", get(version))
        .route("/internal/v1/signals", post(compute_signal))
}

/// Build the service router.
///
/// There is no public route and no `/api/v1` alias: only the internal signal
/// surface and the two probes exist.
pub fn router(state: Arc<StrategyState>) -> Router {
    routes().with_state(state)
}

async fn health(State(state): State<Arc<StrategyState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn ready(State(state): State<Arc<StrategyState>>) -> (StatusCode, Json<HealthReport>) {
    let digest = state.version.image_digest.clone();
    let (ready, report) = state.health.readiness(digest).await;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

async fn version(State(state): State<Arc<StrategyState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the injected profile header.
fn request_profile(headers: &HeaderMap) -> Result<ProfileId, ApiFailure> {
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

/// Compute and publish one signal artifact.
async fn compute_signal(
    State(state): State<Arc<StrategyState>>,
    headers: HeaderMap,
    payload: Result<Json<SignalRequest>, JsonRejection>,
) -> Result<Json<SignalResponse>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload
        .map_err(|_| failure(ErrorCode::SignalInvalid, "the body is not a signal request"))?;
    let response = resolve::compute_signal(&state, profile, &request)
        .await
        .map_err(signal_failure)?;
    Ok(Json(response))
}
