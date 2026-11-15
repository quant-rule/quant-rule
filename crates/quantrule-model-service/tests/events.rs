//! The internal event-batch route: one stored dataset and timeframe.
//!
//! Inspection computes a model, so it lives in the model service. The route
//! returns the same full availability-ordered batch the `events` command
//! publishes, writes no object, and never renders a label.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, canonical, seed_from, state};
use quantrule_api::{Digest, EventBatch, PROFILE_HEADER};
use quantrule_model_service::router;
use std::path::Path;
use std::sync::Arc;
use tower::ServiceExt;

/// The event fixture has one invalid candle-free row; the canonical bytes hold
/// the 260 one-minute bars the batch is computed over.
const FIXTURE_CANDLES: u64 = 260;

async fn seed() -> common::Seed {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/events_m1.csv");
    let text = std::fs::read_to_string(path).expect("fixture");
    let bytes = canonical(&text);
    let candles = bytes.iter().filter(|byte| **byte == b'\n').count() as u64;
    assert_eq!(candles, FIXTURE_CANDLES);
    seed_from(&bytes, candles).await
}

async fn events(
    state: Arc<quantrule_model_service::ModelState>,
    query: &str,
    profile: Option<&str>,
) -> axum::response::Response {
    let mut request = Request::get(format!("/internal/v1/events?{query}"));
    if let Some(profile) = profile {
        request = request.header(PROFILE_HEADER, profile);
    }
    router(state)
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response")
}

#[tokio::test]
async fn a_dataset_and_timeframe_publish_their_event_batch() {
    let seed = seed().await;
    let objects_before = seed.objects.keys();
    let state = state(&seed);
    let query = format!("data_version={}&timeframe=1m", seed.data_version);
    let response = events(Arc::clone(&state), &query, Some("default")).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_bytes(response).await;
    let batch: EventBatch = serde_json::from_slice(&body).expect("event batch");
    assert_eq!(batch.data_version, seed.data_version);
    assert_eq!(batch.timeframe, "1m");
    assert!(!batch.events.is_empty(), "the fixture must publish events");
    for event in &batch.events {
        assert!((1..=3).contains(&event.event_type), "{event:?}");
        assert!(
            event.available_bar >= event.occurred_bar,
            "an event cannot be knowable before it occurred"
        );
    }

    let again = events(state, &query, Some("default")).await;
    assert_eq!(body_bytes(again).await, body, "the batch is deterministic");
    assert_eq!(
        seed.objects.keys(),
        objects_before,
        "inspection writes no object"
    );
}

#[tokio::test]
async fn an_unknown_timeframe_is_rejected() {
    let seed = seed().await;
    let query = format!("data_version={}&timeframe=7m", seed.data_version);
    let response = events(state(&seed), &query, Some("default")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.invalid"), "{text}");
}

#[tokio::test]
async fn a_timeframe_without_a_complete_bar_is_rejected() {
    let seed = seed_from(&common::csv(10), 10).await;
    let query = format!("data_version={}&timeframe=4h", seed.data_version);
    let response = events(state(&seed), &query, Some("default")).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found() {
    let seed = seed().await;
    let query = format!("data_version={}&timeframe=1m", Digest::sha256(b"missing"));
    let response = events(state(&seed), &query, Some("default")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}

#[tokio::test]
async fn the_profile_header_is_required() {
    let seed = seed().await;
    let query = format!("data_version={}&timeframe=1m", seed.data_version);
    let response = events(state(&seed), &query, None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}
