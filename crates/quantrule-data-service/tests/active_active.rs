//! Two independent data-service instances over one catalog and object store.
//!
//! The fast suite uses the in-memory stores, but each state has its own service
//! gates, so the calls do not share the single-process upload/erase mutex. The
//! assertions therefore cover the database-level idempotency rules and the
//! catalog object guard, not just one router's local serialisation.

mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::{CSV, body_bytes, read_dataset, state, upload};
use quantrule_api::{
    ArtifactRecord, DatasetReferenceCount, ErrorCode, ErrorEnvelope, PROFILE_HEADER, PackSummary,
};
use quantrule_data_service::artifacts::ArtifactCatalog;
use quantrule_data_service::{
    DataServices, DataState, DatasetCatalog, MemoryArtifacts, MemoryCatalog, MemoryPacks,
    PackRegistry,
};
use quantrule_runtime::Config;
use quantrule_runtime::adapters::memory::MemoryObjects;
use quantrule_runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use serde_json::{Value, json};
use std::sync::Arc;
use tower::ServiceExt;

const ALT_CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,200.0,201.0,199.0,200.5,10\n\
2024-01-01 00:01:00+00:00,200.5,201.5,200.0,201.0,12\n";

fn one_state(services: DataServices) -> Arc<DataState> {
    state(&Config::default(), services)
}

fn two_states_reading(
    catalog: Arc<MemoryCatalog>,
    objects: MemoryObjects,
) -> (Arc<DataState>, Arc<DataState>) {
    let first = one_state(DataServices {
        catalog: Some(catalog.clone()),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });
    let second = one_state(DataServices {
        catalog: Some(catalog),
        objects: Some(Arc::new(objects)),
        ..DataServices::default()
    });
    (first, second)
}

async fn response(state: Arc<DataState>, request: Request<Body>) -> axum::response::Response {
    quantrule_data_service::router(state)
        .oneshot(request)
        .await
        .expect("the response")
}

fn statuses(first: &axum::response::Response, second: &axum::response::Response) -> [u16; 2] {
    let mut statuses = [first.status().as_u16(), second.status().as_u16()];
    statuses.sort_unstable();
    statuses
}

fn read_error(bytes: &[u8]) -> ErrorEnvelope {
    serde_json::from_slice(bytes).expect("the error envelope")
}

fn strategy(version: &str, reward: f64) -> Value {
    json!({
        "schema": "quantrule.pack/v1",
        "kind": "strategy",
        "version": version,
        "name": "active-active-fade",
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
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                 "source": 4, "direction": 1}
            ],
            "exit": []
        }
    })
}

fn pack_request(document: &Value) -> Request<Body> {
    Request::post("/catalog/packs")
        .header(PROFILE_HEADER, "default")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(document).expect("pack body")))
        .expect("the pack request")
}

async fn archive(state: Arc<DataState>, version: &str) -> axum::response::Response {
    response(
        state,
        Request::post(format!("/catalog/datasets/{version}/archive"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the archive request"),
    )
    .await
}

async fn erase(state: Arc<DataState>, version: &str) -> axum::response::Response {
    response(
        state,
        Request::delete(format!("/catalog/datasets/{version}?confirm={version}"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the erase request"),
    )
    .await
}

async fn register_reference(
    state: Arc<DataState>,
    version: &str,
    reference_id: &str,
) -> axum::response::Response {
    response(
        state,
        Request::post(format!("/internal/v1/datasets/{version}/references"))
            .header(PROFILE_HEADER, "default")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"reference_id": reference_id, "kind": "run"}).to_string(),
            ))
            .expect("the reference request"),
    )
    .await
}

async fn release_reference(
    state: Arc<DataState>,
    version: &str,
    reference_id: &str,
) -> axum::response::Response {
    response(
        state,
        Request::post(format!(
            "/internal/v1/datasets/{version}/references/release"
        ))
        .header(PROFILE_HEADER, "default")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"reference_id": reference_id}).to_string(),
        ))
        .expect("the release request"),
    )
    .await
}

#[tokio::test]
async fn two_identical_uploads_commit_one_row_and_one_object() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let (first, second) = two_states_reading(Arc::clone(&catalog), objects.clone());

    let (left, right) = tokio::join!(
        response(Arc::clone(&first), upload("one.csv")),
        response(Arc::clone(&second), upload("two.csv")),
    );
    assert_eq!(statuses(&left, &right), [200, 201]);
    let left = read_dataset(&body_bytes(left).await);
    let right = read_dataset(&body_bytes(right).await);
    assert_eq!(left, right, "the retry returned a different row");
    assert_eq!(
        catalog
            .count(quantrule_api::ProfileId::parse("default").expect("profile"))
            .await
            .expect("the count"),
        1
    );
    assert_eq!(objects.keys(), vec![left.object_key.clone()]);
}

#[tokio::test]
async fn an_orphan_object_is_reused_by_the_retry() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let ingested =
        quantrule_data_service::ingest::ingest("orphan.csv", CSV.as_bytes()).expect("the ingest");
    let key = format!("datasets/{}", ingested.data_version.as_hex());
    objects
        .store(&key, ingested.content.clone())
        .await
        .expect("the orphan object");
    let state = one_state(DataServices {
        catalog: Some(catalog.clone()),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });

    let response = response(state, upload("orphan.csv")).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let record = read_dataset(&body_bytes(response).await);
    assert_eq!(record.object_key, key);
    assert_eq!(objects.keys(), vec![key], "the retry rewrote the object");
    assert_eq!(
        catalog
            .count(quantrule_api::ProfileId::parse("default").expect("profile"))
            .await
            .expect("the count"),
        1
    );
}

#[tokio::test]
async fn two_different_uploads_commit_two_rows_and_two_objects() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let (first, second) = two_states_reading(Arc::clone(&catalog), objects.clone());

    let (left, right) = tokio::join!(
        response(
            Arc::clone(&first),
            Request::post("/catalog/datasets?name=one.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(CSV))
                .expect("the upload request"),
        ),
        response(
            Arc::clone(&second),
            Request::post("/catalog/datasets?name=two.csv")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(ALT_CSV))
                .expect("the upload request"),
        ),
    );
    assert_eq!(left.status(), StatusCode::CREATED);
    assert_eq!(right.status(), StatusCode::CREATED);
    let left = read_dataset(&body_bytes(left).await);
    let right = read_dataset(&body_bytes(right).await);
    assert_ne!(left.summary.data_version, right.summary.data_version);
    assert_eq!(objects.keys().len(), 2);
    assert_eq!(
        catalog
            .count(quantrule_api::ProfileId::parse("default").expect("profile"))
            .await
            .expect("the count"),
        2
    );
}

#[derive(Clone)]
struct PutFails {
    inner: MemoryObjects,
}

impl ObjectStore for PutFails {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, _key: &str, _bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        Box::pin(async { Err(StoreError("the object store is full".to_owned())) })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        self.inner.get(key)
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        self.inner.exists(key)
    }

    fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
        self.inner.delete(key)
    }
}

#[tokio::test]
async fn an_object_write_failure_publishes_no_row() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let state = one_state(DataServices {
        catalog: Some(catalog.clone()),
        objects: Some(Arc::new(PutFails {
            inner: objects.clone(),
        })),
        ..DataServices::default()
    });

    let response = response(state, upload("candles.csv")).await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let error = read_error(&body_bytes(response).await);
    assert_eq!(error.error.code, ErrorCode::Internal);
    assert_eq!(
        catalog
            .count(quantrule_api::ProfileId::parse("default").expect("profile"))
            .await
            .expect("the count"),
        0,
        "the catalog published a row without its bytes"
    );
    assert!(objects.keys().is_empty());
}

#[tokio::test]
async fn a_duplicate_pack_registration_is_idempotent_across_two_replicas() {
    let packs = Arc::new(MemoryPacks::default());
    let objects = MemoryObjects::default();
    let first = one_state(DataServices {
        packs: Some(packs.clone()),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });
    let second = one_state(DataServices {
        packs: Some(packs.clone()),
        objects: Some(Arc::new(objects)),
        ..DataServices::default()
    });
    let document = strategy("1.0.0", 1.5);

    let (left, right) = tokio::join!(
        response(Arc::clone(&first), pack_request(&document)),
        response(Arc::clone(&second), pack_request(&document)),
    );
    assert_eq!(statuses(&left, &right), [200, 201]);
    let left: quantrule_api::PackRecord =
        serde_json::from_slice(&body_bytes(left).await).expect("the first pack");
    let right: quantrule_api::PackRecord =
        serde_json::from_slice(&body_bytes(right).await).expect("the second pack");
    assert_eq!(left, right);
    assert_eq!(
        packs
            .list(
                quantrule_api::ProfileId::parse("default").expect("profile"),
                quantrule_data_service::packs::PackFilter::default(),
            )
            .await
            .expect("the pack list")
            .len(),
        1
    );

    let changed = strategy("2.0.0", 1.25);
    let other = strategy("2.0.0", 1.75);
    let (left, right) = tokio::join!(
        response(first, pack_request(&changed)),
        response(second, pack_request(&other)),
    );
    assert_eq!(statuses(&left, &right), [201, 409]);
    let conflict = if left.status() == StatusCode::CONFLICT {
        body_bytes(left).await
    } else {
        body_bytes(right).await
    };
    assert_eq!(read_error(&conflict).error.code, ErrorCode::PackConflict);

    let listed = response(
        one_state(DataServices {
            packs: Some(packs),
            objects: None,
            ..DataServices::default()
        }),
        Request::get("/catalog/packs")
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the list request"),
    )
    .await;
    let listed: Vec<PackSummary> =
        serde_json::from_slice(&body_bytes(listed).await).expect("the pack list");
    assert_eq!(
        listed.iter().filter(|pack| pack.version == "2.0.0").count(),
        1,
        "a duplicate label must never name two packs"
    );
}

#[tokio::test]
async fn a_duplicate_artifact_registration_is_idempotent_across_two_replicas() {
    let artifacts = Arc::new(MemoryArtifacts::default());
    let objects = MemoryObjects::default();
    let first = one_state(DataServices {
        artifacts: Some(artifacts.clone()),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });
    let second = one_state(DataServices {
        artifacts: Some(artifacts.clone()),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });
    let body = b"{\"schema\":\"quantrule.signal-artifact/v1\",\"rows\":[]}\n";

    let (left, right) = tokio::join!(
        response(
            Arc::clone(&first),
            Request::put("/internal/v1/artifacts/signal")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(body.to_vec()))
                .expect("the artifact request"),
        ),
        response(
            Arc::clone(&second),
            Request::put("/internal/v1/artifacts/signal")
                .header(PROFILE_HEADER, "default")
                .body(Body::from(body.to_vec()))
                .expect("the artifact request"),
        ),
    );
    assert_eq!(statuses(&left, &right), [200, 201]);
    let left: ArtifactRecord =
        serde_json::from_slice(&body_bytes(left).await).expect("the first artifact");
    let right: ArtifactRecord =
        serde_json::from_slice(&body_bytes(right).await).expect("the second artifact");
    assert_eq!(left, right);
    assert_eq!(
        artifacts
            .list(
                quantrule_api::ProfileId::parse("default").expect("profile"),
                quantrule_api::ArtifactKind::Signal,
            )
            .await
            .expect("the artifact list")
            .len(),
        1
    );
    assert_eq!(objects.keys(), vec![left.object_key]);
}

#[tokio::test]
async fn duplicate_references_commit_once_and_erase_still_counts_them() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let (first, second) = two_states_reading(Arc::clone(&catalog), objects);
    let record =
        read_dataset(&body_bytes(response(Arc::clone(&first), upload("candles.csv")).await).await);
    let version = record.summary.data_version.to_string();

    let (left, right) = tokio::join!(
        register_reference(Arc::clone(&first), &version, "run:one"),
        register_reference(Arc::clone(&second), &version, "run:one"),
    );
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);
    assert_eq!(
        serde_json::from_slice::<DatasetReferenceCount>(&body_bytes(right).await)
            .expect("the count")
            .references,
        1
    );

    let (left, right) = tokio::join!(
        register_reference(Arc::clone(&first), &version, "run:two"),
        register_reference(Arc::clone(&second), &version, "run:three"),
    );
    assert_eq!(left.status(), StatusCode::OK);
    assert_eq!(right.status(), StatusCode::OK);

    let response = response(
        Arc::clone(&first),
        Request::get(format!("/internal/v1/datasets/{version}/references"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the reference list request"),
    )
    .await;
    let references: Vec<quantrule_api::DatasetReference> =
        serde_json::from_slice(&body_bytes(response).await).expect("the reference list");
    assert_eq!(references.len(), 3, "a reference was lost or counted twice");

    assert_eq!(
        archive(Arc::clone(&first), &version).await.status(),
        StatusCode::OK
    );
    let refused = erase(Arc::clone(&second), &version).await;
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    let error = read_error(&body_bytes(refused).await);
    assert_eq!(error.error.code, ErrorCode::HistoryProtected);
    assert_eq!(error.error.message, "the dataset still has 3 reference(s)");

    for reference in ["run:one", "run:two", "run:three"] {
        let response = release_reference(Arc::clone(&second), &version, reference).await;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        erase(Arc::clone(&first), &version).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn an_erase_and_a_concurrent_upload_never_leave_a_row_without_bytes() {
    let catalog = Arc::new(MemoryCatalog::default());
    let objects = MemoryObjects::default();
    let (first, second) = two_states_reading(Arc::clone(&catalog), objects.clone());
    let record =
        read_dataset(&body_bytes(response(Arc::clone(&first), upload("candles.csv")).await).await);
    let version = record.summary.data_version.to_string();
    assert_eq!(
        archive(Arc::clone(&first), &version).await.status(),
        StatusCode::OK
    );

    let (erased, uploaded) = tokio::join!(
        erase(Arc::clone(&first), &version),
        response(
            Arc::clone(&second),
            Request::post("/catalog/datasets?name=other.csv")
                .header(PROFILE_HEADER, "other")
                .body(Body::from(CSV))
                .expect("the upload request"),
        ),
    );
    assert_eq!(erased.status(), StatusCode::OK);
    assert!(
        matches!(uploaded.status(), StatusCode::CREATED | StatusCode::OK),
        "the concurrent upload failed: {}",
        uploaded.status()
    );
    assert!(
        objects
            .exists(&record.object_key)
            .await
            .expect("the object"),
        "a published row points at deleted bytes"
    );
    let uploaded = read_dataset(&body_bytes(uploaded).await);
    assert_eq!(uploaded.object_key, record.object_key);
    assert_eq!(uploaded.summary.data_version, record.summary.data_version);
    assert!(
        catalog
            .get(
                quantrule_api::ProfileId::parse("other").expect("profile"),
                record.summary.data_version.to_string(),
            )
            .await
            .expect("the other profile read")
            .is_some(),
        "the upload answered success without publishing its row"
    );
}

#[tokio::test]
async fn a_missing_pack_object_still_fails_closed() {
    let objects = MemoryObjects::default();
    let state = one_state(DataServices {
        packs: Some(Arc::new(MemoryPacks::default())),
        objects: Some(Arc::new(objects.clone())),
        ..DataServices::default()
    });
    let document = strategy("3.0.0", 1.5);
    let registered = response(Arc::clone(&state), pack_request(&document)).await;
    assert_eq!(registered.status(), StatusCode::CREATED);
    let record: quantrule_api::PackRecord =
        serde_json::from_slice(&body_bytes(registered).await).expect("the pack");
    objects.remove(&record.object_key);
    let id = record.summary.pack_id.to_string();
    let response = response(
        state,
        Request::get(format!("/catalog/packs/{id}"))
            .header(PROFILE_HEADER, "default")
            .body(Body::empty())
            .expect("the pack request"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        read_error(&body_bytes(response).await)
            .error
            .message
            .contains("missing or unreadable")
    );
}
