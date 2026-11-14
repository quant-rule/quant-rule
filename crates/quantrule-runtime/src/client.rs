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

use crate::adapters::BoxFuture;

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
            body: Vec::new(),
        }
    }

    /// A write with a body.
    #[must_use]
    pub fn post(path: impl Into<String>, body: impl Into<Vec<u8>>) -> Self {
        Self {
            method: Method::Post,
            path: path.into(),
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
