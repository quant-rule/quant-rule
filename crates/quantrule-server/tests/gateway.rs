//! The gateway routes that remain local: health, version, hypotheses, profiles,
//! and the client prefix.
//!
//! The data routes are exercised in `quantrule-data-service`; the proxy itself
//! is covered in `tests/proxy.rs`.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body, body_bytes, state, state_with_data, state_with_store};
use quantrule_api::{Digest, PROFILE_HEADER, ProfileId, ProfileSummary};
use quantrule_research::SealedHypothesis;
use quantrule_runtime::client::MemoryDataClient;
use quantrule_server::server::router;
use quantrule_service_api::DatasetObject;
use std::sync::Arc;
use tower::ServiceExt;

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
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
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
    let state = state_with_store().await;
    let payload = serde_json::to_vec(&body()).expect("encode");
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/hypotheses")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let sealed: SealedHypothesis =
        serde_json::from_slice(&body_bytes(response).await).expect("decode");
    let fingerprint = sealed.fingerprint().to_owned();

    let response = router(state)
        .oneshot(
            Request::get(format!("/hypotheses/{fingerprint}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn an_unknown_hypothesis_is_not_found_with_a_code() {
    let response = router(state_with_store().await)
        .oneshot(
            Request::get("/hypotheses/missing")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("hypothesis.not_found"), "{text}");
}

#[tokio::test]
async fn a_malformed_body_is_rejected_with_a_code() {
    let response = router(state_with_store().await)
        .oneshot(
            Request::post("/hypotheses")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("hypothesis.invalid"), "{text}");
}

#[tokio::test]
async fn without_a_store_the_route_reports_internal() {
    let payload = serde_json::to_vec(&body()).expect("encode");
    let response = router(state())
        .oneshot(
            Request::post("/hypotheses")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn a_profile_is_created_and_listed() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ProfileSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("profile");
    assert_eq!(created.profile_id.as_str(), "delta");
    assert_eq!(created.name, "Delta");
    assert_eq!(created.datasets, 0);

    let response = router(state)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "delta")
    );
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "default")
    );
}

#[tokio::test]
async fn a_duplicate_profile_is_idempotent() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let first = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first: ProfileSummary = serde_json::from_slice(&body_bytes(first).await).expect("profile");

    let second = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second: ProfileSummary =
        serde_json::from_slice(&body_bytes(second).await).expect("profile");
    assert_eq!(first, second);
}

#[tokio::test]
async fn an_invalid_profile_is_rejected() {
    let state = state_with_data(MemoryDataClient::default()).await;
    let bad_id = r#"{"profile_id":"Bad!","name":"Bad"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(bad_id))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");

    let blank_name = r#"{"profile_id":"ok","name":"   "}"#;
    let response = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(blank_name))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_profile_is_not_found_before_the_upstream() {
    let response = router(state_with_data(MemoryDataClient::default()).await)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "ghost")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}

#[tokio::test]
async fn a_dataset_contributes_to_its_profile_count() {
    let data = MemoryDataClient::default();
    let alpha = ProfileId::parse("alpha").expect("alpha");
    let version = Digest::sha256(b"candles");
    data.add_dataset(
        &alpha,
        DatasetObject {
            data_version: version.clone(),
            object_key: format!("datasets/{}", version.as_hex()),
            sha256: version,
            candles: 2,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_260,
            bytes: 100,
        },
    );
    let response = router(state_with_data(data).await)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    let alpha = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "alpha")
        .expect("alpha");
    assert_eq!(alpha.datasets, 1);
    let default = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "default")
        .expect("default");
    assert_eq!(default.datasets, 0);
}

#[tokio::test]
async fn the_client_prefix_serves_the_same_routes() {
    let response = router(state())
        .oneshot(
            Request::get("/api/v1/version")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("quantrule.api/v1"), "{text}");
}
