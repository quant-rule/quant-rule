//! HTTP transport for a source adapter.
//!
//! The adapter builds a [`GetRequest`] and the transport performs it. Keeping
//! the transport behind a trait lets fixture tests exercise pagination, retry,
//! and parsing without a network. The only production implementation uses
//! `ureq`, which is synchronous and does not pull in an async runtime.

use std::fmt;
use std::time::Duration;

use ureq::Agent;

/// The largest response body the adapter accepts.
///
/// A full 30,000-row page is roughly 2 MB; the limit leaves room for a
/// provider-side formatting change while refusing an unbounded response.
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

/// The whole-request timeout for one page.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// One HTTP GET request issued by a source adapter.
#[derive(Debug, Clone)]
pub(crate) struct GetRequest {
    /// The endpoint URL.
    pub(crate) url: &'static str,
    /// Query parameters in a fixed order.
    pub(crate) query: Vec<(&'static str, String)>,
    /// Request headers.
    pub(crate) headers: Vec<(&'static str, &'static str)>,
}

/// A provider response, including non-success statuses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HttpResponse {
    /// The HTTP status code.
    pub(crate) status: u16,
    /// The response body decoded as UTF-8.
    pub(crate) body: String,
}

/// A transport could not complete a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TransportError {
    /// The request itself failed, such as a connection, TLS, or timeout error.
    Request(String),
    /// The response body could not be read as UTF-8.
    Body(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(message) => write!(formatter, "HTTP request failed: {message}"),
            Self::Body(message) => write!(formatter, "cannot read response body: {message}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// A synchronous HTTP GET transport.
pub(crate) trait Transport {
    /// Performs one GET request.
    ///
    /// A non-success HTTP status is a successful transport call: the caller
    /// decides whether that status is retryable.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError`] when the request cannot be completed or the
    /// response body cannot be read.
    fn get(&self, request: &GetRequest) -> Result<HttpResponse, TransportError>;
}

/// The production transport, backed by `ureq`.
#[derive(Clone)]
pub(crate) struct HttpTransport {
    agent: Agent,
}

impl HttpTransport {
    /// Builds a transport with a fixed timeout and bounded response bodies.
    pub(crate) fn new() -> Self {
        let config = Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(false)
            .build();

        Self {
            agent: Agent::new_with_config(config),
        }
    }
}

impl Default for HttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl Transport for HttpTransport {
    fn get(&self, request: &GetRequest) -> Result<HttpResponse, TransportError> {
        let mut pending = self.agent.get(request.url);
        for (key, value) in &request.query {
            pending = pending.query(*key, value);
        }
        for (key, value) in &request.headers {
            pending = pending.header(*key, *value);
        }

        let mut response = pending
            .call()
            .map_err(|error| TransportError::Request(error.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_BODY_BYTES)
            .read_to_string()
            .map_err(|error| TransportError::Body(error.to_string()))?;

        Ok(HttpResponse { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpResponse, TransportError};

    #[test]
    fn response_keeps_status_and_body() {
        let response = HttpResponse {
            status: 429,
            body: String::from("slow down"),
        };

        assert_eq!(response.status, 429);
        assert_eq!(response.body, "slow down");
    }

    #[test]
    fn transport_errors_have_stable_messages() {
        assert_eq!(
            TransportError::Request(String::from("timed out")).to_string(),
            "HTTP request failed: timed out"
        );
        assert_eq!(
            TransportError::Body(String::from("invalid UTF-8")).to_string(),
            "cannot read response body: invalid UTF-8"
        );
    }
}
