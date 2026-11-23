//! The stateful configuration rows of a v3 check.
//!
//! The default check renders the Patroni/etcd/HAProxy configuration in memory.
//! An operator-provided `--out` (or `QR_DEPLOY_OUT`) tree is read instead, so
//! the checked bytes are the bytes that will be copied to the hosts. Certificate
//! SANs are queried through the same [`CertificateQuery`] seam as the ingress;
//! a missing `openssl` is a failed `limited` row, never a pass. No secret value
//! or fingerprint is read or printed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::Descriptor;
use crate::render::stateful::{self, stateful_sans};

use super::ingress::CertificateQuery;
use super::report::{CheckReport, CheckRow};

mod config;

/// Append every stateful row, or one failed row per unreadable group.
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

/// The stateful rows in check order.
pub(crate) fn rows(
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    if let Some(directory) = out {
        rows.push(tree_row(descriptor, directory));
    }
    rows.extend(config::patroni_rows(descriptor, out));
    rows.extend(config::etcd_rows(descriptor, out));
    rows.extend(config::haproxy_rows(descriptor, out));
    rows.extend(manifest_rows(descriptor, out));
    rows.extend(certificate_rows(descriptor, out, certificates));
    rows
}

fn tree_row(descriptor: &Descriptor, directory: &Path) -> CheckRow {
    let expected = expected_files(descriptor);
    let mut missing = Vec::new();
    for name in &expected {
        let path = directory.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {}
            Ok(_) => missing.push(format!("{name} is not a regular file")),
            Err(error) => missing.push(format!("{name}: {error}")),
        }
    }
    if missing.is_empty() {
        CheckRow::ok(
            "stateful",
            "render tree",
            "complete",
            format!("{} files under {}", expected.len(), directory.display()),
        )
    } else {
        CheckRow::failed(
            "stateful",
            "render tree",
            "missing",
            format!("{}; run qr-deploy render --target all", missing.join("; ")),
        )
    }
}

fn expected_files(descriptor: &Descriptor) -> Vec<String> {
    let mut files = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        files.push(format!("patroni-{host}.yml"));
        files.push(format!("infra/{host}.yaml"));
    }
    for host in &descriptor.stateful.etcd.nodes {
        files.push(format!("etcd-{host}.conf.yml"));
        files.push(format!("infra/{host}.yaml"));
    }
    for host in &descriptor.stateful.haproxy.nodes {
        files.push(format!("haproxy-{host}.cfg"));
        files.push(format!("infra/{host}.yaml"));
    }
    for host in &descriptor.stateful.rabbitmq.nodes {
        files.push(format!("rabbitmq-{host}.conf"));
        files.push(format!("infra/{host}.yaml"));
    }
    files.push("rabbitmq-enabled_plugins".to_owned());
    for host in &descriptor.stateful.minio.nodes {
        files.push(format!("minio-{host}.conf"));
        files.push(format!("infra/{host}.yaml"));
    }
    files.push("minio-policy.json".to_owned());
    files.push("stateful-manifest.json".to_owned());
    for name in [
        "pki/database-ca.crt",
        "pki/postgres/server.crt",
        "pki/postgres/server.key",
        "pki/etcd/server.crt",
        "pki/etcd/server.key",
        "pki/etcd/peer.crt",
        "pki/etcd/peer.key",
        "pki/etcd/client.crt",
        "pki/etcd/client.key",
        "pki/rabbitmq/ca.crt",
        "pki/rabbitmq/server.crt",
        "pki/rabbitmq/server.key",
        "pki/minio/public.crt",
        "pki/minio/private.key",
        "pki/minio/CAs/ca.crt",
    ] {
        files.push(name.to_owned());
    }
    files.sort();
    files.dedup();
    files
}

pub(super) fn read_text(path: &Path, label: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|error| {
        format!(
            "cannot read the rendered {label} at {}: {error}; run qr-deploy render --target all",
            path.display()
        )
    })
}

fn manifest_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let text = match out {
        Some(directory) => {
            match read_text(
                &directory.join("stateful-manifest.json"),
                "stateful manifest",
            ) {
                Ok(text) => text,
                Err(error) => {
                    return vec![CheckRow::failed(
                        "stateful",
                        "manifest",
                        "unreadable",
                        error,
                    )];
                }
            }
        }
        None => match stateful::stateful_manifest(descriptor) {
            Ok(text) => text,
            Err(error) => {
                return vec![CheckRow::failed(
                    "stateful",
                    "manifest",
                    "render",
                    error.to_string(),
                )];
            }
        },
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return vec![CheckRow::failed(
                "stateful",
                "manifest",
                "invalid",
                format!("stateful-manifest.json is not JSON: {error}"),
            )];
        }
    };
    let mut rows = Vec::new();
    let actual: BTreeSet<String> = value["sans"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let expected: BTreeSet<String> = stateful_sans(descriptor).into_iter().collect();
    if actual == expected {
        let names = expected.into_iter().collect::<Vec<_>>().join(", ");
        rows.push(CheckRow::ok(
            "stateful",
            "manifest",
            "sans",
            format!("SANs match the config aliases: {names}"),
        ));
    } else {
        let missing: Vec<_> = expected.difference(&actual).cloned().collect();
        let extra: Vec<_> = actual.difference(&expected).cloned().collect();
        rows.push(CheckRow::failed(
            "stateful",
            "manifest",
            "sans-mismatch",
            format!(
                "missing [{}]; extra [{}]",
                missing.join(", "),
                extra.join(", ")
            ),
        ));
    }

    let files = value["files"].as_array().cloned().unwrap_or_default();
    let sources: BTreeSet<&str> = files
        .iter()
        .filter_map(|entry| entry["source"].as_str())
        .collect();
    let mut missing = Vec::new();
    for source in expected_sources(descriptor) {
        if !sources.contains(source.as_str()) {
            missing.push(source);
        }
    }
    if missing.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "manifest",
            "files",
            format!(
                "{} files list configs, secrets, and PKI sources",
                files.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "manifest",
            "files",
            format!("missing sources: {}", missing.join(", ")),
        ));
    }

    let mut mode_issues = Vec::new();
    for entry in &files {
        let source = entry["source"].as_str().unwrap_or_default();
        let scope = entry["scope"].as_str().unwrap_or_default();
        let mode = entry["mode"].as_str().unwrap_or_default();
        let expected =
            if source.ends_with(".key") || (scope != "pki" && source != "minio-policy.json") {
                "600"
            } else {
                "644"
            };
        if mode != expected {
            mode_issues.push(format!("{source} is mode {mode}; expected {expected}"));
        }
    }
    if mode_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "manifest",
            "modes",
            "configs and secrets 600; public CA, certificates, and the bucket policy 644",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "manifest",
            "modes",
            mode_issues.join("; "),
        ));
    }

    let forbidden = ["\"value\"", "\"fingerprint\"", "\"password\""];
    let found: Vec<&str> = forbidden
        .iter()
        .copied()
        .filter(|needle| text.contains(needle))
        .collect();
    if found.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "manifest",
            "no-values",
            "no secret value, fingerprint, or password is listed",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "manifest",
            "no-values",
            format!("the manifest contains {}", found.join(", ")),
        ));
    }
    rows
}

fn expected_sources(descriptor: &Descriptor) -> Vec<String> {
    let mut sources = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        sources.push(format!("patroni-{host}.yml"));
    }
    for host in &descriptor.stateful.etcd.nodes {
        sources.push(format!("etcd-{host}.conf.yml"));
    }
    for host in &descriptor.stateful.haproxy.nodes {
        sources.push(format!("haproxy-{host}.cfg"));
    }
    for host in &descriptor.stateful.rabbitmq.nodes {
        sources.push(format!("rabbitmq-{host}.conf"));
    }
    sources.push("rabbitmq-enabled_plugins".to_owned());
    for host in &descriptor.stateful.minio.nodes {
        sources.push(format!("minio-{host}.conf"));
    }
    sources.push("minio-policy.json".to_owned());
    for secret in stateful::STATEFUL_SECRETS {
        sources.push(format!("secrets/{secret}"));
    }
    for secret in stateful::MINIO_SECRETS {
        sources.push(format!("secrets/{secret}"));
    }
    for name in ["object_store_access_key", "object_store_secret_key"] {
        sources.push(format!("secrets/{name}"));
    }
    for name in [
        "pki/database-ca.crt",
        "pki/postgres/server.crt",
        "pki/postgres/server.key",
        "pki/etcd/server.crt",
        "pki/etcd/server.key",
        "pki/etcd/peer.crt",
        "pki/etcd/peer.key",
        "pki/etcd/client.crt",
        "pki/etcd/client.key",
        "pki/rabbitmq/ca.crt",
        "pki/rabbitmq/server.crt",
        "pki/rabbitmq/server.key",
        "pki/minio/public.crt",
        "pki/minio/private.key",
        "pki/minio/CAs/ca.crt",
    ] {
        sources.push(name.to_owned());
    }
    sources
}

fn certificate_rows(
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) -> Vec<CheckRow> {
    let root = match out {
        Some(directory) => directory.join("pki"),
        None => Path::new(&descriptor.endpoints.database.tls_ca)
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    let mut postgres = vec!["postgres".to_owned()];
    postgres.extend(
        descriptor
            .stateful
            .postgres
            .nodes
            .iter()
            .map(|host| format!("patroni-{host}")),
    );
    let etcd: Vec<String> = descriptor
        .stateful
        .etcd
        .nodes
        .iter()
        .map(|host| format!("etcd-{host}"))
        .collect();
    vec![
        certificate_row(
            "postgres certificate",
            &root.join("postgres/server.crt"),
            &postgres,
            certificates,
        ),
        certificate_row(
            "etcd server certificate",
            &root.join("etcd/server.crt"),
            &etcd,
            certificates,
        ),
        certificate_row(
            "etcd peer certificate",
            &root.join("etcd/peer.crt"),
            &etcd,
            certificates,
        ),
    ]
}

fn certificate_row(
    subject: &str,
    path: &Path,
    required: &[String],
    certificates: &dyn CertificateQuery,
) -> CheckRow {
    match certificates.inspect(path) {
        Ok(facts) => {
            if !facts.valid_for_days {
                return CheckRow::failed(
                    "stateful",
                    subject,
                    "expiring",
                    format!("{} expires within 30 days", path.display()),
                );
            }
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
                    "stateful",
                    subject,
                    "san",
                    format!("{} covers {}", path.display(), required.join(", ")),
                )
            } else {
                CheckRow::failed(
                    "stateful",
                    subject,
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
            CheckRow::failed("stateful", subject, state, error)
        }
    }
}

#[cfg(test)]
mod tests;
