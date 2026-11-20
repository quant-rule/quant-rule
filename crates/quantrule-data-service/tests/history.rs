//! Dataset runtime history over the real router.
//!
//! The test proves the client-visible contract: archive hides the row, a
//! registered reference refuses erase, the confirmation and archive order are
//! enforced, and the object is removed only when no profile still shares it.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{body_bytes, read_dataset, state_with_catalog_and_objects, upload};
use quantrule_api::{
    DatasetRecord, DatasetReferenceCount, DatasetSummary, EraseDatasetResponse, PROFILE_HEADER,
};
use quantrule_data_service::router;
use quantrule_runtime::adapters::ObjectStore;
use std::sync::Arc;
use tower::ServiceExt;

async fn call(
    state: Arc<quantrule_data_service::DataState>,
    request: Request<Body>,
) -> axum::response::Response {
    router(state).oneshot(request).await.expect("the response")
}

fn header(profile: &str) -> (&'static str, String) {
    (PROFILE_HEADER, profile.to_owned())
}

async fn upload_for(state: Arc<quantrule_data_service::DataState>, profile: &str) -> DatasetRecord {
    let (header, value) = header(profile);
    let mut request = upload("candles.csv");
    request
        .headers_mut()
        .insert(header, value.parse().expect("the profile header"));
    let response = call(state, request).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_dataset(&body_bytes(response).await)
}

async fn archive(
    state: Arc<quantrule_data_service::DataState>,
    profile: &str,
    version: &str,
) -> axum::response::Response {
    let (header, value) = header(profile);
    call(
        state,
        Request::post(format!("/catalog/datasets/{version}/archive"))
            .header(header, value)
            .body(Body::empty())
            .expect("the archive request"),
    )
    .await
}

async fn erase(
    state: Arc<quantrule_data_service::DataState>,
    profile: &str,
    version: &str,
    confirm: Option<&str>,
) -> axum::response::Response {
    let (header, value) = header(profile);
    let path = match confirm {
        Some(confirm) => format!("/catalog/datasets/{version}?confirm={confirm}"),
        None => format!("/catalog/datasets/{version}"),
    };
    call(
        state,
        Request::delete(path)
            .header(header, value)
            .body(Body::empty())
            .expect("the erase request"),
    )
    .await
}

async fn list(
    state: Arc<quantrule_data_service::DataState>,
    profile: &str,
    archived: bool,
) -> Vec<DatasetSummary> {
    let (header, value) = header(profile);
    let path = if archived {
        "/catalog/datasets?include_archived=true"
    } else {
        "/catalog/datasets"
    };
    let response = call(
        state,
        Request::get(path)
            .header(header, value)
            .body(Body::empty())
            .expect("the list request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("the dataset list")
}

async fn register(
    state: Arc<quantrule_data_service::DataState>,
    profile: &str,
    version: &str,
    reference_id: &str,
) -> DatasetReferenceCount {
    let (header, value) = header(profile);
    let body = serde_json::json!({"reference_id": reference_id, "kind": "run"});
    let response = call(
        state,
        Request::post(format!("/internal/v1/datasets/{version}/references"))
            .header(header, value)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("the reference request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("the reference count")
}

async fn release(
    state: Arc<quantrule_data_service::DataState>,
    profile: &str,
    version: &str,
    reference_id: &str,
) -> DatasetReferenceCount {
    let (header, value) = header(profile);
    let body = serde_json::json!({"reference_id": reference_id});
    let response = call(
        state,
        Request::post(format!(
            "/internal/v1/datasets/{version}/references/release"
        ))
        .header(header, value)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("the release request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&body_bytes(response).await).expect("the reference count")
}

#[tokio::test]
async fn archive_then_reference_then_erase_is_enforced() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let record = upload_for(Arc::clone(&state), "default").await;
    let version = record.summary.data_version.to_string();

    assert_eq!(list(Arc::clone(&state), "default", false).await.len(), 1);

    let archived = archive(Arc::clone(&state), "default", &version).await;
    assert_eq!(archived.status(), StatusCode::OK);
    let archived = read_dataset(&body_bytes(archived).await);
    assert!(archived.summary.archived_at.is_some());
    assert_eq!(list(Arc::clone(&state), "default", false).await.len(), 0);
    assert_eq!(list(Arc::clone(&state), "default", true).await.len(), 1);

    assert_eq!(
        register(Arc::clone(&state), "default", &version, "run:1")
            .await
            .references,
        1
    );
    let refused = erase(Arc::clone(&state), "default", &version, Some(&version)).await;
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    let text = String::from_utf8(body_bytes(refused).await).expect("utf8");
    assert!(text.contains("history.protected"), "{text}");

    assert_eq!(
        release(Arc::clone(&state), "default", &version, "run:1")
            .await
            .references,
        0
    );
    let wrong = erase(
        Arc::clone(&state),
        "default",
        &version,
        Some("sha256:wrong"),
    )
    .await;
    assert_eq!(wrong.status(), StatusCode::CONFLICT);
    let text = String::from_utf8(body_bytes(wrong).await).expect("utf8");
    assert!(text.contains("history.state"), "{text}");
    assert!(
        objects
            .exists(&record.object_key)
            .await
            .expect("the object")
    );

    let erased = erase(Arc::clone(&state), "default", &version, Some(&version)).await;
    assert_eq!(erased.status(), StatusCode::OK);
    let receipt: EraseDatasetResponse =
        serde_json::from_slice(&body_bytes(erased).await).expect("the receipt");
    assert_eq!(receipt.data_version, record.summary.data_version);
    assert_eq!(receipt.bytes, record.summary.bytes);
    assert!(
        !objects
            .exists(&record.object_key)
            .await
            .expect("the object")
    );

    let missing = call(
        Arc::clone(&state),
        Request::get(format!("/catalog/datasets/{version}"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the read request"),
    )
    .await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn erase_requires_archive_first() {
    let (state, _objects) = state_with_catalog_and_objects().await;
    let record = upload_for(Arc::clone(&state), "default").await;
    let version = record.summary.data_version.to_string();
    let response = erase(Arc::clone(&state), "default", &version, Some(&version)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(
        text.contains("archive the dataset before erasing it"),
        "{text}"
    );
}

#[tokio::test]
async fn a_shared_object_survives_one_profile_erase() {
    let (state, objects) = state_with_catalog_and_objects().await;
    let first = upload_for(Arc::clone(&state), "default").await;
    let second = upload_for(Arc::clone(&state), "other").await;
    let version = first.summary.data_version.to_string();
    assert_eq!(first.object_key, second.object_key);

    archive(Arc::clone(&state), "default", &version).await;
    let erased = erase(Arc::clone(&state), "default", &version, Some(&version)).await;
    assert_eq!(erased.status(), StatusCode::OK);
    let receipt: EraseDatasetResponse =
        serde_json::from_slice(&body_bytes(erased).await).expect("the receipt");
    assert_eq!(receipt.bytes, 0);
    assert!(objects.exists(&first.object_key).await.expect("the object"));

    archive(Arc::clone(&state), "other", &version).await;
    let erased = erase(Arc::clone(&state), "other", &version, Some(&version)).await;
    assert_eq!(erased.status(), StatusCode::OK);
    let receipt: EraseDatasetResponse =
        serde_json::from_slice(&body_bytes(erased).await).expect("the receipt");
    assert_eq!(receipt.bytes, first.summary.bytes);
    assert!(!objects.exists(&first.object_key).await.expect("the object"));
}

#[tokio::test]
async fn an_unknown_reference_is_refused() {
    let (state, _objects) = state_with_catalog_and_objects().await;
    let version = quantrule_api::Digest::sha256(b"unknown").to_string();
    let (header, value) = header("default");
    let body = serde_json::json!({"reference_id": "run:missing", "kind": "run"});
    let response = call(
        state,
        Request::post(format!("/internal/v1/datasets/{version}/references"))
            .header(header, value)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("the reference request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("dataset.not_found"), "{text}");
}
