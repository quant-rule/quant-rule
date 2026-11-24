//! Observability tier rows of a v3 check.
//!
//! The rows read the same rendered text the operator copies: the four
//! observability units and the per-host agents, the Prometheus scrape and rule
//! files, the Alertmanager receiver, Loki/promtail, blackbox, Grafana
//! provisioning and the four dashboards, the HAProxy/Patroni/MinIO metric
//! exposure, and the observability manifest. No secret value is read: the
//! Patroni password is a mounted file and the manifest lists names and modes
//! only.
//!
//! The rule rows live in [`alerts`], the scrape rows in [`prometheus`], and the
//! Loki/promtail/blackbox/Grafana/manifest rows in [`config`]. This module owns
//! placement, units, and the cross-cutting port and socket boundaries.

use std::collections::BTreeSet;
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::render::infra;
use crate::render::observability::{
    ALERTMANAGER_CONFIG_TARGET, BLACKBOX_CONFIG_TARGET, GRAFANA_DASHBOARDS_TARGET,
    GRAFANA_PROVISIONING_TARGET, LOKI_CONFIG_TARGET, PATRONI_PASSWORD_TARGET,
    PROMTAIL_CONFIG_TARGET,
};

use super::report::{CheckReport, CheckRow};
use super::stateful::read_text;
use super::text::service_block;

mod alerts;
mod config;
mod prometheus;
#[cfg(test)]
mod tests;

/// Append every observability row from the rendered tree or the in-memory render.
pub(crate) fn append(report: &mut CheckReport, descriptor: &Descriptor, out: Option<&Path>) {
    for row in rows(descriptor, out) {
        report.push(row);
    }
}

/// The observability rows in check order.
pub(crate) fn rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let mut rows = vec![placement_row(descriptor)];
    rows.push(unit_row(
        descriptor,
        out,
        "prometheus",
        &descriptor.observability.prometheus.host,
        &prometheus_unit_requirements(),
        &[],
    ));
    rows.push(unit_row(
        descriptor,
        out,
        "alertmanager",
        &descriptor.observability.alertmanager.host,
        &alertmanager_unit_requirements(),
        &[],
    ));
    rows.push(unit_row(
        descriptor,
        out,
        "loki",
        &descriptor.observability.loki.host,
        &loki_unit_requirements(),
        &[],
    ));
    rows.push(unit_row(
        descriptor,
        out,
        "grafana",
        &descriptor.observability.grafana.host,
        &grafana_unit_requirements(),
        &["GF_SECURITY_ADMIN_PASSWORD"],
    ));
    rows.push(agents_row(descriptor, out));
    rows.extend(prometheus::rows(descriptor, out));
    rows.extend(alerts::rule_rows(out));
    rows.extend(alerts::alertmanager_rows(out));
    rows.push(config::loki_row(out));
    rows.push(config::promtail_row(descriptor, out));
    rows.push(config::blackbox_row(descriptor, out));
    rows.extend(config::grafana_rows(descriptor, out));
    rows.extend(config::manifest_rows(descriptor, out));
    rows.push(config::stack_row(descriptor, out));
    rows.push(config::traefik_row());
    rows.extend(config::haproxy_rows(descriptor, out));
    rows.push(socket_row(descriptor, out));
    rows.push(ports_row(descriptor, out));
    rows
}

fn placement_row(descriptor: &Descriptor) -> CheckRow {
    let prometheus = &descriptor.observability.prometheus.host;
    let alertmanager = &descriptor.observability.alertmanager.host;
    let loki = &descriptor.observability.loki.host;
    let grafana = &descriptor.observability.grafana.host;
    let mut issues = Vec::new();
    for (unit, host) in [
        ("prometheus", prometheus),
        ("alertmanager", alertmanager),
        ("loki", loki),
        ("grafana", grafana),
    ] {
        if !descriptor.hosts.contains_key(host) {
            issues.push(format!("{unit} names undeclared host {host}"));
        }
    }
    if prometheus == alertmanager {
        issues.push("Prometheus and Alertmanager share a host".to_owned());
    }
    let hosts: BTreeSet<&str> = [prometheus, alertmanager, loki, grafana]
        .into_iter()
        .map(String::as_str)
        .collect();
    if hosts.len() == 1 {
        issues.push("all four observability units share one host".to_owned());
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "placement",
            "spread",
            format!(
                "Prometheus {prometheus}, Alertmanager {alertmanager}, Loki {loki}, Grafana {grafana}; Prometheus/Alertmanager distinct; not all four on one host"
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "placement",
            "configuration",
            issues.join("; "),
        )
    }
}

fn prometheus_unit_requirements() -> Vec<String> {
    vec![
        "@sha256:".to_owned(),
        "--config.file=/etc/prometheus/prometheus.yml".to_owned(),
        "--storage.tsdb.path=/prometheus".to_owned(),
        "--storage.tsdb.retention.time=30d".to_owned(),
        "--storage.tsdb.retention.size=20GB".to_owned(),
        "--web.listen-address=0.0.0.0:9090".to_owned(),
        "source: \"/etc/quantrule/prometheus/prometheus.yml\"".to_owned(),
        "target: \"/etc/prometheus/prometheus.yml\"".to_owned(),
        "source: \"/etc/quantrule/prometheus/alert-rules.yml\"".to_owned(),
        "target: \"/etc/prometheus/alert-rules.yml\"".to_owned(),
        "source: \"/etc/quantrule/pki/ca.crt\"".to_owned(),
        "target: \"/etc/quantrule/pki/ca.crt\"".to_owned(),
        "source: \"/etc/quantrule/secrets/patroni_rest_password\"".to_owned(),
        format!("target: \"{PATRONI_PASSWORD_TARGET}\""),
        "source: \"/srv/quantrule/prometheus/data\"".to_owned(),
        "target: \"/prometheus\"".to_owned(),
        "healthcheck:".to_owned(),
        "logging:".to_owned(),
        "driver: \"json-file\"".to_owned(),
        "max-size: \"50m\"".to_owned(),
        "max-file: \"5\"".to_owned(),
    ]
}

fn alertmanager_unit_requirements() -> Vec<String> {
    vec![
        "@sha256:".to_owned(),
        "--config.file=/etc/alertmanager/alertmanager.yml".to_owned(),
        "--storage.path=/alertmanager".to_owned(),
        "--data.retention=120h".to_owned(),
        "--web.listen-address=0.0.0.0:9093".to_owned(),
        "source: \"/etc/quantrule/alertmanager/alertmanager.yml\"".to_owned(),
        format!("target: \"{ALERTMANAGER_CONFIG_TARGET}\""),
        "source: \"/srv/quantrule/alertmanager/data\"".to_owned(),
        "target: \"/alertmanager\"".to_owned(),
        "healthcheck:".to_owned(),
        "logging:".to_owned(),
        "driver: \"json-file\"".to_owned(),
        "max-size: \"50m\"".to_owned(),
        "max-file: \"5\"".to_owned(),
    ]
}

fn loki_unit_requirements() -> Vec<String> {
    vec![
        "@sha256:".to_owned(),
        "-config.file=/etc/loki/loki.yml".to_owned(),
        "source: \"/etc/quantrule/loki/loki.yml\"".to_owned(),
        format!("target: \"{LOKI_CONFIG_TARGET}\""),
        "source: \"/srv/quantrule/loki/data\"".to_owned(),
        "target: \"/loki\"".to_owned(),
        "healthcheck:".to_owned(),
        "logging:".to_owned(),
        "driver: \"json-file\"".to_owned(),
        "max-size: \"50m\"".to_owned(),
        "max-file: \"5\"".to_owned(),
    ]
}

fn grafana_unit_requirements() -> Vec<String> {
    vec![
        "@sha256:".to_owned(),
        "GF_AUTH_ANONYMOUS_ENABLED: \"true\"".to_owned(),
        "GF_AUTH_ANONYMOUS_ORG_ROLE: \"Viewer\"".to_owned(),
        "GF_AUTH_BASIC_ENABLED: \"false\"".to_owned(),
        "GF_AUTH_DISABLE_LOGIN_FORM: \"true\"".to_owned(),
        "GF_USERS_ALLOW_SIGN_UP: \"false\"".to_owned(),
        "source: \"/etc/quantrule/grafana/provisioning\"".to_owned(),
        format!("target: \"{GRAFANA_PROVISIONING_TARGET}\""),
        "source: \"/etc/quantrule/grafana/dashboards\"".to_owned(),
        format!("target: \"{GRAFANA_DASHBOARDS_TARGET}\""),
        "source: \"/srv/quantrule/grafana/data\"".to_owned(),
        "target: \"/var/lib/grafana\"".to_owned(),
        "healthcheck:".to_owned(),
        "logging:".to_owned(),
        "driver: \"json-file\"".to_owned(),
        "max-size: \"50m\"".to_owned(),
        "max-file: \"5\"".to_owned(),
    ]
}

fn unit_row(
    descriptor: &Descriptor,
    out: Option<&Path>,
    name: &str,
    host: &str,
    expected: &[String],
    forbidden: &[&str],
) -> CheckRow {
    match unit_block(descriptor, out, host, name) {
        Err(error) => {
            CheckRow::failed("observability", format!("{name} unit"), "unreadable", error)
        }
        Ok(block) => {
            let mut issues = missing(&block, expected);
            for needle in forbidden {
                if block.contains(needle) {
                    issues.push(format!("the {name} unit contains {needle}"));
                }
            }
            if block.contains("ports:") {
                issues.push(format!("the {name} unit publishes a port"));
            }
            if issues.is_empty() {
                CheckRow::ok(
                    "observability",
                    format!("{name} unit"),
                    "pinned",
                    format!(
                        "on {host}; image pinned by digest; command, config mounts, data volume, 50m x 5 JSON logging, health check; no published port"
                    ),
                )
            } else {
                CheckRow::failed(
                    "observability",
                    format!("{name} unit"),
                    "configuration",
                    issues.join("; "),
                )
            }
        }
    }
}

fn agents_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    for host in descriptor.hosts.keys() {
        let exporter = format!("node-exporter-{host}");
        match unit_block(descriptor, out, host, &exporter) {
            Ok(block) => {
                issues.extend(missing(
                    &block,
                    &[
                        "@sha256:".to_owned(),
                        "--path.rootfs=/host".to_owned(),
                        "--collector.textfile.directory=/var/lib/node_exporter/textfile".to_owned(),
                        "source: \"/\"".to_owned(),
                        "target: \"/host\"".to_owned(),
                        "propagation: \"rslave\"".to_owned(),
                        "source: \"/srv/quantrule/node_exporter/textfile\"".to_owned(),
                        "healthcheck:".to_owned(),
                        "logging:".to_owned(),
                    ],
                ));
                if block.contains("ports:") {
                    issues.push(format!("node-exporter-{host} publishes a port"));
                }
            }
            Err(error) => issues.push(error),
        }
        let promtail = format!("promtail-{host}");
        match unit_block(descriptor, out, host, &promtail) {
            Ok(block) => {
                issues.extend(missing(
                    &block,
                    &[
                        "@sha256:".to_owned(),
                        format!("target: \"{PROMTAIL_CONFIG_TARGET}\""),
                        "source: \"/var/run/docker.sock\"".to_owned(),
                        "target: \"/var/run/docker.sock\"".to_owned(),
                        "source: \"/srv/quantrule/promtail\"".to_owned(),
                        "healthcheck:".to_owned(),
                        "logging:".to_owned(),
                    ],
                ));
                if block.contains("ports:") {
                    issues.push(format!("promtail-{host} publishes a port"));
                }
            }
            Err(error) => issues.push(error),
        }
    }
    let prometheus_host = &descriptor.observability.prometheus.host;
    match unit_block(descriptor, out, prometheus_host, "blackbox") {
        Ok(block) => {
            issues.extend(missing(
                &block,
                &[
                    "@sha256:".to_owned(),
                    format!("--config.file={BLACKBOX_CONFIG_TARGET}"),
                    "source: \"/etc/quantrule/blackbox/blackbox.yml\"".to_owned(),
                    format!("target: \"{BLACKBOX_CONFIG_TARGET}\""),
                    "source: \"/etc/quantrule/pki/ca.crt\"".to_owned(),
                    "healthcheck:".to_owned(),
                    "logging:".to_owned(),
                ],
            ));
            if block.contains("ports:") {
                issues.push("blackbox publishes a port".to_owned());
            }
        }
        Err(error) => issues.push(error),
    }
    for host in descriptor.hosts.keys() {
        if host == prometheus_host {
            continue;
        }
        if unit_text(descriptor, out, host)
            .map(|text| text.contains("\n  blackbox:\n"))
            .unwrap_or(false)
        {
            issues.push(format!("blackbox runs on {host}, not the Prometheus host"));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "agents",
            "per-host",
            format!(
                "node_exporter and promtail on all {} hosts; blackbox on {prometheus_host}; every unit pinned and log-rotated; no published port",
                descriptor.hosts.len()
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "agents",
            "configuration",
            issues.join("; "),
        )
    }
}

fn socket_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    for (name, host) in [
        ("prometheus", &descriptor.observability.prometheus.host),
        ("alertmanager", &descriptor.observability.alertmanager.host),
        ("loki", &descriptor.observability.loki.host),
        ("grafana", &descriptor.observability.grafana.host),
        ("blackbox", &descriptor.observability.prometheus.host),
    ] {
        if let Ok(block) = unit_block(descriptor, out, host, name)
            && block.contains("/var/run/docker.sock")
        {
            issues.push(format!("the {name} unit mounts the Docker socket"));
        }
    }
    for host in descriptor.hosts.keys() {
        match unit_block(descriptor, out, host, &format!("promtail-{host}")) {
            Ok(block) => {
                if !block.contains("source: \"/var/run/docker.sock\"") {
                    issues.push(format!("promtail-{host} has no Docker socket"));
                } else if !block
                    .contains("read_only: true\n        source: \"/var/run/docker.sock\"")
                {
                    issues.push(format!("promtail-{host} mounts the socket writable"));
                }
            }
            Err(error) => issues.push(error),
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "docker socket",
            "read-only",
            "only promtail reads the Docker socket, read-only; no other observability unit mounts it",
        )
    } else {
        CheckRow::failed(
            "observability",
            "docker socket",
            "configuration",
            issues.join("; "),
        )
    }
}

fn ports_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    for host in descriptor.hosts.keys() {
        let relative = format!("infra/{host}.yaml");
        let text = match rendered(out, &relative, || {
            infra::unit_yaml(descriptor, host).ok_or_else(|| format!("no unit for {host}"))
        }) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        if text.contains("ports:") {
            issues.push(format!("infra/{host}.yaml publishes a port"));
        }
        if text.contains("published:") {
            issues.push(format!("infra/{host}.yaml has a published port"));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "ports",
            "overlay-only",
            format!(
                "{} host unit file(s); no observability or infra unit publishes a port",
                descriptor.hosts.len()
            ),
        )
    } else {
        CheckRow::failed("observability", "ports", "overlay-only", issues.join("; "))
    }
}

fn unit_text(descriptor: &Descriptor, out: Option<&Path>, host: &str) -> Result<String, String> {
    match out {
        Some(directory) => read_text(
            &directory.join("infra").join(format!("{host}.yaml")),
            "host unit",
        ),
        None => infra::unit_yaml(descriptor, host)
            .ok_or_else(|| format!("the renderer produced no unit for {host}")),
    }
}

fn unit_block(
    descriptor: &Descriptor,
    out: Option<&Path>,
    host: &str,
    service: &str,
) -> Result<String, String> {
    let text = unit_text(descriptor, out, host)?;
    let block = service_block(&text, service);
    if block.is_empty() {
        Err(format!("infra/{host}.yaml lacks the {service} unit"))
    } else {
        Ok(block.to_owned())
    }
}

fn rendered(
    out: Option<&Path>,
    relative: &str,
    render: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    match out {
        Some(directory) => read_text(&directory.join(relative), relative),
        None => render(),
    }
}

fn missing(text: &str, expected: &[String]) -> Vec<String> {
    expected
        .iter()
        .filter(|needle| !text.contains(needle.as_str()))
        .map(|needle| format!("missing {needle:?}"))
        .collect()
}

fn valid_row(row: &str) -> bool {
    (1..=17).any(|index| row == format!("F{index:02}"))
}
