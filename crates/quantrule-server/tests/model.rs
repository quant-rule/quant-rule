//! The model compute route over a catalogued dataset and config.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, memory_profiles};
use quantrule_api::{ConfigSummary, DatasetRecord, Digest, ModelObject, PROFILE_HEADER};
use quantrule_server::configs::MemoryConfigs;
use quantrule_server::data::MemoryCatalog;
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
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(common::MemoryObjects::default())),
            profiles: Some(memory_profiles(&["default"]).await),
            configs: Some(Arc::new(MemoryConfigs::default())),
            ..Services::default()
        },
    ))
}

fn csv(minutes: usize) -> String {
    let mut text = String::from("date,open,high,low,close,volume\n");
    for offset in 0..minutes {
        let hour = offset / 60;
        let minute = offset % 60;
        let open = 100.0 + offset as f64 * 0.01;
        text.push_str(&format!(
            "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
            open + 0.5,
            open - 0.5,
            open + 0.25
        ));
    }
    text
}

async fn upload(state: Arc<AppState>) -> DatasetRecord {
    let response = router(state)
        .oneshot(
            Request::post("/catalog/datasets?name=model.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(csv(250)))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_slice(&body_bytes(response).await).expect("dataset")
}

async fn configs(state: Arc<AppState>) -> Vec<ConfigSummary> {
    let response = router(state)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("configs")
}

async fn compute(state: Arc<AppState>, payload: serde_json::Value) -> axum::response::Response {
    router(state)
        .oneshot(
            Request::post("/model/compute")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn seed(state: Arc<AppState>) -> (Digest, String, String) {
    let dataset = upload(Arc::clone(&state)).await;
    let configs = configs(state).await;
    let valid = configs
        .iter()
        .find(|config| config.valid)
        .expect("valid")
        .config_id
        .clone();
    let invalid = configs
        .iter()
        .find(|config| !config.valid)
        .expect("invalid")
        .config_id
        .clone();
    (dataset.summary.data_version, valid, invalid)
}

#[tokio::test]
async fn the_full_ladder_is_stored_and_reused() {
    let state = state().await;
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;
    let payload = serde_json::json!({
        "data_version": data_version,
        "config_id": config_id,
    });
    let response = compute(Arc::clone(&state), payload.clone()).await;
    assert_eq!(response.status(), StatusCode::OK);
    let object: ModelObject =
        serde_json::from_slice(&body_bytes(response).await).expect("model object");
    assert_eq!(
        object.timeframes,
        vec![
            "1m".to_owned(),
            "5m".to_owned(),
            "15m".to_owned(),
            "1h".to_owned()
        ]
    );
    assert_eq!(object.data_version, data_version);
    assert!(object.object_key.starts_with("model/"));
    let stored = state
        .objects
        .as_ref()
        .expect("objects")
        .get(&object.object_key)
        .await
        .expect("stored");
    assert_eq!(stored.len() as u64, object.bytes);
    assert_eq!(object.sha256, Digest::sha256(&stored));
    let text = String::from_utf8(stored).expect("utf8");
    assert!(text.contains("\"flat_report\""), "{text}");

    let again = compute(state, payload).await;
    assert_eq!(again.status(), StatusCode::OK);
    let again: ModelObject = serde_json::from_slice(&body_bytes(again).await).expect("again");
    assert_eq!(again.object_key, object.object_key);
}

#[tokio::test]
async fn a_single_timeframe_shard_has_its_own_object() {
    let state = state().await;
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;
    let response = compute(
        state,
        serde_json::json!({
            "data_version": data_version,
            "config_id": config_id,
            "timeframes": ["5m"],
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let object: ModelObject =
        serde_json::from_slice(&body_bytes(response).await).expect("model object");
    assert_eq!(object.timeframes, vec!["5m".to_owned()]);
    assert!(
        object.object_key.ends_with("5m.json"),
        "{}",
        object.object_key
    );
}

#[tokio::test]
async fn an_unknown_timeframe_is_rejected() {
    let state = state().await;
    let (data_version, config_id, _) = seed(Arc::clone(&state)).await;
    let response = compute(
        state,
        serde_json::json!({
            "data_version": data_version,
            "config_id": config_id,
            "timeframes": ["7m"],
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
}

#[tokio::test]
async fn an_invalid_config_creates_no_object() {
    let state = state().await;
    let (data_version, _, invalid) = seed(Arc::clone(&state)).await;
    let response = compute(
        state,
        serde_json::json!({
            "data_version": data_version,
            "config_id": invalid,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found() {
    let state = state().await;
    let (_, config_id, _) = seed(Arc::clone(&state)).await;
    let response = compute(
        state,
        serde_json::json!({
            "data_version": Digest::sha256(b"missing"),
            "config_id": config_id,
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}
