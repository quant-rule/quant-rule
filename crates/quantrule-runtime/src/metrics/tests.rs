//! Unit tests for the deterministic metrics registry and listener.

use super::*;
use std::collections::BTreeMap;

use axum::body::Body;
use axum::http::Request as HttpRequest;
use axum::routing::get;
use tower::ServiceExt;

fn metrics() -> Metrics {
    Metrics::new("data", "1.5.2", Some("sha256:abc".to_owned()))
}

#[test]
fn encoding_is_stable_and_families_are_ordered() {
    let metrics = metrics();
    metrics.set_ready(true);
    metrics.set_dependency("queue", true);
    metrics.set_dependency("postgres", false);
    metrics.http_request_started();
    metrics.observe_http("GET", "/health", 200, Duration::from_millis(12));
    metrics.http_request_finished();
    metrics.observe_http("POST", "/catalog/datasets", 201, Duration::from_millis(600));

    let first = metrics.encode();
    let second = metrics.encode();
    assert_eq!(first, second);
    let families = [
        BUILD_INFO,
        PROCESS_START,
        READY,
        DRAINING,
        DEPENDENCY_UP,
        REQUESTS_TOTAL,
        REQUEST_DURATION,
        REQUESTS_IN_FLIGHT,
        HTTP_RETRIES,
        CONSUMER_RECONNECTS,
        QUEUE_DEPTH,
        WORKERS,
        RUNS_TOTAL,
        STAGES_TOTAL,
        LEADER,
        LEADER_TERM,
        RUNS_MISSING_DIGEST,
        OUTBOX_PENDING,
        AUTH_RELOAD_FAILURES,
        AUTH_TOKEN_AGE,
    ];
    let mut last = 0;
    for family in families {
        let position = first.find(&format!("# TYPE {family} ")).expect(family);
        assert!(position >= last, "{family} is out of order");
        last = position;
    }
    assert!(first.contains(BUILD_INFO));
    assert!(first.contains("service=\"data\""));
    assert!(first.contains("version=\"1.5.2\""));
    assert!(first.contains("image_digest=\"sha256:abc\""));
    assert!(first.contains("quantrule_ready 1"));
    assert!(first.contains("quantrule_http_requests_in_flight 0"));
}

#[test]
fn labels_are_sorted_and_escaped() {
    let metrics = metrics();
    metrics.set_dependency("a\"b\\c\nd", true);
    metrics.observe_http("GET", "/runs/{id}", 200, Duration::from_millis(1));
    let text = metrics.encode();
    let expected = format!("name=\"{}\"}} 1", escape_label("a\"b\\c\nd"));
    assert!(
        text.contains(&expected),
        "expected {expected:?}, output {text:?}"
    );
    assert!(
        text.contains("{method=\"GET\",route=\"/runs/{id}\",status=\"200\"} 1"),
        "sorted request labels missing: {text}"
    );
    assert!(
        text.contains("_bucket{le=\"0.005\",method=\"GET\",route=\"/runs/{id}\"} 1"),
        "sorted histogram labels missing: {text}"
    );
}

#[test]
fn histogram_buckets_are_cumulative() {
    let metrics = metrics();
    for duration in [0.001, 0.02, 1.5] {
        metrics.observe_http("GET", "/health", 200, Duration::from_secs_f64(duration));
    }
    let text = metrics.encode();
    assert!(
        text.contains("_bucket{le=\"0.005\",method=\"GET\",route=\"/health\"} 1"),
        "{text}"
    );
    assert!(
        text.contains("_bucket{le=\"0.01\",method=\"GET\",route=\"/health\"} 1"),
        "{text}"
    );
    assert!(
        text.contains("_bucket{le=\"0.025\",method=\"GET\",route=\"/health\"} 2"),
        "{text}"
    );
    assert!(
        text.contains("_bucket{le=\"1\",method=\"GET\",route=\"/health\"} 2"),
        "{text}"
    );
    assert!(
        text.contains("_bucket{le=\"2.5\",method=\"GET\",route=\"/health\"} 3"),
        "{text}"
    );
    assert!(
        text.contains("_bucket{le=\"+Inf\",method=\"GET\",route=\"/health\"} 3"),
        "{text}"
    );
    assert!(
        text.contains("_count{method=\"GET\",route=\"/health\"} 3"),
        "{text}"
    );
}

#[tokio::test]
async fn the_metrics_endpoint_sets_the_content_type_and_refuses_other_methods() {
    let app = router(metrics());
    let response = app
        .clone()
        .oneshot(
            HttpRequest::get(METRICS_PATH)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some(METRICS_CONTENT_TYPE)
    );
    let response = app
        .oneshot(
            HttpRequest::post(METRICS_PATH)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(
        response.status(),
        axum::http::StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn a_disabled_listener_binds_nothing() {
    let config = Config::default();
    assert_eq!(config.metrics_bind, None);
    let started = start(&config, &metrics()).await.expect("disabled");
    assert_eq!(started, None);
}

#[test]
fn a_public_metrics_bind_is_refused_when_internal_only() {
    let values = BTreeMap::from([
        ("QR_INTERNAL_ONLY".to_owned(), "1".to_owned()),
        ("QR_BIND".to_owned(), "10.0.0.5:8080".to_owned()),
        ("QR_METRICS_BIND".to_owned(), "0.0.0.0:9090".to_owned()),
    ]);
    let error = Config::from_map(&values).expect_err("public metrics bind");
    assert!(error.to_string().contains("QR_METRICS_BIND"), "{error}");
}

#[tokio::test]
async fn a_loopback_listener_starts() {
    let config = Config {
        metrics_bind: Some("127.0.0.1:0".parse().expect("address")),
        ..Config::default()
    };
    let metrics = metrics();
    let address = start(&config, &metrics)
        .await
        .expect("listener")
        .expect("configured");
    assert_eq!(
        address.ip(),
        std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
    );
    assert!(metrics.encode().contains("quantrule_ready"));
}

#[test]
fn worker_and_retry_families_are_deterministic() {
    let metrics = metrics();
    metrics.http_retry("data", "unreachable");
    metrics.http_retry("data", "unreachable");
    metrics.consumer_reconnect("backtest", "run");
    metrics.set_queue_depth("run", 3);
    metrics.set_queue_depth("stage", 1);
    metrics.set_workers(2);
    metrics.record_run("computed");
    metrics.record_run("failed");
    metrics.record_stage("computed");
    metrics.set_leader(true);
    metrics.set_leader_term(7);
    metrics.set_runs_missing_digest(2);
    metrics.set_outbox_pending(4);
    let first = metrics.encode();
    assert_eq!(first, metrics.encode());
    assert!(
        first.contains("quantrule_http_retries_total{reason=\"unreachable\",service=\"data\"} 2"),
        "{first}"
    );
    assert!(
        first.contains(
            "quantrule_consumer_reconnects_total{consumer=\"run\",service=\"backtest\"} 1"
        ),
        "{first}"
    );
    assert!(
        first.contains("quantrule_queue_depth{queue=\"run\"} 3"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_workers{service=\"data\"} 2"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_runs_total{state=\"computed\"} 1"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_stages_total{state=\"computed\"} 1"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_leader{service=\"data\"} 1"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_leader_term{service=\"data\"} 7"),
        "{first}"
    );
    assert!(first.contains("quantrule_runs_missing_digest 2"), "{first}");
    assert!(first.contains("quantrule_outbox_pending 4"), "{first}");
}

#[test]
fn auth_reload_families_are_deterministic() {
    let metrics = metrics();
    metrics.auth_reload_failure("read");
    metrics.auth_reload_failure("read");
    metrics.auth_reload_failure("short");
    metrics.set_auth_token_age(90);
    let first = metrics.encode();
    assert_eq!(first, metrics.encode());
    assert!(
        first.contains("quantrule_auth_reload_failures_total{reason=\"read\"} 2"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_auth_reload_failures_total{reason=\"short\"} 1"),
        "{first}"
    );
    assert!(
        first.contains("quantrule_auth_token_age_seconds 90"),
        "{first}"
    );
}

#[tokio::test]
async fn middleware_labels_the_route_template_not_the_raw_path() {
    let metrics = metrics();
    let app = Router::new()
        .route("/runs/{id}", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn_with_state(
            metrics.clone(),
            track_http,
        ));
    let response = app
        .oneshot(
            HttpRequest::get("/runs/secret-run-id")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let text = metrics.encode();
    assert!(text.contains("route=\"/runs/{id}\""), "{text}");
    assert!(!text.contains("secret-run-id"), "{text}");
    assert!(
        text.contains("{method=\"GET\",route=\"/runs/{id}\",status=\"200\"} 1"),
        "{text}"
    );
}
