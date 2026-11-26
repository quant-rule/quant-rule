//! The model compute route over a catalogued dataset and config.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, seed, state};
use quantrule_api::{Digest, ModelObject, PROFILE_HEADER};
use quantrule_model_service::router;
use std::sync::Arc;
use tower::ServiceExt;

/// The coupled gateway route at `fffb33c3` produced this object for the same
/// fixture. It is the regression oracle for the move: the public model object
/// must stay byte-identical.
///
/// The oracle was regenerated for `1.6.1`. The object carries the compiled
/// package version. The `1.6.0` object is 66,022 bytes and hashes to
/// `sha256:684760a01fce09f3c458ff7dbdfb3090b5d2a5fb06de2e375e850c543d93aae2`;
/// `1.6.1` has the same length and changes only that string. The object key's
/// model-registry component (`2026-11-10.2`) is unchanged, and no reading,
/// event, or report value changed.
const BASELINE_SHA256: &str =
    "sha256:ead976e866f9bd32ae85c5a5c71ca4fc81a54c4a0240a5d78544c24aa63c8061";
const BASELINE_BYTES: u64 = 66_022;

async fn compute(
    state: Arc<quantrule_model_service::ModelState>,
    payload: serde_json::Value,
) -> axum::response::Response {
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

#[tokio::test]
async fn the_full_ladder_is_stored_and_reused() {
    let seed = seed().await;
    let state = state(&seed);
    let payload = serde_json::json!({
        "data_version": seed.data_version,
        "config_id": "cfg_valid",
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
    assert_eq!(object.data_version, seed.data_version);
    assert_eq!(
        object.object_key,
        "model/c39ffbe3a18015ef81c33173615167b1ca7d8aeddf31df589f670868ac496d71/\
         ec654fac9599f62e79e2706abef23dfb7c07c08185aa86db4d8695f0b718d1b3/\
         2026-11-10.2/1m-5m-15m-1h.json"
    );
    assert_eq!(object.sha256.to_string(), BASELINE_SHA256);
    assert_eq!(object.bytes, BASELINE_BYTES);
    let stored = state
        .objects
        .as_ref()
        .expect("objects")
        .get(&object.object_key)
        .await
        .expect("stored");
    assert_eq!(stored.len() as u64, object.bytes);
    assert_eq!(object.sha256, Digest::sha256(&stored));
    assert_eq!(Digest::sha256(&stored).to_string(), BASELINE_SHA256);
    let text = String::from_utf8(stored).expect("utf8");
    assert!(text.contains("\"flat_report\""), "{text}");

    let again = compute(state, payload).await;
    assert_eq!(again.status(), StatusCode::OK);
    let again: ModelObject = serde_json::from_slice(&body_bytes(again).await).expect("again");
    assert_eq!(again.object_key, object.object_key);
    assert_eq!(again.sha256, object.sha256);
}

#[tokio::test]
async fn the_api_versioned_path_answers_the_same_object() {
    let seed = seed().await;
    let state = state(&seed);
    let payload = serde_json::json!({
        "data_version": seed.data_version,
        "config_id": "cfg_valid",
    });
    let response = router(state)
        .oneshot(
            Request::post("/api/v1/model/compute")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let object: ModelObject =
        serde_json::from_slice(&body_bytes(response).await).expect("model object");
    assert_eq!(object.sha256.to_string(), BASELINE_SHA256);
}

#[tokio::test]
async fn a_single_timeframe_shard_has_its_own_object() {
    let seed = seed().await;
    let state = state(&seed);
    let response = compute(
        state,
        serde_json::json!({
            "data_version": seed.data_version,
            "config_id": "cfg_valid",
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
    let seed = seed().await;
    let state = state(&seed);
    let response = compute(
        state,
        serde_json::json!({
            "data_version": seed.data_version,
            "config_id": "cfg_valid",
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
    let seed = seed().await;
    let state = state(&seed);
    let response = compute(
        state,
        serde_json::json!({
            "data_version": seed.data_version,
            "config_id": "cfg_invalid",
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("config.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_dataset_is_not_found() {
    let seed = seed().await;
    let state = state(&seed);
    let response = compute(
        state,
        serde_json::json!({
            "data_version": Digest::sha256(b"missing"),
            "config_id": "cfg_valid",
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}

#[tokio::test]
async fn the_injected_profile_header_is_required() {
    let seed = seed().await;
    let response = router(state(&seed))
        .oneshot(
            Request::post("/model/compute")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "data_version": seed.data_version,
                        "config_id": "cfg_valid",
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}
