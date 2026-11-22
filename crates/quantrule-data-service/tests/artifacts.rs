//! The generic artifact lineage and retention route contracts.
//!
//! The signal writer does not exist yet. These tests exercise the seam it will
//! use: registration by content id, a consumer reference that protects the
//! artifact, and the retention pass that removes only unreferenced rows after
//! the window and records a tombstone.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use common::{body_bytes, state_with_research_artifacts};
use quantrule_api::{
    ArtifactKind, ArtifactRecord, ArtifactReferenceCount, CollectArtifactsResponse,
    DatasetReferenceKind, Digest, ErrorCode, ErrorEnvelope, PROFILE_HEADER,
};
use quantrule_data_service::artifacts::store::{ArtifactCatalog, NewArtifact};
use quantrule_data_service::router;
use quantrule_runtime::adapters::ObjectStore;
use tower::ServiceExt;

fn profile(name: &str) -> quantrule_api::ProfileId {
    quantrule_api::ProfileId::parse(name).expect("profile")
}

fn read_error(bytes: &[u8]) -> ErrorEnvelope {
    serde_json::from_slice(bytes).expect("error envelope")
}

#[tokio::test]
async fn an_artifact_is_registered_and_read_by_content_identity() {
    let (state, objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let bytes = b"{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::put("/internal/v1/artifacts/signal")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(bytes.to_vec()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let record: ArtifactRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("record");
    assert_eq!(record.summary.kind, ArtifactKind::Signal);
    assert_eq!(record.summary.artifact_id, Digest::sha256(bytes));
    assert_eq!(record.summary.bytes, bytes.len() as u64);
    assert!(record.object_key.starts_with("artifacts/"));
    let id = record.summary.artifact_id.to_string();

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::put("/internal/v1/artifacts/signal")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(bytes.to_vec()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/artifacts/signal/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let fetched: ArtifactRecord =
        serde_json::from_slice(&body_bytes(response).await).expect("record");
    assert_eq!(fetched, record);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/internal/v1/artifacts/signal/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_bytes(response).await, bytes);
    assert!(objects.exists(&record.object_key).await.expect("exists"));

    let response = router(state)
        .oneshot(
            Request::get("/internal/v1/artifacts?kind=book")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactInvalid
    );
}

fn metadata_request(id: &Digest, bytes: u64) -> Request<Body> {
    Request::post(format!("/internal/v1/artifacts/signal/{id}"))
        .header(PROFILE_HEADER, "default")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_vec(&serde_json::json!({"bytes": bytes})).expect("body"),
        ))
        .expect("request")
}

#[tokio::test]
async fn an_already_stored_artifact_is_registered_from_metadata_only() {
    let (state, objects, _packs, artifacts) = state_with_research_artifacts().await;
    let bytes = b"{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";
    let id = Digest::sha256(bytes);
    let key = format!("artifacts/{}", id.as_hex());
    objects.store(&key, bytes.to_vec()).await.expect("object");

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(metadata_request(&id, bytes.len() as u64))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let registration: quantrule_service_api::ArtifactRegistration =
        serde_json::from_slice(&body_bytes(response).await).expect("registration");
    assert!(registration.created);
    assert_eq!(registration.record.summary.artifact_id, id);
    assert_eq!(registration.record.summary.bytes, bytes.len() as u64);
    assert_eq!(registration.record.object_key, key);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(metadata_request(&id, bytes.len() as u64))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let duplicate: quantrule_service_api::ArtifactRegistration =
        serde_json::from_slice(&body_bytes(response).await).expect("registration");
    assert!(!duplicate.created);
    assert_eq!(duplicate.record, registration.record);
    assert_eq!(
        artifacts
            .list(profile("default"), ArtifactKind::Signal)
            .await
            .expect("list")
            .len(),
        1
    );
}

#[tokio::test]
async fn metadata_registration_fails_closed_on_a_missing_or_mismatched_object() {
    let (state, objects, _packs, artifacts) = state_with_research_artifacts().await;
    let bytes = b"signal-bytes";
    let id = Digest::sha256(bytes);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(metadata_request(&id, bytes.len() as u64))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactNotFound
    );

    let key = format!("artifacts/{}", id.as_hex());
    objects
        .store(&key, b"different-bytes".to_vec())
        .await
        .expect("wrong object");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(metadata_request(&id, bytes.len() as u64))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactInvalid
    );

    objects
        .store(&key, bytes.to_vec())
        .await
        .expect("right object");
    let response = router(state)
        .oneshot(metadata_request(&id, 1))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactInvalid
    );
    assert!(
        artifacts
            .list(profile("default"), ArtifactKind::Signal)
            .await
            .expect("list")
            .is_empty(),
        "a failed verification publishes no row"
    );
}

#[tokio::test]
async fn a_metadata_registered_artifact_stays_referenced() {
    let (state, objects, _packs, artifacts) = state_with_research_artifacts().await;
    let bytes = b"referenced-signal";
    let id = Digest::sha256(bytes);
    let key = format!("artifacts/{}", id.as_hex());
    objects.store(&key, bytes.to_vec()).await.expect("object");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(metadata_request(&id, bytes.len() as u64))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!("/internal/v1/artifacts/signal/{id}/references"))
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "reference_id": "run:later",
                        "kind": DatasetReferenceKind::Run
                    }))
                    .expect("body"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);

    let response = router(state)
        .oneshot(
            Request::post("/internal/v1/artifacts/collect")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let report: CollectArtifactsResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("report");
    assert_eq!(report.collected, 0);
    assert!(objects.exists(&key).await.expect("object"));
    assert!(
        artifacts
            .get(profile("default"), ArtifactKind::Signal, id.to_string())
            .await
            .expect("get")
            .is_some()
    );
}

#[tokio::test]
async fn a_referenced_artifact_is_retained_and_an_unreferenced_one_is_collected() {
    let (state, objects, _packs, artifacts) = state_with_research_artifacts().await;
    let retained = NewArtifact {
        artifact_id: Digest::sha256(b"retained"),
        kind: ArtifactKind::Signal,
        object_key: format!("artifacts/{}", Digest::sha256(b"retained").as_hex()),
        bytes: 128,
        created_at: "2026-01-01T00:00:00Z".to_owned(),
    };
    let collected = NewArtifact {
        artifact_id: Digest::sha256(b"collected"),
        kind: ArtifactKind::Signal,
        object_key: format!("artifacts/{}", Digest::sha256(b"collected").as_hex()),
        bytes: 256,
        created_at: "2026-01-01T00:00:00Z".to_owned(),
    };
    for artifact in [&retained, &collected] {
        artifacts
            .register(profile("default"), artifact.clone())
            .await
            .expect("register");
        objects
            .store(&artifact.object_key, vec![b'x'; artifact.bytes as usize])
            .await
            .expect("object");
    }

    let retained_id = retained.artifact_id.to_string();
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!(
                "/internal/v1/artifacts/signal/{retained_id}/references"
            ))
            .header(PROFILE_HEADER, "default")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "reference_id": "run:one",
                    "kind": "run"
                }))
                .expect("body"),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let count: ArtifactReferenceCount =
        serde_json::from_slice(&body_bytes(response).await).expect("count");
    assert_eq!(count.references, 1);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post("/internal/v1/artifacts/collect")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let report: CollectArtifactsResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("report");
    assert_eq!(report.collected, 1);
    assert_eq!(report.bytes, 256);
    assert_eq!(report.objects, vec![collected.object_key.clone()]);
    assert!(!objects.exists(&collected.object_key).await.expect("gone"));
    assert!(
        objects
            .exists(&retained.object_key)
            .await
            .expect("retained")
    );
    assert_eq!(
        artifacts.retention_events(profile("default")),
        vec![(collected.artifact_id.clone(), 256)]
    );

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!(
                "/internal/v1/artifacts/signal/{}",
                collected.artifact_id
            ))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactNotFound
    );

    // A release removes the protection; the next pass may now collect it.
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!(
                "/internal/v1/artifacts/signal/{retained_id}/references/release"
            ))
            .header(PROFILE_HEADER, "default")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({"reference_id": "run:one"})).expect("body"),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<ArtifactReferenceCount>(&body_bytes(response).await)
            .expect("count")
            .references,
        0
    );
    let response = router(state)
        .oneshot(
            Request::post("/internal/v1/artifacts/collect")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let report: CollectArtifactsResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("report");
    assert_eq!(report.collected, 1);
    assert_eq!(
        artifacts.retention_events(profile("default")).len(),
        2,
        "every collected artifact keeps one tombstone"
    );
}

#[tokio::test]
async fn a_reference_needs_a_known_artifact_and_a_known_consumer_kind() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let missing = Digest::sha256(b"missing");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::post(format!(
                "/internal/v1/artifacts/signal/{missing}/references"
            ))
            .header(PROFILE_HEADER, "default")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&serde_json::json!({
                    "reference_id": "run:one",
                    "kind": DatasetReferenceKind::Run
                }))
                .expect("body"),
            ))
            .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactNotFound
    );

    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let response = router(state)
        .oneshot(
            Request::post("/internal/v1/artifacts/signal/not-a-digest/references")
                .header(PROFILE_HEADER, "default")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

async fn register_signal(
    state: &std::sync::Arc<quantrule_data_service::DataState>,
    bytes: &[u8],
) -> ArtifactRecord {
    let response = router(std::sync::Arc::clone(state))
        .oneshot(
            Request::put("/internal/v1/artifacts/signal")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(bytes.to_vec()))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    serde_json::from_slice(&body_bytes(response).await).expect("record")
}

#[tokio::test]
async fn public_signal_routes_list_show_and_fetch_verified_bytes() {
    let (state, _objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let bytes = b"{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";
    let record = register_signal(&state, bytes).await;
    let id = record.summary.artifact_id.to_string();

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/signals")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let rows: Vec<ArtifactRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert_eq!(rows, vec![record.clone()]);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/signals/{id}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let shown: ArtifactRecord = serde_json::from_slice(&body_bytes(response).await).expect("show");
    assert_eq!(shown, record);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/signals/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let etag = format!("\"{id}\"");
    assert_eq!(
        response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok()),
        Some(etag.as_str())
    );
    assert_eq!(body_bytes(response).await, bytes);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/signals/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .header(header::IF_NONE_MATCH, etag)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    assert!(body_bytes(response).await.is_empty());

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/signals?limit=1")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let capped: Vec<ArtifactRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert_eq!(capped.len(), 1);

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/signals?limit=0")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactInvalid
    );

    let response = router(state)
        .oneshot(
            Request::get("/signals")
                .header(PROFILE_HEADER, "alpha")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let other: Vec<ArtifactRecord> =
        serde_json::from_slice(&body_bytes(response).await).expect("list");
    assert!(other.is_empty(), "signal rows are profile-scoped");
}

#[tokio::test]
async fn a_public_signal_read_fails_closed_for_an_unknown_id_or_bytes() {
    let (state, objects, _packs, _artifacts) = state_with_research_artifacts().await;
    let missing = Digest::sha256(b"missing-signal");
    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get(format!("/signals/{missing}"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactNotFound
    );

    let response = router(std::sync::Arc::clone(&state))
        .oneshot(
            Request::get("/signals/not-a-digest")
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::ArtifactInvalid
    );

    let bytes = b"the original signal";
    let record = register_signal(&state, bytes).await;
    let id = record.summary.artifact_id.to_string();
    objects
        .store(&record.object_key, b"substituted signal".to_vec())
        .await
        .expect("substituted object");
    let response = router(state)
        .oneshot(
            Request::get(format!("/signals/{id}/bytes"))
                .header(PROFILE_HEADER, "default")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        read_error(&body_bytes(response).await).error.code,
        ErrorCode::Internal
    );
}
