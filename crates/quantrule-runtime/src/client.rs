//! The internal service client contract.
//!
//! A service reaches another only through this seam: a named service, a path on
//! the `quantrule.internal/v1` surface, and an opaque body. The contract is
//! bytes-only on purpose. Typed payloads belong to the service that owns the
//! endpoint, and a concrete HTTP transport lands with the first proxied route;
//! P1 freezes the seam so the gateway and the workers share one vocabulary.
//!
//! Timeouts, retries, and the 503 mapping are transport policy, not contract.
//! They are applied by the concrete client, never by a caller.

use std::collections::BTreeMap;
use std::time::Duration;

use axum::body::Body;
use http::{Request, Response, Uri, header};
use http_body_util::{BodyExt, Limited};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use quantrule_api::{HealthReport, PROFILE_HEADER, ProfileId};
use quantrule_service_api::{ConfigBundle, DatasetObject};

use crate::adapters::BoxFuture;

mod counts;
mod evidence;
mod model;
mod stage;

pub use counts::{COUNT_PAGE, MemoryCounts, RunCounter, SimulationCounter};
pub use evidence::{EvidenceClient, MemoryEvidence};
pub use model::{MemoryModel, ModelClient};
pub use stage::{MemoryStageClient, StageClient};

/// A service reachable on the internal network.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Service {
    /// Ingestion, the catalog, and the dataset ladder.
    Data,
    /// Model compute and model bundles.
    Model,
    /// Run records, finalization, and research memory.
    Evidence,
    /// Chart-first research.
    Simulation,
    /// The backtest worker's health endpoint; it exposes no other route.
    Backtest,
}

impl Service {
    /// The service name used in a health report and in an error.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Data => "data",
            Self::Model => "model",
            Self::Evidence => "evidence",
            Self::Simulation => "simulation",
            Self::Backtest => "backtest",
        }
    }

    /// The environment variable that names its base URL.
    #[must_use]
    pub fn url_variable(self) -> &'static str {
        match self {
            Self::Data => "QR_DATA_URL",
            Self::Model => "QR_MODEL_URL",
            Self::Evidence => "QR_EVIDENCE_URL",
            Self::Simulation => "QR_SIMULATION_URL",
            Self::Backtest => "QR_BACKTEST_HEALTH_URL",
        }
    }
}

/// The HTTP method of an internal call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    /// A read.
    Get,
    /// A write or a submission.
    Post,
}

impl Method {
    /// The wire token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
        }
    }
}

/// One request on the internal surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRequest {
    /// The method.
    pub method: Method,
    /// The path, beginning with `/`.
    pub path: String,
    /// Request headers the transport may forward. The profile header travels
    /// here; a service never invents one.
    pub headers: BTreeMap<String, String>,
    /// The body; empty for a read.
    pub body: Vec<u8>,
}

impl ServiceRequest {
    /// A read.
    #[must_use]
    pub fn get(path: impl Into<String>) -> Self {
        Self {
            method: Method::Get,
            path: path.into(),
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }

    /// Add one request header.
    #[must_use]
    pub fn with_header(mut self, name: &str, value: impl Into<String>) -> Self {
        self.headers.insert(name.to_owned(), value.into());
        self
    }

    /// A write with a body.
    #[must_use]
    pub fn post(path: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: Method::Post,
            path: path.into(),
            headers: BTreeMap::new(),
            body: body.into(),
        }
    }
}

/// One internal response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceResponse {
    /// The HTTP status.
    pub status: u16,
    /// The content type, when the service set one.
    pub content_type: Option<String>,
    /// The entity tag, when the service set one.
    pub etag: Option<String>,
    /// The body bytes; a reference or a small envelope, never a candle series.
    pub body: Vec<u8>,
}

impl ServiceResponse {
    /// A successful JSON response.
    #[must_use]
    pub fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: Some("application/json".to_owned()),
            etag: None,
            body: body.into(),
        }
    }
}

/// Why an internal call did not produce a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// The service could not be reached.
    Unreachable {
        /// Which service.
        service: Service,
        /// The transport message.
        message: String,
    },
    /// The call exceeded its deadline.
    Timeout {
        /// Which service.
        service: Service,
    },
    /// The service has no URL configured.
    Unconfigured {
        /// Which service.
        service: Service,
    },
    /// The service answered with a status the caller must handle.
    Status {
        /// Which service.
        service: Service,
        /// The HTTP status.
        status: u16,
        /// The upstream message, when it carried one.
        message: String,
    },
    /// The service answered with bytes the contract cannot decode.
    InvalidResponse {
        /// Which service.
        service: Service,
        /// What was wrong.
        message: String,
    },
    /// The configured base URL is not an absolute HTTP URL.
    InvalidBase {
        /// Which service.
        service: Service,
        /// What was wrong.
        message: String,
    },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable { service, message } => {
                write!(formatter, "{} is unreachable: {message}", service.name())
            }
            Self::Timeout { service } => {
                write!(formatter, "{} did not answer in time", service.name())
            }
            Self::Unconfigured { service } => write!(
                formatter,
                "{} is not configured; set {}",
                service.name(),
                service.url_variable()
            ),
            Self::Status {
                service,
                status,
                message,
            } => write!(formatter, "{} answered {status}: {message}", service.name()),
            Self::InvalidResponse { service, message } => {
                write!(
                    formatter,
                    "{} answered an invalid body: {message}",
                    service.name()
                )
            }
            Self::InvalidBase { service, message } => {
                write!(
                    formatter,
                    "the {} base URL is invalid: {message}",
                    service.name()
                )
            }
        }
    }
}

impl std::error::Error for ClientError {}

/// The one seam through which a service calls another.
pub trait ServiceClient: Send + Sync {
    /// Send one request to one service.
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>>;
}

/// The data service's internal reads, as the gateway and the worker use them.
///
/// The trait carries references only: a config bundle, a dataset object, and a
/// count. A fake implements it without a socket, so a route or a worker hop is
/// tested against the contract rather than a live service.
pub trait DataClient: Send + Sync {
    /// Read one config bundle, or `None` when the profile holds no such config.
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>>;

    /// Read one dataset object, or `None` when the profile holds no such dataset.
    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>>;

    /// How many datasets a profile holds.
    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>>;
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// The default deadline for one internal request.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_INTERNAL_BODY: usize = 8 * 1024 * 1024;

/// The concrete HTTP transport for the internal surface.
///
/// One client owns one base URL per service. `forward` passes a request and its
/// streaming body through unchanged; `send` collects a small internal envelope.
/// The caller owns the header allowlist and the 503 mapping.
#[derive(Clone)]
pub struct HttpServiceClient {
    bases: std::sync::Arc<BTreeMap<Service, Uri>>,
    client: Client<HttpConnector, Body>,
}

impl std::fmt::Debug for HttpServiceClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpServiceClient")
            .field("services", &self.bases.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl HttpServiceClient {
    /// Build a client for one service from an absolute base URL.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::InvalidBase`] when the URL is not an absolute
    /// HTTP or HTTPS URL.
    pub fn new(service: Service, base: &str) -> Result<Self, ClientError> {
        let base = base.trim_end_matches('/');
        let uri: Uri = base.parse().map_err(|error| ClientError::InvalidBase {
            service,
            message: format!("{base:?}: {error}"),
        })?;
        if uri.scheme().is_none() || uri.authority().is_none() {
            return Err(ClientError::InvalidBase {
                service,
                message: format!("{base:?} is not absolute"),
            });
        }
        let mut connector = HttpConnector::new();
        connector.set_connect_timeout(Some(CONNECT_TIMEOUT));
        let client = Client::builder(TokioExecutor::new()).build(connector);
        Ok(Self {
            bases: std::sync::Arc::new(BTreeMap::from([(service, uri)])),
            client,
        })
    }

    /// Build a client for one service from its `QR_*_URL` variable.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::Unconfigured`] when the variable is absent and
    /// [`ClientError::InvalidBase`] when it is not an absolute URL.
    pub fn from_env(service: Service) -> Result<Self, ClientError> {
        let value = std::env::var(service.url_variable())
            .map_err(|_| ClientError::Unconfigured { service })?;
        if value.trim().is_empty() {
            return Err(ClientError::Unconfigured { service });
        }
        Self::new(service, &value)
    }

    /// Forward one request with its streaming body, preserving status, headers,
    /// and body.
    ///
    /// # Errors
    ///
    /// Returns a transport error when the service is unreachable or does not
    /// answer headers inside the request deadline.
    pub async fn forward(
        &self,
        service: Service,
        request: Request<Body>,
    ) -> Result<Response<Body>, ClientError> {
        self.forward_with_deadline(service, request, REQUEST_TIMEOUT)
            .await
    }

    /// Forward with an explicit deadline.
    ///
    /// A streaming upload, a download, or a model compute may outlive the
    /// default read deadline; the caller names the bound for that work.
    ///
    /// # Errors
    ///
    /// Returns a transport error when the service is unreachable or does not
    /// answer headers inside `deadline`.
    pub async fn forward_with_deadline(
        &self,
        service: Service,
        request: Request<Body>,
        deadline: Duration,
    ) -> Result<Response<Body>, ClientError> {
        let (mut parts, body) = request.into_parts();
        parts.uri = self.endpoint(
            service,
            parts
                .uri
                .path_and_query()
                .map_or("/", http::uri::PathAndQuery::as_str),
        )?;
        let request = Request::from_parts(parts, body);
        let response = tokio::time::timeout(deadline, self.client.request(request))
            .await
            .map_err(|_| ClientError::Timeout { service })?
            .map_err(|error| ClientError::Unreachable {
                service,
                message: error.to_string(),
            })?;
        Ok(response.map(Body::new))
    }

    fn endpoint(&self, service: Service, path: &str) -> Result<Uri, ClientError> {
        let base = self
            .bases
            .get(&service)
            .ok_or(ClientError::Unconfigured { service })?;
        let base = base.to_string();
        let base = base.trim_end_matches('/');
        let joined = format!("{}{}", base, if path.starts_with('/') { path } else { "/" });
        joined
            .parse()
            .map_err(|error| ClientError::InvalidResponse {
                service,
                message: format!("the joined URL {joined:?} is invalid: {error}"),
            })
    }

    fn request(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> Result<Request<Body>, ClientError> {
        let uri = self.endpoint(service, &request.path)?;
        let method = match request.method {
            Method::Get => http::Method::GET,
            Method::Post => http::Method::POST,
        };
        let mut builder = Request::builder().method(method).uri(uri);
        for (name, value) in &request.headers {
            let name = header::HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
                ClientError::InvalidResponse {
                    service,
                    message: format!("a request header name is invalid: {error}"),
                }
            })?;
            let value = header::HeaderValue::from_str(value).map_err(|error| {
                ClientError::InvalidResponse {
                    service,
                    message: format!("a request header value is invalid: {error}"),
                }
            })?;
            builder = builder.header(name, value);
        }
        if !request.body.is_empty() && !request.headers.contains_key("content-type") {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
        }
        builder
            .body(Body::from(request.body))
            .map_err(|error| ClientError::InvalidResponse {
                service,
                message: error.to_string(),
            })
    }
}

impl ServiceClient for HttpServiceClient {
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>> {
        let built = self.request(service, request);
        let client = self.clone();
        Box::pin(async move {
            let request = built?;
            let response = tokio::time::timeout(REQUEST_TIMEOUT, client.client.request(request))
                .await
                .map_err(|_| ClientError::Timeout { service })?
                .map_err(|error| ClientError::Unreachable {
                    service,
                    message: error.to_string(),
                })?;
            let status = response.status().as_u16();
            let content_type = response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let etag = response
                .headers()
                .get(header::ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let body = Limited::new(response.into_body(), MAX_INTERNAL_BODY)
                .collect()
                .await
                .map_err(|error| ClientError::InvalidResponse {
                    service,
                    message: error.to_string(),
                })?
                .to_bytes()
                .to_vec();
            Ok(ServiceResponse {
                status,
                content_type,
                etag,
                body,
            })
        })
    }
}

impl HttpServiceClient {
    /// Read one service's own `/health` report.
    ///
    /// # Errors
    ///
    /// Returns the transport or status error; a missing `/health` is a status
    /// error, not a healthy default.
    pub async fn health(&self, service: Service) -> Result<HealthReport, ClientError> {
        self.get_json::<HealthReport>(service, None, "/health".to_owned())
            .await?
            .ok_or(ClientError::Status {
                service,
                status: 404,
                message: "/health is absent".to_owned(),
            })
    }

    async fn get_json<T>(
        &self,
        service: Service,
        profile: Option<&ProfileId>,
        path: String,
    ) -> Result<Option<T>, ClientError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut request = ServiceRequest::get(path);
        if let Some(profile) = profile {
            request = request.with_header(PROFILE_HEADER, profile.as_str());
        }
        let response = <Self as ServiceClient>::send(self, service, request).await?;
        match response.status {
            200 => serde_json::from_slice(&response.body)
                .map(Some)
                .map_err(|error| ClientError::InvalidResponse {
                    service,
                    message: error.to_string(),
                }),
            404 => Ok(None),
            status => Err(ClientError::Status {
                service,
                status,
                message: upstream_message(&response.body),
            }),
        }
    }

    async fn post_json<T, B>(
        &self,
        service: Service,
        path: String,
        body: &B,
    ) -> Result<T, ClientError>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let body = serde_json::to_vec(body).map_err(|error| ClientError::InvalidResponse {
            service,
            message: error.to_string(),
        })?;
        let response =
            <Self as ServiceClient>::send(self, service, ServiceRequest::post(path, body)).await?;
        match response.status {
            200 => serde_json::from_slice(&response.body).map_err(|error| {
                ClientError::InvalidResponse {
                    service,
                    message: error.to_string(),
                }
            }),
            status => Err(ClientError::Status {
                service,
                status,
                message: upstream_message(&response.body),
            }),
        }
    }
}

fn upstream_message(body: &[u8]) -> String {
    serde_json::from_slice::<quantrule_api::ErrorEnvelope>(body)
        .map(|envelope| envelope.error.message)
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned())
}

impl DataClient for HttpServiceClient {
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/configs/{config_id}"),
                )
                .await
        })
    }

    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    format!("/internal/v1/datasets/{data_version}"),
                )
                .await
        })
    }

    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            client
                .get_json(
                    Service::Data,
                    Some(&profile),
                    "/internal/v1/profile/datasets/count".to_owned(),
                )
                .await?
                .ok_or(ClientError::InvalidResponse {
                    service: Service::Data,
                    message: "the dataset count is missing".to_owned(),
                })
        })
    }
}

/// An in-memory data client, for route and worker tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryDataClient {
    configs: std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String), ConfigBundle>>>,
    datasets: std::sync::Arc<std::sync::Mutex<BTreeMap<(String, String), DatasetObject>>>,
}

impl MemoryDataClient {
    /// Add one config for a profile.
    pub fn add_config(&self, profile: &ProfileId, bundle: ConfigBundle) {
        self.configs.lock().expect("the config map").insert(
            (profile.as_str().to_owned(), bundle.config_id.clone()),
            bundle,
        );
    }

    /// Add one dataset for a profile.
    pub fn add_dataset(&self, profile: &ProfileId, object: DatasetObject) {
        self.datasets.lock().expect("the dataset map").insert(
            (profile.as_str().to_owned(), object.data_version.to_string()),
            object,
        );
    }
}

impl DataClient for MemoryDataClient {
    fn config_bundle(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigBundle>, ClientError>> {
        let configs = std::sync::Arc::clone(&self.configs);
        Box::pin(async move {
            Ok(configs
                .lock()
                .expect("the config map")
                .get(&(profile.as_str().to_owned(), config_id))
                .cloned())
        })
    }

    fn dataset_object(
        &self,
        profile: ProfileId,
        data_version: String,
    ) -> BoxFuture<Result<Option<DatasetObject>, ClientError>> {
        let datasets = std::sync::Arc::clone(&self.datasets);
        Box::pin(async move {
            Ok(datasets
                .lock()
                .expect("the dataset map")
                .get(&(profile.as_str().to_owned(), data_version))
                .cloned())
        })
    }

    fn dataset_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let datasets = std::sync::Arc::clone(&self.datasets);
        Box::pin(async move {
            Ok(datasets
                .lock()
                .expect("the dataset map")
                .keys()
                .filter(|(owner, _)| owner.as_str() == profile.as_str())
                .count() as u64)
        })
    }
}

/// An in-memory client that records calls and replays configured responses.
///
/// It is the fake the plan requires for a service test: a gateway route or a
/// worker hop is exercised without a socket, and the recorded call is the
/// assertion.
#[derive(Debug, Clone, Default)]
pub struct MemoryClient {
    responses: std::sync::Arc<
        std::sync::Mutex<std::collections::BTreeMap<(Service, String), ServiceResponse>>,
    >,
    calls: std::sync::Arc<std::sync::Mutex<Vec<(Service, ServiceRequest)>>>,
}

impl MemoryClient {
    /// An empty client.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the response for one service and path.
    pub fn respond(&self, service: Service, path: impl Into<String>, response: ServiceResponse) {
        self.responses
            .lock()
            .expect("the response lock")
            .insert((service, path.into()), response);
    }

    /// The calls this client has seen, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<(Service, ServiceRequest)> {
        self.calls.lock().expect("the call lock").clone()
    }
}

impl ServiceClient for MemoryClient {
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>> {
        let responses = std::sync::Arc::clone(&self.responses);
        let calls = std::sync::Arc::clone(&self.calls);
        Box::pin(async move {
            calls
                .lock()
                .expect("the call lock")
                .push((service, request.clone()));
            responses
                .lock()
                .expect("the response lock")
                .get(&(service, request.path))
                .cloned()
                .ok_or(ClientError::Unconfigured { service })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn a_request_reaches_the_named_service() {
        let client = MemoryClient::new();
        client.respond(
            Service::Data,
            "/internal/v1/datasets/sha256:x",
            ServiceResponse::json(200, b"{}".to_vec()),
        );
        let response = client
            .send(
                Service::Data,
                ServiceRequest::get("/internal/v1/datasets/sha256:x"),
            )
            .await
            .expect("response");
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type.as_deref(), Some("application/json"));
        assert_eq!(client.calls().len(), 1);
        assert_eq!(client.calls()[0].0, Service::Data);
        assert_eq!(client.calls()[0].1.method, Method::Get);
    }

    #[tokio::test]
    async fn a_path_on_another_service_is_not_found() {
        let client = MemoryClient::new();
        client.respond(
            Service::Evidence,
            "/runs",
            ServiceResponse::json(200, b"[]".to_vec()),
        );
        let error = client
            .send(Service::Data, ServiceRequest::get("/runs"))
            .await
            .expect_err("no data route");
        assert_eq!(
            error,
            ClientError::Unconfigured {
                service: Service::Data
            }
        );
    }

    #[test]
    fn every_service_has_one_name_and_one_variable() {
        let services = [
            Service::Data,
            Service::Model,
            Service::Evidence,
            Service::Simulation,
            Service::Backtest,
        ];
        for (index, service) in services.iter().enumerate() {
            assert!(!service.name().is_empty());
            assert!(service.url_variable().starts_with("QR_"));
            assert!(service.url_variable().ends_with("_URL"));
            assert!(
                !services[..index]
                    .iter()
                    .any(|other| other.name() == service.name())
            );
        }
    }

    #[test]
    fn a_missing_url_names_the_service_and_the_variable() {
        let message = ClientError::Unconfigured {
            service: Service::Model,
        }
        .to_string();
        assert!(message.contains("model"), "{message}");
        assert!(message.contains("QR_MODEL_URL"), "{message}");
    }
}
