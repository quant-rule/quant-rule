//! The gateway on the embedded SQLite profile.
//!
//! The test builds the same state the desktop branch builds: one SQLite
//! database, the gateway baseline applied to the shared ledger, the profile
//! store, and a `sqlite` health probe. It then asks the real router for
//! `/health` and `/profiles`. The binary itself is a thin branch over these
//! pieces; the manual verification in the AI log runs that branch.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{HealthReport, ProfileSummary};
use quantrule_gateway::adapters::sqlite::SqliteProfileStore;
use quantrule_gateway::server::{AppState, Services, router};
use quantrule_runtime::adapters::Database;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

async fn state() -> Arc<AppState> {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "gateway",
        &quantrule_gateway::migrations::sqlite(),
    )
    .await
    .expect("the gateway baseline");

    let mut health = HealthRegistry::new();
    let ping = database.clone();
    health.register("sqlite", move || {
        let ping = ping.clone();
        async move { ping.ping().await.map_err(|error| error.to_string()) }
    });

    Arc::new(AppState::with_services(
        &Config::default(),
        health,
        Services {
            profiles: Some(Arc::new(SqliteProfileStore::new(database))),
            ..Services::default()
        },
    ))
}

async fn body(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("the body")
        .to_vec()
}

#[tokio::test]
async fn health_names_sqlite_and_profiles_answers_from_the_sqlite_store() {
    let app = router(state().await);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("the request"),
        )
        .await
        .expect("health");
    assert_eq!(response.status(), StatusCode::OK);
    let report: HealthReport = serde_json::from_slice(&body(response).await).expect("health json");
    assert_eq!(report.services.len(), 1);
    assert_eq!(report.services[0].name, "sqlite");
    assert_eq!(report.state, quantrule_api::ServiceState::Healthy);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/profiles")
                .body(Body::empty())
                .expect("the request"),
        )
        .await
        .expect("profiles");
    assert_eq!(response.status(), StatusCode::OK);
    let profiles: Vec<ProfileSummary> =
        serde_json::from_slice(&body(response).await).expect("profiles json");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].profile_id.as_str(), "default");
    assert_eq!(profiles[0].name, "Default");
    assert_eq!(profiles[0].datasets, 0);
    assert_eq!(profiles[0].runs, 0);
    assert_eq!(profiles[0].simulations, 0);
}
