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
        assert!(
            text.contains("metrics::start(&config, &metrics)"),
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
fn the_gateway_does_not_proxy_metrics() {
    let server = read("crates/quantrule-gateway/src/server.rs");
    assert!(
        !server.contains("/metrics"),
        "the gateway must not add a /metrics proxy route"
    );
}
