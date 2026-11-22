//! Observability wiring guard.
//!
//! The metrics listener and structured logging are started by every service
//! binary, the six HTTP services instrument their routers, and the backtest
//! worker does not gain a request middleware. Compose enables JSON logs and the
//! internal metrics bind without publishing a metrics port. This test reads the
//! checked-in sources and Compose file so a newly added service or a dropped
//! call fails the fast gate.

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the runtime manifest has two ancestors")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

const SERVICES: [(&str, &str, bool); 7] = [
    (
        "gateway",
        "crates/quantrule-gateway/src/bin/qr_gateway.rs",
        true,
    ),
    ("data", "crates/quantrule-data-service/src/main.rs", true),
    ("model", "crates/quantrule-model-service/src/main.rs", true),
    (
        "evidence",
        "crates/quantrule-evidence-service/src/main.rs",
        true,
    ),
    (
        "simulation",
        "crates/quantrule-simulation-service/src/main.rs",
        true,
    ),
    (
        "strategy",
        "crates/quantrule-strategy-service/src/main.rs",
        true,
    ),
    (
        "backtest",
        "crates/quantrule-backtest-service/src/main.rs",
        false,
    ),
];

#[test]
fn every_service_starts_metrics_and_structured_logging() {
    for (service, path, _http) in SERVICES {
        let text = read(path);
        assert!(
            text.contains(&format!("logging::init(\"{service}\"")),
            "{path} does not initialize structured logging for {service}"
        );
        let starts_listener = text.contains("metrics::start(&config, &metrics)")
            || text.contains("metrics::start_router(&config");
        assert!(
            starts_listener,
            "{path} does not start the internal metrics listener"
        );
        assert!(
            text.contains(&format!("Metrics::new(\n        \"{service}\"")),
            "{path} does not name {service} in its build info"
        );
    }
}

#[test]
fn http_services_instrument_the_router_and_backtest_does_not() {
    for (service, path, http) in SERVICES {
        let text = if service == "gateway" {
            read("crates/quantrule-gateway/src/server.rs")
        } else {
            read(path)
        };
        let instrumented = text.contains("observability::instrument(");
        assert_eq!(
            instrumented, http,
            "{path}: {service} instrumentation must be {http}"
        );
    }
}

#[test]
fn compose_enables_json_logs_and_the_internal_metrics_bind() {
    let compose = read("compose.yaml");
    assert!(
        compose.contains("QR_LOG_FORMAT: json"),
        "Compose does not enable JSON logs"
    );
    assert!(
        compose.contains("QR_METRICS_BIND: 0.0.0.0:9090"),
        "Compose does not enable the internal metrics listener"
    );
    assert!(
        !compose.contains("9090:9090"),
        "Compose publishes the metrics port"
    );
    assert!(
        !compose.contains("prometheus"),
        "Compose adds a Prometheus service; PLAT-006 owns it"
    );
}

#[test]
fn every_service_serves_a_readiness_route() {
    for (service, path) in [
        ("gateway", "crates/quantrule-gateway/src/server.rs"),
        ("data", "crates/quantrule-data-service/src/routes.rs"),
        ("model", "crates/quantrule-model-service/src/routes.rs"),
        (
            "evidence",
            "crates/quantrule-evidence-service/src/routes/mod.rs",
        ),
        (
            "simulation",
            "crates/quantrule-simulation-service/src/routes/mod.rs",
        ),
        (
            "strategy",
            "crates/quantrule-strategy-service/src/routes.rs",
        ),
        ("backtest", "crates/quantrule-backtest-service/src/main.rs"),
    ] {
        let text = read(path);
        assert!(
            text.contains(".route(\"/ready\""),
            "{path} does not serve /ready for {service}"
        );
    }
    let backtest = read("crates/quantrule-backtest-service/src/main.rs");
    assert!(
        backtest.contains(".route(\"/health\"") && backtest.contains(".route(\"/ready\""),
        "the backtest internal listener does not serve /health and /ready"
    );
}

#[test]
fn the_drain_deadlines_fit_the_stop_grace_period() {
    let compose = read("compose.yaml");
    assert!(
        compose.contains("stop_grace_period: 40s"),
        "Compose does not keep a 40 s stop grace period"
    );
    for file in [
        "deploy/multi-host/compose.core.yaml",
        "deploy/multi-host/compose.compute.yaml",
        "deploy/multi-host/compose.gateway.yaml",
    ] {
        assert!(
            read(file).contains("stop_grace_period: 40s"),
            "{file} does not keep a 40 s stop grace period"
        );
    }
    assert!(
        read("Dockerfile").contains("STOPSIGNAL SIGTERM"),
        "the image does not declare SIGTERM as its stop signal"
    );
}

#[test]
fn the_gateway_does_not_proxy_metrics() {
    let server = read("crates/quantrule-gateway/src/server.rs");
    assert!(
        !server.contains("/metrics"),
        "the gateway must not add a /metrics proxy route"
    );
}
