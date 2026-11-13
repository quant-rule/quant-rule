//! The HTTP surface of the runtime.
//!
//! Phase 1 serves `/health` and `/version`. The research routes seal and read a
//! hypothesis; the rest arrive with their services.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use quantrule_api::{ErrorCode, ErrorEnvelope, HealthReport, VersionInfo};
use quantrule_research::{Hypothesis, SealedHypothesis};

use crate::research::HypothesisStore;
use crate::runtime::clock;
use crate::runtime::config::Config;
use crate::runtime::error::RuntimeError;
use crate::runtime::health::HealthRegistry;

/// What a handler needs.
pub struct AppState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Where sealed hypotheses live; absent when no database is configured.
    pub store: Option<Arc<dyn HypothesisStore>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("store", &self.store.is_some())
            .finish()
    }
}

impl AppState {
    /// Build the state without a store.
    #[must_use]
    pub fn new(config: &Config, health: HealthRegistry) -> Self {
        Self::with_store(config, health, None)
    }

    /// Build the state with an optional store.
    #[must_use]
    pub fn with_store(
        config: &Config,
        health: HealthRegistry,
        store: Option<Arc<dyn HypothesisStore>>,
    ) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            store,
        }
    }
}

/// A failed request: a status and the shared envelope.
type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Build the router.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/hypotheses", post(create_hypothesis))
        .route("/hypotheses/{fingerprint}", get(get_hypothesis))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<AppState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

async fn create_hypothesis(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<Hypothesis>, JsonRejection>,
) -> Result<(StatusCode, Json<SealedHypothesis>), ApiFailure> {
    let Json(draft) = payload
        .map_err(|_| failure(ErrorCode::HypothesisInvalid, "the body is not a hypothesis"))?;
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    let sealed = draft
        .seal(clock::now_utc())
        .map_err(|error| failure(ErrorCode::HypothesisInvalid, error.to_string()))?;
    store
        .insert(sealed.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(sealed)))
}

async fn get_hypothesis(
    State(state): State<Arc<AppState>>,
    Path(fingerprint): Path<String>,
) -> Result<Json<SealedHypothesis>, ApiFailure> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    match store.load(fingerprint.clone()).await {
        Ok(Some(sealed)) => Ok(Json(sealed)),
        Ok(None) => Err(failure(
            ErrorCode::HypothesisNotFound,
            format!("hypothesis {fingerprint} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

/// Bind and serve until a termination signal arrives.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the address cannot be bound or the
/// server fails.
pub async fn serve(config: &Config, state: Arc<AppState>) -> Result<(), RuntimeError> {
    let listener = tokio::net::TcpListener::bind(&config.bind)
        .await
        .map_err(|error| RuntimeError::Serve(format!("bind {}: {error}", config.bind)))?;
    axum::serve(listener, router(state))
        .with_graceful_shutdown(crate::runtime::shutdown::signal())
        .await
        .map_err(|error| RuntimeError::Serve(format!("serve: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::MemoryStore;
    use axum::body::Body;
    use axum::http::Request;
    use quantrule_research::{CostModel, Partition, Split};
    use tower::ServiceExt;

    fn state() -> Arc<AppState> {
        Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
    }

    fn state_with_store() -> Arc<AppState> {
        Arc::new(AppState::with_store(
            &Config::default(),
            HealthRegistry::new(),
            Some(Arc::new(MemoryStore::default())),
        ))
    }

    fn body() -> serde_json::Value {
        let split = Split::new(
            Partition::new("dev", 100, 200).expect("dev"),
            Partition::new("val", 200, 300).expect("val"),
        )
        .expect("split");
        serde_json::json!({
            "premise": "a retest holds",
            "rule": "strategy/retest.toml",
            "dataset": "sha256:abc",
            "costs": CostModel::new(1.0, 0.0, 0.0).expect("costs"),
            "split": split,
            "criteria": "expectancy above zero",
        })
    }

    #[tokio::test]
    async fn health_is_served_as_json() {
        let response = router(state())
            .oneshot(
                Request::get("/health")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn version_reports_the_contract() {
        let response = router(state())
            .oneshot(
                Request::get("/version")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .expect("body");
        let text = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(text.contains("quantrule.api/v1"), "{text}");
    }

    #[tokio::test]
    async fn an_unknown_path_is_not_found() {
        let response = router(state())
            .oneshot(Request::get("/nope").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn a_hypothesis_is_sealed_and_read_back() {
        let state = state_with_store();
        let payload = serde_json::to_vec(&body()).expect("encode");
        let response = router(Arc::clone(&state))
            .oneshot(
                Request::post("/hypotheses")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let bytes = axum::body::to_bytes(response.into_body(), 16_384)
            .await
            .expect("body");
        let sealed: SealedHypothesis = serde_json::from_slice(&bytes).expect("decode");
        let fingerprint = sealed.fingerprint().to_owned();

        let response = router(state)
            .oneshot(
                Request::get(format!("/hypotheses/{fingerprint}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn an_unknown_hypothesis_is_not_found_with_a_code() {
        let response = router(state_with_store())
            .oneshot(
                Request::get("/hypotheses/missing")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let bytes = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .expect("body");
        let text = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(text.contains("hypothesis.not_found"), "{text}");
    }

    #[tokio::test]
    async fn a_malformed_body_is_rejected_with_a_code() {
        let response = router(state_with_store())
            .oneshot(
                Request::post("/hypotheses")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let bytes = axum::body::to_bytes(response.into_body(), 8192)
            .await
            .expect("body");
        let text = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(text.contains("hypothesis.invalid"), "{text}");
    }

    #[tokio::test]
    async fn without_a_store_the_route_reports_internal() {
        let payload = serde_json::to_vec(&body()).expect("encode");
        let response = router(state())
            .oneshot(
                Request::post("/hypotheses")
                    .header("content-type", "application/json")
                    .body(Body::from(payload))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
