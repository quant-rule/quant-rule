//! The gateway routes, exercised over HTTP with in-memory services.
//!
//! The tests assert on status, envelope code, and body, not on a handler's
//! internals, so a route change is visible the way a client would see it.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    CSV, body, body_bytes, read_dataset, state, state_with_catalog, state_with_catalog_and_objects,
    state_with_small_upload, state_with_store, upload,
};
use quantrule_api::{DatasetSummary, Digest, PROFILE_HEADER, ProfileSummary, TimeframeInspection};
use quantrule_research::SealedHypothesis;
use quantrule_server::runtime::adapters::ObjectStore;
use quantrule_server::runtime::server::router;
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
    let state = state_with_catalog().await;
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
    let state = state_with_catalog().await;
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
    let state = state_with_catalog().await;
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
async fn an_unknown_profile_is_not_found() {
    let response = router(state_with_catalog().await)
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
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, "alpha")
                .body(Body::from(CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router(state)
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
async fn a_dataset_is_uploaded_listed_and_read() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    assert_eq!(record.summary.name, "candles.csv");
    assert_eq!(record.summary.candles, 2);
    assert!(record.object_key.starts_with("datasets/"));
    let stored = objects.get(&record.object_key).await.expect("object");
    assert_eq!(
        stored,
        b"1704067200,100,101,99,100.5,10\n1704067260,100.5,101.5,100,101,12"
    );
    assert_eq!(Digest::sha256(&stored), record.summary.data_version);
    let version = record.summary.data_version.to_string();

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<DatasetSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "candles.csv");

    let response = router(state)
        .oneshot(
            Request::get(format!("/catalog/datasets/{version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_dataset(&body_bytes(response).await), record);
}

#[tokio::test]
async fn the_same_bytes_are_idempotent() {
    let state = state_with_catalog().await;
    let first = router(Arc::clone(&state))
        .oneshot(upload("one.csv"))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = read_dataset(&body_bytes(first).await);

    let second = router(Arc::clone(&state))
        .oneshot(upload("two.csv"))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second = read_dataset(&body_bytes(second).await);
    assert_eq!(first.summary.data_version, second.summary.data_version);
    assert_eq!(first.summary.name, second.summary.name);
    assert_eq!(first.object_key, second.object_key);

    let response = router(state)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<DatasetSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert_eq!(listed.len(), 1);
}

#[tokio::test]
async fn two_profiles_share_the_object_and_keep_their_own_rows() {
    let state = state_with_catalog().await;
    let first = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, "alpha")
                .body(Body::from(CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = read_dataset(&body_bytes(first).await);

    let second = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, "beta")
                .body(Body::from(CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::CREATED);
    let second = read_dataset(&body_bytes(second).await);
    assert_eq!(first.object_key, second.object_key);

    let response = router(state)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "gamma")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<DatasetSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert!(listed.is_empty());
}

#[tokio::test]
async fn a_missing_profile_is_refused() {
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .body(Body::from(CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}

#[tokio::test]
async fn an_unknown_version_is_not_found() {
    let version = Digest::sha256(b"missing").to_string();
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::get(format!("/catalog/datasets/{version}"))
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
async fn a_broken_line_is_rejected_with_a_code() {
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::post("/catalog/datasets?name=broken.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from("2024-01-01 00:00:00+00:00,100,90,95\n"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.invalid"), "{text}");
}

#[tokio::test]
async fn without_a_catalog_the_route_reports_internal() {
    let response = router(state())
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn an_oversized_upload_is_refused() {
    let response = router(state_with_small_upload().await)
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("upload.too_large"), "{text}");
}

#[tokio::test]
async fn a_dataset_is_inspected_by_its_version() {
    let (state, _objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/inspect/candles?data_version={version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let summary: DatasetSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("summary");
    assert_eq!(summary.candles, 2);
    assert_eq!(summary.first_bar, 1_704_067_200);

    let response = router(state)
        .oneshot(
            Request::get(format!("/inspect/timeframes?data_version={version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let inspection: TimeframeInspection =
        serde_json::from_slice(&body_bytes(response).await).expect("inspection");
    assert_eq!(inspection.data_version, record.summary.data_version);
    let codes: Vec<&str> = inspection
        .timeframes
        .iter()
        .map(|bars| bars.timeframe.as_str())
        .collect();
    assert_eq!(codes, vec!["1m", "5m", "15m", "1h", "4h"]);
    assert_eq!(inspection.timeframes[0].bars, 2);
    assert_eq!(inspection.timeframes[1].bars, 0);
    assert_eq!(inspection.timeframes[1].partial_buckets, 1);
}

#[tokio::test]
async fn an_unknown_version_cannot_be_inspected() {
    let version = Digest::sha256(b"missing").to_string();
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::get(format!("/inspect/candles?data_version={version}"))
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
async fn a_missing_object_is_a_server_error() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    objects.remove(&record.object_key);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/inspect/candles?data_version={version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn a_corrupt_object_is_refused() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    objects
        .put(&record.object_key, b"not,a,candle".to_vec())
        .await
        .expect("overwrite");
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/inspect/timeframes?data_version={version}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn the_candle_stream_is_byte_identical() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/candles"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/csv")
    );
    let etag = format!("\"{}\"", record.summary.data_version);
    assert_eq!(
        response
            .headers()
            .get("etag")
            .and_then(|value| value.to_str().ok()),
        Some(etag.as_str())
    );
    assert_eq!(
        body_bytes(response).await,
        b"1704067200,100,101,99,100.5,10\n1704067260,100.5,101.5,100,101,12"
    );
}

#[tokio::test]
async fn another_profile_cannot_read_the_stream() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, "alpha")
                .body(Body::from(CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/candles"))
                .header(PROFILE_HEADER, "beta")
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
async fn two_profiles_keep_independent_hypotheses() {
    let state = state_with_store().await;
    let payload = serde_json::to_vec(&body()).expect("encode");

    let mut fingerprints = Vec::new();
    for profile in ["alpha", "beta"] {
        let response = router(Arc::clone(&state))
            .oneshot(
                Request::post("/hypotheses")
                    .header(PROFILE_HEADER, profile)
                    .header("content-type", "application/json")
                    .body(Body::from(payload.clone()))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::CREATED);
        let sealed: SealedHypothesis =
            serde_json::from_slice(&body_bytes(response).await).expect("decode");
        fingerprints.push(sealed.fingerprint().to_owned());
    }
    assert_eq!(fingerprints[0], fingerprints[1]);

    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/hypotheses/{}", fingerprints[0]))
                .header(PROFILE_HEADER, "alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(state)
        .oneshot(
            Request::get(format!("/hypotheses/{}", fingerprints[0]))
                .header(PROFILE_HEADER, "gamma")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}
