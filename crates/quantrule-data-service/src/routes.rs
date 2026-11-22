//! The data service's HTTP surface.
//!
//! These are the catalog, dataset, config, and inspection routes that the
//! gateway used to serve. The service trusts the profile header the gateway
//! injects; it owns no profile store and never reads the `gateway` schema.
//!
//! The internal routes expose references only: a config bundle, a dataset
//! object, and a count. No internal response carries a candle or a reading
//! series.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Json, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Router, routing::post};
use quantrule_api::{
    ConfigSummary, DatasetRecord, DatasetReference, DatasetReferenceCount, DatasetReferenceRequest,
    DatasetSummary, Digest, EraseDatasetResponse, ErrorCode, ErrorEnvelope, HealthReport,
    PROFILE_HEADER, ProfileId, QualityReport, ReleaseDatasetReferenceRequest, TimeframeBars,
    TimeframeInspection, UploadConfigRequest, VersionInfo,
};
use quantrule_data::Timeframe;
use quantrule_service_api::{ConfigBundle, DatasetObject};
use serde::Deserialize;

use crate::catalog::CatalogError;
use crate::configs::{ConfigRecord, build, scan};
use crate::ingest::{Ingested, ingest_file};
use crate::inspect::{Inspection, inspect, ladder};
use crate::state::DataState;
use crate::tolerant::{TolerantIngested, ingest_tolerant};
use crate::upload::{Spool, SpoolError, spool};
use quantrule_runtime::adapters::put_if_absent;
use quantrule_runtime::clock;

/// A failed request: a status and the shared envelope.
pub(crate) type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

pub(crate) fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

fn failure_with_status(
    status: StatusCode,
    code: ErrorCode,
    message: impl Into<String>,
) -> ApiFailure {
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// The routes before state is attached.
fn routes() -> Router<Arc<DataState>> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/version", get(version))
        .route("/catalog/datasets", get(list_datasets).post(create_dataset))
        .route("/catalog/datasets/tolerant", post(create_dataset_tolerant))
        .route(
            "/catalog/datasets/{version}",
            get(get_dataset).delete(erase_dataset),
        )
        .route("/catalog/datasets/{version}/archive", post(archive_dataset))
        .route(
            "/catalog/configs",
            get(list_configs)
                .post(upload_config)
                .layer(DefaultBodyLimit::max(4 * 1024 * 1024)),
        )
        .route("/inspect/candles", get(inspect_candles))
        .route("/inspect/timeframes", get(inspect_timeframes))
        .merge(crate::packs::routes())
        .merge(crate::artifacts::routes())
        .route("/datasets/{version}/candles", get(get_candles))
        .route("/datasets/{version}/ladder/{timeframe}", get(get_ladder))
        .route("/internal/v1/configs/{config_id}", get(internal_config))
        .route("/internal/v1/datasets/{version}", get(internal_dataset))
        .route(
            "/internal/v1/datasets/{version}/references",
            get(list_dataset_references).post(register_dataset_reference),
        )
        .route(
            "/internal/v1/datasets/{version}/references/release",
            post(release_dataset_reference),
        )
        .route(
            "/internal/v1/profile/datasets/count",
            get(internal_dataset_count),
        )
}

/// Build the service router.
///
/// The gateway forwards the client path unchanged, so the same routes answer at
/// the root and under `/api/v1`. `/internal/v1` is service-to-service only.
pub fn router(state: Arc<DataState>) -> Router {
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .with_state(state)
}

async fn health(State(state): State<Arc<DataState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn ready(State(state): State<Arc<DataState>>) -> (StatusCode, Json<HealthReport>) {
    let digest = state.version.image_digest.clone();
    let (ready, report) = state.health.readiness(digest).await;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

async fn version(State(state): State<Arc<DataState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the injected profile header.
///
/// The gateway validates that the profile exists and injects the normalized
/// value. The data service trusts that value; a direct caller cannot reach it on
/// the published surface.
pub(crate) fn request_profile(headers: &HeaderMap) -> Result<ProfileId, ApiFailure> {
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

/// The name an upload carries.
#[derive(Debug, Deserialize)]
struct DatasetUpload {
    name: Option<String>,
}

async fn create_dataset(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
    request: Request,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
    let (profile, name) = upload_parts(&headers, query)?;
    let body = spool_upload(&state, request.into_body()).await?;
    let path = body.path().to_path_buf();
    let ingested = tokio::task::spawn_blocking(move || ingest_file(&name, &path))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    store_dataset(&state, profile, ingested, None).await
}

async fn create_dataset_tolerant(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
    request: Request,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
    let (profile, name) = upload_parts(&headers, query)?;
    let body = spool_upload(&state, request.into_body()).await?;
    let path = body.path().to_path_buf();
    let ingested = tokio::task::spawn_blocking(move || ingest_tolerant_file(&name, &path))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    store_dataset(&state, profile, ingested.dataset, Some(ingested.quality)).await
}

fn upload_parts(
    headers: &HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
) -> Result<(ProfileId, String), ApiFailure> {
    let profile = request_profile(headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the query is not a dataset upload",
        )
    })?;
    Ok((profile, query.name.unwrap_or_default()))
}

async fn spool_upload(state: &DataState, body: Body) -> Result<Spool, ApiFailure> {
    spool(body, state.max_upload, &std::env::temp_dir())
        .await
        .map_err(|error| match error {
            SpoolError::TooLarge => {
                failure(ErrorCode::UploadTooLarge, "the upload exceeds the limit")
            }
            other => failure(ErrorCode::DatasetInvalid, other.to_string()),
        })
}

fn ingest_tolerant_file(
    name: &str,
    path: &std::path::Path,
) -> Result<TolerantIngested, crate::ingest::IngestError> {
    let bytes =
        std::fs::read(path).map_err(|error| crate::ingest::IngestError::Io(error.to_string()))?;
    ingest_tolerant(name, &bytes)
}

async fn store_dataset(
    state: &DataState,
    profile: ProfileId,
    ingested: Ingested,
    quality: Option<QualityReport>,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
    let _gate = state.dataset_gate.lock().await;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let hex = ingested.data_version.as_hex();
    let key = format!("datasets/{hex}");
    let record = DatasetRecord {
        summary: DatasetSummary {
            data_version: ingested.data_version.clone(),
            name: ingested.name,
            candles: ingested.candles,
            first_bar: ingested.first_bar,
            last_bar: ingested.last_bar,
            bytes: ingested.bytes,
            archived_at: None,
        },
        object_key: key.clone(),
        sha256: ingested.data_version.clone(),
        quality,
        created_at: clock::now_utc()
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
    };

    // The object is written before the row. The database guard makes the
    // put-and-insert pair serial across replicas, so an erase cannot remove the
    // bytes between them. A failed put leaves no row; a failed insert leaves an
    // orphan object, which is the safe direction and is reused by a retry.
    let guard = catalog
        .object_guard(key.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let stored = async {
        put_if_absent(objects.as_ref(), &key, ingested.content)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
        catalog
            .insert_or_get(profile, record)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))
    }
    .await;
    if let Err(error) = guard.release().await {
        tracing::warn!(%error, "the dataset object guard did not release");
    }
    let inserted = stored?;
    let status = if inserted.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(inserted.record)))
}

/// Upload a run-config bundle, validate it, and catalogue it.
async fn upload_config(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    payload: Result<Json<UploadConfigRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ConfigSummary>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|rejection| {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            failure(ErrorCode::UploadTooLarge, "a config bundle exceeds 4 MiB")
        } else {
            failure(ErrorCode::ConfigInvalid, "the body is not a config bundle")
        }
    })?;
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let now = clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let staging = std::env::temp_dir();
    let record =
        tokio::task::spawn_blocking(move || build(&request.name, &request.files, &now, &staging))
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
            .map_err(|error| failure(ErrorCode::ConfigInvalid, error.0))?;

    if let Some(existing) = store
        .get(profile.clone(), record.config_id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    {
        return Ok((StatusCode::OK, Json(existing.summary())));
    }
    store
        .upsert(profile, record.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(record.summary())))
}

/// Scan the read-only mount into a profile's config rows.
pub(crate) async fn sync_configs(state: &DataState, profile: &ProfileId) -> Result<(), ApiFailure> {
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let root = state.config_root.clone();
    let now = clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let records = tokio::task::spawn_blocking(move || scan(&root, &now))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    for record in records {
        store
            .upsert(profile.clone(), record)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }
    Ok(())
}

async fn list_configs(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConfigSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    sync_configs(&state, &profile).await?;
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let records = store
        .list(profile)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(records.iter().map(ConfigRecord::summary).collect()))
}

/// One config as another service consumes it.
async fn internal_config(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(config_id): Path<String>,
) -> Result<Json<ConfigBundle>, ApiFailure> {
    let profile = request_profile(&headers)?;
    sync_configs(&state, &profile).await?;
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let record = store
        .get(profile, config_id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let record = record.ok_or_else(|| {
        failure_with_status(
            StatusCode::NOT_FOUND,
            ErrorCode::ConfigInvalid,
            format!("config {config_id} is unknown"),
        )
    })?;
    Ok(Json(ConfigBundle {
        config_id: record.config_id,
        files: record.files,
        valid: record.valid,
        message: record.message,
        config_hash: record.config_hash,
    }))
}

/// One dataset as another service consumes it.
async fn internal_dataset(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<Json<DatasetObject>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let version = Digest::parse(&version)
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
    Ok(Json(DatasetObject {
        data_version: record.summary.data_version,
        object_key: record.object_key,
        sha256: record.sha256,
        candles: record.summary.candles,
        first_bar: record.summary.first_bar,
        last_bar: record.summary.last_bar,
        bytes: record.summary.bytes,
    }))
}

async fn internal_dataset_count(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
) -> Result<Json<u64>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let count = catalog
        .count(profile)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(count))
}

/// The dataset an inspection names.
#[derive(Debug, Deserialize)]
struct InspectQuery {
    data_version: String,
}

/// Build or reuse one timeframe's canonical CSV.
async fn get_ladder(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((version, code)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiFailure> {
    let timeframe = Timeframe::from_code(&code).ok_or_else(|| {
        failure(
            ErrorCode::DatasetInvalid,
            format!("unknown timeframe {code}"),
        )
    })?;
    let (record, source) = load_dataset_object(&state, &headers, &version).await?;
    if timeframe == Timeframe::M1 {
        return Ok(csv_response(&headers, source));
    }
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let key = format!(
        "ladder/{}/full/{}",
        record.summary.data_version.as_hex(),
        timeframe.code()
    );
    if objects
        .exists(&key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    {
        let cached = objects
            .get(&key)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
        return Ok(csv_response(&headers, cached));
    }
    let object = ladder(&source, timeframe)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    objects
        .put(&key, object.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(csv_response(&headers, object))
}

/// A CSV body with a content ETag, or `304` when the caller already has it.
fn csv_response(headers: &HeaderMap, bytes: Vec<u8>) -> Response {
    let etag = format!("\"{}\"", Digest::sha256(&bytes));
    csv_with_etag(headers, etag, bytes)
}

/// Build the CSV response, honoring `If-None-Match`.
fn csv_with_etag(headers: &HeaderMap, etag: String, bytes: Vec<u8>) -> Response {
    if if_none_match(headers, &etag) {
        return (StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response();
    }
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (header::ETAG, etag),
        ],
        bytes,
    )
        .into_response()
}

/// Whether the caller's `If-None-Match` covers this ETag.
fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|candidate| {
                let candidate = candidate.trim();
                candidate == "*" || candidate == etag
            })
        })
}

/// Resolve a dataset and read its stored bytes, verifying the digest.
async fn load_dataset_object(
    state: &DataState,
    headers: &HeaderMap,
    requested: &str,
) -> Result<(DatasetRecord, Vec<u8>), ApiFailure> {
    let profile = request_profile(headers)?;
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
    state: &DataState,
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
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<Response, ApiFailure> {
    let (record, bytes) = load_dataset_object(&state, &headers, &version).await?;
    let etag = format!("\"{}\"", record.summary.data_version);
    Ok(csv_with_etag(&headers, etag, bytes))
}

async fn inspect_candles(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<InspectQuery>, QueryRejection>,
) -> Result<Json<DatasetSummary>, ApiFailure> {
    let (record, _inspection) = load_inspection(&state, &headers, query).await?;
    Ok(Json(record.summary))
}

async fn inspect_timeframes(
    State(state): State<Arc<DataState>>,
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

/// The optional history filter on a dataset list.
#[derive(Debug, Deserialize)]
struct DatasetListQuery {
    include_archived: Option<bool>,
}

async fn list_datasets(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<DatasetListQuery>, QueryRejection>,
) -> Result<Json<Vec<DatasetSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the query is not a dataset filter",
        )
    })?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let records = if query.include_archived.unwrap_or(false) {
        catalog.list_including_archived(profile).await
    } else {
        catalog.list(profile).await
    }
    .map_err(catalog_failure)?;
    Ok(Json(
        records.into_iter().map(|record| record.summary).collect(),
    ))
}

async fn get_dataset(
    State(state): State<Arc<DataState>>,
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

/// The typed confirmation required by a dataset erase.
#[derive(Debug, Deserialize)]
struct EraseDatasetQuery {
    confirm: Option<String>,
}

fn parse_dataset_version(version: &str) -> Result<String, ApiFailure> {
    Digest::parse(version)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))
}

fn catalog_failure(error: CatalogError) -> ApiFailure {
    match error {
        CatalogError::NotFound => failure(ErrorCode::DatasetNotFound, "the dataset is unknown"),
        CatalogError::Archived => {
            failure(ErrorCode::HistoryState, "the dataset is already archived")
        }
        CatalogError::NotArchived => failure(
            ErrorCode::HistoryState,
            "archive the dataset before erasing it",
        ),
        CatalogError::Referenced(count) => failure(
            ErrorCode::HistoryProtected,
            format!("the dataset still has {count} reference(s)"),
        ),
        CatalogError::AlreadyExists => {
            failure(ErrorCode::HistoryState, "the dataset is already catalogued")
        }
        CatalogError::Store(error) => failure(ErrorCode::Internal, error),
    }
}

async fn archive_dataset(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<Json<DatasetRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let version = parse_dataset_version(&version)?;
    let archived_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let _gate = state.dataset_gate.lock().await;
    catalog
        .archive(profile, version, archived_at)
        .await
        .map(Json)
        .map_err(catalog_failure)
}

async fn erase_dataset(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
    query: Result<Query<EraseDatasetQuery>, QueryRejection>,
) -> Result<Json<EraseDatasetResponse>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let version = parse_dataset_version(&version)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the query is not a dataset erase confirmation",
        )
    })?;
    let confirmed = query.confirm.as_deref().is_some_and(|confirm| {
        confirm == version || confirm.strip_prefix("sha256:") == version.strip_prefix("sha256:")
    });
    if !confirmed {
        return Err(failure(
            ErrorCode::HistoryState,
            "erase requires --confirm with the exact dataset id",
        ));
    }
    let _gate = state.dataset_gate.lock().await;
    let outcome = catalog
        .erase(profile, version.clone())
        .await
        .map_err(catalog_failure)?;
    let bytes = if outcome.object_shared {
        0
    } else {
        erase_object(
            catalog,
            objects.as_ref(),
            &outcome.record.object_key,
            outcome.record.summary.bytes,
        )
        .await
    };
    Ok(Json(EraseDatasetResponse {
        data_version: outcome.record.summary.data_version,
        bytes,
    }))
}

/// Delete an unshared dataset object under the database object guard.
///
/// The row and its tombstone are already committed. The guard is taken after
/// that commit, so an upload that is between its object write and its row insert
/// either finishes first (the recheck sees the row and keeps the bytes) or waits
/// (it writes the bytes after this deletion). A failed guard or delete leaves an
/// orphan object, which is safe; a missing object is never published.
async fn erase_object(
    catalog: &Arc<dyn crate::catalog::DatasetCatalog>,
    objects: &dyn quantrule_runtime::adapters::ObjectStore,
    key: &str,
    bytes: u64,
) -> u64 {
    let guard = match catalog.object_guard(key.to_owned()).await {
        Ok(guard) => guard,
        Err(error) => {
            tracing::warn!(object_key = key, %error, "erased dataset kept an orphan object");
            return 0;
        }
    };
    let referenced = match catalog.object_referenced(key.to_owned()).await {
        Ok(referenced) => referenced,
        Err(error) => {
            tracing::warn!(object_key = key, %error, "erased dataset kept an orphan object");
            let _ = guard.release().await;
            return 0;
        }
    };
    let bytes = if referenced {
        // A concurrent upload inserted its row after the erase transaction's
        // shared-object check. The object is now referenced again.
        0
    } else {
        match objects.delete(key).await {
            Ok(()) => bytes,
            Err(error) => {
                // The row and its tombstone are already committed. An orphan
                // object is a storage leak, never a missing shared artifact, so
                // the erase stands and the cleanup failure is reported here.
                tracing::warn!(
                    object_key = key,
                    %error,
                    "erased dataset left an orphan object"
                );
                0
            }
        }
    };
    if let Err(error) = guard.release().await {
        tracing::warn!(object_key = key, %error, "the dataset object guard did not release");
    }
    bytes
}

async fn register_dataset_reference(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
    payload: Result<Json<DatasetReferenceRequest>, JsonRejection>,
) -> Result<Json<DatasetReferenceCount>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let version = parse_dataset_version(&version)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the body is not a dataset reference",
        )
    })?;
    let reference_id = request.reference_id.trim().to_owned();
    if reference_id.is_empty() || reference_id.len() > 256 {
        return Err(failure(
            ErrorCode::DatasetInvalid,
            "a reference id must be between 1 and 256 characters",
        ));
    }
    let created_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let references = catalog
        .add_reference(
            profile,
            version,
            DatasetReference {
                reference_id,
                kind: request.kind,
                created_at,
            },
        )
        .await
        .map_err(catalog_failure)?;
    Ok(Json(DatasetReferenceCount { references }))
}

async fn release_dataset_reference(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
    payload: Result<Json<ReleaseDatasetReferenceRequest>, JsonRejection>,
) -> Result<Json<DatasetReferenceCount>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let version = parse_dataset_version(&version)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the body is not a reference release",
        )
    })?;
    let references = catalog
        .release_reference(profile, version, request.reference_id)
        .await
        .map_err(catalog_failure)?;
    Ok(Json(DatasetReferenceCount { references }))
}

async fn list_dataset_references(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(version): Path<String>,
) -> Result<Json<Vec<DatasetReference>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let version = parse_dataset_version(&version)?;
    catalog
        .references(profile, version)
        .await
        .map(Json)
        .map_err(catalog_failure)
}
