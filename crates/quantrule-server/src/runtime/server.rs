//! The HTTP surface of the runtime.
//!
//! The gateway serves `/health` and `/version`, seals and reads a hypothesis,
//! and ingests a dataset into the catalog. The remaining routes arrive with
//! their services.

use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use quantrule_api::{
    CreateProfileRequest, DatasetRecord, DatasetSummary, Digest, ErrorCode, ErrorEnvelope,
    HealthReport, PROFILE_HEADER, ProfileId, ProfileSummary, TimeframeBars, TimeframeInspection,
    VersionInfo,
};
use quantrule_research::{Hypothesis, SealedHypothesis};
use serde::Deserialize;

use crate::data::{CatalogError, DatasetCatalog, Inspection, ingest, inspect};
use crate::profile::{ProfileError, ProfileRecord, ProfileStore};
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
    /// Which profiles exist.
    pub profiles: Option<Arc<dyn ProfileStore>>,
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
    /// Which profiles exist; absent when no database is configured.
    pub profiles: Option<Arc<dyn ProfileStore>>,
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
            .field("profiles", &self.profiles.is_some())
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
            profiles: services.profiles,
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
        .route("/profiles", get(list_profiles).post(create_profile))
        .route("/inspect/candles", get(inspect_candles))
        .route("/inspect/timeframes", get(inspect_timeframes))
        .route("/datasets/{version}/candles", get(get_candles))
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

/// Read the profile header and confirm the profile exists. Every record is
/// scoped, so a request without one is refused rather than defaulted.
async fn request_profile(state: &AppState, headers: &HeaderMap) -> Result<ProfileId, ApiFailure> {
    let Some(value) = headers.get(PROFILE_HEADER) else {
        return Err(failure(
            ErrorCode::ProfileNotFound,
            format!("{PROFILE_HEADER} is required"),
        ));
    };
    let text = value
        .to_str()
        .map_err(|_| failure(ErrorCode::ProfileInvalid, "the profile header is not text"))?;
    let profile = ProfileId::parse(text)
        .map_err(|error| failure(ErrorCode::ProfileInvalid, error.to_string()))?;
    let store = state
        .profiles
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no profile store is configured"))?;
    match store.get(profile.clone()).await {
        Ok(Some(_)) => Ok(profile),
        Ok(None) => Err(failure(
            ErrorCode::ProfileNotFound,
            format!("profile {profile} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
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

/// Build the wire summary for one profile.
async fn profile_summary(
    state: &AppState,
    record: ProfileRecord,
) -> Result<ProfileSummary, ApiFailure> {
    let datasets = match state.catalog.as_ref() {
        Some(catalog) => catalog
            .count(record.profile_id.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
        None => 0,
    };
    // Runs and simulations gain their routes with their services; until then
    // their counts are zero rather than a number no service can support.
    Ok(ProfileSummary {
        profile_id: record.profile_id,
        name: record.name,
        datasets,
        runs: 0,
        simulations: 0,
    })
}

async fn create_profile(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<CreateProfileRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProfileSummary>), ApiFailure> {
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::ProfileInvalid, "the body is not a profile"))?;
    let profile_id = ProfileId::parse(request.profile_id.as_str())
        .map_err(|error| failure(ErrorCode::ProfileInvalid, error.to_string()))?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(failure(ErrorCode::ProfileInvalid, "a profile needs a name"));
    }
    let store = state
        .profiles
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no profile store is configured"))?;
    let record = ProfileRecord {
        profile_id: profile_id.clone(),
        name: name.to_owned(),
        created_at: clock::now_utc(),
    };
    match store.create(record.clone()).await {
        Ok(()) => {
            let summary = profile_summary(&state, record).await?;
            Ok((StatusCode::CREATED, Json(summary)))
        }
        Err(ProfileError::AlreadyExists) => match store.get(profile_id).await {
            Ok(Some(existing)) => {
                let summary = profile_summary(&state, existing).await?;
                Ok((StatusCode::OK, Json(summary)))
            }
            Ok(None) => Err(failure(
                ErrorCode::Internal,
                "the profile store refused a duplicate but holds no record",
            )),
            Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
        },
        Err(ProfileError::Store(error)) => Err(failure(ErrorCode::Internal, error)),
    }
}

async fn list_profiles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProfileSummary>>, ApiFailure> {
    let store = state
        .profiles
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no profile store is configured"))?;
    let records = store
        .list()
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let mut summaries = Vec::with_capacity(records.len());
    for record in records {
        summaries.push(profile_summary(&state, record).await?);
    }
    Ok(Json(summaries))
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
    let profile = request_profile(&state, &headers).await?;
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

/// The dataset an inspection names.
#[derive(Debug, Deserialize)]
struct InspectQuery {
    data_version: String,
}

/// Resolve a dataset and read its stored bytes, verifying the digest.
async fn load_dataset_object(
    state: &AppState,
    headers: &HeaderMap,
    requested: &str,
) -> Result<(DatasetRecord, Vec<u8>), ApiFailure> {
    let profile = request_profile(state, headers).await?;
    let version = Digest::parse(requested)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?
        .to_string();
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let record = match catalog.get(profile, version.clone()).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return Err(failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {version} is unknown"),
            ));
        }
        Err(error) => return Err(failure(ErrorCode::Internal, error.to_string())),
    };
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&record.object_key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if Digest::sha256(&bytes) != record.summary.data_version {
        return Err(failure(
            ErrorCode::Internal,
            "the stored object does not match the catalog",
        ));
    }
    Ok((record, bytes))
}

/// Resolve a dataset and re-read its bytes into a ladder inspection.
async fn load_inspection(
    state: &AppState,
    headers: &HeaderMap,
    query: Result<Query<InspectQuery>, QueryRejection>,
) -> Result<(DatasetRecord, Inspection), ApiFailure> {
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::DatasetInvalid, "the query names no data_version"))?;
    let (record, bytes) = load_dataset_object(state, headers, &query.data_version).await?;
    let inspection =
        inspect(&bytes).map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    Ok((record, inspection))
}

/// Serve the canonical bytes a run consumes.
async fn get_candles(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<impl IntoResponse, ApiFailure> {
    let (record, bytes) = load_dataset_object(&state, &headers, &version).await?;
    let etag = format!("\"{}\"", record.summary.data_version);
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (header::ETAG, etag),
        ],
        bytes,
    ))
}

async fn inspect_candles(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<InspectQuery>, QueryRejection>,
) -> Result<Json<DatasetSummary>, ApiFailure> {
    let (record, _inspection) = load_inspection(&state, &headers, query).await?;
    Ok(Json(record.summary))
}

async fn inspect_timeframes(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<InspectQuery>, QueryRejection>,
) -> Result<Json<TimeframeInspection>, ApiFailure> {
    let (record, inspection) = load_inspection(&state, &headers, query).await?;
    let timeframes = inspection
        .timeframes
        .into_iter()
        .map(|coverage| TimeframeBars {
            timeframe: coverage.timeframe.code().to_owned(),
            bars: coverage.bars,
            partial_buckets: coverage.partial_buckets,
            first_bar: coverage
                .first_bar
                .and_then(|value| u64::try_from(value).ok()),
            last_bar: coverage
                .last_bar
                .and_then(|value| u64::try_from(value).ok()),
        })
        .collect();
    Ok(Json(TimeframeInspection {
        data_version: record.summary.data_version,
        timeframes,
    }))
}

async fn list_datasets(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<DatasetSummary>>, ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
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
    let profile = request_profile(&state, &headers).await?;
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
