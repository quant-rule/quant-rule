//! The research-memory surface: sealing, reading, and profile scoping.

mod common;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::body_bytes;
use quantrule_api::PROFILE_HEADER;
use quantrule_evidence_service::research::MemoryStore;
use quantrule_evidence_service::{EvidenceServices, EvidenceState, router};
use quantrule_research::{CostModel, Partition, SealedHypothesis, Split};
use quantrule_runtime::{Config, HealthRegistry};
use tower::ServiceExt;

fn state() -> Arc<EvidenceState> {
    Arc::new(EvidenceState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        EvidenceServices {
            hypotheses: Some(Arc::new(MemoryStore::default())),
            ..EvidenceServices::default()
        },
    ))
}

fn body() -> serde_json::Value {
    let split = Split::new(
        Partition::new("dev", 100, 200).expect("dev"),
        Partition::new("val", 200, 300).expect("val"),
    )
    .expect("split");
    serde_json::json!({
        "premise": "a retest holds",
        "rule": "strategy/retest.toml",
        "dataset": "sha256:abc",
        "costs": CostModel::new(1.0, 0.0, 0.0).expect("costs"),
        "split": split,
        "criteria": "expectancy above zero",
    })
}

#[tokio::test]
async fn a_hypothesis_is_sealed_and_read_back() {
    let state = state();
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
    let back: SealedHypothesis =
        serde_json::from_slice(&body_bytes(response).await).expect("decode");
    assert_eq!(back, sealed);
}

#[tokio::test]
async fn a_hypothesis_is_profile_scoped() {
    let state = state();
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
    let sealed: SealedHypothesis =
        serde_json::from_slice(&body_bytes(response).await).expect("decode");
    let fingerprint = sealed.fingerprint().to_owned();

    let response = router(state)
        .oneshot(
            Request::get(format!("/hypotheses/{fingerprint}"))
                .header(PROFILE_HEADER, "alpha")
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
async fn an_unknown_hypothesis_is_not_found_with_a_code() {
    let response = router(state())
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
    let response = router(state())
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
    let state = Arc::new(EvidenceState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        EvidenceServices::default(),
    ));
    let payload = serde_json::to_vec(&body()).expect("encode");
    let response = router(state)
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
async fn the_client_prefix_serves_the_same_route() {
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

#[tokio::test]
async fn an_unknown_profile_value_is_not_silently_defaulted() {
    let response = router(state())
        .oneshot(
            Request::get("/hypotheses/missing")
                .header(PROFILE_HEADER, "Bad!")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");
}
