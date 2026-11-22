//! Request and trace identifiers at the HTTP boundary.
//!
//! The identifiers are observability data. They travel in headers and logs and
//! are never written into an artifact, a model reading, a strategy decision or
//! a run digest. A malformed client value is replaced rather than trusted, and
//! a replacement is deterministic for one process instance and counter value.
//!
//! The gateway accepts `x-request-id` only when it matches
//! `^[A-Za-z0-9._-]{1,64}$` and a `traceparent` only when it is a valid W3C
//! `00-<32hex>-<16hex>-<2hex>` value. Each service repeats the validation, opens
//! one request span, logs one completion event with the matched route template,
//! and returns `x-request-id`.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::extract::{Request, State};
use axum::http::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

use crate::metrics::{matched_route, method_label};

/// The request identifier header.
pub const REQUEST_ID_HEADER: &str = "x-request-id";
/// The W3C trace context header.
pub const TRACEPARENT_HEADER: &str = "traceparent";
/// The maximum accepted request identifier length.
pub const MAX_REQUEST_ID_LEN: usize = 64;

/// The process-local source of deterministic request identifiers.
#[derive(Clone, Debug)]
pub struct RequestIds {
    instance: Arc<str>,
    counter: Arc<AtomicU64>,
}

impl RequestIds {
    /// Build the source from an explicit instance name.
    ///
    /// The instance is sanitized to the accepted character set and truncated so
    /// the instance, separator and counter fit in 64 characters.
    #[must_use]
    pub fn new(instance: &str) -> Self {
        Self {
            instance: Arc::from(sanitize_instance(instance)),
            counter: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Build the source from `QR_INSTANCE_ID`, the hostname, or the service name.
    ///
    /// The process id keeps two processes on one host distinct. The counter
    /// makes the minted value reproducible for one process lifetime.
    #[must_use]
    pub fn from_env(service: &str) -> Self {
        let base = std::env::var("QR_INSTANCE_ID")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .or_else(|| {
                std::env::var("HOSTNAME")
                    .ok()
                    .map(|value| value.trim().to_owned())
                    .filter(|value| !value.is_empty())
            })
            .unwrap_or_else(|| service.to_owned());
        Self::new(&format!("{base}-{}", std::process::id()))
    }

    /// The sanitized instance prefix.
    #[must_use]
    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// Mint the next request identifier.
    ///
    /// The counter starts at one and increases monotonically, so repeated calls
    /// on a fresh source are deterministic.
    #[must_use]
    pub fn next(&self) -> String {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);
        format!("{}-{counter:016x}", self.instance)
    }
}

fn sanitize_instance(instance: &str) -> String {
    let mut sanitized = String::with_capacity(instance.len().min(47));
    for character in instance.chars() {
        if sanitized.len() >= 47 {
            break;
        }
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            sanitized.push(character);
        } else {
            sanitized.push('_');
        }
    }
    if sanitized.is_empty() {
        "instance".to_owned()
    } else {
        sanitized
    }
}

/// Whether a client value is an acceptable request identifier.
#[must_use]
pub fn is_valid_request_id(value: &str) -> bool {
    (1..=MAX_REQUEST_ID_LEN).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

/// One accepted or derived trace context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// The 32-hex trace identifier.
    pub trace_id: String,
    /// The normalized `traceparent` value to forward.
    pub traceparent: String,
}

/// Parse a W3C `traceparent` value.
///
/// Only version `00` is accepted. An all-zero trace id or parent span id is
/// invalid. Hex digits may arrive in either case and are normalized to lower
/// case.
#[must_use]
pub fn parse_traceparent(value: &str) -> Option<TraceContext> {
    let value = value.trim();
    if value.len() != 55 || !value.is_ascii() {
        return None;
    }
    let bytes = value.as_bytes();
    if bytes[2] != b'-' || bytes[35] != b'-' || bytes[52] != b'-' {
        return None;
    }
    if &value[0..2] != "00" {
        return None;
    }
    let trace_id = &value[3..35];
    let span_id = &value[36..52];
    let flags = &value[53..55];
    if !is_hex(trace_id) || !is_hex(span_id) || !is_hex(flags) {
        return None;
    }
    if trace_id.bytes().all(|byte| byte == b'0') || span_id.bytes().all(|byte| byte == b'0') {
        return None;
    }
    let trace_id = trace_id.to_ascii_lowercase();
    let span_id = span_id.to_ascii_lowercase();
    let flags = flags.to_ascii_lowercase();
    Some(TraceContext {
        traceparent: format!("00-{trace_id}-{span_id}-{flags}"),
        trace_id,
    })
}

fn is_hex(value: &str) -> bool {
    value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Derive a deterministic 32-hex trace identifier from a request identifier.
#[must_use]
pub fn derive_trace_id(seed: &str) -> String {
    let first = fnv1a_64(seed.as_bytes(), 0xcbf2_9ce4_8422_2325);
    let second = fnv1a_64(seed.as_bytes(), 0x9e37_79b9_7f4a_7c15);
    let (first, second) = if first == 0 && second == 0 {
        (1, 0)
    } else {
        (first, second)
    };
    format!("{first:016x}{second:016x}")
}

/// Derive a deterministic 16-hex parent span identifier.
fn derive_span_id(seed: &str) -> String {
    let span = fnv1a_64(seed.as_bytes(), 0x517c_c1b7_2722_0a95);
    let span = if span == 0 { 1 } else { span };
    format!("{span:016x}")
}

/// Build a valid `traceparent` for a derived trace.
#[must_use]
pub fn traceparent_for(trace_id: &str, seed: &str) -> String {
    format!("00-{trace_id}-{}-01", derive_span_id(seed))
}

fn fnv1a_64(bytes: &[u8], seed: u64) -> u64 {
    let mut hash = seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Axum middleware that validates or mints the identifiers and logs completion.
pub async fn assign(State(ids): State<RequestIds>, mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_valid_request_id(value))
        .map_or_else(|| ids.next(), str::to_owned);
    let context = request
        .headers()
        .get(TRACEPARENT_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_traceparent)
        .unwrap_or_else(|| {
            let trace_id = derive_trace_id(&request_id);
            TraceContext {
                traceparent: traceparent_for(&trace_id, &request_id),
                trace_id,
            }
        });
    insert_header(request.headers_mut(), REQUEST_ID_HEADER, &request_id);
    insert_header(
        request.headers_mut(),
        TRACEPARENT_HEADER,
        &context.traceparent,
    );

    let method = method_label(request.method()).to_owned();
    let route = matched_route(&request).to_owned();
    let span = tracing::info_span!(
        "http.request",
        request_id = %request_id,
        trace_id = %context.trace_id,
        method = %method,
        route = %route,
    );
    let started = Instant::now();
    let response = next.run(request).instrument(span.clone()).await;
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::info!(
        parent: &span,
        request_id = %request_id,
        trace_id = %context.trace_id,
        method = %method,
        route = %route,
        status = response.status().as_u16(),
        duration_ms,
        "request completed"
    );
    let mut response = response;
    insert_header(response.headers_mut(), REQUEST_ID_HEADER, &request_id);
    response
}

fn insert_header(headers: &mut axum::http::HeaderMap, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        headers.insert(HeaderName::from_static(name), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    #[test]
    fn request_ids_are_validated_by_the_documented_grammar() {
        for valid in ["a", "A-Z_0.9", &"a".repeat(64)] {
            assert!(is_valid_request_id(valid), "{valid:?}");
        }
        for invalid in ["", "with space", "semi;colon", "sla/sh", &"a".repeat(65)] {
            assert!(!is_valid_request_id(invalid), "{invalid:?}");
        }
    }

    #[test]
    fn minting_is_deterministic_and_sanitized() {
        let ids = RequestIds::new("node one/../x");
        assert_eq!(ids.instance(), "node_one_.._x");
        assert_eq!(ids.next(), "node_one_.._x-0000000000000001");
        assert_eq!(ids.next(), "node_one_.._x-0000000000000002");
        let long = RequestIds::new(&"x".repeat(100));
        assert!(long.next().len() <= MAX_REQUEST_ID_LEN);
    }

    #[test]
    fn a_valid_traceparent_is_accepted_and_normalized() {
        let value = "00-4BF92F3577B34DA6A3CE929D0E0E4736-00F067AA0BA902B7-01";
        let context = parse_traceparent(value).expect("valid");
        assert_eq!(context.trace_id, "4bf92f3577b34da6a3ce929d0e0e4736");
        assert_eq!(
            context.traceparent,
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01"
        );
    }

    #[test]
    fn malformed_traceparents_are_refused() {
        for invalid in [
            "",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7",
            "01-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
            "00-00000000000000000000000000000000-00f067aa0ba902b7-01",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-0000000000000000-01",
            "00-4bf92f3577b34da6a3ce929d0e0e473g-00f067aa0ba902b7-01",
        ] {
            assert!(parse_traceparent(invalid).is_none(), "{invalid:?}");
        }
    }

    #[test]
    fn a_derived_trace_is_32_hex_and_stable() {
        let first = derive_trace_id("instance-0000000000000001");
        let second = derive_trace_id("instance-0000000000000001");
        assert_eq!(first, second);
        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, "0".repeat(32));
        let traceparent = traceparent_for(&first, "seed");
        assert!(parse_traceparent(&traceparent).is_some(), "{traceparent}");
    }

    async fn echo(headers: axum::http::HeaderMap) -> String {
        format!(
            "{}|{}",
            headers
                .get(REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or(""),
            headers
                .get(TRACEPARENT_HEADER)
                .and_then(|value| value.to_str().ok())
                .unwrap_or("")
        )
    }

    fn app(ids: RequestIds) -> Router {
        Router::new()
            .route("/echo", get(echo))
            .layer(axum::middleware::from_fn_with_state(ids, assign))
    }

    #[tokio::test]
    async fn middleware_preserves_a_valid_id_and_replaces_an_invalid_one() {
        let app = app(RequestIds::new("node"));
        let response = app
            .clone()
            .oneshot(
                HttpRequest::get("/echo")
                    .header(REQUEST_ID_HEADER, "client-request_1")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("client-request_1")
        );

        let response = app
            .oneshot(
                HttpRequest::get("/echo")
                    .header(REQUEST_ID_HEADER, "not valid")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(
            response
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|value| value.to_str().ok()),
            Some("node-0000000000000001")
        );
    }

    #[tokio::test]
    async fn middleware_propagates_the_trace_id() {
        let app = app(RequestIds::new("node"));
        let response = app
            .oneshot(
                HttpRequest::get("/echo")
                    .header(
                        TRACEPARENT_HEADER,
                        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .expect("body");
        let text = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(text.contains("4bf92f3577b34da6a3ce929d0e0e4736"), "{text}");
    }
}
