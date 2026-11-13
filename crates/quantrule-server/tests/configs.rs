//! The configuration catalog, scanned from the fixture mount.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, memory_profiles};
use quantrule_api::{ConfigOrigin, ConfigSummary, PROFILE_HEADER};
use quantrule_server::configs::MemoryConfigs;
use quantrule_server::runtime::server::router;
use quantrule_server::runtime::{AppState, Config, HealthRegistry, Services};
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

async fn state() -> Arc<AppState> {
    let config = Config {
        config_root: Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config"),
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            configs: Some(Arc::new(MemoryConfigs::default())),
            profiles: Some(memory_profiles(&["default"]).await),
            ..Services::default()
        },
    ))
}

async fn list(state: Arc<AppState>) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

#[tokio::test]
async fn the_mount_is_scanned_and_listed() {
    let response = list(state().await).await;
    assert_eq!(response.status(), StatusCode::OK);
    let records: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("configs");
    assert_eq!(records.len(), 2);

    let valid = records
        .iter()
        .find(|record| record.name == "valid/run.json")
        .expect("valid");
    assert!(valid.valid);
    assert_eq!(valid.origin, ConfigOrigin::Repository);
    assert!(valid.config_hash.is_some());

    let invalid = records
        .iter()
        .find(|record| record.name == "invalid/run.json")
        .expect("invalid");
    assert!(!invalid.valid);
    assert!(invalid.config_hash.is_none());
    assert!(
        invalid
            .message
            .as_deref()
            .is_some_and(|message| message.contains("missing-data.json")),
        "{:?}",
        invalid.message
    );
}

#[tokio::test]
async fn a_second_scan_does_not_duplicate() {
    let state = state().await;
    let first = list(Arc::clone(&state)).await;
    let first: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(first).await).expect("first");
    let second = list(state).await;
    let second: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_eq!(first, second);
}

#[tokio::test]
async fn a_missing_profile_is_refused() {
    let response = router(state().await)
        .oneshot(
            Request::get("/catalog/configs")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}
