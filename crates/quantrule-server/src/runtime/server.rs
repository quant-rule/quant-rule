//! The HTTP surface of the runtime.
//!
//! The gateway serves `/health` and `/version`, seals and reads a hypothesis,
//! and ingests a dataset into the catalog. The remaining routes arrive with
//! their services.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use quantrule_api::{
    DatasetRecord, DatasetSummary, Digest, ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER,
    ProfileId, VersionInfo,
};
use quantrule_research::{Hypothesis, SealedHypothesis};
use serde::Deserialize;

use crate::data::{CatalogError, DatasetCatalog, ingest};
use crate::research::HypothesisStore;
use crate::runtime::adapters::ObjectStore;
use crate::runtime::clock;
use crate::runtime::config::Config;
use crate::runtime::error::RuntimeError;
use crate::runtime::health::HealthRegistry;

/// The services a gateway can reach.
#[derive(Default)]
pub struct Services {
    /// Where sealed hypotheses live.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where datasets are catalogued.
    pub catalog: Option<Arc<dyn DatasetCatalog>>,
    /// Where the canonical bytes live.
    pub objects: Option<Arc<dyn ObjectStore>>,
}

/// What a handler needs.
pub struct AppState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Where sealed hypotheses live; absent when no database is configured.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where datasets are catalogued; absent when no database is configured.
    pub catalog: Option<Arc<dyn DatasetCatalog>>,
    /// Where canonical dataset bytes live; absent when no object store is
    /// configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// The largest upload the gateway will buffer.
    pub max_upload: usize,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("hypotheses", &self.hypotheses.is_some())
            .field("catalog", &self.catalog.is_some())
            .field("objects", &self.objects.is_some())
            .field("max_upload", &self.max_upload)
            .finish()
    }
}

impl AppState {
    /// Build the state without any backing service.
    #[must_use]
    pub fn new(config: &Config, health: HealthRegistry) -> Self {
        Self::with_services(config, health, Services::default())
    }

    /// Build the state with whatever backing services are configured.
    #[must_use]
    pub fn with_services(config: &Config, health: HealthRegistry, services: Services) -> Self {
        Self {
            version: VersionInfo::current(env!("CARGO_PKG_VERSION"), config.image_digest.clone()),
            health,
            hypotheses: services.hypotheses,
            catalog: services.catalog,
            objects: services.objects,
            max_upload: usize::try_from(config.upload_limit_bytes).unwrap_or(usize::MAX),
        }
    }
}

/// A failed request: a status and the shared envelope.
type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Build the router.
pub fn router(state: Arc<AppState>) -> Router {
    let limit = state.max_upload;
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/hypotheses", post(create_hypothesis))
        .route("/hypotheses/{fingerprint}", get(get_hypothesis))
        .route("/catalog/datasets", get(list_datasets).post(create_dataset))
        .route("/catalog/datasets/{version}", get(get_dataset))
        .layer(DefaultBodyLimit::max(limit))
        .with_state(state)
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn version(State(state): State<Arc<AppState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the profile header. Every record is scoped, so a request without one is
/// refused rather than defaulted.
fn request_profile(headers: &HeaderMap) -> Result<ProfileId, ApiFailure> {
    let Some(value) = headers.get(PROFILE_HEADER) else {
        return Err(failure(
            ErrorCode::ProfileNotFound,
            format!("{PROFILE_HEADER} is required"),
        ));
    };
    let text = value
        .to_str()
        .map_err(|_| failure(ErrorCode::ProfileInvalid, "the profile header is not text"))?;
    ProfileId::parse(text).map_err(|error| failure(ErrorCode::ProfileInvalid, error.to_string()))
}

async fn create_hypothesis(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<Hypothesis>, JsonRejection>,
) -> Result<(StatusCode, Json<SealedHypothesis>), ApiFailure> {
    let Json(draft) = payload
        .map_err(|_| failure(ErrorCode::HypothesisInvalid, "the body is not a hypothesis"))?;
    let store = state
        .hypotheses
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    let sealed = draft
        .seal(clock::now_utc())
        .map_err(|error| failure(ErrorCode::HypothesisInvalid, error.to_string()))?;
    store
        .insert(sealed.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(sealed)))
}

async fn get_hypothesis(
    State(state): State<Arc<AppState>>,
    Path(fingerprint): Path<String>,
) -> Result<Json<SealedHypothesis>, ApiFailure> {
    let store = state
        .hypotheses
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    match store.load(fingerprint.clone()).await {
        Ok(Some(sealed)) => Ok(Json(sealed)),
        Ok(None) => Err(failure(
            ErrorCode::HypothesisNotFound,
            format!("hypothesis {fingerprint} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

/// The name an upload carries.
#[derive(Debug, Deserialize)]
struct DatasetUpload {
    name: Option<String>,
}

async fn create_dataset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
    payload: Result<Bytes, BytesRejection>,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the query is not a dataset upload",
        )
    })?;
    let body = payload.map_err(|rejection| {
        let status = rejection.into_response().status();
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            failure(ErrorCode::UploadTooLarge, "the upload exceeds the limit")
        } else {
            failure(ErrorCode::DatasetInvalid, "the upload could not be read")
        }
    })?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;

    let name = query.name.as_deref().unwrap_or("");
    let ingested = ingest(name, &body)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    let hex = ingested.data_version.as_hex();
    let key = format!("datasets/{hex}");
    let present = objects
        .exists(&key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if !present {
        objects
            .put(&key, ingested.content)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }
    let record = DatasetRecord {
        summary: DatasetSummary {
            data_version: ingested.data_version.clone(),
            name: ingested.name,
            candles: ingested.candles,
            first_bar: ingested.first_bar,
            last_bar: ingested.last_bar,
            bytes: ingested.bytes,
        },
        object_key: key,
        sha256: ingested.data_version.clone(),
        created_at: clock::now_utc(),
    };

    match catalog.insert(profile.clone(), record.clone()).await {
        Ok(()) => Ok((StatusCode::CREATED, Json(record))),
        Err(CatalogError::AlreadyExists) => {
            let version = record.summary.data_version.to_string();
            match catalog.get(profile, version).await {
                Ok(Some(existing)) => Ok((StatusCode::OK, Json(existing))),
                Ok(None) => Err(failure(
                    ErrorCode::Internal,
                    "the catalog refused a duplicate but holds no record",
                )),
                Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
            }
        }
        Err(CatalogError::Store(error)) => Err(failure(ErrorCode::Internal, error)),
    }
}

async fn list_datasets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<DatasetSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let records = catalog
        .list(profile)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(
        records.into_iter().map(|record| record.summary).collect(),
    ))
}

async fn get_dataset(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<Json<DatasetRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let version = Digest::parse(&version)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?
        .to_string();
    match catalog.get(profile, version.clone()).await {
        Ok(Some(record)) => Ok(Json(record)),
        Ok(None) => Err(failure(
            ErrorCode::DatasetNotFound,
            format!("dataset {version} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

/// Bind and serve until a termination signal arrives.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the address cannot be bound or the
/// server fails.
pub async fn serve(config: &Config, state: Arc<AppState>) -> Result<(), RuntimeError> {
    let listener = tokio::net::TcpListener::bind(&config.bind)
        .await
        .map_err(|error| RuntimeError::Serve(format!("bind {}: {error}", config.bind)))?;
    axum::serve(listener, router(state))
        .with_graceful_shutdown(crate::runtime::shutdown::signal())
        .await
        .map_err(|error| RuntimeError::Serve(format!("serve: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::MemoryCatalog;
    use crate::research::MemoryStore;
    use crate::runtime::adapters::{Backend, BoxFuture, StoreError};
    use axum::body::Body;
    use axum::http::Request;
    use quantrule_research::{CostModel, Partition, Split};
    use std::collections::BTreeMap;
    use std::sync::Mutex;
    use tower::ServiceExt;

    /// An object store that keeps bytes in memory, so the route is tested
    /// without S3.
    #[derive(Clone, Default)]
    struct MemoryObjects {
        inner: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
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
            },
        ))
    }

    fn state_with_catalog_and_objects() -> (Arc<AppState>, MemoryObjects) {
        let objects = MemoryObjects::default();
        let state = Arc::new(AppState::with_services(
            &Config::default(),
            HealthRegistry::new(),
            Services {
                hypotheses: None,
                catalog: Some(Arc::new(MemoryCatalog::default())),
                objects: Some(Arc::new(objects.clone())),
            },
        ));
        (state, objects)
    }

    fn state_with_catalog() -> Arc<AppState> {
        state_with_catalog_and_objects().0
    }

    fn state_with_small_upload() -> Arc<AppState> {
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
    async fn a_dataset_is_uploaded_listed_and_read() {
        let (state, objects) = state_with_catalog_and_objects();
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
        let state = state_with_catalog();
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
        let state = state_with_catalog();
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
        let response = router(state_with_catalog())
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
        let response = router(state_with_catalog())
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
        let response = router(state_with_catalog())
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
        let response = router(state_with_small_upload())
            .oneshot(upload("candles.csv"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let text = String::from_utf8(body_bytes(response).await).expect("utf8");
        assert!(text.contains("upload.too_large"), "{text}");
    }
}
