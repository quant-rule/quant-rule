//! The generic artifact lineage and retention route contracts.
//!
//! The signal writer does not exist yet. These tests exercise the seam it will
//! use: registration by content id, a consumer reference that protects the
//! artifact, and the retention pass that removes only unreferenced rows after
//! the window and records a tombstone.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
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
