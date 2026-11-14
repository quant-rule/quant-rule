//! The simulation service's HTTP surface.
//!
//! The eleven research endpoints live here. The service trusts the profile the
//! gateway injects and owns no profile store. It has no queue, no run store, no
//! model client, and no strategy input; `/internal/v1/*` is not served.
//!
//! The gateway forwards the client path unchanged, so the same routes answer at
//! the root and under `/api/v1`.

pub mod check;
pub mod marks;
pub mod runs;

use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use quantrule_api::{
    ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, VersionInfo,
};
use quantrule_runtime::adapters::ObjectStore;

use crate::bundle::BundleError;
use crate::records::RunRecord;
use crate::research::ResearchError;
use crate::state::SimulationState;
use crate::store::{SimulationStore, StoreError};

/// A failed request: a status and the shared envelope.
pub(crate) type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

/// Build one failure.
pub(crate) fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// The routes before state is attached.
fn routes() -> Router<Arc<SimulationState>> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route(
            "/simulation/runs",
            get(runs::list_runs).post(runs::create_run),
        )
        .route("/simulation/runs/{id}", get(runs::get_run))
        .route("/simulation/runs/{id}/windows", post(runs::select_windows))
        .route(
            "/simulation/runs/{id}/windows/{chart}/series",
            get(runs::get_series),
        )
        .route(
            "/simulation/runs/{id}/marks",
            get(marks::list_marks).post(marks::create_mark),
        )
        .route(
            "/simulation/runs/{id}/marks/{mark_id}",
            delete(marks::delete_mark),
        )
        .route("/simulation/runs/{id}/oracle", post(marks::run_oracle))
        .route("/simulation/runs/{id}/check", post(check::run_check))
        .route(
            "/simulation/runs/{id}/decision",
            post(check::record_decision),
        )
}

/// Build the service router.
pub fn router(state: Arc<SimulationState>) -> Router {
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .with_state(state)
}

async fn health(State(state): State<Arc<SimulationState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<SimulationState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the injected profile header.
///
/// The gateway validates that the profile exists and injects the normalized
/// value. The service trusts that value; a direct caller cannot reach the
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

/// Read the repository, or say the service has none.
pub(crate) fn store(state: &SimulationState) -> Result<&Arc<dyn SimulationStore>, ApiFailure> {
    state.store.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::Internal,
            "no simulation repository is configured",
        )
    })
}

/// Read the object store, or say the service has none.
pub(crate) fn objects(state: &SimulationState) -> Result<&Arc<dyn ObjectStore>, ApiFailure> {
    state.objects.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no object store is configured",
        )
    })
}

/// Load one run in the profile, or `404`.
pub(crate) async fn load_run(
    state: &SimulationState,
    profile: &ProfileId,
    sim_id: &str,
) -> Result<RunRecord, ApiFailure> {
    let found = store(state)?
        .get(profile.clone(), sim_id.to_owned())
        .await
        .map_err(store_failure)?;
    found.ok_or_else(|| {
        failure(
            ErrorCode::SimulationNotFound,
            format!("simulation {sim_id} is unknown"),
        )
    })
}

/// Map a store failure onto the public envelope.
pub(crate) fn store_failure(error: StoreError) -> ApiFailure {
    match error {
        StoreError::NotFound => failure(ErrorCode::SimulationNotFound, error.to_string()),
        StoreError::Locked => failure(ErrorCode::SimulationLocked, error.to_string()),
        StoreError::Conflict(message) => failure(ErrorCode::SimulationLocked, message),
        StoreError::Invalid(message) | StoreError::Backend(message) => {
            failure(ErrorCode::Internal, message)
        }
    }
}

/// Map a research failure onto the public envelope.
pub(crate) fn research_failure(error: ResearchError) -> ApiFailure {
    match error {
        ResearchError::NoSelection | ResearchError::UnknownChart(_) => {
            failure(ErrorCode::SimulationNotFound, error.to_string())
        }
        ResearchError::Mark(message) => failure(ErrorCode::SimulationMarkInvalid, message),
        ResearchError::CheckIncomplete(_) => {
            failure(ErrorCode::SimulationCheckIncomplete, error.to_string())
        }
        ResearchError::Selection(message)
        | ResearchError::Oracle(message)
        | ResearchError::Check(message) => failure(ErrorCode::Internal, message),
    }
}

/// Map a bundle failure onto the public envelope.
pub(crate) fn bundle_failure(error: BundleError) -> ApiFailure {
    match error {
        BundleError::Missing | BundleError::DigestMismatch => {
            failure(ErrorCode::SimulationNotFound, error.to_string())
        }
        BundleError::Unavailable => failure(ErrorCode::ServiceUnavailable, error.to_string()),
        BundleError::Malformed(message) => failure(ErrorCode::SimulationMarkInvalid, message),
        BundleError::Empty => failure(ErrorCode::SimulationMarkInvalid, error.to_string()),
    }
}
