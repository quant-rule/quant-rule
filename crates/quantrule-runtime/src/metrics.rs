//! Deterministic runtime metrics and the internal Prometheus listener.
//!
//! The registry is deliberately small: the family names, label names and
//! histogram buckets are compile-time constants, and no caller can invent a
//! metric name. A route label comes from the axum `MatchedPath` template, never
//! from the request path, so a run id, profile id, token or object hash cannot
//! become a label.
//!
//! The listener is internal. `QR_METRICS_BIND` is unset by default, a public
//! bind is refused when `QR_INTERNAL_ONLY` is set, and the gateway never proxies
//! it. The gateway's registry holds only gateway middleware series; it never
//! re-emits an upstream family.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::{MatchedPath, Request, State};
use axum::http::{HeaderValue, Method, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::get;

use crate::config::Config;
use crate::error::RuntimeError;

/// The internal metrics path.
pub const METRICS_PATH: &str = "/metrics";
/// The Prometheus text exposition content type.
pub const METRICS_CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";
/// The route label used when no route matched.
pub const UNMATCHED_ROUTE: &str = "unmatched";
/// The fixed histogram buckets, in seconds.
pub const HISTOGRAM_BUCKETS: [f64; 11] = [
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

const BUILD_INFO: &str = "quantrule_build_info";
const PROCESS_START: &str = "quantrule_process_start_time_seconds";
const READY: &str = "quantrule_ready";
const DRAINING: &str = "quantrule_draining";
const DEPENDENCY_UP: &str = "quantrule_dependency_up";
const REQUESTS_TOTAL: &str = "quantrule_http_requests_total";
const REQUEST_DURATION: &str = "quantrule_http_request_duration_seconds";
const REQUESTS_IN_FLIGHT: &str = "quantrule_http_requests_in_flight";
const HTTP_RETRIES: &str = "quantrule_http_retries_total";
const CONSUMER_RECONNECTS: &str = "quantrule_consumer_reconnects_total";
const QUEUE_DEPTH: &str = "quantrule_queue_depth";
const WORKERS: &str = "quantrule_workers";
const RUNS_TOTAL: &str = "quantrule_runs_total";
const STAGES_TOTAL: &str = "quantrule_stages_total";
const LEADER: &str = "quantrule_leader";
const LEADER_TERM: &str = "quantrule_leader_term";
const RUNS_MISSING_DIGEST: &str = "quantrule_runs_missing_digest";
const OUTBOX_PENDING: &str = "quantrule_outbox_pending";
const AUTH_RELOAD_FAILURES: &str = "quantrule_auth_reload_failures_total";
const AUTH_TOKEN_AGE: &str = "quantrule_auth_token_age_seconds";

/// The static identity a process publishes at `/metrics`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    /// The service name, for example `gateway`.
    pub service: String,
    /// The product version.
    pub version: String,
    /// The image digest, or an empty label when the process has none.
    pub image_digest: Option<String>,
}

impl BuildInfo {
    /// Build the identity for one process.
    #[must_use]
    pub fn new(
        service: impl Into<String>,
        version: impl Into<String>,
        image_digest: Option<String>,
    ) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            image_digest,
        }
    }
}

/// A deterministic metrics registry.
///
/// Cloning is cheap and shares one state, so the middleware, the health
/// registry and the listener observe the same series.
#[derive(Clone)]
pub struct Metrics {
    inner: Arc<Inner>,
}

struct Inner {
    build: BuildInfo,
    process_start_seconds: u64,
    ready: AtomicBool,
    draining: AtomicBool,
    dependencies: Mutex<BTreeMap<String, bool>>,
    in_flight: AtomicU64,
    requests: Mutex<BTreeMap<RequestKey, u64>>,
    durations: Mutex<BTreeMap<RouteKey, Histogram>>,
    retries: Mutex<BTreeMap<(String, String), u64>>,
    reconnects: Mutex<BTreeMap<(String, String), u64>>,
    queue_depth: Mutex<BTreeMap<String, u64>>,
    workers: AtomicU64,
    runs: Mutex<BTreeMap<String, u64>>,
    stages: Mutex<BTreeMap<String, u64>>,
    leader: AtomicBool,
    leader_term: AtomicU64,
    runs_missing_digest: AtomicU64,
    outbox_pending: AtomicU64,
    auth_reload_failures: Mutex<BTreeMap<String, u64>>,
    auth_token_age_seconds: AtomicU64,
}

impl std::fmt::Debug for Metrics {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Metrics")
            .field("build", &self.inner.build)
            .field("ready", &self.inner.ready.load(Ordering::Relaxed))
            .field("draining", &self.inner.draining.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RequestKey {
    method: String,
    route: String,
    status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct RouteKey {
    method: String,
    route: String,
}

#[derive(Debug, Clone)]
struct Histogram {
    buckets: [u64; HISTOGRAM_BUCKETS.len()],
    count: u64,
    sum: f64,
}

impl Histogram {
    fn new() -> Self {
        Self {
            buckets: [0; HISTOGRAM_BUCKETS.len()],
            count: 0,
            sum: 0.0,
        }
    }

    fn observe(&mut self, seconds: f64) {
        for (index, bound) in HISTOGRAM_BUCKETS.iter().enumerate() {
            if seconds <= *bound {
                self.buckets[index] = self.buckets[index].saturating_add(1);
            }
        }
        self.count = self.count.saturating_add(1);
        self.sum += seconds;
    }
}

impl Metrics {
    /// Build an empty registry for one process.
    #[must_use]
    pub fn new(
        service: impl Into<String>,
        version: impl Into<String>,
        image_digest: Option<String>,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                build: BuildInfo::new(service, version, image_digest),
                process_start_seconds: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |duration| duration.as_secs()),
                ready: AtomicBool::new(false),
                draining: AtomicBool::new(false),
                dependencies: Mutex::new(BTreeMap::new()),
                in_flight: AtomicU64::new(0),
                requests: Mutex::new(BTreeMap::new()),
                durations: Mutex::new(BTreeMap::new()),
                retries: Mutex::new(BTreeMap::new()),
                reconnects: Mutex::new(BTreeMap::new()),
                queue_depth: Mutex::new(BTreeMap::new()),
                workers: AtomicU64::new(0),
                runs: Mutex::new(BTreeMap::new()),
                stages: Mutex::new(BTreeMap::new()),
                leader: AtomicBool::new(false),
                leader_term: AtomicU64::new(0),
                runs_missing_digest: AtomicU64::new(0),
                outbox_pending: AtomicU64::new(0),
                auth_reload_failures: Mutex::new(BTreeMap::new()),
                auth_token_age_seconds: AtomicU64::new(0),
            }),
        }
    }

    /// The process identity published in `quantrule_build_info`.
    #[must_use]
    pub fn build_info(&self) -> &BuildInfo {
        &self.inner.build
    }

    /// Set the process readiness gauge.
    pub fn set_ready(&self, ready: bool) {
        self.inner.ready.store(ready, Ordering::Relaxed);
    }

    /// Set the draining gauge. Slice 2 owns the drain transition.
    pub fn set_draining(&self, draining: bool) {
        self.inner.draining.store(draining, Ordering::Relaxed);
    }

    /// Record one dependency probe result.
    pub fn set_dependency(&self, name: impl Into<String>, up: bool) {
        lock(&self.inner.dependencies).insert(name.into(), up);
    }

    /// Count one started HTTP request.
    pub fn http_request_started(&self) {
        self.inner.in_flight.fetch_add(1, Ordering::Relaxed);
    }

    /// Count one finished HTTP request and observe its duration.
    pub fn observe_http(&self, method: &str, route: &str, status: u16, duration: Duration) {
        let key = RequestKey {
            method: method.to_owned(),
            route: route.to_owned(),
            status,
        };
        let mut requests = lock(&self.inner.requests);
        let entry = requests.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
        drop(requests);

        let route_key = RouteKey {
            method: method.to_owned(),
            route: route.to_owned(),
        };
        lock(&self.inner.durations)
            .entry(route_key)
            .or_insert_with(Histogram::new)
            .observe(duration.as_secs_f64());
    }

    /// Count one finished HTTP request without observing a duration.
    pub fn http_request_finished(&self) {
        self.inner.in_flight.fetch_sub(1, Ordering::Relaxed);
    }

    /// Count one retried internal HTTP request.
    ///
    /// `reason` is a fixed class such as `unreachable`, `timeout` or
    /// `status_503`; it is never an upstream body or a path.
    pub fn http_retry(&self, service: &str, reason: &str) {
        let key = (service.to_owned(), reason.to_owned());
        let mut retries = lock(&self.inner.retries);
        let entry = retries.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Count one consumer reconnect.
    pub fn consumer_reconnect(&self, service: &str, consumer: &str) {
        let key = (service.to_owned(), consumer.to_owned());
        let mut reconnects = lock(&self.inner.reconnects);
        let entry = reconnects.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Set the depth of one declared queue.
    pub fn set_queue_depth(&self, queue: &str, depth: u64) {
        lock(&self.inner.queue_depth).insert(queue.to_owned(), depth);
    }

    /// Set the number of workers this process runs.
    pub fn set_workers(&self, workers: u64) {
        self.inner.workers.store(workers, Ordering::Relaxed);
    }

    /// Count one run outcome.
    pub fn record_run(&self, state: &str) {
        let mut runs = lock(&self.inner.runs);
        let entry = runs.entry(state.to_owned()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Count one diagnostic stage outcome.
    pub fn record_stage(&self, state: &str) {
        let mut stages = lock(&self.inner.stages);
        let entry = stages.entry(state.to_owned()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Set whether this replica currently holds the evidence leader lease.
    pub fn set_leader(&self, leader: bool) {
        self.inner.leader.store(leader, Ordering::Relaxed);
    }

    /// Set the evidence leader term this replica currently holds; zero when it
    /// is a standby.
    pub fn set_leader_term(&self, term: u64) {
        self.inner.leader_term.store(term, Ordering::Relaxed);
    }

    /// Set the number of succeeded runs whose digest is still missing.
    pub fn set_runs_missing_digest(&self, missing: u64) {
        self.inner
            .runs_missing_digest
            .store(missing, Ordering::Relaxed);
    }

    /// Set the number of outbox notifications still pending.
    pub fn set_outbox_pending(&self, pending: u64) {
        self.inner.outbox_pending.store(pending, Ordering::Relaxed);
    }

    /// Count one failed bearer-token reload by stable reason.
    pub fn auth_reload_failure(&self, reason: &str) {
        let mut failures = lock(&self.inner.auth_reload_failures);
        let entry = failures.entry(reason.to_owned()).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    /// Set the age of the bearer token currently in use, in seconds.
    pub fn set_auth_token_age(&self, seconds: u64) {
        self.inner
            .auth_token_age_seconds
            .store(seconds, Ordering::Relaxed);
    }

    /// Encode every family in the Prometheus text 0.0.4 format.
    ///
    /// Families are written in a fixed order and every label set is sorted, so
    /// two encodes of the same state produce identical bytes.
    #[must_use]
    pub fn encode(&self) -> String {
        let mut output = String::new();
        self.write_build(&mut output);
        write_header(
            &mut output,
            PROCESS_START,
            "gauge",
            "Process start time since the Unix epoch in seconds.",
        );
        write_sample(
            &mut output,
            PROCESS_START,
            &[],
            self.inner.process_start_seconds,
        );
        write_header(
            &mut output,
            READY,
            "gauge",
            "Whether the process is ready to serve.",
        );
        write_sample(
            &mut output,
            READY,
            &[],
            u64::from(self.inner.ready.load(Ordering::Relaxed)),
        );
        write_header(
            &mut output,
            DRAINING,
            "gauge",
            "Whether the process is draining.",
        );
        write_sample(
            &mut output,
            DRAINING,
            &[],
            u64::from(self.inner.draining.load(Ordering::Relaxed)),
        );
        write_header(
            &mut output,
            DEPENDENCY_UP,
            "gauge",
            "Whether a named dependency probe passed.",
        );
        for (name, up) in lock(&self.inner.dependencies).iter() {
            write_sample(
                &mut output,
                DEPENDENCY_UP,
                &[("name", name.as_str())],
                u64::from(*up),
            );
        }
        write_header(
            &mut output,
            REQUESTS_TOTAL,
            "counter",
            "HTTP requests by normalized method, route template and status.",
        );
        for (key, count) in lock(&self.inner.requests).iter() {
            let status = key.status.to_string();
            write_sample(
                &mut output,
                REQUESTS_TOTAL,
                &[
                    ("method", key.method.as_str()),
                    ("route", key.route.as_str()),
                    ("status", status.as_str()),
                ],
                *count,
            );
        }
        self.write_durations(&mut output);
        write_header(
            &mut output,
            REQUESTS_IN_FLIGHT,
            "gauge",
            "HTTP requests currently in flight.",
        );
        write_sample(
            &mut output,
            REQUESTS_IN_FLIGHT,
            &[],
            self.inner.in_flight.load(Ordering::Relaxed),
        );
        self.write_retries(&mut output);
        self.write_reconnects(&mut output);
        write_header(
            &mut output,
            QUEUE_DEPTH,
            "gauge",
            "Messages waiting in a declared queue.",
        );
        for (queue, depth) in lock(&self.inner.queue_depth).iter() {
            write_sample(
                &mut output,
                QUEUE_DEPTH,
                &[("queue", queue.as_str())],
                *depth,
            );
        }
        write_header(&mut output, WORKERS, "gauge", "Workers this process runs.");
        write_sample(
            &mut output,
            WORKERS,
            &[("service", self.inner.build.service.as_str())],
            self.inner.workers.load(Ordering::Relaxed),
        );
        write_header(
            &mut output,
            RUNS_TOTAL,
            "counter",
            "Run outcomes recorded by the worker.",
        );
        for (state, count) in lock(&self.inner.runs).iter() {
            write_sample(
                &mut output,
                RUNS_TOTAL,
                &[("state", state.as_str())],
                *count,
            );
        }
        write_header(
            &mut output,
            STAGES_TOTAL,
            "counter",
            "Diagnostic stage outcomes recorded by the worker.",
        );
        for (state, count) in lock(&self.inner.stages).iter() {
            write_sample(
                &mut output,
                STAGES_TOTAL,
                &[("state", state.as_str())],
                *count,
            );
        }
        write_header(
            &mut output,
            LEADER,
            "gauge",
            "Whether this replica holds the evidence leader lease.",
        );
        write_sample(
            &mut output,
            LEADER,
            &[("service", self.inner.build.service.as_str())],
            u64::from(self.inner.leader.load(Ordering::Relaxed)),
        );
        write_header(
            &mut output,
            LEADER_TERM,
            "gauge",
            "The evidence leader term this replica holds; zero on a standby.",
        );
        write_sample(
            &mut output,
            LEADER_TERM,
            &[("service", self.inner.build.service.as_str())],
            self.inner.leader_term.load(Ordering::Relaxed),
        );
        write_header(
            &mut output,
            RUNS_MISSING_DIGEST,
            "gauge",
            "Succeeded runs whose canonical digest is still missing.",
        );
        write_sample(
            &mut output,
            RUNS_MISSING_DIGEST,
            &[],
            self.inner.runs_missing_digest.load(Ordering::Relaxed),
        );
        write_header(
            &mut output,
            OUTBOX_PENDING,
            "gauge",
            "Run notifications waiting to be published.",
        );
        write_sample(
            &mut output,
            OUTBOX_PENDING,
            &[],
            self.inner.outbox_pending.load(Ordering::Relaxed),
        );
        write_header(
            &mut output,
            AUTH_RELOAD_FAILURES,
            "counter",
            "Bearer-token reload failures by stable reason.",
        );
        for (reason, count) in lock(&self.inner.auth_reload_failures).iter() {
            write_sample(
                &mut output,
                AUTH_RELOAD_FAILURES,
                &[("reason", reason.as_str())],
                *count,
            );
        }
        write_header(
            &mut output,
            AUTH_TOKEN_AGE,
            "gauge",
            "Age of the bearer token currently in use, in seconds.",
        );
        write_sample(
            &mut output,
            AUTH_TOKEN_AGE,
            &[],
            self.inner.auth_token_age_seconds.load(Ordering::Relaxed),
        );
        output
    }

    fn write_retries(&self, output: &mut String) {
        write_header(
            output,
            HTTP_RETRIES,
            "counter",
            "Retried internal HTTP requests by target service and retry reason.",
        );
        for ((service, reason), count) in lock(&self.inner.retries).iter() {
            write_sample(
                output,
                HTTP_RETRIES,
                &[("reason", reason.as_str()), ("service", service.as_str())],
                *count,
            );
        }
    }

    fn write_reconnects(&self, output: &mut String) {
        write_header(
            output,
            CONSUMER_RECONNECTS,
            "counter",
            "Queue consumer reconnects by service and consumer kind.",
        );
        for ((service, consumer), count) in lock(&self.inner.reconnects).iter() {
            write_sample(
                output,
                CONSUMER_RECONNECTS,
                &[
                    ("consumer", consumer.as_str()),
                    ("service", service.as_str()),
                ],
                *count,
            );
        }
    }

    fn write_build(&self, output: &mut String) {
        write_header(
            output,
            BUILD_INFO,
            "gauge",
            "Build identity of the process, always one.",
        );
        let digest = self.inner.build.image_digest.as_deref().unwrap_or("");
        write_sample(
            output,
            BUILD_INFO,
            &[
                ("image_digest", digest),
                ("service", self.inner.build.service.as_str()),
                ("version", self.inner.build.version.as_str()),
            ],
            1,
        );
    }

    fn write_durations(&self, output: &mut String) {
        write_header(
            output,
            REQUEST_DURATION,
            "histogram",
            "HTTP request duration in seconds by normalized method and route template.",
        );
        for (key, histogram) in lock(&self.inner.durations).iter() {
            for (index, bound) in HISTOGRAM_BUCKETS.iter().enumerate() {
                let le = bound.to_string();
                let rendered = render_labels(&[
                    ("le", le.as_str()),
                    ("method", key.method.as_str()),
                    ("route", key.route.as_str()),
                ]);
                let _ = writeln!(
                    output,
                    "{REQUEST_DURATION}_bucket{rendered} {}",
                    histogram.buckets[index]
                );
            }
            let rendered = render_labels(&[
                ("le", "+Inf"),
                ("method", key.method.as_str()),
                ("route", key.route.as_str()),
            ]);
            let _ = writeln!(
                output,
                "{REQUEST_DURATION}_bucket{rendered} {}",
                histogram.count
            );
            let rendered = render_labels(&[
                ("method", key.method.as_str()),
                ("route", key.route.as_str()),
            ]);
            let _ = writeln!(output, "{REQUEST_DURATION}_sum{rendered} {}", histogram.sum);
            let _ = writeln!(
                output,
                "{REQUEST_DURATION}_count{rendered} {}",
                histogram.count
            );
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write_header(output: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(output, "# HELP {name} {help}");
    let _ = writeln!(output, "# TYPE {name} {kind}");
}

fn write_sample(output: &mut String, name: &str, labels: &[(&str, &str)], value: u64) {
    let rendered = render_labels(labels);
    let _ = writeln!(output, "{name}{rendered} {value}");
}

fn render_labels(labels: &[(&str, &str)]) -> String {
    if labels.is_empty() {
        return String::new();
    }
    let mut sorted = labels.to_vec();
    sorted.sort_by(|left, right| left.0.cmp(right.0));
    let mut rendered = String::from("{");
    for (index, (name, value)) in sorted.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        let _ = write!(rendered, "{name}=\"{}\"", escape_label(value));
    }
    rendered.push('}');
    rendered
}

fn escape_label(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Normalize an HTTP method into a bounded label.
#[must_use]
pub fn method_label(method: &Method) -> &'static str {
    match *method {
        Method::GET => "GET",
        Method::POST => "POST",
        Method::PUT => "PUT",
        Method::DELETE => "DELETE",
        Method::PATCH => "PATCH",
        Method::HEAD => "HEAD",
        Method::OPTIONS => "OPTIONS",
        Method::TRACE => "TRACE",
        _ => "OTHER",
    }
}

/// The matched route template, or the fixed unmatched label.
#[must_use]
pub fn matched_route(request: &Request) -> &str {
    request
        .extensions()
        .get::<MatchedPath>()
        .map_or(UNMATCHED_ROUTE, MatchedPath::as_str)
}

/// Axum middleware that counts one request and observes its duration.
pub async fn track_http(State(metrics): State<Metrics>, request: Request, next: Next) -> Response {
    let method = method_label(request.method()).to_owned();
    let route = matched_route(&request).to_owned();
    metrics.http_request_started();
    let _in_flight = InFlightGuard(metrics.clone());
    let started = Instant::now();
    let response = next.run(request).await;
    metrics.observe_http(
        &method,
        &route,
        response.status().as_u16(),
        started.elapsed(),
    );
    response
}

struct InFlightGuard(Metrics);

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.0.http_request_finished();
    }
}

/// The internal `/metrics` router.
pub fn router(metrics: Metrics) -> Router {
    Router::new()
        .route(METRICS_PATH, get(render))
        .with_state(metrics)
}

async fn render(State(metrics): State<Metrics>) -> Response {
    let mut response = metrics.encode().into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(METRICS_CONTENT_TYPE),
    );
    response
}

/// Bind and start the internal listener when `QR_METRICS_BIND` is set.
///
/// The caller keeps the process alive; the listener task stops with it. A bind
/// failure is a startup error, because a configured listener that silently
/// fails would hide the metrics the operator asked for.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the configured address is invalid for
/// the internal profile or cannot be bound.
pub async fn start(config: &Config, metrics: &Metrics) -> Result<Option<SocketAddr>, RuntimeError> {
    start_router(config, router(metrics.clone())).await
}

/// Bind and start the internal listener with an explicit router.
///
/// The backtest worker composes `/metrics` with its own `/health` and `/ready`
/// through this seam. The caller keeps the process alive; the listener task
/// stops with it.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the configured address is invalid for
/// the internal profile or cannot be bound.
pub async fn start_router(
    config: &Config,
    router: Router,
) -> Result<Option<SocketAddr>, RuntimeError> {
    let Some(bind) = config.metrics_bind else {
        return Ok(None);
    };
    config
        .check_metrics_bind()
        .map_err(|error| RuntimeError::Serve(error.to_string()))?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|error| RuntimeError::Serve(format!("bind metrics {bind}: {error}")))?;
    let address = listener
        .local_addr()
        .map_err(|error| RuntimeError::Serve(format!("metrics address: {error}")))?;
    tokio::spawn(async move {
        if let Err(error) = axum::serve(listener, router).await {
            tracing::error!(%error, "metrics listener stopped");
        }
    });
    Ok(Some(address))
}

#[cfg(test)]
mod tests;
