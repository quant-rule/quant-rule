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
            profiles: Some(memory_profiles(&["default", "alpha"]).await),
            ..Services::default()
        },
    ))
}

async fn list_for(state: Arc<AppState>, profile: &str) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, profile)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn list(state: Arc<AppState>) -> axum::response::Response {
    list_for(state, "default").await
}

#[tokio::test]
async fn the_mount_is_scanned_and_listed() {
    let response = list(state().await).await;
    assert_eq!(response.status(), StatusCode::OK);
    let records: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("configs");
    assert_eq!(records.len(), 3);

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
async fn configs_are_profile_scoped_but_the_bundle_is_shared() {
    let state = state().await;
    let default = list(Arc::clone(&state)).await;
    let default: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(default).await).expect("default");
    let alpha = list_for(Arc::clone(&state), "alpha").await;
    let alpha: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(alpha).await).expect("alpha");
    assert_eq!(default.len(), 3);
    assert_eq!(alpha.len(), 3);

    let response = upload(Arc::clone(&state), bundle()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let uploaded: ConfigSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("upload");

    let default = list(Arc::clone(&state)).await;
    let default: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(default).await).expect("default");
    let alpha = list_for(state, "alpha").await;
    let alpha: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(alpha).await).expect("alpha");
    assert_eq!(default.len(), 4);
    assert_eq!(alpha.len(), 3);
    assert!(
        default
            .iter()
            .any(|config| config.config_id == uploaded.config_id)
    );
    assert!(
        !alpha
            .iter()
            .any(|config| config.config_id == uploaded.config_id)
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

fn bundle() -> serde_json::Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid");
    let mut run: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("run.json")).expect("run"))
            .expect("run json");
    run["name"] = serde_json::json!("uploaded-run");
    let mut files = serde_json::Map::new();
    files.insert(
        "run.json".to_owned(),
        serde_json::json!(serde_json::to_string(&run).expect("run text")),
    );
    for name in [
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
    ] {
        files.insert(
            name.to_owned(),
            serde_json::json!(std::fs::read_to_string(root.join(name)).expect("fixture")),
        );
    }
    serde_json::json!({ "name": "uploaded", "files": files })
}

async fn upload(state: Arc<AppState>, payload: serde_json::Value) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

#[tokio::test]
async fn an_uploaded_bundle_is_catalogued() {
    let state = state().await;
    let response = upload(Arc::clone(&state), bundle()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: ConfigSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("config");
    assert!(record.valid);
    assert_eq!(record.origin, ConfigOrigin::Upload);
    assert_eq!(record.name, "uploaded");
    assert!(record.config_hash.is_some());

    let response = list(state).await;
    let records: Vec<ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("configs");
    assert_eq!(records.len(), 4);
    assert!(
        records
            .iter()
            .any(|item| item.config_id == record.config_id)
    );
}

#[tokio::test]
async fn a_duplicate_upload_is_a_lookup() {
    let state = state().await;
    let first = upload(Arc::clone(&state), bundle()).await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first: ConfigSummary = serde_json::from_slice(&body_bytes(first).await).expect("first");
    let second = upload(state, bundle()).await;
    assert_eq!(second.status(), StatusCode::OK);
    let second: ConfigSummary = serde_json::from_slice(&body_bytes(second).await).expect("second");
    assert_eq!(first.config_id, second.config_id);
}

#[tokio::test]
async fn an_uploaded_invalid_bundle_is_catalogued_with_its_message() {
    let mut payload = bundle();
    payload["files"]
        .as_object_mut()
        .expect("files")
        .remove("data.json");
    let state = state().await;
    let response = upload(Arc::clone(&state), payload).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: ConfigSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("config");
    assert!(!record.valid);
    assert!(record.config_hash.is_none());
    assert!(
        record
            .message
            .as_deref()
            .is_some_and(|message| message.contains("data.json")),
        "{:?}",
        record.message
    );
}

#[tokio::test]
async fn a_bundle_path_cannot_escape() {
    let mut payload = bundle();
    payload["files"]
        .as_object_mut()
        .expect("files")
        .insert("../escape.json".to_owned(), serde_json::json!("{}"));
    let response = upload(state().await, payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
}

#[tokio::test]
async fn a_bundle_without_run_json_is_refused() {
    let mut payload = bundle();
    payload["files"]
        .as_object_mut()
        .expect("files")
        .remove("run.json");
    let response = upload(state().await, payload).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
