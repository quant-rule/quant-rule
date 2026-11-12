//! The HTTP surface of the runtime.
//!
//! Phase 1 serves `/health` and `/version`. Later phases add the routes; the
//! router, the state, and the graceful shutdown do not change.

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use quantrule_api::{HealthReport, VersionInfo};

use crate::runtime::config::Config;
use crate::runtime::error::RuntimeError;
use crate::runtime::health::HealthRegistry;

/// What a handler needs.
#[derive(Debug)]
pub struct AppState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
}

impl AppState {
    /// Build the state from a configuration and a registry.
    #[must_use]
    pub fn new(config: &Config, health: HealthRegistry) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
        }
    }
}

/// Build the router.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<AppState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
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
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    fn state() -> Arc<AppState> {
        Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
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
}
