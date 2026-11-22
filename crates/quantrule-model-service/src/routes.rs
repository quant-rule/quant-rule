//! The model service's HTTP surface.
//!
//! The public route is the model compute the gateway proxies; the internal route
//! is the content-addressed bundle reference the backtest worker consumes. The
//! service trusts the profile header the gateway validates and injects: it owns
//! no profile store and never reads a run schema.
//!
//! The public response is unchanged from the coupled gateway route: the same
//! request deserialization, the same data reads through [`DataClient`], the same
//! compute, and the same [`ModelObject`] bytes.

use std::sync::Arc;

use axum::Router;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Json, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use quantrule_api::{
    Digest, ErrorCode, ErrorEnvelope, EventBatch, EventRecord, HealthReport, ModelComputeRequest,
    ModelObject, PROFILE_HEADER, ProfileId, VersionInfo,
};
use quantrule_app::config::RunConfig;
use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;
use quantrule_runtime::client::ClientError;
use quantrule_service_api::ModelBundleManifest;
use serde::Deserialize;

use crate::bundle::{self, BundleError};
use crate::model::series::series;
use crate::model::{ModelConfig, ModelError, compute_sharded, load_run_config};
use crate::state::ModelState;

/// A failed request: a status and the shared envelope.
type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Map a data-service client failure onto the public envelope.
///
/// A transport failure is a 503 `service.unavailable`; an undecodable body is a
/// 500 `internal`. This is the same mapping the coupled gateway route used.
fn data_failure(error: ClientError) -> ApiFailure {
    match &error {
        ClientError::InvalidResponse { .. } | ClientError::InvalidBase { .. } => {
            failure(ErrorCode::Internal, error.to_string())
        }
        ClientError::Unreachable { .. }
        | ClientError::Timeout { .. }
        | ClientError::Unconfigured { .. }
        | ClientError::Interrupted { .. }
        | ClientError::Status { .. } => failure(ErrorCode::ServiceUnavailable, error.to_string()),
    }
}

/// The routes before state is attached.
fn routes() -> Router<Arc<ModelState>> {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/version", get(version))
        .route("/model/compute", post(compute_model))
        .route("/internal/v1/events", get(event_batch))
        .route("/internal/v1/model/bundle", post(compute_bundle))
}

/// Build the service router.
///
/// The gateway forwards the client path unchanged, so the public route answers at
/// the root and under `/api/v1`. `/internal/v1` is service-to-service only and is
/// never proxied by the gateway.
pub fn router(state: Arc<ModelState>) -> Router {
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .with_state(state)
}

async fn health(State(state): State<Arc<ModelState>>) -> Json<HealthReport> {
    let digest = state.version.image_digest.clone();
    Json(state.health.report(digest).await)
}

async fn ready(State(state): State<Arc<ModelState>>) -> (StatusCode, Json<HealthReport>) {
    let digest = state.version.image_digest.clone();
    let (ready, report) = state.health.readiness(digest).await;
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(report))
}

async fn version(State(state): State<Arc<ModelState>>) -> Json<VersionInfo> {
    Json(state.version.clone())
}

/// Read the injected profile header.
///
/// The gateway validates that the profile exists and injects the normalized
/// value. The model service trusts that value; a direct caller cannot reach it on
/// the published surface.
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

/// The model inputs resolved from the data service and the object store.
struct ModelInputs {
    data_version: Digest,
    config_id: String,
    config_hash: Digest,
    config: RunConfig,
    model: ModelConfig,
    bytes: Vec<u8>,
}

/// Resolve the dataset and config references, then materialize the run config.
async fn load_inputs(
    state: &ModelState,
    profile: ProfileId,
    data_version: Digest,
    config_id: String,
) -> Result<ModelInputs, ApiFailure> {
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let dataset = data
        .dataset_object(profile.clone(), data_version.to_string())
        .await
        .map_err(data_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {data_version} is unknown"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&dataset.object_key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if Digest::sha256(&bytes) != dataset.sha256 {
        return Err(failure(
            ErrorCode::Internal,
            "the stored object does not match the catalog",
        ));
    }

    let config = data
        .config_bundle(profile, config_id.clone())
        .await
        .map_err(data_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ConfigInvalid,
                format!("config {config_id} is unknown"),
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
    let config_hash = config
        .config_hash
        .clone()
        .ok_or_else(|| failure(ErrorCode::ConfigInvalid, "a valid config carries no hash"))?;
    let files = config.files.clone();
    let run = tokio::task::spawn_blocking(move || load_run_config(&files, &std::env::temp_dir()))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::ConfigInvalid, error.to_string()))?;
    let model = ModelConfig::from_run(&run);

    Ok(ModelInputs {
        data_version,
        config_id,
        config_hash,
        config: run,
        model,
        bytes,
    })
}

/// Parse a requested ladder, or `None` for the configured one.
fn parse_timeframes(values: Option<Vec<String>>) -> Result<Option<Vec<Timeframe>>, ApiFailure> {
    let Some(values) = values else {
        return Ok(None);
    };
    if values.is_empty() {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            "the model ladder is empty",
        ));
    }
    let mut parsed = Vec::with_capacity(values.len());
    for value in values {
        let timeframe = Timeframe::from_code(&value).ok_or_else(|| {
            failure(
                ErrorCode::ConfigInvalid,
                format!("unknown timeframe {value}"),
            )
        })?;
        parsed.push(timeframe);
    }
    Ok(Some(parsed))
}

/// Compute the model for a dataset and config, then store one JSON object.
async fn compute_model(
    State(state): State<Arc<ModelState>>,
    headers: HeaderMap,
    payload: Result<Json<ModelComputeRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ModelObject>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload
        .map_err(|_| failure(ErrorCode::ConfigInvalid, "the body is not a model request"))?;
    let ModelComputeRequest {
        data_version,
        config_id,
        timeframes,
    } = request;

    let inputs = load_inputs(&state, profile, data_version, config_id).await?;
    let requested = parse_timeframes(timeframes)?;

    let ModelInputs {
        data_version,
        config_id,
        config_hash,
        config: _,
        model,
        bytes,
    } = inputs;
    let limit = model.data.max_candles;
    let set = tokio::task::spawn_blocking(move || series(&bytes, limit))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    let label = data_version.to_string();
    let computed = tokio::task::spawn_blocking(move || {
        compute_sharded(&set, &model, requested.as_deref(), &label)
    })
    .await
    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    .map_err(|error| match error {
        ModelError::UnsupportedTimeframe(_) | ModelError::EmptyLadder | ModelError::Config(_) => {
            failure(ErrorCode::ConfigInvalid, error.to_string())
        }
        ModelError::Output(_) => failure(ErrorCode::Internal, error.to_string()),
    })?;

    let model_version = crate::bundle::MODEL_VERSION;
    let codes: Vec<String> = computed
        .timeframes
        .iter()
        .map(|timeframe| timeframe.code().to_owned())
        .collect();
    let ladder = codes.join("-");
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let key = format!(
        "model/{}/{}/{}/{}.json",
        data_version.as_hex(),
        config_hash.as_hex(),
        model_version,
        ladder
    );
    let present = objects
        .exists(&key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if !present {
        objects
            .put(&key, computed.bytes.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }

    Ok((
        StatusCode::OK,
        Json(ModelObject {
            data_version,
            config_id,
            config_hash,
            model_version: model_version.to_owned(),
            timeframes: codes,
            object_key: key,
            sha256: Digest::sha256(&computed.bytes),
            bytes: computed.bytes.len() as u64,
        }),
    ))
}

/// The query on the internal event-batch route.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventsQuery {
    data_version: Digest,
    timeframe: String,
}

/// Read one stored dataset by reference and verify its bytes.
async fn dataset_bytes(
    state: &ModelState,
    profile: ProfileId,
    data_version: &Digest,
) -> Result<Vec<u8>, ApiFailure> {
    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    let dataset = data
        .dataset_object(profile, data_version.to_string())
        .await
        .map_err(data_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {data_version} is unknown"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&dataset.object_key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if Digest::sha256(&bytes) != dataset.sha256 {
        return Err(failure(
            ErrorCode::Internal,
            "the stored object does not match the catalog",
        ));
    }
    Ok(bytes)
}

/// Publish one engine event record in the API shape.
fn published(record: &quantrule_model::events::EventRecord) -> EventRecord {
    EventRecord {
        event_code: record.event_code,
        event_type: record.event_type.code(),
        event_subtype: record.event_subtype_code,
        source: record.source_code,
        occurred_bar: u64::try_from(record.occurred_bar_index).unwrap_or(u64::MAX),
        available_bar: u64::try_from(record.available_bar_index).unwrap_or(u64::MAX),
        direction: record.direction_code,
        reference_code: record.reference_code,
        primary_value: record.primary_value,
        reference_value: record.reference_value,
    }
}

/// Compute one timeframe's event batch for a stored dataset.
///
/// Inspection is outside the run and writes nothing. The batch is the one the
/// `events` command publishes: the registry's default configuration over the
/// requested timeframe alone. A timeframe's events do not depend on the other
/// members of the ladder, so the single-timeframe shard is the full answer.
async fn event_batch(
    State(state): State<Arc<ModelState>>,
    headers: HeaderMap,
    query: Result<Query<EventsQuery>, QueryRejection>,
) -> Result<Json<EventBatch>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::DatasetInvalid,
            "the query names no data_version and timeframe",
        )
    })?;
    let timeframe = Timeframe::from_code(&query.timeframe).ok_or_else(|| {
        failure(
            ErrorCode::DatasetInvalid,
            format!("unknown timeframe {}", query.timeframe),
        )
    })?;
    let bytes = dataset_bytes(&state, profile, &query.data_version).await?;
    let code = timeframe.code().to_owned();
    let events = tokio::task::spawn_blocking(move || {
        let set = series(&bytes, quantrule_data::MAX_SERIES)
            .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
        let run = quantrule_app::pipeline::model_run_from_set_with_ladder(
            &set,
            &[timeframe],
            &MarketStateConfig::default(),
        );
        let model = run
            .timeframe(u64::from(timeframe.seconds()))
            .ok_or_else(|| {
                failure(
                    ErrorCode::DatasetInvalid,
                    format!("the {} timeframe has no complete bar", timeframe.code()),
                )
            })?;
        Ok::<_, ApiFailure>(model.all_events().iter().map(published).collect())
    })
    .await
    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))??;
    Ok(Json(EventBatch {
        data_version: query.data_version,
        timeframe: code,
        events,
    }))
}

/// One internal model-bundle request.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelBundleRequest {
    data_version: Digest,
    config_id: String,
}

/// Compute the model once, store the bundle artifacts, and return the manifest.
async fn compute_bundle(
    State(state): State<Arc<ModelState>>,
    headers: HeaderMap,
    payload: Result<Json<ModelBundleRequest>, JsonRejection>,
) -> Result<Json<ModelBundleManifest>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::ConfigInvalid,
            "the body is not a model bundle request",
        )
    })?;
    let inputs = load_inputs(&state, profile, request.data_version, request.config_id).await?;
    let ModelInputs {
        data_version,
        config_id,
        config_hash,
        config,
        model,
        bytes,
    } = inputs;

    let build_version = data_version.clone();
    let build_hash = config_hash.clone();
    let build_image = state.version.image_digest.clone();
    let built = tokio::task::spawn_blocking(move || {
        bundle::build(
            &config,
            &model,
            &bytes,
            &build_version,
            &build_hash,
            build_image.as_ref(),
        )
    })
    .await
    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    .map_err(bundle_failure)?;

    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let manifest = bundle::publish(
        objects.as_ref(),
        &built,
        data_version,
        config_id,
        config_hash,
    )
    .await
    .map_err(bundle_failure)?;
    Ok(Json(manifest))
}

/// Map a bundle failure onto the public envelope.
fn bundle_failure(error: BundleError) -> ApiFailure {
    match error {
        BundleError::EmptyLadder => failure(ErrorCode::ConfigInvalid, error.to_string()),
        BundleError::EmptyPeriod | BundleError::Series(_) => {
            failure(ErrorCode::DatasetInvalid, error.to_string())
        }
        BundleError::Output(_) | BundleError::Store(_) => {
            failure(ErrorCode::Internal, error.to_string())
        }
    }
}
