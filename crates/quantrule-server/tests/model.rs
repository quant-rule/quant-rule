//! The model compute route over a catalogued dataset and config.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, canonical, memory_profiles};
use quantrule_api::{Digest, ModelObject, PROFILE_HEADER};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_server::server::router;
use quantrule_server::server::{AppState, Services};
use quantrule_service_api::{ConfigBundle, DatasetObject};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

fn valid_files() -> BTreeMap<String, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid");
    [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
    ]
    .into_iter()
    .map(|name| {
        (
            name.to_owned(),
            std::fs::read_to_string(root.join(name)).expect("fixture"),
        )
    })
    .collect()
}

async fn state() -> Arc<AppState> {
    let objects = common::MemoryObjects::default();
    let data = MemoryDataClient::default();
    let profile = quantrule_api::ProfileId::parse("default").expect("profile");

    let dataset = csv(250);
    let version = Digest::sha256(&dataset);
    let key = format!("datasets/{}", version.as_hex());
    objects
        .put(&key, dataset.clone())
        .await
        .expect("dataset object");
    data.add_dataset(
        &profile,
        DatasetObject {
            data_version: version.clone(),
            object_key: key,
            sha256: version.clone(),
            candles: 250,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_200 + 249 * 60,
            bytes: dataset.len() as u64,
        },
    );
    data.add_config(
        &profile,
        ConfigBundle {
            config_id: "cfg_valid".to_owned(),
            files: valid_files(),
            valid: true,
            message: None,
            config_hash: Some(Digest::sha256(b"valid")),
        },
    );
    data.add_config(
        &profile,
        ConfigBundle {
            config_id: "cfg_invalid".to_owned(),
            files: BTreeMap::from([("run.json".to_owned(), "{}".to_owned())]),
            valid: false,
            message: Some("the bundle has no data.json".to_owned()),
            config_hash: None,
        },
    );

    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            objects: Some(Arc::new(objects)),
            profiles: Some(memory_profiles(&["default"]).await),
            data: Some(Arc::new(data)),
            ..Services::default()
        },
    ))
}

fn csv(minutes: usize) -> Vec<u8> {
    let mut text = String::new();
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
    canonical(&text)
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
    let _ = state;
    (
        Digest::sha256(&csv(250)),
        "cfg_valid".to_owned(),
        "cfg_invalid".to_owned(),
    )
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
