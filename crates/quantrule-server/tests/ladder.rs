//! The stored ladder routes, exercised over HTTP with in-memory services.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{
    body_bytes, minutes, read_dataset, state_with_catalog, state_with_catalog_and_objects, upload,
};
use quantrule_api::PROFILE_HEADER;
use quantrule_server::runtime::adapters::ObjectStore;
use quantrule_server::runtime::server::router;
use std::sync::Arc;
use tower::ServiceExt;

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
