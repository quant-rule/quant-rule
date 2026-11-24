//! MinIO rows of a v3 check.
//!
//! The default check renders the per-node `minio.conf`, the bucket policy, the
//! HAProxy object-store frontend, the host units, and the stateful manifest in
//! memory. An operator-provided `--out` (or `QR_DEPLOY_OUT`) tree is read
//! instead, so the checked bytes are the bytes that will be copied to the
//! hosts. The MinIO leaf SANs and expiry are queried through the ingress
//! [`CertificateQuery`] seam; a missing `openssl` is a failed `limited` row,
//! never a pass. No secret value is read or printed and no Docker command is
//! run.

use std::collections::BTreeSet;
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::render::{infra, minio, stateful, swarm};
use crate::validate::secrets::OBJECT_STORE_USER;
use crate::validate::v3::minio_parity;

use super::ingress::CertificateQuery;
use super::report::{CheckReport, CheckRow};
use super::stateful::read_text;
use super::text::{section, service_block};

/// Append every MinIO row, or one failed row per unreadable group.
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

/// The MinIO rows in check order.
pub(crate) fn rows(
    descriptor: &Descriptor,
    out: Option<&Path>,
    certificates: &dyn CertificateQuery,
) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    rows.extend(config_rows(descriptor, out));
    rows.push(archive_reader_row(out));
    rows.push(layout_row(descriptor));
    rows.extend(unit_rows(descriptor, out));
    rows.extend(haproxy_rows(descriptor, out));
    rows.extend(manifest_rows(descriptor, out));
    rows.push(certificate_row(descriptor, out, certificates));
    rows.extend(app_rows(descriptor, out));
    rows
}

fn minio_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.minio.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("minio-{host}.conf")),
                "MinIO config",
            )?,
            None => minio::minio_conf(descriptor, host).map_err(|error| error.to_string())?,
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

fn stack_text(descriptor: &Descriptor, out: Option<&Path>) -> Result<String, String> {
    match out {
        Some(directory) => read_text(&directory.join("stack.yaml"), "stack"),
        None => swarm::stack_yaml(descriptor).map_err(|error| error.to_string()),
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
            "minio",
            subject,
            state,
            format!("{} node config(s); {detail}", texts.len()),
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            subject,
            "configuration",
            issues.join("; "),
        ));
    }
}

/// The explicit volume list, in descriptor order.
fn expected_volumes(descriptor: &Descriptor) -> String {
    let mut volumes = Vec::new();
    for (index, node) in descriptor.stateful.minio.nodes.iter().enumerate() {
        let drives = descriptor
            .stateful
            .minio
            .drives
            .get(index)
            .copied()
            .unwrap_or(0);
        for drive in 1..=drives {
            volumes.push(format!("https://minio-{node}:9000/data{drive}"));
        }
    }
    format!("MINIO_VOLUMES=\"{}\"", volumes.join(" "))
}

/// One secret file's value, trimmed, for the forbidden-value scan.
fn secret_value(descriptor: &Descriptor, name: &str) -> Result<String, String> {
    let path = descriptor
        .secrets
        .files
        .get(name)
        .ok_or_else(|| format!("secrets.files.{name} is not declared"))?;
    std::fs::read_to_string(path)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("cannot read {name} at {path}: {error}"))
}

fn config_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match minio_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("minio", "minio", "unreadable", error)],
    };
    let mut rows = Vec::new();
    let volumes = expected_volumes(descriptor);
    let parity = minio_parity(descriptor).ok();
    let standard = parity.map(|value| format!("MINIO_STORAGE_CLASS_STANDARD=\"EC:{value}\""));
    let mut required = vec![
        volumes.as_str(),
        "MINIO_SERVER_URL=\"https://minio:9000\"",
        "MINIO_BROWSER=\"off\"",
        "MINIO_ROOT_USER_FILE=\"/run/secrets/minio_root_user\"",
        "MINIO_ROOT_PASSWORD_FILE=\"/run/secrets/minio_root_password\"",
        "MINIO_CONFIG_ENV_FILE=\"/etc/minio/minio.conf\"",
        "MINIO_REGION=\"us-east-1\"",
        // The Prometheus job scrapes MinIO on the overlay with public metrics
        // auth. The unit publishes no port, so this never reaches the underlay.
        "MINIO_PROMETHEUS_AUTH_TYPE=\"public\"",
    ];
    if let Some(value) = standard.as_deref() {
        required.push(value);
    }
    push_requirement(
        &mut rows,
        "minio",
        "conf",
        &texts,
        &required,
        &["MINIO_ROOT_USER=", "MINIO_ROOT_PASSWORD="],
        "explicit volumes, browser off, server URL, EC parity, root _FILE references, region",
    );

    let mut issues = Vec::new();
    for (host, text) in &texts {
        for name in [
            "minio_root_user",
            "minio_root_password",
            "object_store_secret_key",
        ] {
            match secret_value(descriptor, name) {
                Ok(value) if !value.is_empty() && text.contains(&value) => {
                    issues.push(format!("{host} contains the value of {name}"));
                }
                Ok(_) => {}
                Err(error) => issues.push(format!("{host}: {error}")),
            }
        }
        if text.contains("9001") {
            issues.push(format!("{host} mentions the MinIO console port 9001"));
        }
    }
    if issues.is_empty() {
        rows.push(CheckRow::ok(
            "minio",
            "minio",
            "no-secret",
            "no root or app secret value and no console port in any node config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "minio",
            "no-secret",
            issues.join("; "),
        ));
    }

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
            "minio",
            "minio",
            "overlay-only",
            "no host underlay address in any node config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "minio",
            "overlay-only",
            underlay.join("; "),
        ));
    }
    rows
}

/// The read-only policy the object archive uses as its source identity.
fn archive_reader_row(out: Option<&Path>) -> CheckRow {
    let policy = match out {
        Some(directory) => match read_text(
            &directory.join("minio-archive-policy.json"),
            "object-archive policy",
        ) {
            Ok(text) => text,
            Err(error) => return CheckRow::failed("minio", "archive-reader", "unreadable", error),
        },
        None => minio::archive_reader_policy(),
    };
    let mut missing = Vec::new();
    for needle in [
        "\"s3:GetBucketLocation\"",
        "\"s3:ListBucket\"",
        "\"s3:GetObject\"",
        "\"arn:aws:s3:::quantrule\"",
        "\"arn:aws:s3:::quantrule/*\"",
    ] {
        if !policy.contains(needle) {
            missing.push(format!("the policy lacks {needle}"));
        }
    }
    let mut forbidden = Vec::new();
    for needle in [
        "s3:PutObject",
        "s3:DeleteObject",
        "s3:AbortMultipartUpload",
        "s3:ListMultipartUploadParts",
        "s3:CreateBucket",
        "s3:DeleteBucket",
        "s3:PutBucketPolicy",
        "s3:*",
        "admin:",
        "arn:aws:s3:::*",
    ] {
        if policy.contains(needle) {
            forbidden.push(format!("the policy grants {needle}"));
        }
    }
    if missing.is_empty() && forbidden.is_empty() {
        CheckRow::ok(
            "minio",
            "archive-reader",
            "read-only",
            format!(
                "{} policy allows list/get on the bucket and no write, delete, admin, or wildcard action; the tool mirrors one way and never deletes the source",
                minio::ARCHIVE_READER_USER
            ),
        )
    } else {
        CheckRow::failed(
            "minio",
            "archive-reader",
            "policy",
            [missing, forbidden].concat().join("; "),
        )
    }
}

fn layout_row(descriptor: &Descriptor) -> CheckRow {
    let parity = match minio_parity(descriptor) {
        Ok(parity) => parity,
        Err(error) => {
            return CheckRow::failed("minio", "minio", "layout", error.to_string());
        }
    };
    let total: u32 = descriptor.stateful.minio.drives.iter().sum();
    let max = descriptor
        .stateful
        .minio
        .drives
        .iter()
        .copied()
        .max()
        .unwrap_or(0);
    let quorum = total - parity + 1;
    let remaining = total - max;
    if remaining < quorum {
        return CheckRow::failed(
            "minio",
            "minio",
            "quorum",
            format!(
                "EC:{parity} over {total} drives leaves {remaining} drives after losing the largest node; the write quorum is {quorum}"
            ),
        );
    }
    CheckRow::ok(
        "minio",
        "minio",
        "quorum",
        format!(
            "{} nodes / {total} drives; EC:{parity}; one node loss leaves {remaining} drives and the write quorum {quorum}",
            descriptor.stateful.minio.nodes.len()
        ),
    )
}

fn unit_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let mut issues = Vec::new();
    for (index, host) in descriptor.stateful.minio.nodes.iter().enumerate() {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        let service = format!("minio-{host}");
        let minio = service_block(&text, &service);
        if minio.is_empty() {
            issues.push(format!("infra/{host}.yaml lacks the {service} unit"));
            continue;
        }
        let drives = descriptor
            .stateful
            .minio
            .drives
            .get(index)
            .copied()
            .unwrap_or(0);
        let mut required = vec![
            format!("hostname: \"minio-{host}\""),
            format!("container_name: \"quantrule-minio-{host}\""),
            format!("            \"minio-{host}\"\n"),
            "MINIO_CONFIG_ENV_FILE: \"/etc/minio/minio.conf\"".to_owned(),
            "MINIO_SERVER_URL: \"https://minio:9000\"".to_owned(),
            "MINIO_BROWSER: \"off\"".to_owned(),
            "MINIO_ROOT_USER_FILE: \"/run/secrets/minio_root_user\"".to_owned(),
            "MINIO_ROOT_PASSWORD_FILE: \"/run/secrets/minio_root_password\"".to_owned(),
            "/etc/quantrule/minio/minio.conf".to_owned(),
            "/etc/quantrule/pki/minio".to_owned(),
            "target: \"/certs\"".to_owned(),
            "$$(cat /run/secrets/minio_root_user)".to_owned(),
            "$$(cat /run/secrets/minio_root_password)".to_owned(),
            ". /etc/minio/minio.conf".to_owned(),
            "exec minio server --certs-dir /certs".to_owned(),
        ];
        for drive in 1..=drives {
            required.push(format!("/srv/quantrule/minio/{host}/drive{drive}"));
            required.push(format!("target: \"/data{drive}\""));
        }
        for needle in &required {
            if !minio.contains(needle) {
                issues.push(format!("infra/{host}.yaml lacks {needle:?}"));
            }
        }
        for forbidden in ["\n            \"minio\"\n", "ports:", "9001"] {
            if minio.contains(forbidden) {
                issues.push(format!("infra/{host}.yaml contains {forbidden}"));
            }
        }
        if text.matches(&format!("\n  {service}:\n")).count() != 1 {
            issues.push(format!("infra/{host}.yaml does not have one {service} key"));
        }
    }
    if issues.is_empty() {
        vec![CheckRow::ok(
            "minio",
            "minio",
            "unit",
            format!(
                "{} node unit(s); unique key/hostname/alias, certs dir, config/PKI/root and drive mounts, wrapper command, no console or published port",
                descriptor.stateful.minio.nodes.len()
            ),
        )]
    } else {
        vec![CheckRow::failed(
            "minio",
            "minio",
            "unit",
            issues.join("; "),
        )]
    }
}

fn haproxy_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match haproxy_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("minio", "haproxy", "unreadable", error)],
    };
    let mut rows = Vec::new();
    push_requirement(
        &mut rows,
        "haproxy",
        "frontend",
        &texts,
        &[
            "frontend minio",
            "bind *:9000",
            "default_backend minio_nodes",
        ],
        &["bind *:9001"],
        "TCP passthrough on 9000 only",
    );

    let mut backend_issues = Vec::new();
    for (host, text) in &texts {
        let backend = section(text, "\nbackend minio_nodes");
        if backend.is_empty() {
            backend_issues.push(format!("{host} lacks backend minio_nodes"));
            continue;
        }
        for expected in ["option tcp-check", "tcp-check connect"] {
            if !backend.contains(expected) {
                backend_issues.push(format!("{host} lacks {expected:?}"));
            }
        }
        for node in &descriptor.stateful.minio.nodes {
            let expected = format!("server minio-{node} minio-{node}:9000 check");
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
            "minio",
            "haproxy",
            "backend",
            format!(
                "{} health-checked server(s); a marked-down MinIO node closes its sessions",
                descriptor.stateful.minio.nodes.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
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
        if !haproxy.contains("\"minio\"") {
            unit_issues.push(format!("infra/{host}.yaml lacks the minio alias"));
        }
        if haproxy.contains("ports:") {
            unit_issues.push(format!("infra/{host}.yaml publishes a port"));
        }
    }
    if unit_issues.is_empty() {
        rows.push(CheckRow::ok(
            "minio",
            "haproxy",
            "unit",
            "the shared minio alias is on the HAProxy pair; no published port",
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "haproxy",
            "unit",
            unit_issues.join("; "),
        ));
    }
    rows
}

fn manifest_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let text = match manifest_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed("minio", "manifest", "unreadable", error)];
        }
    };
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return vec![CheckRow::failed(
                "minio",
                "manifest",
                "invalid",
                format!("stateful-manifest.json is not JSON: {error}"),
            )];
        }
    };
    let files = value["files"].as_array().cloned().unwrap_or_default();
    let minio_hosts: BTreeSet<String> = descriptor.stateful.minio.nodes.iter().cloned().collect();
    let entry = |source: &str| {
        files
            .iter()
            .find(|entry| entry["source"].as_str() == Some(source))
    };
    let entry_hosts = |entry: &serde_json::Value| -> BTreeSet<String> {
        entry["hosts"]
            .as_array()
            .map(|hosts| {
                hosts
                    .iter()
                    .filter_map(|host| host.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };

    let mut host_issues = Vec::new();
    let mut mode_issues = Vec::new();
    for host in &descriptor.stateful.minio.nodes {
        let source = format!("minio-{host}.conf");
        match entry(&source) {
            Some(value) => {
                let hosts = entry_hosts(value);
                if hosts != BTreeSet::from([host.clone()]) {
                    host_issues.push(format!(
                        "{source} is listed for [{}]; expected [{host}]",
                        hosts.into_iter().collect::<Vec<_>>().join(", ")
                    ));
                }
                if value["target"].as_str() != Some(minio::MINIO_CONFIG_HOST) {
                    host_issues.push(format!(
                        "{source} targets {:?}; expected {:?}",
                        value["target"].as_str().unwrap_or_default(),
                        minio::MINIO_CONFIG_HOST
                    ));
                }
                let mode = value["mode"].as_str().unwrap_or_default();
                if mode != "600" {
                    mode_issues.push(format!("{source} is mode {mode}; expected 600"));
                }
            }
            None => host_issues.push(format!("the manifest omits {source}")),
        }
    }
    for (source, mode) in [
        ("minio-policy.json", "644"),
        ("minio-archive-policy.json", "644"),
        ("secrets/minio_root_user", "600"),
        ("secrets/minio_root_password", "600"),
        ("secrets/object_store_access_key", "600"),
        ("secrets/object_store_secret_key", "600"),
        ("pki/minio/public.crt", "644"),
        ("pki/minio/private.key", "600"),
        ("pki/minio/CAs/ca.crt", "644"),
    ] {
        match entry(source) {
            Some(value) => {
                let hosts = entry_hosts(value);
                if hosts != minio_hosts {
                    host_issues.push(format!(
                        "{source} is listed for [{}]; expected the MinIO host set [{}]",
                        hosts.into_iter().collect::<Vec<_>>().join(", "),
                        minio_hosts.iter().cloned().collect::<Vec<_>>().join(", ")
                    ));
                }
                let actual = value["mode"].as_str().unwrap_or_default();
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
            "minio",
            "manifest",
            "hosts",
            format!(
                "only the {} MinIO host(s) receive the config, root files, app bootstrap pair, policy, and PKI",
                minio_hosts.len()
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "manifest",
            "hosts",
            host_issues.join("; "),
        ));
    }
    if mode_issues.is_empty() {
        rows.push(CheckRow::ok(
            "minio",
            "manifest",
            "minio-modes",
            "config, root files, app bootstrap pair, and private key are 600; policy, public certificate, and CA are 644",
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "manifest",
            "minio-modes",
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
    let expected: BTreeSet<String> = minio::minio_sans(descriptor).into_iter().collect();
    let missing: Vec<_> = expected.difference(&actual).cloned().collect();
    let extra: Vec<_> = actual
        .iter()
        .filter(|name| name.starts_with("minio") && !expected.contains(*name))
        .cloned()
        .collect();
    if missing.is_empty() && extra.is_empty() {
        rows.push(CheckRow::ok(
            "minio",
            "manifest",
            "sans",
            format!(
                "the manifest carries every MinIO name: {}",
                expected.into_iter().collect::<Vec<_>>().join(", ")
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "manifest",
            "sans",
            format!(
                "missing [{}]; extra MinIO names [{}]",
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
        Some(directory) => directory.join("pki/minio/public.crt"),
        None => Path::new(&descriptor.endpoints.object_store.tls_ca)
            .parent()
            .map(|parent| parent.join("minio").join("public.crt"))
            .unwrap_or_else(|| Path::new("minio/public.crt").to_path_buf()),
    };
    match certificates.inspect(&path) {
        Ok(facts) => {
            if !facts.valid_for_days {
                return CheckRow::failed(
                    "minio",
                    "minio certificate",
                    "expiring",
                    format!("{} expires within 30 days", path.display()),
                );
            }
            let required = minio::minio_sans(descriptor);
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
                    "minio",
                    "minio certificate",
                    "san",
                    format!("{} covers {}", path.display(), required.join(", ")),
                )
            } else {
                CheckRow::failed(
                    "minio",
                    "minio certificate",
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
            CheckRow::failed("minio", "minio certificate", state, error)
        }
    }
}

fn app_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let stack = match stack_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => return vec![CheckRow::failed("minio", "app", "unreadable", error)],
    };
    let mut rows = Vec::new();
    let mut issues = Vec::new();
    let url = "QR_OBJECT_STORE_URL: \"https://minio:9000\"";
    let urls = stack.matches(url).count();
    if urls != descriptor.services.len() {
        issues.push(format!(
            "{urls} service(s) use the MinIO endpoint; expected {}",
            descriptor.services.len()
        ));
    }
    for required in [
        "QR_OBJECT_STORE_ACCESS_KEY_FILE: \"/run/secrets/object_store_access_key\"",
        "QR_OBJECT_STORE_SECRET_KEY_FILE: \"/run/secrets/object_store_secret_key\"",
        "QR_OBJECT_STORE_TLS_CA_PATH: \"/run/quantrule/pki/object-store-ca.crt\"",
    ] {
        if !stack.contains(required) {
            issues.push(format!("stack.yaml lacks {required:?}"));
        }
    }
    if stack.contains("minio_root_user") || stack.contains("minio_root_password") {
        issues.push("stack.yaml mounts a MinIO root credential".to_owned());
    }
    if issues.is_empty() {
        rows.push(CheckRow::ok(
            "minio",
            "app",
            "object-store",
            format!(
                "{} services use https://minio:9000 with _FILE credentials and the private CA; no root mount",
                urls
            ),
        ));
    } else {
        rows.push(CheckRow::failed(
            "minio",
            "app",
            "object-store",
            issues.join("; "),
        ));
    }

    match secret_value(descriptor, "object_store_access_key") {
        Ok(value) if value == OBJECT_STORE_USER => rows.push(CheckRow::ok(
            "minio",
            "app",
            "credentials",
            format!("the app access key is the dedicated {OBJECT_STORE_USER} user"),
        )),
        Ok(_) => rows.push(CheckRow::failed(
            "minio",
            "app",
            "credentials",
            format!("the app access key is not {OBJECT_STORE_USER}"),
        )),
        Err(error) => rows.push(CheckRow::failed("minio", "app", "credentials", error)),
    }

    if descriptor.backup.object_replication {
        rows.push(CheckRow::failed(
            "minio",
            "app",
            "replication",
            "backup.object_replication is true; HA-013 owns off-host/archive and any second-site copy",
        ));
    } else {
        rows.push(CheckRow::ok(
            "minio",
            "app",
            "replication",
            "object_replication is false; HA-013 owns off-host/archive and any second-site copy",
        ));
    }
    rows
}

#[cfg(test)]
mod tests;
