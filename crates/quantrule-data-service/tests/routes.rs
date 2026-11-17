//! The data service's public routes, exercised over HTTP with in-memory stores.
//!
//! The assertions are the client-visible status, envelope code, body, content
//! type, and ETag, so a moved handler that changes a byte is visible here.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    CSV, body_bytes, minutes, read_dataset, state_with_catalog, state_with_catalog_and_objects,
    state_with_small_upload, upload,
};
use quantrule_api::{DatasetSummary, Digest, PROFILE_HEADER, TimeframeInspection};
use quantrule_data_service::router;
use quantrule_runtime::adapters::ObjectStore;
use std::sync::Arc;
use tower::ServiceExt;

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
    let response = router(state_with_catalog().await)
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let no_catalog = common::state(
        &quantrule_runtime::Config::default(),
        quantrule_data_service::DataServices::default(),
    );
    let response = router(no_catalog)
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
async fn a_five_minute_ladder_is_built_and_reused() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets?name=candles.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(minutes(12)))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let expected = b"1704067200,100,105,99,104.5,50\n1704067500,105,110,104,109.5,50";
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::get(format!("/datasets/{version}/ladder/5m"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get("etag").is_some());
    assert_eq!(body_bytes(response).await, expected);

    let key = format!("ladder/{}/full/5m", record.summary.data_version.as_hex());
    assert_eq!(objects.get(&key).await.expect("cached"), expected);

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/ladder/5m"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_bytes(response).await, expected);
}

#[tokio::test]
async fn the_m1_ladder_is_the_candle_stream() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/ladder/1m"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body_bytes(response).await,
        b"1704067200,100,101,99,100.5,10\n1704067260,100.5,101.5,100,101,12"
    );
}

#[tokio::test]
async fn a_timeframe_without_a_complete_bar_is_refused() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/ladder/1h"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_timeframe_is_refused() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(upload("candles.csv"))
        .await
        .expect("response");
    let record = read_dataset(&body_bytes(response).await);
    let version = record.summary.data_version.to_string();

    let response = router(state)
        .oneshot(
            Request::get(format!("/datasets/{version}/ladder/7m"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn the_client_prefix_serves_the_same_routes() {
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::get("/api/v1/catalog/datasets")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

const TOLERANT_CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:02:30+00:00,100,101,99,100.5,10\n\
2024-01-01 00:02:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:01:00+00:00,100,101,99,100.5,10\n\
2024-01-01 00:03:00+00:00,100,90,95,99,10\n\
2024-01-01 00:04:00+00:00,100,101,99,100.5,10\n";

#[tokio::test]
async fn a_tolerant_upload_keeps_the_good_rows_and_reports_the_bad() {
    let state = state_with_catalog().await;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/datasets/tolerant?name=tolerant.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(TOLERANT_CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    assert_eq!(record.summary.candles, 4);
    let quality = record.quality.clone().expect("quality");
    assert_eq!(quality.rows_read, 8);
    assert_eq!(quality.accepted, 4);
    assert_eq!(quality.rejected, 4);
    assert_eq!(quality.duplicates, 1);
    assert_eq!(quality.out_of_order, 1);
    assert_eq!(quality.invalid, 2);
    let reasons: Vec<&str> = quality
        .skipped
        .iter()
        .map(|row| row.reason.as_str())
        .collect();
    assert_eq!(
        reasons,
        vec!["duplicate", "misaligned", "out_of_order", "invalid"]
    );

    let version = record.summary.data_version.to_string();
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
    let stored = read_dataset(&body_bytes(response).await);
    assert_eq!(stored.quality, record.quality);
}

#[tokio::test]
async fn the_strict_route_still_refuses_the_bad_rows() {
    let state = state_with_catalog().await;
    let response = router(state)
        .oneshot(
            Request::post("/catalog/datasets?name=strict.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(TOLERANT_CSV))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.invalid"), "{text}");
}

#[tokio::test]
async fn a_tolerant_upload_with_no_valid_row_is_refused() {
    let state = state_with_catalog().await;
    let csv = "date,open,high,low,close\nnot,a,candle\n";
    let response = router(state)
        .oneshot(
            Request::post("/catalog/datasets/tolerant?name=bad.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(csv))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn an_oversized_config_bundle_is_refused() {
    let state = state_with_catalog().await;
    let response = router(state)
        .oneshot(
            Request::post("/catalog/configs")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from("x".repeat(5 * 1024 * 1024)))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("upload.too_large"), "{text}");
}
