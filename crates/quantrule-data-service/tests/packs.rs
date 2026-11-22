//! The pack registry's public and internal route contracts.
//!
//! The assertions are the client-visible status, envelope code, body, canonical
//! bytes and ETag, so a moved handler that changes a byte is visible here. The
//! lifecycle test proves that validation, freeze and retire are explicit,
//! idempotent where the contract says so, and that a frozen pack cannot be
//! mutated or deleted.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{body_bytes, state_with_research_artifacts};
use quantrule_api::{
    Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER, PackKind, PackRecord, PackState, PackSummary,
};
use quantrule_data_service::router;
use serde_json::{Value, json};
use tower::ServiceExt;

fn strategy(version: &str, reward: f64) -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": version,
        "name": "compression-fade-5m",
        "owner": "operator",
        "payload": {
            "mode": "signal",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": reward},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1}
            ],
            "exit": []
        }
    })
}

fn body(document: &Value) -> Body {
    Body::from(serde_json::to_vec(document).expect("encode"))
}

fn create(document: &Value) -> Request<Body> {
    Request::post("/catalog/packs")
        .header(PROFILE_HEADER, "default")
        .header(header::CONTENT_TYPE, "application/json")
        .body(body(document))
        .expect("request")
}

fn read_pack(bytes: &[u8]) -> PackRecord {
    serde_json::from_slice(bytes).expect("pack record")
}

fn read_error(bytes: &[u8]) -> ErrorEnvelope {
    serde_json::from_slice(bytes).expect("error envelope")
}

#[tokio::test]
async fn a_pack_is_created_listed_shown_and_fetched_by_content_identity() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let document = strategy("1.0.0", 1.5);
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&document))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_pack(&body_bytes(response).await);
    assert_eq!(record.summary.kind, PackKind::Strategy);
    assert_eq!(record.summary.state, PackState::Draft);
    assert_eq!(record.summary.name, "compression-fade-5m");
    assert_eq!(record.summary.version, "1.0.0");
    assert_eq!(record.sha256, record.summary.pack_id);
    assert_eq!(record.events.len(), 1);
    assert_eq!(record.events[0].action, "create");
    assert!(record.object_key.starts_with("packs/"));
    let id = record.summary.pack_id.to_string();

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/catalog/packs")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<PackSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].pack_id, record.summary.pack_id);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/catalog/packs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(read_pack(&body_bytes(response).await), record);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/packs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/catalog/packs/{id}/bytes"))
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
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("application/json")
    );
    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .expect("etag")
        .to_owned();
    assert_eq!(etag, format!("\"{id}\""));
    let bytes = body_bytes(response).await;
    assert_eq!(Digest::sha256(&bytes), record.summary.pack_id);
    assert!(bytes.ends_with(b"\n"));

    let response = router(state)
        .oneshot(
            Request::get(format!("/catalog/packs/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
}

#[tokio::test]
async fn the_same_bytes_are_idempotent_and_a_label_with_different_bytes_is_refused() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let document = strategy("1.0.0", 1.5);
    let first = router(std::sync::Arc::clone(&state))
        .oneshot(create(&document))
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = read_pack(&body_bytes(first).await);

    let second = router(std::sync::Arc::clone(&state))
        .oneshot(create(&document))
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second = read_pack(&body_bytes(second).await);
    assert_eq!(second.summary.pack_id, first.summary.pack_id);
    assert_eq!(second.events, first.events);

    let changed = strategy("1.0.0", 2.0);
    let response = router(state)
        .oneshot(create(&changed))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let error = read_error(&body_bytes(response).await);
    assert_eq!(error.error.code, ErrorCode::PackConflict);
    assert!(error.error.message.contains("already names"));
}

#[tokio::test]
async fn freeze_is_explicit_idempotent_and_immutable() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&strategy("1.0.0", 1.5)))
        .await
        .expect("response");
    let record = read_pack(&body_bytes(response).await);
    let id = record.summary.pack_id.to_string();

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/freeze"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::PackConflict
    );

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/validate"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let validated = read_pack(&body_bytes(response).await);
    assert_eq!(validated.summary.state, PackState::Validated);
    assert_eq!(validated.validations.len(), 1);
    assert!(validated.validations[0].valid);
    assert!(validated.summary.validated_at.is_some());

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/freeze"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let frozen = read_pack(&body_bytes(response).await);
    assert_eq!(frozen.summary.state, PackState::Frozen);
    assert!(frozen.summary.frozen_at.is_some());

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/freeze"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let again = read_pack(&body_bytes(response).await);
    assert_eq!(
        again.events, frozen.events,
        "a repeat freeze added an event"
    );
    assert_eq!(again.summary.frozen_at, frozen.summary.frozen_at);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/validate"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/catalog/packs/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let bytes = body_bytes(response).await;
    assert_eq!(Digest::sha256(&bytes), record.summary.pack_id);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/retire"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let retired = read_pack(&body_bytes(response).await);
    assert_eq!(retired.summary.state, PackState::Retired);
    assert!(retired.summary.retired_at.is_some());

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/catalog/packs/{id}/retire"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        read_pack(&body_bytes(response).await).events,
        retired.events
    );

    // Retire is not delete: the show route and the bytes remain.
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/catalog/packs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let response = router(state)
        .oneshot(
            Request::delete(format!("/catalog/packs/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn a_profile_only_sees_its_own_packs() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&strategy("1.0.0", 1.5)))
        .await
        .expect("response");
    let record = read_pack(&body_bytes(response).await);
    let id = record.summary.pack_id.to_string();

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/catalog/packs")
                .header(PROFILE_HEADER, "other")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<PackSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert!(listed.is_empty());

    let response = router(state)
        .oneshot(
            Request::get(format!("/catalog/packs/{id}"))
                .header(PROFILE_HEADER, "other")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::PackNotFound
    );
}

#[tokio::test]
async fn a_missing_pack_object_fails_closed_on_every_byte_reading_route() {
    let (state, objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&strategy("1.0.0", 1.5)))
        .await
        .expect("response");
    let record = read_pack(&body_bytes(response).await);
    let id = record.summary.pack_id.to_string();
    objects.remove(&record.object_key);

    for (method, path) in [
        ("GET", format!("/catalog/packs/{id}")),
        ("GET", format!("/catalog/packs/{id}/bytes")),
        ("POST", format!("/catalog/packs/{id}/validate")),
        ("POST", format!("/catalog/packs/{id}/freeze")),
        ("POST", format!("/catalog/packs/{id}/retire")),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("request");
        let response = router(std::sync::Arc::clone(&state))
            .oneshot(request)
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{method} served a pack whose object is absent"
        );
        let error = read_error(&body_bytes(response).await);
        assert_eq!(error.error.code, ErrorCode::Internal);
        assert!(
            error.error.message.contains("missing or unreadable"),
            "the failure must name the missing object: {}",
            error.error.message
        );
    }

    // The row is still there; only the bytes are missing. A registration with
    // the same content writes the object first and repairs the identity.
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&strategy("1.0.0", 1.5)))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(objects.read(&record.object_key).is_some());
}

#[tokio::test]
async fn a_tampered_pack_object_fails_closed_on_every_byte_reading_route() {
    let (state, objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&strategy("1.0.0", 1.5)))
        .await
        .expect("response");
    let record = read_pack(&body_bytes(response).await);
    let id = record.summary.pack_id.to_string();
    objects
        .store(&record.object_key, b"not the pack".to_vec())
        .await
        .expect("tampered object");

    for (method, path) in [
        ("GET", format!("/catalog/packs/{id}")),
        ("GET", format!("/catalog/packs/{id}/bytes")),
        ("POST", format!("/catalog/packs/{id}/validate")),
        ("POST", format!("/catalog/packs/{id}/freeze")),
        ("POST", format!("/catalog/packs/{id}/retire")),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("request");
        let response = router(std::sync::Arc::clone(&state))
            .oneshot(request)
            .await
            .expect("response");
        assert_eq!(
            response.status(),
            StatusCode::INTERNAL_SERVER_ERROR,
            "{method} served a pack whose object hash is wrong"
        );
        let error = read_error(&body_bytes(response).await);
        assert_eq!(error.error.code, ErrorCode::Internal);
        assert!(
            error.error.message.contains("does not match"),
            "the failure must name the identity mismatch: {}",
            error.error.message
        );
    }
}

#[tokio::test]
async fn an_invalid_pack_and_an_unknown_id_are_refused() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let mut document = strategy("1.0.0", 1.5);
    document["payload"]["venue"] = json!("binance");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(create(&document))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::PackInvalid
    );

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post("/catalog/packs")
                .header(PROFILE_HEADER, "default")
                .body(Body::from("{not json"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let missing = Digest::sha256(b"missing");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/catalog/packs/{missing}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::PackNotFound
    );

    let response = router(state)
        .oneshot(
            Request::get("/catalog/packs?kind=book")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::PackInvalid
    );
}
