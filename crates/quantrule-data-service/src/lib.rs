//! The data service surface.
//!
//! Milestone M1 exposes `/health` and `/version` only. The catalog, dataset,
//! config, and inspection routes stay in the gateway until P3 moves them, so
//! this crate deliberately has no engine, database, or object-store dependency.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use quantrule_api::{HealthReport, VersionInfo};
use quantrule_runtime::Config;
use quantrule_runtime::health::HealthRegistry;

/// What the health and version routes read.
struct ServiceState {
    version: VersionInfo,
    health: HealthRegistry,
}

/// Build the service router.
///
/// The two routes are the milestone's whole surface. A domain route added here
/// before P3 would claim a service the gateway still owns.
pub fn router(config: &Config, health: HealthRegistry) -> Router {
    let state = Arc::new(ServiceState {
        version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
        health,
    });
    Router::new()
        .route("/health", get(health_report))
        .route("/version", get(version))
        .with_state(state)
}

async fn health_report(State(state): State<Arc<ServiceState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<ServiceState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}
