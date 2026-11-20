//! The gateway HTTP surface.
//!
//! The gateway serves `/health`, `/version`, and profiles. The data, model,
//! evidence, and simulation surfaces are proxied: `/catalog/*`, `/datasets/*`,
//! `/inspect/*`, `/model/*`, `/runs/*`, `/stages/*`, `/hypotheses/*`,
//! `/research/*`, `/operations`, and `/simulation/*` are forwarded to their
//! service with the validated profile header, the method, the path, the query,
//! and a streamed body. The one rewritten route is `/inspect/events`: inspection
//! computes a model, so the gateway forwards it to the model service's internal
//! event-batch route instead of the data service. The gateway has no dataset,
//! config, inspection, model, run, stage, research, operations, or simulation
//! handler. `/stages` is a distinct prefix from `/runs`: a diagnostic stage path
//! is forwarded unchanged and is never rewritten into a run path.
//!
//! Every API route passes through the bearer seam when `QR_BEARER_TOKEN` is set;
//! the static client is served by the fallback outside that layer.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, StatusCode, Uri, header};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use axum::{Json, Router};
use quantrule_api::{
    ErrorCode, ErrorEnvelope, HealthReport, PROFILE_HEADER, ProfileId, ServiceHealth, ServiceState,
    VersionInfo,
};
use quantrule_runtime::client::{
    ClientError, DataClient, HttpServiceClient, REQUEST_TIMEOUT, RunCounter, Service,
    SimulationCounter,
};
use quantrule_runtime::config::Config;
use quantrule_runtime::error::RuntimeError;
use quantrule_runtime::health::HealthRegistry;

use crate::profile::{ProfileStore, create_profile, list_profiles};

/// The services a gateway can reach.
#[derive(Default)]
pub struct Services {
    /// Which profiles exist.
    pub profiles: Option<Arc<dyn ProfileStore>>,
    /// The data service's typed reads.
    pub data: Option<Arc<dyn DataClient>>,
    /// The data service's HTTP transport, for the proxy.
    pub upstream: Option<Arc<HttpServiceClient>>,
    /// The model service's HTTP transport, for the proxy.
    pub model_upstream: Option<Arc<HttpServiceClient>>,
    /// The evidence service's HTTP transport, for the proxy.
    pub evidence_upstream: Option<Arc<HttpServiceClient>>,
    /// The simulation service's HTTP transport, for the proxy.
    pub simulation_upstream: Option<Arc<HttpServiceClient>>,
    /// The evidence service's profile run count.
    pub runs: Option<Arc<dyn RunCounter>>,
    /// The simulation service's profile simulation count.
    pub simulations: Option<Arc<dyn SimulationCounter>>,
}

/// What a handler needs.
pub struct AppState {
    /// Product and contract versions.
    pub version: VersionInfo,
    /// The dependency probes.
    pub health: HealthRegistry,
    /// Which profiles exist; absent when no database is configured.
    pub profiles: Option<Arc<dyn ProfileStore>>,
    /// Where the built client lives; absent in a client-less test or probe.
    pub static_dir: Option<PathBuf>,
    /// The data service's typed reads; absent in a test that does not use them.
    pub data: Option<Arc<dyn DataClient>>,
    /// The data service's HTTP transport; absent in a test that does not proxy.
    pub upstream: Option<Arc<HttpServiceClient>>,
    /// The model service's HTTP transport; absent in a test that does not proxy.
    pub model_upstream: Option<Arc<HttpServiceClient>>,
    /// The evidence service's HTTP transport; absent in a test that does not proxy.
    pub evidence_upstream: Option<Arc<HttpServiceClient>>,
    /// The simulation service's HTTP transport; absent in a test that does not proxy.
    pub simulation_upstream: Option<Arc<HttpServiceClient>>,
    /// The evidence service's profile run count; absent in a test that does not use it.
    pub runs: Option<Arc<dyn RunCounter>>,
    /// The simulation service's profile count; absent in a test that does not use it.
    pub simulations: Option<Arc<dyn SimulationCounter>>,
    /// The bearer token the gateway requires, when one is configured.
    pub bearer_token: Option<Arc<str>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AppState")
            .field("version", &self.version)
            .field("health", &self.health)
            .field("profiles", &self.profiles.is_some())
            .field("static_dir", &self.static_dir)
            .field("data", &self.data.is_some())
            .field("upstream", &self.upstream.is_some())
            .field("model_upstream", &self.model_upstream.is_some())
            .field("evidence_upstream", &self.evidence_upstream.is_some())
            .field("simulation_upstream", &self.simulation_upstream.is_some())
            .field("runs", &self.runs.is_some())
            .field("simulations", &self.simulations.is_some())
            .field("bearer_token", &self.bearer_token.is_some())
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
            profiles: services.profiles,
            static_dir: config.static_dir.clone(),
            data: services.data,
            upstream: services.upstream,
            model_upstream: services.model_upstream,
            evidence_upstream: services.evidence_upstream,
            simulation_upstream: services.simulation_upstream,
            runs: services.runs,
            simulations: services.simulations,
            bearer_token: config.bearer_token.as_deref().map(Arc::<str>::from),
        }
    }
}

/// A streaming upload/download or a model compute gets a transfer deadline.
const LONG_REQUEST_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// The deadline for one proxied request.
///
/// A candle download, an artifact download, a dataset upload, or a model
/// compute can outlive the default read deadline; every other call keeps it.
fn proxy_deadline(method: &Method, path: &str) -> Duration {
    let long = (method == Method::POST
        && matches!(
            path,
            "/catalog/datasets" | "/catalog/datasets/tolerant" | "/model/compute"
        ))
        || (method == Method::GET
            && (path.starts_with("/datasets/") || path.contains("/artifacts/")));
    if long {
        LONG_REQUEST_TIMEOUT
    } else {
        REQUEST_TIMEOUT
    }
}

/// Forward one proxied request under the deadline its route needs.
async fn forward_upstream(
    upstream: &HttpServiceClient,
    service: Service,
    request: Request<axum::body::Body>,
) -> Result<Response, ClientError> {
    let deadline = proxy_deadline(request.method(), request.uri().path());
    upstream
        .forward_with_deadline(service, request, deadline)
        .await
}

/// A failed request: a status and the shared envelope.
pub(crate) type ApiFailure = (StatusCode, Json<ErrorEnvelope>);

pub(crate) fn failure(code: ErrorCode, message: impl Into<String>) -> ApiFailure {
    let status = StatusCode::from_u16(code.status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (status, Json(ErrorEnvelope::of(code, message)))
}

/// Map a service client failure onto the public envelope.
///
/// A transport failure is a 503 `service.unavailable`; an undecodable body is a
/// 500 `internal`. A 4xx the caller did not expect is an internal defect, not a
/// client error, because the gateway already validated the request.
pub(crate) fn service_failure(error: ClientError) -> ApiFailure {
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
        .route("/ready", get(ready))
        .route("/version", get(version))
        .route("/profiles", get(list_profiles).post(create_profile))
        .route("/runs", any(proxy_evidence))
        .route("/runs/{id}", any(proxy_evidence))
        .route("/runs/{id}/archive", any(proxy_evidence))
        .route("/runs/{id}/cancel", any(proxy_evidence))
        .route("/runs/{id}/digest", any(proxy_evidence))
        .route("/runs/{id}/artifacts/{*name}", any(proxy_evidence))
        .route("/hypotheses", any(proxy_evidence))
        .route("/hypotheses/{*rest}", any(proxy_evidence))
        .route("/research", any(proxy_evidence))
        .route("/research/{*rest}", any(proxy_evidence))
        .route("/operations", any(proxy_evidence))
        .route("/operations/{*rest}", any(proxy_evidence))
        .route("/stages", any(proxy_evidence))
        .route("/stages/{*rest}", any(proxy_evidence))
        .route("/catalog", any(proxy_data))
        .route("/catalog/{*rest}", any(proxy_data))
        .route("/datasets/{*rest}", any(proxy_data))
        .route("/inspect/events", get(proxy_events))
        .route("/inspect/{*rest}", any(proxy_data))
        .route("/model", any(proxy_model))
        .route("/model/{*rest}", any(proxy_model))
        .route("/simulation", any(proxy_simulation))
        .route("/simulation/{*rest}", any(proxy_simulation))
}

/// Build the router.
///
/// The client speaks `/api/v1`, the base in the contract. The same routes answer
/// at the root so a probe and a same-origin operator keep working without the
/// prefix. The API routes carry the bearer seam; the fallback does not, so the
/// static client stays reachable when a token is configured.
pub fn router(state: Arc<AppState>) -> Router {
    let api = Router::new()
        .merge(routes())
        .nest("/api/v1", routes())
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            crate::auth::authorize,
        ));
    Router::new()
        .merge(api)
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

/// The readiness probe: local checks plus every configured upstream's `/health`.
///
/// `/health` is the local liveness probe; `/ready` is the dependency authority
/// and returns `503` when a required service is down or unreachable.
async fn ready(State(state): State<Arc<AppState>>) -> (StatusCode, Json<HealthReport>) {
    let digest = state.version.image_digest.clone();
    let mut report = state.health.report(digest).await;
    let probes = [
        ("data", state.upstream.as_ref(), Service::Data),
        ("model", state.model_upstream.as_ref(), Service::Model),
        (
            "evidence",
            state.evidence_upstream.as_ref(),
            Service::Evidence,
        ),
        (
            "simulation",
            state.simulation_upstream.as_ref(),
            Service::Simulation,
        ),
    ];
    for (name, client, service) in probes {
        let entry = match client {
            Some(client) => match client.health(service).await {
                Ok(remote) if remote.service == name => {
                    report.queue_depth += remote.queue_depth;
                    report.workers += remote.workers;
                    ServiceHealth {
                        name: name.to_owned(),
                        state: remote.state,
                        detail: (remote.state == ServiceState::Down)
                            .then(|| "the service reports down".to_owned()),
                    }
                }
                Ok(remote) => ServiceHealth {
                    name: name.to_owned(),
                    state: ServiceState::Down,
                    detail: Some(format!(
                        "the service that answered identifies itself as {:?}",
                        remote.service
                    )),
                },
                Err(error) => ServiceHealth {
                    name: name.to_owned(),
                    state: ServiceState::Down,
                    detail: Some(error.to_string()),
                },
            },
            None => ServiceHealth {
                name: name.to_owned(),
                state: ServiceState::Down,
                detail: Some("not configured".to_owned()),
            },
        };
        report.services.push(entry);
    }
    report.state = HealthReport::aggregate(report.services.clone());
    let status = if report.state == ServiceState::Down {
        StatusCode::SERVICE_UNAVAILABLE
    } else {
        StatusCode::OK
    };
    (status, Json(report))
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
    let response = forward_upstream(upstream.as_ref(), Service::Data, forwarded)
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

/// Forward an event inspection to `qr-model`.
///
/// The public path is `/inspect/events`, but inspection computes a model, and
/// the data service never computes one. The gateway validates the profile,
/// keeps the query, and sends the request to the model service's internal
/// event-batch route. The client cannot choose another internal path: the
/// gateway builds it here.
async fn proxy_events(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_events(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_events(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.model_upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the model service is not configured",
        ));
    };
    let query = request
        .uri()
        .query()
        .map(|query| format!("?{query}"))
        .unwrap_or_default();
    let headers = proxy_request_headers(request.headers(), &profile);
    let mut forwarded = Request::builder()
        .method(Method::GET)
        .uri(format!("/internal/v1/events{query}"))
        .body(axum::body::Body::empty())
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    *forwarded.headers_mut() = headers;
    let response = forward_upstream(upstream.as_ref(), Service::Model, forwarded)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the model service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}

/// Forward a model request to `qr-model`.
///
/// The same boundary as the data proxy: validate the profile, replace any
/// client-supplied value with the validated one, filter the headers, stream the
/// body, and pass the status, content type, ETag, and error envelope through. An
/// unreachable service is `503 service.unavailable` with no local fallback.
async fn proxy_model(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_model(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_model(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.model_upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the model service is not configured",
        ));
    };
    if !model_path_allowed(request.uri().path()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::of(
                ErrorCode::Internal,
                "the path is not a model route",
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
    let response = forward_upstream(upstream.as_ref(), Service::Model, forwarded)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the model service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}

/// A path the model proxy owns. `/internal/v1` is never in this set.
fn model_path_allowed(path: &str) -> bool {
    const PREFIXES: [&str; 1] = ["/model"];
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

/// Forward a simulation request to `qr-simulation`.
///
/// The same boundary as the other proxies: validate the profile, replace any
/// client-supplied value with the validated one, filter the headers, stream the
/// body, and pass the status, content type, ETag, and error envelope through. An
/// unreachable service is `503 service.unavailable` with no local fallback. The
/// gateway keeps no simulation store and writes no simulation row.
async fn proxy_simulation(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_simulation(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_simulation(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.simulation_upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the simulation service is not configured",
        ));
    };
    if !simulation_path_allowed(request.uri().path()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::of(
                ErrorCode::Internal,
                "the path is not a simulation route",
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
    let response = forward_upstream(upstream.as_ref(), Service::Simulation, forwarded)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the simulation service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}

/// A path the simulation proxy owns. `/internal/v1` is never in this set.
fn simulation_path_allowed(path: &str) -> bool {
    const PREFIXES: [&str; 1] = ["/simulation"];
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

/// Forward a run or hypothesis request to `qr-evidence`.
///
/// The same boundary as the data and model proxies: validate the profile,
/// replace any client-supplied value with the validated one, filter the
/// headers, stream the body, and pass the status, content type, ETag, and error
/// envelope through. An unreachable service is `503 service.unavailable` with no
/// local fallback. The gateway writes no run row.
async fn proxy_evidence(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
) -> Response {
    match forward_evidence(&state, request).await {
        Ok(response) => response,
        Err(failure) => failure.into_response(),
    }
}

async fn forward_evidence(
    state: &AppState,
    request: Request<axum::body::Body>,
) -> Result<Response, ApiFailure> {
    let profile = {
        let headers = request.headers();
        request_profile(state, headers).await?
    };
    let Some(upstream) = state.evidence_upstream.as_ref() else {
        return Err(failure(
            ErrorCode::ServiceUnavailable,
            "the evidence service is not configured",
        ));
    };
    if !evidence_path_allowed(request.uri().path()) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorEnvelope::of(
                ErrorCode::Internal,
                "the path is not an evidence route",
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
    let response = forward_upstream(upstream.as_ref(), Service::Evidence, forwarded)
        .await
        .map_err(|error| {
            failure(
                ErrorCode::ServiceUnavailable,
                format!("the evidence service is unavailable: {error}"),
            )
        })?;
    Ok(proxy_response(response))
}

/// A path the evidence proxy owns. `/internal/v1` is never in this set.
///
/// An artifact name is a relative path: nested names such as
/// `execution/fills.csv` are served, while an absolute path, an empty segment,
/// a `.` or `..` segment, a backslash, or an encoded separator or parent path
/// is refused before it reaches evidence.
fn evidence_path_allowed(path: &str) -> bool {
    const PREFIXES: [&str; 5] = [
        "/runs",
        "/stages",
        "/hypotheses",
        "/research",
        "/operations",
    ];
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("%2e") || lowered.contains("%2f") || lowered.contains("%5c") {
        return false;
    }
    let allowed = PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }) && !path
        .trim_start_matches('/')
        .split('/')
        .any(|segment| segment.is_empty() || segment == ".." || segment == ".");
    allowed
        && artifact_name(path).is_none_or(|name| {
            !name.is_empty()
                && !name.starts_with('/')
                && !name.contains('\\')
                && name
                    .split('/')
                    .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
        })
}

/// The nested artifact name in an artifact download path.
fn artifact_name(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("/runs/")?;
    let (id, name) = rest.split_once("/artifacts/")?;
    if id.is_empty() {
        return None;
    }
    Some(name)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_routes_get_the_transfer_deadline() {
        assert_eq!(
            proxy_deadline(&Method::POST, "/catalog/datasets"),
            LONG_REQUEST_TIMEOUT
        );
        assert_eq!(
            proxy_deadline(&Method::POST, "/model/compute"),
            LONG_REQUEST_TIMEOUT
        );
        assert_eq!(
            proxy_deadline(&Method::GET, "/datasets/sha256:abc/candles"),
            LONG_REQUEST_TIMEOUT
        );
        assert_eq!(
            proxy_deadline(&Method::GET, "/runs/sha256:abc/artifacts/results.json"),
            LONG_REQUEST_TIMEOUT
        );
        assert_eq!(
            proxy_deadline(&Method::GET, "/stages/runs/abc/artifacts/results.json"),
            LONG_REQUEST_TIMEOUT
        );
        assert_eq!(proxy_deadline(&Method::GET, "/runs"), REQUEST_TIMEOUT);
        assert_eq!(proxy_deadline(&Method::POST, "/runs"), REQUEST_TIMEOUT);
        assert_eq!(
            proxy_deadline(&Method::GET, "/inspect/candles"),
            REQUEST_TIMEOUT
        );
    }
}
