//! Prometheus scrape rows.
//!
//! The rows parse the rendered `prometheus.yml` by scrape job: the app DNS SD,
//! the ingress, the stateful endpoints, the per-host agents, blackbox, and the
//! self-scrape. Authentication is asserted by file path, never by value.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::render::observability::{
    self, ALERTMANAGER_PORT, BLACKBOX_PORT, ETCD_METRICS_PORT, GRAFANA_PORT, HAPROXY_METRICS_PORT,
    LOKI_PORT, NODE_EXPORTER_PORT, OBSERVABILITY_CA_TARGET, PATRONI_PASSWORD_TARGET,
    PATRONI_REST_PORT, PROMETHEUS_PORT, PROMTAIL_PORT, RABBITMQ_METRICS_PORT, TRAEFIK_METRICS_PORT,
};

use super::super::report::CheckRow;
use super::{missing, rendered};

/// The scrape rows in check order.
pub(super) fn rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let text = match rendered(out, "prometheus.yml", || {
        Ok(observability::prometheus_config(descriptor))
    }) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed(
                "observability",
                "prometheus.yml",
                "unreadable",
                error,
            )];
        }
    };
    let jobs = scrape_jobs(&text);
    let mut rows = vec![scrape_row(descriptor, &jobs)];
    rows.push(auth_row(&jobs));
    rows.push(overlay_row(descriptor, &text));
    rows
}

fn scrape_row(descriptor: &Descriptor, jobs: &BTreeMap<String, String>) -> CheckRow {
    let required = [
        "app",
        "ingress",
        "patroni",
        "etcd",
        "haproxy",
        "rabbitmq",
        "minio",
        "node",
        "promtail",
        "blackbox",
        "prometheus",
        "alertmanager",
        "loki",
        "grafana",
    ];
    let mut issues: Vec<String> = required
        .iter()
        .filter(|job| !jobs.contains_key(**job))
        .map(|job| format!("missing scrape job {job}"))
        .collect();
    for (job, expected) in app_job()
        .into_iter()
        .chain(ingress_job())
        .chain(stateful_jobs(descriptor))
        .chain(self_jobs())
    {
        if let Some(block) = jobs.get(job) {
            issues.extend(missing(block, &expected));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "prometheus.yml",
            "scrape",
            format!(
                "{} jobs: app DNS SD over tasks.<service>:9100, ingress 8082, Patroni 8008, etcd 2381, HAProxy 8404, RabbitMQ 15692, MinIO 9000, node 9100, promtail 9080, blackbox, self, Alertmanager, Loki, Grafana",
                jobs.len()
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "prometheus.yml",
            "scrape",
            issues.join("; "),
        )
    }
}

fn auth_row(jobs: &BTreeMap<String, String>) -> CheckRow {
    let mut issues = Vec::new();
    if let Some(patroni) = jobs.get("patroni") {
        issues.extend(missing(
            patroni,
            &[
                "basic_auth:".to_owned(),
                "username: \"patroni\"".to_owned(),
                format!("password_file: \"{PATRONI_PASSWORD_TARGET}\""),
            ],
        ));
        if patroni.contains("password: \"") {
            issues.push("the Patroni job renders a password value".to_owned());
        }
    }
    if let Some(minio) = jobs.get("minio") {
        issues.extend(missing(
            minio,
            &[
                "scheme: \"https\"".to_owned(),
                "metrics_path: \"/minio/v2/metrics/cluster\"".to_owned(),
                format!("ca_file: \"{OBSERVABILITY_CA_TARGET}\""),
            ],
        ));
    }
    if let Some(blackbox) = jobs.get("blackbox") {
        issues.extend(missing(
            blackbox,
            &[
                "metrics_path: \"/probe\"".to_owned(),
                "\"https_2xx\"".to_owned(),
                "check: \"health\"".to_owned(),
                "check: \"ready\"".to_owned(),
                format!("blackbox:{BLACKBOX_PORT}"),
            ],
        ));
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "prometheus.yml",
            "auth-and-tls",
            "Patroni basic_auth password_file; MinIO HTTPS with the private CA; blackbox health/ready through the CA",
        )
    } else {
        CheckRow::failed(
            "observability",
            "prometheus.yml",
            "auth-and-tls",
            issues.join("; "),
        )
    }
}

fn overlay_row(descriptor: &Descriptor, text: &str) -> CheckRow {
    let addresses: BTreeSet<&str> = descriptor
        .hosts
        .values()
        .map(|host| host.address.as_str())
        .collect();
    let underlay: Vec<&str> = addresses
        .iter()
        .copied()
        .filter(|address| text.contains(*address))
        .collect();
    if underlay.is_empty() {
        CheckRow::ok(
            "observability",
            "prometheus.yml",
            "overlay-only",
            "every target is an overlay service name or alias; no host underlay address",
        )
    } else {
        CheckRow::failed(
            "observability",
            "prometheus.yml",
            "overlay-only",
            format!(
                "underlay addresses in the scrape config: {}",
                underlay.join(", ")
            ),
        )
    }
}

fn app_job() -> Vec<(&'static str, Vec<String>)> {
    let mut expected = vec![
        format!("port: {NODE_EXPORTER_PORT}"),
        "type: \"A\"".to_owned(),
        "refresh_interval: \"30s\"".to_owned(),
        "target_label: \"service\"".to_owned(),
    ];
    for service in [
        "gateway",
        "data",
        "model",
        "strategy",
        "backtest",
        "evidence",
        "simulation",
    ] {
        expected.push(format!("\"tasks.{service}\""));
    }
    vec![("app", expected)]
}

fn ingress_job() -> Vec<(&'static str, Vec<String>)> {
    vec![(
        "ingress",
        vec![
            "\"tasks.ingress\"".to_owned(),
            format!("port: {TRAEFIK_METRICS_PORT}"),
        ],
    )]
}

fn stateful_jobs(descriptor: &Descriptor) -> Vec<(&'static str, Vec<String>)> {
    vec![
        (
            "patroni",
            descriptor
                .stateful
                .postgres
                .nodes
                .iter()
                .map(|host| format!("patroni-{host}:{PATRONI_REST_PORT}"))
                .chain(["metrics_path: \"/metrics\"".to_owned()])
                .collect(),
        ),
        (
            "etcd",
            descriptor
                .stateful
                .etcd
                .nodes
                .iter()
                .map(|host| format!("etcd-{host}:{ETCD_METRICS_PORT}"))
                .chain(["metrics_path: \"/metrics\"".to_owned()])
                .collect(),
        ),
        (
            "haproxy",
            descriptor
                .stateful
                .haproxy
                .nodes
                .iter()
                .map(|host| format!("haproxy-{host}:{HAPROXY_METRICS_PORT}"))
                .chain(["metrics_path: \"/metrics\"".to_owned()])
                .collect(),
        ),
        (
            "rabbitmq",
            descriptor
                .stateful
                .rabbitmq
                .nodes
                .iter()
                .map(|host| format!("rabbitmq-{host}:{RABBITMQ_METRICS_PORT}"))
                .chain(["metrics_path: \"/metrics\"".to_owned()])
                .collect(),
        ),
        (
            "minio",
            descriptor
                .stateful
                .minio
                .nodes
                .iter()
                .map(|host| format!("minio-{host}:9000"))
                .collect(),
        ),
        (
            "node",
            descriptor
                .hosts
                .keys()
                .map(|host| format!("node-{host}:{NODE_EXPORTER_PORT}"))
                .collect(),
        ),
        (
            "promtail",
            descriptor
                .hosts
                .keys()
                .map(|host| format!("promtail-{host}:{PROMTAIL_PORT}"))
                .collect(),
        ),
    ]
}

fn self_jobs() -> Vec<(&'static str, Vec<String>)> {
    vec![
        ("prometheus", vec![format!("localhost:{PROMETHEUS_PORT}")]),
        (
            "alertmanager",
            vec![format!("alertmanager:{ALERTMANAGER_PORT}")],
        ),
        ("loki", vec![format!("loki:{LOKI_PORT}")]),
        ("grafana", vec![format!("grafana:{GRAFANA_PORT}")]),
    ]
}

fn scrape_jobs(text: &str) -> BTreeMap<String, String> {
    let Some(start) = text.find("scrape_configs:") else {
        return BTreeMap::new();
    };
    let mut jobs = BTreeMap::new();
    for item in text[start..].split("\n  -").skip(1) {
        let name = item.lines().find_map(|line| {
            line.trim()
                .strip_prefix("job_name: \"")
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        });
        if let Some(name) = name {
            jobs.insert(name, item.to_owned());
        }
    }
    jobs
}
