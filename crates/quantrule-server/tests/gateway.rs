//! The gateway routes, exercised over HTTP with in-memory services.
//!
//! The tests assert on status, envelope code, and body, not on a handler's
//! internals, so a route change is visible the way a client would see it.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use quantrule_api::{
    DatasetRecord, DatasetSummary, Digest, PROFILE_HEADER, ProfileId, ProfileSummary,
    TimeframeInspection,
};
use quantrule_research::{CostModel, Partition, SealedHypothesis, Split};
use quantrule_server::data::MemoryCatalog;
use quantrule_server::profile::{MemoryProfiles, ProfileRecord, ProfileStore};
use quantrule_server::research::MemoryStore;
use quantrule_server::runtime::adapters::{Backend, BoxFuture, ObjectStore, StoreError};
use quantrule_server::runtime::server::router;
use quantrule_server::runtime::{AppState, Config, HealthRegistry, Services};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

/// An object store that keeps bytes in memory, so the route is tested
/// without S3.
#[derive(Clone, Default)]
struct MemoryObjects {
    inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
}

impl MemoryObjects {
    fn remove(&self, key: &str) {
        self.inner.lock().expect("the object lock").remove(key);
    }
}

impl ObjectStore for MemoryObjects {
    fn backend(&self) -> Backend {
        Backend::Filesystem
    }

    fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .insert(key, bytes);
            Ok(())
        })
    }

    fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .get(&key)
                .cloned()
                .ok_or_else(|| StoreError(format!("{key} is absent")))
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
        let inner = Arc::clone(&self.inner);
        let key = key.to_owned();
        Box::pin(async move {
            Ok(inner
                .lock()
                .map_err(|_| StoreError("the object lock is poisoned".to_owned()))?
                .contains_key(&key))
        })
    }
}

const CSV: &str = "date,open,high,low,close,volume\n\
2024-01-01 00:00:00+00:00,100.0,101.0,99.0,100.5,10\n\
2024-01-01 00:01:00+00:00,100.5,101.5,100.0,101.0,12\n";

fn state() -> Arc<AppState> {
    Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
}

fn state_with_store() -> Arc<AppState> {
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            hypotheses: Some(Arc::new(MemoryStore::default())),
            catalog: None,
            objects: None,
            profiles: None,
        },
    ))
}

async fn memory_profiles(ids: &[&str]) -> Arc<MemoryProfiles> {
    let store = Arc::new(MemoryProfiles::default());
    for id in ids {
        store
            .create(ProfileRecord {
                profile_id: ProfileId::parse(id).expect("profile"),
                name: (*id).to_owned(),
                created_at: "2026-11-13T00:00:00Z".to_owned(),
            })
            .await
            .expect("create profile");
    }
    store
}

async fn state_with_catalog_and_objects() -> (Arc<AppState>, MemoryObjects) {
    let objects = MemoryObjects::default();
    let state = Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            hypotheses: None,
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(objects.clone())),
            profiles: Some(memory_profiles(&["default", "alpha", "beta", "gamma"]).await),
        },
    ));
    (state, objects)
}

async fn state_with_catalog() -> Arc<AppState> {
    state_with_catalog_and_objects().await.0
}

async fn state_with_small_upload() -> Arc<AppState> {
    let config = Config {
        upload_limit_bytes: 16,
        ..Config::default()
    };
    Arc::new(AppState::with_services(
        &config,
        HealthRegistry::new(),
        Services {
            hypotheses: None,
            catalog: Some(Arc::new(MemoryCatalog::default())),
            objects: Some(Arc::new(MemoryObjects::default())),
            profiles: Some(memory_profiles(&["default"]).await),
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

fn upload(name: &str) -> Request<Body> {
    Request::post(format!("/catalog/datasets?name={name}"))
        .header(PROFILE_HEADER, "default")
        .body(Body::from(CSV))
        .expect("request")
}

fn read_dataset(bytes: &[u8]) -> DatasetRecord {
    serde_json::from_slice(bytes).expect("dataset record")
}

async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

#[tokio::test]
async fn health_is_served_as_json() {
    let response = router(state())
        .oneshot(
            Request::get("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn version_reports_the_contract() {
    let response = router(state())
        .oneshot(
            Request::get("/version")
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
async fn an_unknown_path_is_not_found() {
    let response = router(state())
        .oneshot(Request::get("/nope").body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_hypothesis_is_sealed_and_read_back() {
    let state = state_with_store();
    let payload = serde_json::to_vec(&body()).expect("encode");
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/hypotheses")
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
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn an_unknown_hypothesis_is_not_found_with_a_code() {
    let response = router(state_with_store())
        .oneshot(
            Request::get("/hypotheses/missing")
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
    let response = router(state_with_store())
        .oneshot(
            Request::post("/hypotheses")
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
    let payload = serde_json::to_vec(&body()).expect("encode");
    let response = router(state())
        .oneshot(
            Request::post("/hypotheses")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn a_profile_is_created_and_listed() {
    let state = state_with_catalog().await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::CREATED);
    let created: ProfileSummary =
        serde_json::from_slice(&body_bytes(response).await).expect("profile");
    assert_eq!(created.profile_id.as_str(), "delta");
    assert_eq!(created.name, "Delta");
    assert_eq!(created.datasets, 0);

    let response = router(state)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "delta")
    );
    assert!(
        listed
            .iter()
            .any(|profile| profile.profile_id.as_str() == "default")
    );
}

#[tokio::test]
async fn a_duplicate_profile_is_idempotent() {
    let state = state_with_catalog().await;
    let payload = r#"{"profile_id":"delta","name":"Delta"}"#;
    let first = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(first.status(), StatusCode::CREATED);
    let first: ProfileSummary = serde_json::from_slice(&body_bytes(first).await).expect("profile");

    let second = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(second.status(), StatusCode::OK);
    let second: ProfileSummary =
        serde_json::from_slice(&body_bytes(second).await).expect("profile");
    assert_eq!(first, second);
}

#[tokio::test]
async fn an_invalid_profile_is_rejected() {
    let state = state_with_catalog().await;
    let bad_id = r#"{"profile_id":"Bad!","name":"Bad"}"#;
    let response = router(Arc::clone(&state))
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(bad_id))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");

    let blank_name = r#"{"profile_id":"ok","name":"   "}"#;
    let response = router(state)
        .oneshot(
            Request::post("/profiles")
                .header("content-type", "application/json")
                .body(Body::from(blank_name))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.invalid"), "{text}");
}

#[tokio::test]
async fn an_unknown_profile_is_not_found() {
    let response = router(state_with_catalog().await)
        .oneshot(
            Request::get("/catalog/datasets")
                .header(PROFILE_HEADER, "ghost")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let text = String::from_utf8(body_bytes(response).await).expect("utf8");
    assert!(text.contains("profile.not_found"), "{text}");
}

#[tokio::test]
async fn a_dataset_contributes_to_its_profile_count() {
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
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = router(state)
        .oneshot(
            Request::get("/profiles")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    let listed: Vec<ProfileSummary> =
        serde_json::from_slice(&body_bytes(response).await).expect("profiles");
    let alpha = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "alpha")
        .expect("alpha");
    assert_eq!(alpha.datasets, 1);
    let default = listed
        .iter()
        .find(|profile| profile.profile_id.as_str() == "default")
        .expect("default");
    assert_eq!(default.datasets, 0);
}

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
    let response = router(state())
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
