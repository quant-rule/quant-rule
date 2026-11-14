//! The gateway HTTP surface.
//!
//! The gateway serves `/health`, `/version`, profiles, hypotheses, runs, and the
//! public model route until their services land. The data surface is proxied:
//! `/catalog/*`, `/datasets/*`, and `/inspect/*` are forwarded to `qr-data` with
//! the validated profile header, the method, the path, the query, and a streamed
//! body. The gateway has no dataset, config, or inspection handler.

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, post};
use axum::{Json, Router};
use quantrule_api::{
    Digest, ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, RunDetail, RunState,
    RunSummary, SubmitRunRequest, SubmitRunResponse, VersionInfo,
};
use quantrule_research::{Hypothesis, SealedHypothesis};
use quantrule_runtime::adapters::{ObjectStore, Queue};
use quantrule_runtime::client::{ClientError, DataClient, HttpServiceClient, Service};
use quantrule_runtime::clock;
use quantrule_runtime::config::Config;
use quantrule_runtime::error::RuntimeError;
use quantrule_runtime::health::HealthRegistry;
use quantrule_runtime::queue::{ROUTING_CANCELLED, ROUTING_REQUESTED};

use crate::profile::{ProfileStore, create_profile, list_profiles};
use crate::research::HypothesisStore;
use crate::runs::{CancelOutcome, NewRun, RunStore, derive};

/// The services a gateway can reach.
#[derive(Default)]
pub struct Services {
    /// Where sealed hypotheses live.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where canonical bytes live; the model route reads its input here until
    /// the model service owns it.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// Which profiles exist.
    pub profiles: Option<Arc<dyn ProfileStore>>,
    /// Where runs are recorded.
    pub runs: Option<Arc<dyn RunStore>>,
    /// The control-plane queue.
    pub queue: Option<Arc<dyn Queue>>,
    /// The data service's typed reads.
    pub data: Option<Arc<dyn DataClient>>,
    /// The data service's HTTP transport, for the proxy.
    pub upstream: Option<Arc<HttpServiceClient>>,
}

/// What a handler needs.
pub struct AppState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Where sealed hypotheses live; absent when no database is configured.
    pub hypotheses: Option<Arc<dyn HypothesisStore>>,
    /// Where canonical bytes live; absent when no object store is configured.
    pub objects: Option<Arc<dyn ObjectStore>>,
    /// Which profiles exist; absent when no database is configured.
    pub profiles: Option<Arc<dyn ProfileStore>>,
    /// Where the built client lives; absent in a client-less test or probe.
    pub static_dir: Option<PathBuf>,
    /// Where runs are recorded; absent when no database is configured.
    pub runs: Option<Arc<dyn RunStore>>,
    /// The control-plane queue; absent when the broker is unreachable.
    pub queue: Option<Arc<dyn Queue>>,
    /// The data service's typed reads; absent in a test that does not use them.
    pub data: Option<Arc<dyn DataClient>>,
    /// The data service's HTTP transport; absent in a test that does not proxy.
    pub upstream: Option<Arc<HttpServiceClient>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("hypotheses", &self.hypotheses.is_some())
            .field("objects", &self.objects.is_some())
            .field("profiles", &self.profiles.is_some())
            .field("static_dir", &self.static_dir)
            .field("runs", &self.runs.is_some())
            .field("queue", &self.queue.is_some())
            .field("data", &self.data.is_some())
            .field("upstream", &self.upstream.is_some())
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
            objects: services.objects,
            profiles: services.profiles,
            static_dir: config.static_dir.clone(),
            runs: services.runs,
            queue: services.queue,
            data: services.data,
            upstream: services.upstream,
        }
    }
}

/// A failed request: a status and the shared envelope.
pub(crate) type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

pub(crate) fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Map a data-service client failure onto the public envelope.
///
/// A transport failure is a 503 `service.unavailable`; an undecodable body is a
/// 500 `internal`. A 4xx the caller did not expect is an internal defect, not a
/// client error, because the gateway already validated the request.
pub(crate) fn data_failure(error: ClientError) -> ApiFailure {
    match &error {
        ClientError::InvalidResponse { .. } | ClientError::InvalidBase { .. } => {
            failure(ErrorCode::Internal, error.to_string())
        }
        ClientError::Unreachable { .. }
        | ClientError::Timeout { .. }
        | ClientError::Unconfigured { .. }
        | ClientError::Status { .. } => failure(ErrorCode::ServiceUnavailable, error.to_string()),
    }
}

/// The API routes, before state and the body limit are attached.
fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/hypotheses", post(create_hypothesis))
        .route("/hypotheses/{fingerprint}", get(get_hypothesis))
        .route("/model/compute", post(crate::model::compute_model))
        .route("/runs", get(list_runs).post(submit_run))
        .route("/runs/{id}", get(get_run))
        .route("/runs/{id}/cancel", post(cancel_run))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route("/catalog", any(proxy_data))
        .route("/catalog/{*rest}", any(proxy_data))
        .route("/datasets/{*rest}", any(proxy_data))
        .route("/inspect/{*rest}", any(proxy_data))
}

/// Build the router.
///
/// The client speaks `/api/v1`, the base in the contract. The same routes answer
/// at the root so a probe and a same-origin operator keep working without the
/// prefix.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .fallback(static_client)
        .with_state(state)
}

/// Serve the built client, or the SPA entry point, for a non-API path.
async fn static_client(
    State(state): State<Arc<AppState>>,
    method: axum::http::Method,
    uri: Uri,
) -> impl IntoResponse {
    crate::static_files::serve(&state, &uri, &method).await
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
pub(crate) async fn request_profile(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<ProfileId, ApiFailure> {
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

    let data = state.data.as_ref().ok_or_else(|| {
        failure(
            ErrorCode::ServiceUnavailable,
            "no data service is configured",
        )
    })?;
    match data
        .dataset_object(profile.clone(), request.data_version.to_string())
        .await
        .map_err(data_failure)?
    {
        Some(_) => {}
        None => {
            return Err(failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {} is unknown", request.data_version),
            ));
        }
    }

    let config = data
        .config_bundle(profile.clone(), request.config_id.clone())
        .await
        .map_err(data_failure)?
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

    let model_version = model_version(config.files.get("run.json").map_or("", String::as_str));
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
    // recover, and the worker treats a repeated message as a lookup. The
    // message carries the run id only; the worker reads the rest from the run
    // store, so a queued message cannot disagree with the recorded provenance.
    let queue = state
        .queue
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::QueueUnavailable, "no run queue is configured"))?;
    let message = serde_json::json!({ "run_id": record.run_id.to_string() });
    let payload = serde_json::to_vec(&message)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    queue
        .publish(ROUTING_REQUESTED, payload)
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
    query: Result<axum::extract::Query<RunListQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Vec<RunSummary>>, ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let axum::extract::Query(query) =
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

/// The optional filters on a run list.
#[derive(Debug, serde::Deserialize)]
struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
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
        Ok(CancelOutcome::Cancelled(record)) => {
            // The run row is authoritative; the message is an observation for
            // the worker's queue. A publish failure must not undo the cancel.
            if let Some(queue) = state.queue.as_ref() {
                let payload = serde_json::to_vec(&serde_json::json!({
                    "run_id": record.run_id.to_string(),
                }))
                .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
                if let Err(error) = queue.publish(ROUTING_CANCELLED, payload).await {
                    tracing::warn!(run_id = %record.run_id, %error, "cancel message was not published");
                }
            }
            Ok((StatusCode::ACCEPTED, Json(record.detail())))
        }
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

/// Forward a data request to `qr-data`.
///
/// The gateway validates the profile, replaces any client-supplied profile
/// value with the validated one, filters the request headers to the allowlist,
/// and streams the body. The status, content type, ETag, and error envelope are
/// passed through unchanged. An unreachable service is `503 service.unavailable`
/// with no local fallback.
async fn proxy_data(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_data(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_data(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the data service is not configured",
        ));
    };
    if !data_path_allowed(request.uri().path()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::of(
                ErrorCode::Internal,
                "the path is not a data route",
            )),
        ));
    }

    let original = request
        .extensions()
        .get::<axum::extract::OriginalUri>()
        .map(|original| original.0.clone());
    let (parts, body) = request.into_parts();
    let mut forwarded = Request::from_parts(parts, body);
    if let Some(original) = original {
        *forwarded.uri_mut() = original;
    }
    let headers = proxy_request_headers(forwarded.headers(), &profile);
    *forwarded.headers_mut() = headers;
    let response = upstream
        .forward(Service::Data, forwarded)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the data service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}

/// A path the data proxy owns. `/internal/v1` is never in this set.
fn data_path_allowed(path: &str) -> bool {
    const PREFIXES: [&str; 3] = ["/catalog", "/datasets", "/inspect"];
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("%2e") || lowered.contains("%2f") || lowered.contains("%5c") {
        return false;
    }
    PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }) && !path
        .split('/')
        .any(|segment| segment == ".." || segment == ".")
}

/// The request headers the gateway forwards. Everything else, including a
/// client-set profile, host, authorization, and internal headers, is dropped.
fn proxy_request_headers(source: &HeaderMap, profile: &ProfileId) -> HeaderMap {
    const ALLOWED: [&str; 4] = ["content-type", "accept", "if-none-match", "content-length"];
    let mut headers = HeaderMap::new();
    for name in ALLOWED {
        if let Some(value) = source.get(name) {
            headers.insert(header::HeaderName::from_static(name), value.clone());
        }
    }
    if let Ok(value) = header::HeaderValue::from_str(profile.as_str()) {
        headers.insert(
            header::HeaderName::from_static("x-quantrule-profile"),
            value,
        );
    }
    headers
}

/// The response headers the gateway forwards. A hop-by-hop header is dropped.
fn proxy_response(response: Response<axum::body::Body>) -> Response {
    const ALLOWED: [&str; 5] = [
        "content-type",
        "etag",
        "last-modified",
        "cache-control",
        "content-disposition",
    ];
    let (parts, body) = response.into_parts();
    let mut filtered = HeaderMap::new();
    for name in ALLOWED {
        if let Some(value) = parts.headers.get(name) {
            filtered.insert(header::HeaderName::from_static(name), value.clone());
        }
    }
    let mut response = Response::new(body);
    *response.status_mut() = parts.status;
    *response.version_mut() = parts.version;
    *response.headers_mut() = filtered;
    response
}

/// Bind and serve until a termination signal arrives.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the address cannot be bound or the
/// server fails.
pub async fn serve(config: &Config, state: Arc<AppState>) -> Result<(), RuntimeError> {
    quantrule_runtime::serve(&config.bind, router(state)).await
}
