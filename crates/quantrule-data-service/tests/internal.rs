//! The internal references the gateway and the worker consume.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    body_bytes, read_dataset, state, state_with_catalog_and_objects, state_with_configs, upload,
};
use quantrule_api::{Digest, PROFILE_HEADER};
use quantrule_data_service::router;
use quantrule_service_api::{ConfigBundle, DatasetObject};
use std::sync::Arc;
use tower::ServiceExt;

#[tokio::test]
async fn a_dataset_object_is_a_reference_not_a_candle_series() {
    let (state, _objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/internal/v1/datasets/{version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let object: DatasetObject =
        serde_json::from_slice(&body_bytes(response).await).expect("dataset object");
    assert_eq!(object.data_version, record.summary.data_version);
    assert_eq!(object.object_key, record.object_key);
    assert_eq!(object.sha256, record.summary.data_version);
    assert_eq!(object.candles, 2);
    assert_eq!(object.bytes, record.summary.bytes);
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found_internally() {
    let version = Digest::sha256(b"missing").to_string();
    let response = router(state_with_catalog_and_objects().await.0)
        .oneshot(
            Request::get(format!("/internal/v1/datasets/{version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}

#[tokio::test]
async fn a_config_bundle_carries_its_files_and_hash() {
    let response = router(state_with_configs().await)
        .oneshot(
            Request::get("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<quantrule_api::ConfigSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("configs");
    let valid = listed
        .iter()
        .find(|config| config.valid)
        .expect("valid")
        .config_id
        .clone();

    let response = router(state_with_configs().await)
        .oneshot(
            Request::get(format!("/internal/v1/configs/{valid}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let bundle: ConfigBundle = serde_json::from_slice(&body_bytes(response).await).expect("bundle");
    assert!(bundle.valid);
    assert_eq!(bundle.config_id, valid);
    assert!(bundle.config_hash.is_some());
    assert!(bundle.files.contains_key("run.json"));
    assert!(bundle.files.contains_key("data.json"));
}

#[tokio::test]
async fn an_unknown_config_is_not_found_internally() {
    let response = router(state_with_configs().await)
        .oneshot(
            Request::get("/internal/v1/configs/cfg_missing")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
}

#[tokio::test]
async fn the_dataset_count_is_scoped_by_profile() {
    let state = state_with_catalog_and_objects().await.0;
    let response = router(Arc::clone(&state))
        .oneshot(upload("one.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get("/internal/v1/profile/datasets/count")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let count: u64 = serde_json::from_slice(&body_bytes(response).await).expect("count");
    assert_eq!(count, 1);

    let response = router(state)
        .oneshot(
            Request::get("/internal/v1/profile/datasets/count")
                .header(PROFILE_HEADER, "alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let count: u64 = serde_json::from_slice(&body_bytes(response).await).expect("count");
    assert_eq!(count, 0);
}

#[tokio::test]
async fn every_internal_route_requires_the_injected_profile() {
    let version = Digest::sha256(b"any").to_string();
    for path in [
        format!("/internal/v1/datasets/{version}"),
        "/internal/v1/configs/cfg_any".to_owned(),
        "/internal/v1/profile/datasets/count".to_owned(),
    ] {
        let response = router(state_with_catalog_and_objects().await.0)
            .oneshot(
                Request::get(path.clone())
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let text = String::from_utf8(body_bytes(response).await).expect("utf8");
        assert!(text.contains("profile.not_found"), "{path}: {text}");
    }
}

#[tokio::test]
async fn an_empty_service_still_answers_the_internal_surface() {
    let response = router(state(
        &quantrule_runtime::Config::default(),
        Default::default(),
    ))
    .oneshot(
        Request::get("/internal/v1/profile/datasets/count")
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("request"),
    )
    .await
    .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
