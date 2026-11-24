//! Loki, promtail, blackbox, Grafana, manifest, and metric-exposure rows.
//!
//! The rows read the rendered text; the Grafana dashboards are parsed with
//! `serde_json` so a malformed checked-in dashboard fails the check rather than
//! reaching a browser.

use std::collections::BTreeSet;
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::render::observability::{
    self, DASHBOARDS, GRAFANA_DASHBOARDS_TARGET, LOKI_PORT, OBSERVABILITY_CA_TARGET,
    PROMETHEUS_PORT,
};
use crate::render::{ingress, stateful, swarm};

use super::super::report::CheckRow;
use super::{missing, rendered, unit_block};

/// The Loki retention row.
pub(super) fn loki_row(out: Option<&Path>) -> CheckRow {
    let text = match rendered(out, "loki.yml", || Ok(observability::loki_config())) {
        Ok(text) => text,
        Err(error) => {
            return CheckRow::failed("observability", "loki.yml", "unreadable", error);
        }
    };
    let issues = missing(
        &text,
        &[
            "retention_period: \"720h\"".to_owned(),
            "retention_enabled: true".to_owned(),
            "delete_request_store: \"filesystem\"".to_owned(),
            "chunks_directory: \"/loki/chunks\"".to_owned(),
            "store: \"tsdb\"".to_owned(),
            "object_store: \"filesystem\"".to_owned(),
        ],
    );
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "loki.yml",
            "retention",
            "30-day retention with compactor deletion on the filesystem store",
        )
    } else {
        CheckRow::failed("observability", "loki.yml", "retention", issues.join("; "))
    }
}

/// The per-host promtail label-whitelist row.
pub(super) fn promtail_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    for host in descriptor.hosts.keys() {
        let relative = format!("promtail-{host}.yml");
        let text = match rendered(out, &relative, || {
            observability::promtail_config(descriptor, host).map_err(|error| error.to_string())
        }) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        issues.extend(missing(
            &text,
            &[
                "unix:///var/run/docker.sock".to_owned(),
                "http://loki:3100/loki/api/v1/push".to_owned(),
                "docker_sd_configs:".to_owned(),
                "pipeline_stages:".to_owned(),
                "docker: {}".to_owned(),
                "action: \"labeldrop\"".to_owned(),
                "__meta_docker_.*".to_owned(),
                "target_label: \"container\"".to_owned(),
                "target_label: \"service\"".to_owned(),
                "target_label: \"host\"".to_owned(),
                format!("replacement: \"{host}\""),
            ],
        ));
        for line in text.lines() {
            let Some(label) = line
                .trim()
                .strip_prefix("target_label: \"")
                .and_then(|value| value.strip_suffix('"'))
            else {
                continue;
            };
            if !matches!(label, "container" | "service" | "host" | "job" | "stream") {
                issues.push(format!("{host} forwards the label {label}"));
            }
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "promtail",
            "whitelist",
            format!(
                "{} host config(s); Docker JSON with only container, service, host, job, and stream",
                descriptor.hosts.len()
            ),
        )
    } else {
        CheckRow::failed("observability", "promtail", "whitelist", issues.join("; "))
    }
}

/// The blackbox module row.
pub(super) fn blackbox_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let text = match rendered(out, "blackbox.yml", || {
        Ok(observability::blackbox_config(descriptor))
    }) {
        Ok(text) => text,
        Err(error) => {
            return CheckRow::failed("observability", "blackbox.yml", "unreadable", error);
        }
    };
    let issues = missing(
        &text,
        &[
            "https_2xx".to_owned(),
            "prober: \"http\"".to_owned(),
            "preferred_ip_protocol: \"ip4\"".to_owned(),
            "timeout: \"5s\"".to_owned(),
            format!("ca_file: \"{OBSERVABILITY_CA_TARGET}\""),
            format!("server_name: \"{}\"", descriptor.ingress.dns),
            "valid_status_codes:".to_owned(),
        ],
    );
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "blackbox.yml",
            "probes",
            format!(
                "https_2xx; /health and /ready through the private CA and server name {}",
                descriptor.ingress.dns
            ),
        )
    } else {
        CheckRow::failed("observability", "blackbox.yml", "probes", issues.join("; "))
    }
}

/// The Grafana datasource, provider, dashboard, and access rows.
pub(super) fn grafana_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    let datasources = match rendered(
        out,
        "grafana/provisioning/datasources/datasources.yml",
        || Ok(observability::grafana_datasources()),
    ) {
        Ok(text) => text,
        Err(error) => {
            rows.push(CheckRow::failed(
                "observability",
                "grafana datasources",
                "unreadable",
                error,
            ));
            return rows;
        }
    };
    let mut issues = missing(
        &datasources,
        &[
            "uid: \"qr-prometheus\"".to_owned(),
            "uid: \"qr-loki\"".to_owned(),
            format!("url: \"http://prometheus:{PROMETHEUS_PORT}\""),
            format!("url: \"http://loki:{LOKI_PORT}\""),
            "editable: false".to_owned(),
        ],
    );
    if datasources.matches("uid: \"").count() != 2 {
        issues.push("expected exactly two fixed datasource UIDs".to_owned());
    }
    rows.push(if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "grafana datasources",
            "fixed-uids",
            "two fixed UIDs qr-prometheus and qr-loki; both read-only",
        )
    } else {
        CheckRow::failed(
            "observability",
            "grafana datasources",
            "fixed-uids",
            issues.join("; "),
        )
    });

    let provider = match rendered(
        out,
        "grafana/provisioning/dashboards/dashboards.yml",
        || Ok(observability::grafana_provider()),
    ) {
        Ok(text) => text,
        Err(error) => {
            rows.push(CheckRow::failed(
                "observability",
                "grafana provider",
                "unreadable",
                error,
            ));
            return rows;
        }
    };
    let provider_issues = missing(
        &provider,
        &[
            "allowUiUpdates: false".to_owned(),
            "disableDeletion: true".to_owned(),
            format!("path: \"{GRAFANA_DASHBOARDS_TARGET}\""),
        ],
    );
    rows.push(if provider_issues.is_empty() {
        CheckRow::ok(
            "observability",
            "grafana provider",
            "read-only",
            "disableDeletion true; allowUiUpdates false; the dashboard directory is fixed",
        )
    } else {
        CheckRow::failed(
            "observability",
            "grafana provider",
            "read-only",
            provider_issues.join("; "),
        )
    });

    rows.push(dashboard_row(out));

    let block = match unit_block(
        descriptor,
        out,
        &descriptor.observability.grafana.host,
        "grafana",
    ) {
        Ok(block) => block,
        Err(error) => {
            rows.push(CheckRow::failed(
                "observability",
                "grafana access",
                "unreadable",
                error,
            ));
            return rows;
        }
    };
    let mut access = missing(
        &block,
        &[
            "GF_AUTH_ANONYMOUS_ENABLED: \"true\"".to_owned(),
            "GF_AUTH_ANONYMOUS_ORG_ROLE: \"Viewer\"".to_owned(),
            "GF_AUTH_BASIC_ENABLED: \"false\"".to_owned(),
            "GF_AUTH_DISABLE_LOGIN_FORM: \"true\"".to_owned(),
        ],
    );
    for forbidden in [
        "GF_SECURITY_ADMIN_PASSWORD",
        "admin_password",
        "user: \"admin\"",
    ] {
        if block.contains(forbidden) {
            access.push(format!("the Grafana unit contains {forbidden}"));
        }
    }
    rows.push(if access.is_empty() {
        CheckRow::ok(
            "observability",
            "grafana access",
            "anonymous-viewer",
            "anonymous Viewer; basic auth and the login form are disabled; no default admin secret",
        )
    } else {
        CheckRow::failed(
            "observability",
            "grafana access",
            "configuration",
            access.join("; "),
        )
    });
    rows
}

fn dashboard_row(out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    let mut found_uids = BTreeSet::new();
    for (name, uid) in DASHBOARDS {
        let relative = format!("grafana/dashboards/{name}");
        let text = match rendered(out, &relative, || {
            observability::dashboard(name)
                .map(str::to_owned)
                .ok_or_else(|| format!("the checked-in dashboard {name} is missing"))
        }) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        let value: serde_json::Value = match serde_json::from_str(&text) {
            Ok(value) => value,
            Err(error) => {
                issues.push(format!("{name} is not JSON: {error}"));
                continue;
            }
        };
        if value["uid"].as_str() != Some(uid) {
            issues.push(format!("{name} has UID {:?}, expected {uid}", value["uid"]));
        }
        if value["editable"] != serde_json::Value::Bool(false) {
            issues.push(format!("{name} is not read-only"));
        }
        if value["title"].as_str().is_none_or(str::is_empty) {
            issues.push(format!("{name} has no title"));
        }
        let mut uids = BTreeSet::new();
        datasource_uids(&value, &mut uids);
        for datasource in uids {
            if !matches!(datasource.as_str(), "qr-prometheus" | "qr-loki") {
                issues.push(format!(
                    "{name} references unknown datasource UID {datasource}"
                ));
            } else {
                found_uids.insert(datasource);
            }
        }
    }
    if found_uids.is_empty() {
        issues.push("no dashboard references a provisioned datasource".to_owned());
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "grafana dashboards",
            "four",
            format!(
                "{} JSON dashboards with UIDs {}; every datasource UID exists",
                DASHBOARDS.len(),
                DASHBOARDS
                    .iter()
                    .map(|(_, uid)| *uid)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "grafana dashboards",
            "configuration",
            issues.join("; "),
        )
    }
}

fn datasource_uids(value: &serde_json::Value, uids: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Object(map) => {
            if map.contains_key("type")
                && let Some(uid) = map.get("uid").and_then(serde_json::Value::as_str)
            {
                uids.insert(uid.to_owned());
            }
            for child in map.values() {
                datasource_uids(child, uids);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                datasource_uids(child, uids);
            }
        }
        _ => {}
    }
}

/// The manifest sources, modes, and no-secret row.
pub(super) fn manifest_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let text = match rendered(out, "observability-manifest.json", || {
        observability::observability_manifest(descriptor).map_err(|error| error.to_string())
    }) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed(
                "observability",
                "manifest",
                "unreadable",
                error,
            )];
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return vec![CheckRow::failed(
                "observability",
                "manifest",
                "invalid",
                format!("observability-manifest.json is not JSON: {error}"),
            )];
        }
    };
    let files = value["files"].as_array().cloned().unwrap_or_default();
    let sources: BTreeSet<&str> = files
        .iter()
        .filter_map(|entry| entry["source"].as_str())
        .collect();
    let mut expected: Vec<String> = [
        "prometheus.yml",
        "alert-rules.yml",
        "alertmanager.yml",
        "loki.yml",
        "blackbox.yml",
        "grafana/provisioning/datasources/datasources.yml",
        "grafana/provisioning/dashboards/dashboards.yml",
        "pki/database-ca.crt",
        "secrets/patroni_rest_password",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    expected.extend(
        DASHBOARDS
            .iter()
            .map(|(name, _)| format!("grafana/dashboards/{name}")),
    );
    expected.extend(
        descriptor
            .hosts
            .keys()
            .map(|host| format!("promtail-{host}.yml")),
    );
    let missing_sources: Vec<&str> = expected
        .iter()
        .filter(|source| !sources.contains(source.as_str()))
        .map(String::as_str)
        .collect();
    let mut mode_issues = Vec::new();
    for entry in &files {
        let source = entry["source"].as_str().unwrap_or_default();
        let mode = entry["mode"].as_str().unwrap_or_default();
        let expected_mode = if source.starts_with("secrets/") {
            "600"
        } else {
            "644"
        };
        if mode != expected_mode {
            mode_issues.push(format!("{source} is mode {mode}; expected {expected_mode}"));
        }
    }
    let mut rows = vec![if missing_sources.is_empty() && mode_issues.is_empty() {
        CheckRow::ok(
            "observability",
            "manifest",
            "files",
            format!(
                "{} entries list every config and dashboard, the per-host promtail file, the CA, and the password file with the right mode",
                files.len()
            ),
        )
    } else {
        let mut issues = Vec::new();
        if !missing_sources.is_empty() {
            issues.push(format!("missing sources: {}", missing_sources.join(", ")));
        }
        issues.extend(mode_issues);
        CheckRow::failed("observability", "manifest", "files", issues.join("; "))
    }];
    let mut secret = Vec::new();
    for needle in ["\"value\"", "\"fingerprint\"", "\"password\""] {
        if text.contains(needle) {
            secret.push(format!("the manifest contains {needle}"));
        }
    }
    for needle in ["\"ports\"", "\"published\"", "webhook_configs"] {
        if text.contains(needle) {
            secret.push(format!("the manifest contains {needle}"));
        }
    }
    rows.push(if secret.is_empty() {
        CheckRow::ok(
            "observability",
            "manifest",
            "no-secret",
            "no value, fingerprint, password, public port, or external receiver is listed",
        )
    } else {
        CheckRow::failed("observability", "manifest", "no-secret", secret.join("; "))
    });
    rows
}

/// The app-stack log rotation row.
pub(super) fn stack_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let text = match rendered(out, "stack.yaml", || {
        swarm::stack_yaml(descriptor).map_err(|error| error.to_string())
    }) {
        Ok(text) => text,
        Err(error) => {
            return CheckRow::failed("observability", "app logging", "unreadable", error);
        }
    };
    let mut issues = missing(
        &text,
        &[
            "driver: \"json-file\"".to_owned(),
            "max-size: \"50m\"".to_owned(),
            "max-file: \"5\"".to_owned(),
        ],
    );
    let blocks = text.matches("logging:").count();
    if blocks < descriptor.services.len() {
        issues.push(format!(
            "{blocks} logging blocks for {} services",
            descriptor.services.len()
        ));
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "app logging",
            "json-file",
            format!(
                "{} long-running service(s) carry the docker json-file driver with 50m x 5 rotation",
                descriptor.services.len()
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "app logging",
            "configuration",
            issues.join("; "),
        )
    }
}

/// The Traefik internal metrics row.
pub(super) fn traefik_row() -> CheckRow {
    let text = ingress::STATIC_CONFIG;
    let mut issues = missing(
        text,
        &[
            "metrics:".to_owned(),
            "address: \":8082\"".to_owned(),
            "entryPoint: metrics".to_owned(),
        ],
    );
    if text.contains("address: \":80\"") {
        issues.push("Traefik exposes a public port 80 entrypoint".to_owned());
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "traefik",
            "metrics",
            "one internal 8082 metrics entrypoint selected by metrics.prometheus.entryPoint; no public metrics port",
        )
    } else {
        CheckRow::failed("observability", "traefik", "metrics", issues.join("; "))
    }
}

/// The HAProxy native metrics row.
pub(super) fn haproxy_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let mut issues = Vec::new();
    for host in &descriptor.stateful.haproxy.nodes {
        let relative = format!("haproxy-{host}.cfg");
        let text = match rendered(out, &relative, || {
            stateful::haproxy_cfg(descriptor, host).map_err(|error| error.to_string())
        }) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        issues.extend(missing(
            &text,
            &[
                "listen stats".to_owned(),
                "bind 0.0.0.0:8404".to_owned(),
                "http-request use-service prometheus-exporter if { path /metrics }".to_owned(),
            ],
        ));
        if text.contains("bind *:443") || text.contains("bind :443") {
            issues.push(format!("{host} binds 443"));
        }
    }
    vec![if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "haproxy",
            "metrics",
            format!(
                "{} node(s) serve native Prometheus metrics on 8404; no 443 bind",
                descriptor.stateful.haproxy.nodes.len()
            ),
        )
    } else {
        CheckRow::failed("observability", "haproxy", "metrics", issues.join("; "))
    }]
}
