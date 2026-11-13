//! The HTTP surface of the runtime.
//!
//! The gateway serves `/health` and `/version`, seals and reads a hypothesis,
//! and ingests a dataset into the catalog. The remaining routes arrive with
//! their services.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::rejection::{BytesRejection, JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{HeaderMap, StatusCode, Uri, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use quantrule_api::{
    ConfigSummary, CreateProfileRequest, DatasetRecord, DatasetSummary, Digest, ErrorCode,
    ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, ProfileSummary, QualityReport,
    RunDetail, RunState, RunSummary, SubmitRunRequest, SubmitRunResponse, TimeframeBars,
    TimeframeInspection, UploadConfigRequest, VersionInfo,
};
use quantrule_data::Timeframe;
use quantrule_research::{Hypothesis, SealedHypothesis};
use serde::Deserialize;

use crate::configs::{ConfigCatalog, ConfigRecord, build, scan};
use crate::data::{
    CatalogError, DatasetCatalog, Ingested, Inspection, TolerantIngested, ingest, ingest_tolerant,
    inspect, ladder,
};
use crate::profile::{ProfileError, ProfileRecord, ProfileStore};
use crate::research::HypothesisStore;
use crate::runs::{CancelOutcome, NewRun, RunStore, derive};
use crate::runtime::adapters::{ObjectStore, Queue};
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
    /// Where run configs are catalogued.
    pub configs: Option<Arc<dyn ConfigCatalog>>,
    /// Where runs are recorded.
    pub runs: Option<Arc<dyn RunStore>>,
    /// The control-plane queue.
    pub queue: Option<Arc<dyn Queue>>,
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
    /// Where the built client lives; absent in a client-less test or probe.
    pub static_dir: Option<PathBuf>,
    /// Where the read-only run-config mount lives.
    pub config_root: PathBuf,
    /// Where run configs are catalogued; absent when no database is configured.
    pub configs: Option<Arc<dyn ConfigCatalog>>,
    /// Where runs are recorded; absent when no database is configured.
    pub runs: Option<Arc<dyn RunStore>>,
    /// The control-plane queue; absent when the broker is unreachable.
    pub queue: Option<Arc<dyn Queue>>,
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
            .field("static_dir", &self.static_dir)
            .field("config_root", &self.config_root)
            .field("configs", &self.configs.is_some())
            .field("runs", &self.runs.is_some())
            .field("queue", &self.queue.is_some())
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
            static_dir: config.static_dir.clone(),
            config_root: config.config_root.clone(),
            configs: services.configs,
            runs: services.runs,
            queue: services.queue,
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

/// The API routes, before state and the body limit are attached.
fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/hypotheses", post(create_hypothesis))
        .route("/hypotheses/{fingerprint}", get(get_hypothesis))
        .route("/catalog/datasets", get(list_datasets).post(create_dataset))
        .route("/catalog/datasets/tolerant", post(create_dataset_tolerant))
        .route("/catalog/datasets/{version}", get(get_dataset))
        .route(
            "/catalog/configs",
            get(list_configs)
                .post(upload_config)
                .layer(DefaultBodyLimit::max(4 * 1024 * 1024)),
        )
        .route("/runs", get(list_runs).post(submit_run))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/cancel", post(cancel_run))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route("/inspect/candles", get(inspect_candles))
        .route("/inspect/timeframes", get(inspect_timeframes))
        .route("/datasets/{version}/candles", get(get_candles))
        .route("/datasets/{version}/ladder/{timeframe}", get(get_ladder))
}

/// Build the router.
///
/// The client speaks `/api/v1`, the base in the contract. The same routes answer
/// at the root so a probe and a same-origin operator keep working without the
/// prefix.
pub fn router(state: Arc<AppState>) -> Router {
    let limit = state.max_upload;
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .fallback(static_client)
        .layer(DefaultBodyLimit::max(limit))
        .with_state(state)
}

/// Serve the built client, or the SPA entry point, for a non-API path.
async fn static_client(
    State(state): State<Arc<AppState>>,
    method: axum::http::Method,
    uri: Uri,
) -> impl IntoResponse {
    crate::runtime::static_files::serve(&state, &uri, &method).await
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
    headers: HeaderMap,
    payload: Result<Json<Hypothesis>, JsonRejection>,
) -> Result<(StatusCode, Json<SealedHypothesis>), ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
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
        .insert(profile, sealed.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(sealed)))
}

async fn get_hypothesis(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(fingerprint): Path<String>,
) -> Result<Json<SealedHypothesis>, ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let store = state
        .hypotheses
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no hypothesis store is configured"))?;
    match store.load(profile, fingerprint.clone()).await {
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
    let (profile, name, body) = read_dataset_upload(&state, &headers, query, payload).await?;
    let ingested = ingest(&name, &body)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    store_dataset(&state, profile, ingested, None).await
}

async fn create_dataset_tolerant(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
    payload: Result<Bytes, BytesRejection>,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
    let (profile, name, body) = read_dataset_upload(&state, &headers, query, payload).await?;
    let TolerantIngested { dataset, quality } = ingest_tolerant(&name, &body)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    store_dataset(&state, profile, dataset, Some(quality)).await
}

async fn read_dataset_upload(
    state: &AppState,
    headers: &HeaderMap,
    query: Result<Query<DatasetUpload>, QueryRejection>,
    payload: Result<Bytes, BytesRejection>,
) -> Result<(ProfileId, String, Bytes), ApiFailure> {
    let profile = request_profile(state, headers).await?;
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
    Ok((profile, query.name.unwrap_or_default(), body))
}

async fn store_dataset(
    state: &AppState,
    profile: ProfileId,
    ingested: Ingested,
    quality: Option<QualityReport>,
) -> Result<(StatusCode, Json<DatasetRecord>), ApiFailure> {
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
        quality,
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

/// Upload a run-config bundle, validate it, and catalogue it.
async fn upload_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<UploadConfigRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ConfigSummary>), ApiFailure> {
    let _profile = request_profile(&state, &headers).await?;
    let Json(request) = payload
        .map_err(|_| failure(ErrorCode::ConfigInvalid, "the body is not a config bundle"))?;
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let now = clock::now_utc();
    let staging = std::env::temp_dir();
    let record =
        tokio::task::spawn_blocking(move || build(&request.name, &request.files, &now, &staging))
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
            .map_err(|error| failure(ErrorCode::ConfigInvalid, error.0))?;

    if let Some(existing) = store
        .get(record.config_id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    {
        return Ok((StatusCode::OK, Json(existing.summary())));
    }
    store
        .upsert(record.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok((StatusCode::CREATED, Json(record.summary())))
}

/// The optional filters on a run list.
#[derive(Debug, Deserialize)]
struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
}

/// The model identity recorded in a run config, or `none` when absent.
fn model_version(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("model_id")
                .and_then(|value| value.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "none".to_owned())
}

async fn submit_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<SubmitRunRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<SubmitRunResponse>), ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::RunInvalid, "the body is not a run submission"))?;

    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    match catalog
        .get(profile.clone(), request.data_version.to_string())
        .await
    {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {} is unknown", request.data_version),
            ));
        }
        Err(error) => return Err(failure(ErrorCode::Internal, error.to_string())),
    }

    let configs = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let config = configs
        .get(request.config_id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .ok_or_else(|| {
            failure(
                ErrorCode::ConfigInvalid,
                format!("config {} is unknown", request.config_id),
            )
        })?;
    if !config.valid {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            config
                .message
                .unwrap_or_else(|| "the config is invalid".to_owned()),
        ));
    }
    let Some(config_hash) = config.config_hash.clone() else {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "a valid config carries no hash",
        ));
    };

    let model_version = model_version(&config.body);
    let image_digest = state
        .version
        .image_digest
        .clone()
        .unwrap_or_else(|| Digest::sha256(b"no-image-digest"));
    let run_id = derive(
        &request.data_version,
        &config_hash,
        &model_version,
        &image_digest,
    );
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let outcome = runs
        .submit(
            profile.clone(),
            NewRun {
                run_id,
                config_id: request.config_id,
                data_version: request.data_version,
                config_hash,
                model_version,
                image_digest,
                owner: profile.as_str().to_owned(),
                submitted_at: clock::now_utc(),
            },
        )
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let record = outcome.record().clone();

    // Publish for a duplicate too: a retry after a lost publish must be able to
    // recover, and the worker treats a repeated message as a lookup.
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::QueueUnavailable, "no run queue is configured"))?;
    let message = serde_json::json!({
        "run_id": record.run_id.to_string(),
        "data_version": record.data_version.to_string(),
        "config_id": record.config_id,
    });
    let payload = serde_json::to_vec(&message)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    queue
        .publish("run.requested", payload)
        .await
        .map_err(|error| failure(ErrorCode::QueueUnavailable, error.to_string()))?;

    let response = SubmitRunResponse {
        run_id: record.run_id,
        status: record.status,
        created: outcome.created(),
    };
    let status = if outcome.created() {
        StatusCode::ACCEPTED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(response)))
}

async fn list_runs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    query: Result<Query<RunListQuery>, QueryRejection>,
) -> Result<Json<Vec<RunSummary>>, ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::RunInvalid, "the query is not a run filter"))?;
    let status = match query.status.as_deref() {
        None => None,
        Some(value) => Some(RunState::parse(value).ok_or_else(|| {
            failure(ErrorCode::RunInvalid, format!("unknown run status {value}"))
        })?),
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    let summaries = runs
        .list(profile, status, limit)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(summaries))
}

fn parse_run_id(id: &str) -> Result<String, ApiFailure> {
    Digest::parse(id)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::RunInvalid, error.to_string()))
}

async fn get_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<RunDetail>, ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.get(profile, id.clone()).await {
        Ok(Some(record)) => Ok(Json(record.detail())),
        Ok(None) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

async fn cancel_run(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<RunDetail>), ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let id = parse_run_id(&id)?;
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    match runs.cancel(profile, id.clone()).await {
        Ok(CancelOutcome::Cancelled(record)) => Ok((StatusCode::ACCEPTED, Json(record.detail()))),
        Ok(CancelOutcome::Conflict(record)) => Err(failure(
            ErrorCode::RunConflict,
            format!(
                "run {} is already {}",
                record.run_id,
                record.status.as_str()
            ),
        )),
        Ok(CancelOutcome::Missing) => Err(failure(
            ErrorCode::RunNotFound,
            format!("run {id} is unknown"),
        )),
        Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
    }
}

/// Scan the read-only mount, refresh the catalog, and list every config.
async fn list_configs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ConfigSummary>>, ApiFailure> {
    let _profile = request_profile(&state, &headers).await?;
    let store = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let root = state.config_root.clone();
    let now = clock::now_utc();
    let records = tokio::task::spawn_blocking(move || scan(&root, &now))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    for record in records {
        store
            .upsert(record)
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }
    let records = store
        .list()
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(Json(records.iter().map(ConfigRecord::summary).collect()))
}

/// The dataset an inspection names.
#[derive(Debug, Deserialize)]
struct InspectQuery {
    data_version: String,
}

/// Build or reuse one timeframe's canonical CSV.
async fn get_ladder(
    State(state): State<Arc<AppState>>,
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
        return Ok(csv_response(source));
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
        return Ok(csv_response(cached));
    }
    let object = ladder(&source, timeframe)
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    objects
        .put(&key, object.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    Ok(csv_response(object))
}

/// A CSV body with a content ETag.
fn csv_response(bytes: Vec<u8>) -> impl IntoResponse {
    let etag = format!("\"{}\"", Digest::sha256(&bytes));
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv".to_owned()),
            (header::ETAG, etag),
        ],
        bytes,
    )
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
