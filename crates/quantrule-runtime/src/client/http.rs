//! The concrete internal HTTP transport.
//!
//! One client owns one base URL per service. `forward` passes a request and its
//! streaming body through unchanged; `send` collects a small internal envelope.
//! The caller owns the header allowlist and the 503 mapping. The default path is
//! one attempt; [`HttpServiceClient::with_retry`] opts a client into the bounded
//! policy for audited idempotent calls only.

use std::collections::BTreeMap;
use std::time::Duration;

use axum::body::Body;
use http::{Request, Response, Uri, header};
use http_body_util::{BodyExt, Limited};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use quantrule_api::{HealthReport, PROFILE_HEADER, ProfileId};

use super::{ClientError, Method, Service, ServiceClient, ServiceRequest, ServiceResponse};
use crate::adapters::BoxFuture;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// The default deadline for one internal request.
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_INTERNAL_BODY: usize = 8 * 1024 * 1024;

/// A transport failure before a response status exists.
type TransportFuture = BoxFuture<Result<Response<Body>, String>>;

/// The socket seam, so the retry policy is tested without a live service.
trait Transport: Send + Sync {
    fn send(&self, request: Request<Body>) -> TransportFuture;
}

struct HyperTransport {
    client: Client<HttpConnector, Body>,
}

impl Transport for HyperTransport {
    fn send(&self, request: Request<Body>) -> TransportFuture {
        let client = self.client.clone();
        Box::pin(async move {
            client
                .request(request)
                .await
                .map(|response| response.map(Body::new))
                .map_err(|error| error.to_string())
        })
    }
}

/// The retry settings one client applies to audited idempotent calls.
#[derive(Debug, Clone)]
struct RetryContext {
    policy: crate::retry::Policy,
    shutdown: crate::shutdown::Shutdown,
    metrics: crate::metrics::Metrics,
}

/// The concrete HTTP transport for the internal surface.
///
/// One client owns one base URL per service. `forward` passes a request and its
/// streaming body through unchanged; `send` collects a small internal envelope.
/// The caller owns the header allowlist and the 503 mapping. The default path is
/// one attempt; [`Self::with_retry`] opts a client into the bounded policy for
/// audited idempotent calls only.
#[derive(Clone)]
pub struct HttpServiceClient {
    bases: std::sync::Arc<BTreeMap<Service, Uri>>,
    transport: std::sync::Arc<dyn Transport>,
    retry: Option<RetryContext>,
}

impl std::fmt::Debug for HttpServiceClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpServiceClient")
            .field("services", &self.bases.keys().collect::<Vec<_>>())
            .field("retry", &self.retry.is_some())
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
            transport: std::sync::Arc::new(HyperTransport { client }),
            retry: None,
        })
    }

    /// Opt this client into the bounded retry policy for audited idempotent
    /// calls.
    ///
    /// The GET reads and the two dataset-reference writes opt in. A claim, a
    /// model compute, a signal create, a stage create, a decision, a cancel and
    /// every streaming body stay on the single-attempt path.
    #[must_use]
    pub fn with_retry(
        mut self,
        metrics: crate::metrics::Metrics,
        shutdown: crate::shutdown::Shutdown,
    ) -> Self {
        self.retry = Some(RetryContext {
            policy: crate::retry::Policy::default(),
            shutdown,
            metrics,
        });
        self
    }

    /// Opt into retries with an explicit policy, for a test.
    #[cfg(test)]
    fn with_policy(
        mut self,
        policy: crate::retry::Policy,
        metrics: crate::metrics::Metrics,
        shutdown: crate::shutdown::Shutdown,
    ) -> Self {
        self.retry = Some(RetryContext {
            policy,
            shutdown,
            metrics,
        });
        self
    }

    #[cfg(test)]
    fn with_transport(mut self, transport: std::sync::Arc<dyn Transport>) -> Self {
        self.transport = transport;
        self
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
        tokio::time::timeout(deadline, self.transport.send(request))
            .await
            .map_err(|_| ClientError::Timeout { service })?
            .map_err(|message| ClientError::Unreachable { service, message })
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

impl HttpServiceClient {
    /// Send exactly one attempt. This is the default path and the only path a
    /// non-idempotent call may use.
    async fn send_once(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> Result<ServiceResponse, ClientError> {
        let request = self.request(service, request)?;
        let response = tokio::time::timeout(REQUEST_TIMEOUT, self.transport.send(request))
            .await
            .map_err(|_| ClientError::Timeout { service })?
            .map_err(|message| ClientError::Unreachable { service, message })?;
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
    }

    /// Send an audited idempotent call under the configured policy.
    async fn send_retryable(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> Result<ServiceResponse, ClientError> {
        let Some(context) = self.retry.clone() else {
            return self.send_once(service, request).await;
        };
        let client = self.clone();
        let attempt_request = request.clone();
        crate::retry::execute(
            context.policy,
            service,
            context.shutdown,
            Some(context.metrics),
            move || {
                let client = client.clone();
                let request = attempt_request.clone();
                async move { client.send_once(service, request).await }
            },
        )
        .await
        .map_err(|error| match error {
            crate::retry::RetryError::Shutdown => ClientError::Interrupted { service },
            crate::retry::RetryError::Failed(error) => error,
        })
    }
}

impl ServiceClient for HttpServiceClient {
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>> {
        let client = self.clone();
        Box::pin(async move { client.send_once(service, request).await })
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

    /// Read one service's `/ready` report and HTTP status.
    ///
    /// `503` is a valid readiness answer, not a transport error: the caller
    /// needs the report to say which dependency is down or that the service is
    /// draining. Every other non-`200` status is a status error.
    ///
    /// # Errors
    ///
    /// Returns the transport or status error, and an invalid body as
    /// [`ClientError::InvalidResponse`].
    pub async fn ready(&self, service: Service) -> Result<(u16, HealthReport), ClientError> {
        // A readiness answer is sampled, not retried: a deliberate `503` must be
        // reported now, not after a backoff.
        let response = self
            .send_once(service, ServiceRequest::get("/ready"))
            .await?;
        match response.status {
            200 | 503 => {
                let report = serde_json::from_slice(&response.body).map_err(|error| {
                    ClientError::InvalidResponse {
                        service,
                        message: error.to_string(),
                    }
                })?;
                Ok((response.status, report))
            }
            status => Err(ClientError::Status {
                service,
                status,
                message: upstream_message(&response.body),
            }),
        }
    }

    pub(crate) async fn get_json<T>(
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
        let response = self.send_retryable(service, request).await?;
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

    /// Read one raw internal body, or `None` when the service answers `404`.
    pub(crate) async fn get_bytes(
        &self,
        service: Service,
        profile: Option<&ProfileId>,
        path: String,
    ) -> Result<Option<Vec<u8>>, ClientError> {
        let mut request = ServiceRequest::get(path);
        if let Some(profile) = profile {
            request = request.with_header(PROFILE_HEADER, profile.as_str());
        }
        let response = self.send_retryable(service, request).await?;
        match response.status {
            200 => Ok(Some(response.body)),
            404 => Ok(None),
            status => Err(ClientError::Status {
                service,
                status,
                message: upstream_message(&response.body),
            }),
        }
    }

    pub(crate) async fn post_json<T, B>(
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

    pub(crate) async fn post_json_for<T, B>(
        &self,
        service: Service,
        profile: &ProfileId,
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
        let request =
            ServiceRequest::post(path, body).with_header(PROFILE_HEADER, profile.as_str());
        let response = <Self as ServiceClient>::send(self, service, request).await?;
        match response.status {
            // A profile-scoped create may answer 201 for the first writer and
            // 200 for an idempotent duplicate; the body carries `created`.
            200 | 201 => serde_json::from_slice(&response.body).map_err(|error| {
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

    /// The audited idempotent write path.
    ///
    /// Only a write whose contract states that a repeat is a no-op may call
    /// this. The two dataset-reference calls qualify; a claim, a compute, a
    /// signal create, a stage create and a decision do not.
    pub(crate) async fn post_json_for_retryable<T, B>(
        &self,
        service: Service,
        profile: &ProfileId,
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
        let request =
            ServiceRequest::post(path, body).with_header(PROFILE_HEADER, profile.as_str());
        let response = self.send_retryable(service, request).await?;
        match response.status {
            200 | 201 => serde_json::from_slice(&response.body).map_err(|error| {
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

pub(crate) fn upstream_message(body: &[u8]) -> String {
    serde_json::from_slice::<quantrule_api::ErrorEnvelope>(body)
        .map(|envelope| envelope.error.message)
        .unwrap_or_else(|_| String::from_utf8_lossy(body).into_owned())
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A transport that answers a scripted status sequence and counts calls.
    struct ScriptedTransport {
        calls: std::sync::Arc<AtomicUsize>,
        statuses: Mutex<VecDeque<u16>>,
        fail: bool,
    }

    impl ScriptedTransport {
        fn new(statuses: impl IntoIterator<Item = u16>) -> Self {
            Self {
                calls: std::sync::Arc::new(AtomicUsize::new(0)),
                statuses: Mutex::new(statuses.into_iter().collect()),
                fail: false,
            }
        }

        fn unavailable() -> Self {
            Self {
                calls: std::sync::Arc::new(AtomicUsize::new(0)),
                statuses: Mutex::new(VecDeque::new()),
                fail: true,
            }
        }

        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl Transport for ScriptedTransport {
        fn send(&self, _request: Request<Body>) -> TransportFuture {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let status = self
                .statuses
                .lock()
                .expect("statuses")
                .pop_front()
                .unwrap_or(200);
            let fail = self.fail;
            Box::pin(async move {
                if fail {
                    return Err("connection refused".to_owned());
                }
                Response::builder()
                    .status(status)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .map_err(|error| error.to_string())
            })
        }
    }

    fn retrying_client(
        transport: std::sync::Arc<ScriptedTransport>,
        metrics: crate::metrics::Metrics,
        shutdown: crate::shutdown::Shutdown,
    ) -> HttpServiceClient {
        HttpServiceClient::new(Service::Data, "http://data:8080")
            .expect("client")
            .with_transport(transport)
            .with_policy(crate::retry::Policy::default(), metrics, shutdown)
    }

    #[tokio::test]
    async fn the_default_send_is_one_attempt() {
        let transport = std::sync::Arc::new(ScriptedTransport::new([503]));
        let client = HttpServiceClient::new(Service::Data, "http://data:8080")
            .expect("client")
            .with_transport(std::sync::Arc::clone(&transport) as std::sync::Arc<dyn Transport>);
        let response = client
            .send(Service::Data, ServiceRequest::get("/x"))
            .await
            .expect("response");
        assert_eq!(response.status, 503);
        assert_eq!(transport.calls(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_get_retries_but_a_post_and_a_stream_do_not() {
        let transport = std::sync::Arc::new(ScriptedTransport::new([503, 503, 200]));
        let metrics = crate::metrics::Metrics::new("data", "1.5.2", None);
        let (_, shutdown) = crate::shutdown::Shutdown::channel();
        let client = retrying_client(std::sync::Arc::clone(&transport), metrics.clone(), shutdown);

        let value: serde_json::Value = client
            .get_json(Service::Data, None, "/x".to_owned())
            .await
            .expect("read")
            .expect("body");
        assert_eq!(value, serde_json::json!({}));
        assert_eq!(transport.calls(), 3);

        // A POST keeps the single-attempt path even on a retrying client.
        *transport.statuses.lock().expect("statuses") = VecDeque::from([503, 503, 200]);
        transport.calls.store(0, Ordering::SeqCst);
        let error = client
            .post_json::<serde_json::Value, _>(
                Service::Data,
                "/write".to_owned(),
                &serde_json::json!({}),
            )
            .await
            .expect_err("the write must not retry");
        assert!(matches!(error, ClientError::Status { status: 503, .. }));
        assert_eq!(transport.calls(), 1);

        // A forwarded streaming body keeps the single-attempt path too.
        *transport.statuses.lock().expect("statuses") = VecDeque::from([503, 200]);
        transport.calls.store(0, Ordering::SeqCst);
        let request = Request::builder()
            .method("POST")
            .uri("/stream")
            .body(Body::from("bytes"))
            .expect("request");
        let response = client
            .forward(Service::Data, request)
            .await
            .expect("the stream response is passed through");
        assert_eq!(response.status(), 503);
        assert_eq!(transport.calls(), 1);

        let text = metrics.encode();
        assert!(
            text.contains("quantrule_http_retries_total{reason=\"status_503\",service=\"data\"} 2"),
            "{text}"
        );
    }

    #[tokio::test(start_paused = true)]
    async fn an_unreachable_read_retries_and_a_drain_stops_it() {
        let transport = std::sync::Arc::new(ScriptedTransport::unavailable());
        let metrics = crate::metrics::Metrics::new("data", "1.5.2", None);
        let (trigger, shutdown) = crate::shutdown::Shutdown::channel();
        let client = retrying_client(std::sync::Arc::clone(&transport), metrics.clone(), shutdown);
        trigger.trigger();
        let error = client
            .get_json::<serde_json::Value>(Service::Data, None, "/x".to_owned())
            .await
            .expect_err("a drain stops the read");
        assert!(matches!(error, ClientError::Interrupted { .. }));
        assert_eq!(transport.calls(), 0);

        let (_, shutdown) = crate::shutdown::Shutdown::channel();
        let client = retrying_client(transport.clone(), metrics, shutdown);
        let error = client
            .get_json::<serde_json::Value>(Service::Data, None, "/x".to_owned())
            .await
            .expect_err("the retry budget is spent");
        assert!(matches!(error, ClientError::Unreachable { .. }));
        assert_eq!(transport.calls(), 3);
    }
}
