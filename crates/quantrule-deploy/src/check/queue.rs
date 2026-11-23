//! RabbitMQ queue rows of a v3 check.
//!
//! The default check renders the broker configuration, the plugin list, the
//! HAProxy queue frontend, and the host units in memory. An operator-provided
//! `--out` (or `QR_DEPLOY_OUT`) tree is read instead, so the checked bytes are
//! the bytes that will be copied to the hosts. The RabbitMQ leaf SANs and expiry
//! are queried through the ingress [`CertificateQuery`] seam; a missing
//! `openssl` is a failed `limited` row, never a pass. No secret value is read or
//! printed and no Docker command is run.

use std::collections::BTreeSet;
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::render::{infra, queue, stateful};

use super::ingress::CertificateQuery;
use super::report::{CheckReport, CheckRow};
use super::stateful::read_text;

/// Append every queue row, or one failed row per unreadable group.
pub(crate) fn append(
    report: &mut CheckReport,
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) {
    for row in rows(descriptor, out, certificates) {
        report.push(row);
    }
}

/// The queue rows in check order.
pub(crate) fn rows(
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    rows.extend(broker_rows(descriptor, out));
    rows.extend(haproxy_rows(descriptor, out));
    rows.extend(unit_rows(descriptor, out));
    rows.extend(manifest_rows(descriptor, out));
    rows.push(certificate_row(descriptor, out, certificates));
    rows
}

fn broker_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.rabbitmq.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("rabbitmq-{host}.conf")),
                "RabbitMQ config",
            )?,
            None => queue::rabbitmq_conf(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn haproxy_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.haproxy.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("haproxy-{host}.cfg")),
                "HAProxy config",
            )?,
            None => stateful::haproxy_cfg(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
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

fn missing_needles(texts: &[(String, String)], needles: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for (host, text) in texts {
        for needle in needles {
            if !text.contains(needle) {
                missing.push(format!("{host} lacks {needle:?}"));
            }
        }
    }
    missing
}

fn found_needles(texts: &[(String, String)], needles: &[&str]) -> Vec<String> {
    let mut found = Vec::new();
    for (host, text) in texts {
        for needle in needles {
            if text.contains(needle) {
                found.push(format!("{host} contains {needle:?}"));
            }
        }
    }
    found
}

fn push_requirement(
    rows: &mut Vec<CheckRow>,
    subject: &str,
    state: &str,
    texts: &[(String, String)],
    required: &[&str],
    forbidden: &[&str],
    detail: &str,
) {
    let mut issues = missing_needles(texts, required);
    issues.extend(found_needles(texts, forbidden));
    if issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            subject,
            state,
            format!("{} node config(s); {detail}", texts.len()),
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            subject,
            "configuration",
            issues.join("; "),
        ));
    }
}

fn broker_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match broker_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("queue", "rabbitmq", "unreadable", error)],
    };
    let mut rows = Vec::new();
    push_requirement(
        &mut rows,
        "rabbitmq",
        "tls-only",
        &texts,
        &[
            "listeners.tcp = none",
            "listeners.ssl.default = 5671",
            "ssl_options.cacertfile = /var/lib/rabbitmq/pki/ca.crt",
            "ssl_options.certfile = /var/lib/rabbitmq/pki/server.crt",
            "ssl_options.keyfile = /var/lib/rabbitmq/pki/server.key",
            "ssl_options.verify = verify_none",
            "ssl_options.fail_if_no_peer_cert = false",
            "ssl_options.versions.1 = tlsv1.2",
            "ssl_options.versions.2 = tlsv1.3",
        ],
        &[
            "5672",
            "listeners.tcp.default",
            "management.tcp",
            "rabbitmq_management",
        ],
        "TLS 1.2/1.3 on 5671; no plaintext or management listener",
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "guest-loopback",
        &texts,
        &["loopback_users.guest = true"],
        &["loopback_users.guest = false"],
        "the default guest user stays on loopback",
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "no-policy",
        &texts,
        &[],
        &["policy.", "runs.", "stages."],
        "no operator policy targets an application queue",
    );

    let mut cluster_issues = missing_needles(
        &texts,
        &[
            "cluster_formation.peer_discovery_backend = rabbit_peer_discovery_classic_config",
            "cluster_formation.node_type = disc",
        ],
    );
    let brokers = &descriptor.stateful.rabbitmq.nodes;
    for (host, text) in &texts {
        let members = text
            .matches("cluster_formation.classic_config.nodes.")
            .count();
        if members != brokers.len() {
            cluster_issues.push(format!(
                "{host} declares {members} classic peers; expected {}",
                brokers.len()
            ));
        }
        for (index, broker) in brokers.iter().enumerate() {
            let expected = format!(
                "cluster_formation.classic_config.nodes.{} = rabbit@rabbitmq-{broker}",
                index + 1
            );
            if !text.contains(&expected) {
                cluster_issues.push(format!("{host} lacks {expected}"));
            }
        }
    }
    if cluster_issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "rabbitmq",
            "cluster",
            format!(
                "{} brokers use classic discovery of {} short node names",
                brokers.len(),
                brokers.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "rabbitmq",
            "cluster",
            cluster_issues.join("; "),
        ));
    }
    push_requirement(
        &mut rows,
        "rabbitmq",
        "partition-handling",
        &texts,
        &["cluster_partition_handling = pause_minority"],
        &[
            "cluster_partition_handling = autoheal",
            "cluster_partition_handling = ignore",
        ],
        "a minority partition pauses",
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "queue-default",
        &texts,
        &["default_queue_type = quorum"],
        &["default_queue_type = classic"],
        "the only operator queue default is quorum",
    );
    let disk = format!(
        "disk_free_limit.absolute = {}MiB",
        descriptor.stateful.rabbitmq.disk_free_limit_mib
    );
    let memory = format!(
        "vm_memory_high_watermark.relative = 0.{:02}",
        descriptor.stateful.rabbitmq.memory_high_watermark_percent
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "alarms",
        &texts,
        &[
            disk.as_str(),
            memory.as_str(),
            "vm_memory_high_watermark_paging_ratio = 0.5",
        ],
        &[],
        "disk and memory alarms match the descriptor",
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "metrics",
        &texts,
        &["prometheus.tcp.port = 15692"],
        &["management.tcp.port", "15672"],
        "Prometheus on the overlay; management stays off",
    );
    push_requirement(
        &mut rows,
        "rabbitmq",
        "logging",
        &texts,
        &[
            "log.console = true",
            "log.console.formatter = json",
            "log.file = false",
        ],
        &[],
        "one JSON object per line on the console",
    );

    let addresses: BTreeSet<&str> = descriptor
        .hosts
        .values()
        .map(|host| host.address.as_str())
        .collect();
    let mut underlay = Vec::new();
    for (host, text) in &texts {
        for address in &addresses {
            if text.contains(*address) {
                underlay.push(format!("{host} contains {address}"));
            }
        }
    }
    if underlay.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "rabbitmq",
            "overlay-only",
            "no host underlay address in any broker config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "rabbitmq",
            "overlay-only",
            underlay.join("; "),
        ));
    }
    rows
}

/// One named HAProxy section, from its header to the next `frontend`, `backend`,
/// or `listen` header. The header includes its leading newline, so
/// `default_backend` does not match a backend header. An absent header yields
/// the empty string.
fn section<'a>(text: &'a str, header: &str) -> &'a str {
    let Some(start) = text.find(header) else {
        return "";
    };
    let rest = &text[start..];
    let end = rest[1..]
        .find("\nfrontend ")
        .into_iter()
        .chain(rest[1..].find("\nbackend "))
        .chain(rest[1..].find("\nlisten "))
        .min()
        .map_or(rest.len(), |index| index + 1);
    &rest[..end]
}

/// One 2-space-indented Compose service block. The header includes its leading
/// newline so a mount path or a network alias cannot match it.
fn service_block<'a>(text: &'a str, key: &str) -> &'a str {
    let header = format!("\n  {key}:\n");
    let Some(start) = text.find(&header) else {
        return "";
    };
    let rest = &text[start + header.len()..];
    let end = rest
        .match_indices("\n  ")
        .find(|(index, _)| {
            let after = &rest[index + 3..];
            !after.starts_with(' ') && !after.starts_with('\n')
        })
        .map_or(rest.len(), |(index, _)| index);
    &rest[..end]
}

fn haproxy_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match haproxy_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("queue", "haproxy", "unreadable", error)],
    };
    let mut rows = Vec::new();
    push_requirement(
        &mut rows,
        "haproxy",
        "frontend",
        &texts,
        &[
            "frontend rabbitmq",
            "bind *:5671",
            "default_backend rabbitmq_nodes",
        ],
        &["bind *:5672", "bind *:15672"],
        "TCP passthrough on 5671 only",
    );

    let mut backend_issues = Vec::new();
    for (host, text) in &texts {
        let backend = section(text, "\nbackend rabbitmq_nodes");
        if backend.is_empty() {
            backend_issues.push(format!("{host} lacks backend rabbitmq_nodes"));
            continue;
        }
        for expected in ["option tcp-check", "tcp-check connect"] {
            if !backend.contains(expected) {
                backend_issues.push(format!("{host} lacks {expected:?}"));
            }
        }
        for broker in &descriptor.stateful.rabbitmq.nodes {
            let expected = format!("server rabbitmq-{broker} rabbitmq-{broker}:5671 check");
            if !backend.contains(&expected) {
                backend_issues.push(format!("{host} lacks {expected}"));
            }
        }
        if !backend.contains("on-marked-down shutdown-sessions") {
            backend_issues.push(format!("{host} lacks on-marked-down shutdown-sessions"));
        }
    }
    if backend_issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "haproxy",
            "backend",
            format!(
                "{} health-checked server(s); a marked-down broker closes its sessions",
                descriptor.stateful.rabbitmq.nodes.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "haproxy",
            "backend",
            backend_issues.join("; "),
        ));
    }

    let mut unit_issues = Vec::new();
    for host in &descriptor.stateful.haproxy.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                unit_issues.push(error);
                continue;
            }
        };
        let haproxy = service_block(&text, "haproxy");
        if haproxy.is_empty() {
            unit_issues.push(format!("infra/{host}.yaml lacks the haproxy unit"));
            continue;
        }
        if !haproxy.contains("\"rabbitmq\"") {
            unit_issues.push(format!("infra/{host}.yaml lacks the rabbitmq alias"));
        }
        if haproxy.contains("ports:") {
            unit_issues.push(format!("infra/{host}.yaml publishes a port"));
        }
    }
    if unit_issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "haproxy",
            "unit",
            "the shared rabbitmq alias is on the HAProxy pair; no published port",
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "haproxy",
            "unit",
            unit_issues.join("; "),
        ));
    }
    rows
}

fn unit_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let mut issues = Vec::new();
    for host in &descriptor.stateful.rabbitmq.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        let broker = service_block(&text, &format!("rabbitmq-broker-{host}"));
        if broker.is_empty() {
            issues.push(format!(
                "infra/{host}.yaml lacks the rabbitmq-broker-{host} unit"
            ));
            continue;
        }
        let required = [
            format!("hostname: \"rabbitmq-{host}\""),
            format!("RABBITMQ_NODENAME: \"rabbit@rabbitmq-{host}\""),
            "RABBITMQ_CONFIG_FILE: \"/var/lib/rabbitmq/rabbitmq.conf\"".to_owned(),
            "RABBITMQ_ENABLED_PLUGINS_FILE: \"/var/lib/rabbitmq/enabled_plugins\"".to_owned(),
            "/etc/quantrule/rabbitmq/rabbitmq.conf".to_owned(),
            "/etc/quantrule/rabbitmq/enabled_plugins".to_owned(),
            "/etc/quantrule/rabbitmq/erlang.cookie".to_owned(),
            "/etc/quantrule/pki/rabbitmq".to_owned(),
            "/srv/quantrule/rabbitmq/data".to_owned(),
            "/srv/quantrule/rabbitmq/definitions".to_owned(),
            "install -o rabbitmq -g rabbitmq -m 600 /etc/quantrule/rabbitmq/erlang.cookie"
                .to_owned(),
            "install -o rabbitmq -g rabbitmq -m 600 /etc/quantrule/pki/rabbitmq/server.key"
                .to_owned(),
            "gosu rabbitmq rabbitmq-server".to_owned(),
        ];
        for needle in &required {
            if !broker.contains(needle) {
                issues.push(format!("infra/{host}.yaml lacks {needle:?}"));
            }
        }
        for forbidden in ["RABBITMQ_USE_LONGNAME", "ports:", "\"rabbitmq\""] {
            if broker.contains(forbidden) {
                issues.push(format!("infra/{host}.yaml contains {forbidden}"));
            }
        }
    }
    if issues.is_empty() {
        vec![CheckRow::ok(
            "queue",
            "rabbitmq",
            "unit",
            format!(
                "{} broker unit(s); short hostname/node name, mounts, cookie/TLS install, no plaintext alias or port",
                descriptor.stateful.rabbitmq.nodes.len()
            ),
        )]
    } else {
        vec![CheckRow::failed(
            "queue",
            "rabbitmq",
            "unit",
            issues.join("; "),
        )]
    }
}

fn manifest_text(descriptor: &Descriptor, out: Option<&Path>) -> Result<String, String> {
    match out {
        Some(directory) => read_text(
            &directory.join("stateful-manifest.json"),
            "stateful manifest",
        ),
        None => stateful::stateful_manifest(descriptor).map_err(|error| error.to_string()),
    }
}

fn manifest_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let text = match manifest_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed("queue", "manifest", "unreadable", error)];
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return vec![CheckRow::failed(
                "queue",
                "manifest",
                "invalid",
                format!("stateful-manifest.json is not JSON: {error}"),
            )];
        }
    };
    let files = value["files"].as_array().cloned().unwrap_or_default();
    let brokers: BTreeSet<&str> = descriptor
        .stateful
        .rabbitmq
        .nodes
        .iter()
        .map(String::as_str)
        .collect();
    let entry = |source: &str| {
        files
            .iter()
            .find(|entry| entry["source"].as_str() == Some(source))
    };

    let mut host_issues = Vec::new();
    let mut mode_issues = Vec::new();
    for entry_value in &files {
        let source = entry_value["source"].as_str().unwrap_or_default();
        let hosts = entry_value["hosts"]
            .as_array()
            .map(|hosts| {
                hosts
                    .iter()
                    .filter_map(|host| host.as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if hosts.is_empty() {
            host_issues.push(format!("{source} has no hosts selector"));
        }
        for host in &hosts {
            if !descriptor.hosts.contains_key(*host) {
                host_issues.push(format!("{source} names undeclared host {host}"));
            }
        }
    }
    for broker in &descriptor.stateful.rabbitmq.nodes {
        let expected = [broker.as_str()];
        let source = format!("rabbitmq-{broker}.conf");
        match entry(&source) {
            Some(entry_value) => {
                let hosts: Vec<&str> = entry_value["hosts"]
                    .as_array()
                    .map(|hosts| hosts.iter().filter_map(|host| host.as_str()).collect())
                    .unwrap_or_default();
                if hosts != expected {
                    host_issues.push(format!(
                        "{source} is listed for [{}]; expected [{broker}]",
                        hosts.join(", ")
                    ));
                }
                let mode = entry_value["mode"].as_str().unwrap_or_default();
                if mode != "600" {
                    mode_issues.push(format!("{source} is mode {mode}; expected 600"));
                }
            }
            None => host_issues.push(format!("the manifest omits {source}")),
        }
    }
    for (source, mode) in [
        ("rabbitmq-enabled_plugins", "600"),
        ("secrets/rabbitmq_erlang_cookie", "600"),
        ("pki/rabbitmq/ca.crt", "644"),
        ("pki/rabbitmq/server.crt", "644"),
        ("pki/rabbitmq/server.key", "600"),
    ] {
        match entry(source) {
            Some(entry_value) => {
                let hosts: BTreeSet<&str> = entry_value["hosts"]
                    .as_array()
                    .map(|hosts| hosts.iter().filter_map(|host| host.as_str()).collect())
                    .unwrap_or_default();
                if hosts != brokers {
                    host_issues.push(format!(
                        "{source} is listed for [{}]; expected the broker set [{}]",
                        hosts.into_iter().collect::<Vec<_>>().join(", "),
                        brokers.iter().copied().collect::<Vec<_>>().join(", ")
                    ));
                }
                let actual = entry_value["mode"].as_str().unwrap_or_default();
                if actual != mode {
                    mode_issues.push(format!("{source} is mode {actual}; expected {mode}"));
                }
            }
            None => host_issues.push(format!("the manifest omits {source}")),
        }
    }
    let mut rows = Vec::new();
    if host_issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "manifest",
            "hosts",
            format!(
                "only the {} broker host(s) receive the cookie, queue key, queue PKI, and broker config",
                brokers.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "manifest",
            "hosts",
            host_issues.join("; "),
        ));
    }
    if mode_issues.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "manifest",
            "broker-modes",
            "broker config/cookie/key are 600; queue CA and certificate are 644",
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "manifest",
            "broker-modes",
            mode_issues.join("; "),
        ));
    }

    let actual: BTreeSet<String> = value["sans"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let expected: BTreeSet<String> = queue::queue_sans(descriptor).into_iter().collect();
    let missing: Vec<_> = expected.difference(&actual).cloned().collect();
    let extra: Vec<_> = actual
        .iter()
        .filter(|name| name.starts_with("rabbitmq") && !expected.contains(*name))
        .cloned()
        .collect();
    if missing.is_empty() && extra.is_empty() {
        rows.push(CheckRow::ok(
            "queue",
            "manifest",
            "sans",
            format!(
                "the manifest carries every queue alias: {}",
                expected.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "queue",
            "manifest",
            "sans",
            format!(
                "missing [{}]; extra queue names [{}]",
                missing.join(", "),
                extra.join(", ")
            ),
        ));
    }
    rows
}

fn certificate_row(
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) -> CheckRow {
    let path = match out {
        Some(directory) => directory.join("pki/rabbitmq/server.crt"),
        None => Path::new(&descriptor.endpoints.queue.tls_ca)
            .parent()
            .map(|parent| parent.join("rabbitmq").join("server.crt"))
            .unwrap_or_else(|| Path::new("rabbitmq/server.crt").to_path_buf()),
    };
    match certificates.inspect(&path) {
        Ok(facts) => {
            if !facts.valid_for_days {
                return CheckRow::failed(
                    "queue",
                    "rabbitmq certificate",
                    "expiring",
                    format!("{} expires within 30 days", path.display()),
                );
            }
            let required = queue::queue_sans(descriptor);
            let missing: Vec<&str> = required
                .iter()
                .filter(|name| {
                    !facts
                        .dns_names
                        .iter()
                        .any(|actual| actual.eq_ignore_ascii_case(name))
                })
                .map(String::as_str)
                .collect();
            if missing.is_empty() {
                CheckRow::ok(
                    "queue",
                    "rabbitmq certificate",
                    "san",
                    format!("{} covers {}", path.display(), required.join(", ")),
                )
            } else {
                CheckRow::failed(
                    "queue",
                    "rabbitmq certificate",
                    "san-mismatch",
                    format!(
                        "{} does not cover {}; found [{}]",
                        path.display(),
                        missing.join(", "),
                        facts.dns_names.join(", ")
                    ),
                )
            }
        }
        Err(error) => {
            let state = if error.starts_with("cannot run") {
                "limited"
            } else {
                "invalid"
            };
            CheckRow::failed("queue", "rabbitmq certificate", state, error)
        }
    }
}

#[cfg(test)]
mod tests;
