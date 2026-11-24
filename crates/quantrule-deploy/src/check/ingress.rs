//! The ingress check rows: configuration, certificate, probe, and mounts.
//!
//! The default check reads the checked-in static and rendered dynamic Traefik
//! configuration, every rendered keepalived file, and the operator certificate
//! files. A [`CertificateQuery`] seam runs `openssl` in production and a fake in
//! tests; a missing `openssl` is a failed row, never a silent pass. The probe
//! rows check the HTTPS security headers, the plain-HTTP closure after the HTTPS
//! rows succeed, and the running ingress mounts through the read-only
//! [`DockerQuery`] seam. A live VIP and firewall control is lab-only and is
//! reported as `limited`, not as a pass.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::descriptor::v3::{Descriptor, IngressTls};
use crate::render::ingress::{
    self, INGRESS_CERT_CONFIG, INGRESS_KEY_SECRET, TRAEFIK_CONFIG, TRAEFIK_DYNAMIC_CONFIG,
};
use crate::render::stack_name;

use super::remote::DockerQuery;
use super::report::{CheckReport, CheckRow};
#[cfg(test)]
use super::v3::ProbeResponse;
use super::v3::{FIXTURE_MISSING, Probe};

/// The certificate facts the check needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateFacts {
    /// The DNS subject alternative names.
    pub dns_names: Vec<String>,
    /// Whether the certificate is valid for at least the next 30 days.
    pub valid_for_days: bool,
}

/// The certificate inspection seam.
pub trait CertificateQuery {
    /// Inspect one PEM certificate.
    ///
    /// # Errors
    ///
    /// Returns display text when the tool is missing or the certificate is
    /// malformed or expiring. The message never carries a key value.
    fn inspect(&self, path: &Path) -> Result<CertificateFacts, String>;
}

/// The production adapter: `openssl x509 -noout -checkend 2592000 -ext subjectAltName`.
#[derive(Debug, Clone)]
pub struct CommandCertificate {
    executable: String,
}

impl CommandCertificate {
    /// Use `QR_DEPLOY_OPENSSL` when it names an executable, otherwise `openssl`.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            executable: std::env::var("QR_DEPLOY_OPENSSL").unwrap_or_else(|_| "openssl".to_owned()),
        }
    }
}

impl CertificateQuery for CommandCertificate {
    fn inspect(&self, path: &Path) -> Result<CertificateFacts, String> {
        let output = Command::new(&self.executable)
            .args([
                "x509",
                "-noout",
                "-checkend",
                "2592000",
                "-ext",
                "subjectAltName",
                "-in",
            ])
            .arg(path)
            .output()
            .map_err(|error| {
                format!(
                    "cannot run {} x509: {error}; the SAN and the 30-day validity were not checked",
                    self.executable
                )
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let detail = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or("the certificate was refused");
            return Err(format!(
                "openssl x509 refused the certificate: {}",
                truncate(detail, 160)
            ));
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let dns_names = parse_dns_names(&text);
        if dns_names.is_empty() {
            return Err("the certificate reports no DNS subject alternative name".to_owned());
        }
        Ok(CertificateFacts {
            dns_names,
            valid_for_days: true,
        })
    }
}

fn truncate(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}

/// Extract every `DNS:` name from `openssl x509 -ext subjectAltName` output.
fn parse_dns_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    for token in text.split(|character: char| {
        character == ',' || character == '\n' || character == '\r' || character == ' '
    }) {
        let token = token.trim();
        if let Some(name) = token.strip_prefix("DNS:")
            && !name.is_empty()
            && !names.iter().any(|existing| existing == name)
        {
            names.push(name.to_owned());
        }
    }
    names
}

/// The local ingress rows: static, dynamic, keepalived, and certificate.
pub(crate) fn local_rows(descriptor: &Descriptor) -> Vec<CheckRow> {
    local_rows_with(descriptor, &CommandCertificate::from_env())
}

/// The local ingress rows against an explicit certificate seam.
pub(crate) fn local_rows_with(
    descriptor: &Descriptor,
    certificate: &dyn CertificateQuery,
) -> Vec<CheckRow> {
    let mut rows = vec![static_row(), dynamic_row(descriptor)];
    rows.extend(keepalived_rows(descriptor));
    rows.push(certificate_files_row(descriptor));
    if descriptor.ingress.tls == IngressTls::Cert {
        rows.push(certificate_query_row(descriptor, certificate));
    }
    rows
}

fn static_row() -> CheckRow {
    let issues = static_issues(ingress::STATIC_CONFIG);
    if issues.is_empty() {
        CheckRow::ok(
            "ingress",
            "traefik static",
            "443-only",
            "one websecure 443 entrypoint; no port 80; dashboard/API off; internal metrics on 8082; ACME off; TLS 1.2+; 900 s timeouts; Authorization dropped",
        )
    } else {
        CheckRow::failed(
            "ingress",
            "traefik static",
            "configuration",
            issues.join("; "),
        )
    }
}

/// The static rules, as a pure function so a test can mutate one field.
fn static_issues(text: &str) -> Vec<String> {
    let mut issues = Vec::new();
    if !text.contains("\n  websecure:") || text.matches("\n  websecure:").count() != 1 {
        issues.push("expected exactly one websecure entrypoint".to_owned());
    }
    if !text.contains("address: \":443\"") {
        issues.push("the websecure entrypoint is not on 443".to_owned());
    }
    for forbidden in [
        "address: \":80\"",
        "\n  web:",
        "acme:",
        "certificatesResolvers:",
    ] {
        if text.contains(forbidden) {
            issues.push(format!(
                "found forbidden static configuration {forbidden:?}"
            ));
        }
    }
    for expected in [
        "dashboard: false",
        "insecure: false",
        "minVersion: VersionTLS12",
        "readTimeout: 900s",
        "writeTimeout: 900s",
        "idleTimeout: 900s",
        "format: json",
        "Authorization: drop",
        "certFile: /etc/traefik/certs/ingress.crt",
        "keyFile: /etc/traefik/certs/ingress.key",
        "metrics:",
        "address: \":8082\"",
        "entryPoint: metrics",
    ] {
        if !text.contains(expected) {
            issues.push(format!("missing static configuration {expected:?}"));
        }
    }
    issues
}

fn dynamic_row(descriptor: &Descriptor) -> CheckRow {
    let issues = dynamic_issues(&ingress::dynamic_yaml(descriptor), descriptor);
    if issues.is_empty() {
        CheckRow::ok(
            "ingress",
            "traefik dynamic",
            "router",
            format!(
                "Host({}) on websecure; tls; gateway {}; HSTS; nosniff; no-referrer; Server stripped; 512 MiB body",
                descriptor.ingress.dns,
                format_args!("http://{}_gateway:8080", stack_name(descriptor))
            ),
        )
    } else {
        CheckRow::failed(
            "ingress",
            "traefik dynamic",
            "configuration",
            issues.join("; "),
        )
    }
}

fn dynamic_issues(text: &str, descriptor: &Descriptor) -> Vec<String> {
    let mut issues = Vec::new();
    for expected in [
        format!("Host(`{}`)", descriptor.ingress.dns),
        "websecure".to_owned(),
        "tls: {}".to_owned(),
        format!("http://{}_gateway:8080", stack_name(descriptor)),
        "stsSeconds: 31536000".to_owned(),
        "stsIncludeSubdomains: true".to_owned(),
        "stsPreload: false".to_owned(),
        "contentTypeNosniff: true".to_owned(),
        "referrerPolicy: \"no-referrer\"".to_owned(),
        "Server: \"\"".to_owned(),
        "maxRequestBodyBytes: 536870912".to_owned(),
    ] {
        if !text.contains(&expected) {
            issues.push(format!("missing dynamic configuration {expected:?}"));
        }
    }
    if text.contains("basicAuth") {
        issues.push("found an ingress authentication middleware".to_owned());
    }
    issues
}

fn keepalived_rows(descriptor: &Descriptor) -> Vec<CheckRow> {
    let mut rows = Vec::new();
    for (index, host) in descriptor.ingress.hosts.iter().enumerate() {
        let subject = format!("keepalived {host}");
        match keepalived_issues(descriptor, host, index) {
            Ok(issues) if issues.is_empty() => rows.push(CheckRow::ok(
                "ingress",
                subject,
                "vrrp",
                "VIP/32 on the descriptor interface; unicast peer; priority 101/100; 443 nc check fall 3/rise 2",
            )),
            Ok(issues) => rows.push(CheckRow::failed("ingress", subject, "configuration", issues.join("; "))),
            Err(error) => rows.push(CheckRow::failed("ingress", subject, "render", error)),
        }
    }
    rows
}

fn keepalived_issues(
    descriptor: &Descriptor,
    host: &str,
    index: usize,
) -> Result<Vec<String>, String> {
    let text =
        ingress::keepalived_conf(descriptor, host, index).map_err(|error| error.to_string())?;
    Ok(keepalived_text_issues(&text, descriptor, host, index))
}

fn keepalived_text_issues(
    text: &str,
    descriptor: &Descriptor,
    host: &str,
    index: usize,
) -> Vec<String> {
    let mut issues = Vec::new();
    let expected_state = if index == 0 {
        "state MASTER"
    } else {
        "state BACKUP"
    };
    let expected_priority = if index == 0 {
        "priority 101"
    } else {
        "priority 100"
    };
    for expected in [
        expected_state,
        expected_priority,
        "virtual_router_id 51",
        "advert_int 1",
        "nc -z 127.0.0.1 443",
        "fall 3",
        "rise 2",
        "track_script",
    ] {
        if !text.contains(expected) {
            issues.push(format!("missing {expected}"));
        }
    }
    let interface = &descriptor.ingress.interface;
    if !text.contains(&format!("interface {interface}")) {
        issues.push(format!("missing interface {interface}"));
    }
    if let Some(vip) = descriptor.ingress.vip.as_deref()
        && !text.contains(&format!("{vip}/32 dev {interface}"))
    {
        issues.push(format!("missing {vip}/32 on {interface}"));
    }
    if let Some(own) = descriptor
        .hosts
        .get(host)
        .and_then(|value| value.public_address.as_deref())
        && !text.contains(&format!("unicast_src_ip {own}"))
    {
        issues.push(format!("missing unicast source {own}"));
    }
    for peer in &descriptor.ingress.hosts {
        if peer == host {
            continue;
        }
        if let Some(address) = descriptor
            .hosts
            .get(peer)
            .and_then(|value| value.public_address.as_deref())
            && !text.contains(address)
        {
            issues.push(format!("missing unicast peer {address}"));
        }
    }
    issues
}

fn certificate_files_row(descriptor: &Descriptor) -> CheckRow {
    if descriptor.ingress.tls != IngressTls::Cert {
        return CheckRow::ok(
            "ingress",
            "certificate files",
            "limited",
            "cert mode is the v1.5.10 path; a non-cert descriptor is refused before this row",
        );
    }
    let mut issues = Vec::new();
    let certificate = descriptor.ingress.files.get("certificate");
    let key = descriptor.ingress.files.get("key");
    match certificate {
        Some(path) => check_public_mode(path, "certificate", &mut issues),
        None => issues.push("missing ingress.files.certificate".to_owned()),
    }
    match key {
        Some(path) => check_key_mode(path, &mut issues),
        None => issues.push("missing ingress.files.key".to_owned()),
    }
    if issues.is_empty() {
        CheckRow::ok(
            "ingress",
            "certificate files",
            "mode",
            "certificate readable and not group/world-writable; key mode 600",
        )
    } else {
        CheckRow::failed("ingress", "certificate files", "file", issues.join("; "))
    }
}

fn file_metadata(path: &str, label: &str, issues: &mut Vec<String>) -> Option<std::fs::Metadata> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            issues.push(format!("cannot read the {label} {path}: {error}"));
            return None;
        }
    };
    if metadata.file_type().is_symlink() {
        issues.push(format!("the {label} {path} is a symlink"));
        return None;
    }
    if !metadata.is_file() {
        issues.push(format!("the {label} {path} is not a regular file"));
        return None;
    }
    Some(metadata)
}

fn check_public_mode(path: &str, label: &str, issues: &mut Vec<String>) {
    let metadata = file_metadata(path, label, issues);
    #[cfg(unix)]
    if let Some(metadata) = metadata {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode & 0o400 == 0 {
            issues.push(format!("the {label} {path} is not owner-readable"));
        }
        if mode & 0o022 != 0 {
            issues.push(format!(
                "the {label} {path} is group/world-writable (mode {mode:o})"
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = (metadata, path, label);
}

fn check_key_mode(path: &str, issues: &mut Vec<String>) {
    let metadata = file_metadata(path, "key", issues);
    #[cfg(unix)]
    if let Some(metadata) = metadata {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            issues.push(format!(
                "the ingress key {path} must be mode 600; found {mode:o}"
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = (metadata, path);
}

fn certificate_query_row(descriptor: &Descriptor, certificate: &dyn CertificateQuery) -> CheckRow {
    let Some(path) = descriptor.ingress.files.get("certificate") else {
        return CheckRow::failed(
            "ingress",
            "certificate",
            "missing",
            "ingress.files.certificate is not declared",
        );
    };
    match certificate.inspect(Path::new(path)) {
        Ok(facts) => {
            let mut issues = Vec::new();
            if !facts.valid_for_days {
                issues.push("the certificate expires within 30 days".to_owned());
            }
            if !facts
                .dns_names
                .iter()
                .any(|name| name.eq_ignore_ascii_case(&descriptor.ingress.dns))
            {
                issues.push(format!(
                    "the certificate SAN {:?} does not name {}",
                    facts.dns_names, descriptor.ingress.dns
                ));
            }
            if issues.is_empty() {
                CheckRow::ok(
                    "ingress",
                    "certificate",
                    "valid",
                    format!(
                        "SAN names {}; at least 30 days of validity",
                        descriptor.ingress.dns
                    ),
                )
            } else {
                CheckRow::failed("ingress", "certificate", "invalid", issues.join("; "))
            }
        }
        Err(error) => {
            let state = if error.starts_with("cannot run") {
                "limited"
            } else {
                "invalid"
            };
            CheckRow::failed("ingress", "certificate", state, error)
        }
    }
}

/// The probe rows: three HTTPS endpoints, the headers, then port-80 closure.
pub(crate) fn probe_rows(report: &mut CheckReport, descriptor: &Descriptor, probe: &dyn Probe) {
    let base = format!("https://{}", descriptor.ingress.dns);
    let mut https_ok = true;
    let mut health_headers: Option<BTreeMap<String, String>> = None;
    for (path, kind) in [
        ("/health", "health"),
        ("/ready", "ready"),
        ("/version", "version"),
    ] {
        let url = format!("{base}{path}");
        match probe.get(&url) {
            Err(error) => {
                https_ok = false;
                report.push(CheckRow::failed("ingress", url, kind, error));
            }
            Ok(response) if !(200..300).contains(&response.status) => {
                https_ok = false;
                report.push(CheckRow::failed(
                    "ingress",
                    url,
                    kind,
                    format!("HTTP {}", response.status),
                ));
            }
            Ok(response) if kind == "version" => {
                match parse_version(&response.body, &descriptor.image.digest) {
                    Ok(()) => {
                        let mut row = CheckRow::ok("ingress", url, kind, "digest matches");
                        row.digest = Some(descriptor.image.digest.clone());
                        report.push(row);
                    }
                    Err(error) => {
                        https_ok = false;
                        report.push(CheckRow::failed("ingress", url, kind, error));
                    }
                }
            }
            Ok(response) => match parse_health(&response.body) {
                Ok(state) if state == "healthy" => {
                    report.push(CheckRow::ok("ingress", url, kind, state));
                    if kind == "health" {
                        health_headers = Some(response.headers);
                    }
                }
                Ok(state) => {
                    https_ok = false;
                    report.push(CheckRow::failed(
                        "ingress",
                        url,
                        kind,
                        format!("state is {state}"),
                    ));
                }
                Err(error) => {
                    https_ok = false;
                    report.push(CheckRow::failed("ingress", url, kind, error));
                }
            },
        }
    }

    match health_headers {
        Some(headers) => {
            let issues = header_issues(&headers);
            if issues.is_empty() {
                report.push(CheckRow::ok(
                    "ingress",
                    "response headers",
                    "hardened",
                    "HSTS one year with includeSubDomains and no preload; nosniff; no-referrer; Server absent",
                ));
            } else {
                https_ok = false;
                report.push(CheckRow::failed(
                    "ingress",
                    "response headers",
                    "header",
                    issues.join("; "),
                ));
            }
        }
        None => https_ok = false,
    }

    let http_url = format!("http://{}/", descriptor.ingress.dns);
    match probe.get(&http_url) {
        Ok(response) => report.push(CheckRow::failed(
            "ingress",
            http_url,
            "port-80",
            format!("port 80 answered HTTP {}", response.status),
        )),
        Err(error) if error.starts_with(FIXTURE_MISSING) => report.push(CheckRow::failed(
            "ingress",
            http_url,
            "port-80",
            "the recorded probe fixture has no plain-HTTP entry; refusal is not proven",
        )),
        Err(_) if https_ok => report.push(CheckRow::ok(
            "ingress",
            http_url,
            "port-80",
            "no HTTP response; the transport failed as expected",
        )),
        Err(_) => report.push(CheckRow::failed(
            "ingress",
            http_url,
            "not-attempted",
            "the HTTPS rows did not all pass; port-80 closure cannot be attributed to the ingress",
        )),
    }
}

fn header_issues(headers: &BTreeMap<String, String>) -> Vec<String> {
    let mut issues = Vec::new();
    let hsts = headers
        .get("strict-transport-security")
        .map(String::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !hsts.contains("max-age=31536000") || !hsts.contains("includesubdomains") {
        issues.push("HSTS is missing one year with includeSubDomains".to_owned());
    }
    if hsts.contains("preload") {
        issues.push("HSTS advertises preload".to_owned());
    }
    if headers.get("x-content-type-options").map(String::as_str) != Some("nosniff") {
        issues.push("X-Content-Type-Options is not nosniff".to_owned());
    }
    if headers.get("referrer-policy").map(String::as_str) != Some("no-referrer") {
        issues.push("Referrer-Policy is not no-referrer".to_owned());
    }
    if headers.get("server").is_some_and(|value| !value.is_empty()) {
        issues.push("the Server header is present".to_owned());
    }
    issues
}

#[derive(Deserialize)]
struct VersionBody {
    #[serde(default)]
    image_digest: Option<String>,
}

fn parse_version(body: &[u8], expected: &str) -> Result<(), String> {
    let version: VersionBody =
        serde_json::from_slice(body).map_err(|error| format!("invalid JSON: {error}"))?;
    match version.image_digest.as_deref() {
        Some(actual) if actual == expected => Ok(()),
        Some(actual) => Err(format!("image digest {actual} does not match {expected}")),
        None => Err("version response has no image_digest".to_owned()),
    }
}

#[derive(Deserialize)]
struct HealthBody {
    state: String,
}

fn parse_health(body: &[u8]) -> Result<String, String> {
    let health: HealthBody =
        serde_json::from_slice(body).map_err(|error| format!("invalid JSON: {error}"))?;
    Ok(health.state)
}

/// The remote ingress rows: the four mounts and the lab-only exposure row.
pub(crate) fn remote_rows(descriptor: &Descriptor, docker: &dyn DockerQuery) -> Vec<CheckRow> {
    let name = format!("{}_ingress", stack_name(descriptor));
    let mut rows = Vec::new();
    match docker.service_inspect(&name) {
        Ok(text) => rows.push(mounts_row(descriptor, &text)),
        Err(error) => rows.push(CheckRow::failed(
            "ingress",
            format!("service {name} mounts"),
            "missing",
            format!("deploy.remote.docker: {error}"),
        )),
    }
    rows.push(CheckRow::ok(
        "ingress",
        "vip/firewall",
        "limited",
        "VIP ownership and firewall exposure are lab-only; deploy/v3/test.sh --confirm checks them",
    ));
    rows
}

fn mounts_row(descriptor: &Descriptor, text: &str) -> CheckRow {
    let value: serde_json::Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => {
            return CheckRow::failed(
                "ingress",
                "service mounts",
                "inspect",
                format!("cannot parse the ingress inspect output: {error}"),
            );
        }
    };
    let value = match value {
        serde_json::Value::Array(mut values) if values.len() == 1 => values.remove(0),
        other => other,
    };
    let configs = value.pointer("/Spec/TaskTemplate/ContainerSpec/Configs");
    let secrets = value.pointer("/Spec/TaskTemplate/ContainerSpec/Secrets");
    let config_map = mount_map(configs);
    let secret_map = mount_map(secrets);
    let mut expected = vec![
        (TRAEFIK_CONFIG, "/etc/traefik/traefik.yml"),
        (TRAEFIK_DYNAMIC_CONFIG, "/etc/traefik/dynamic/dynamic.yml"),
    ];
    if descriptor.ingress.tls == IngressTls::Cert {
        expected.push((INGRESS_CERT_CONFIG, "/etc/traefik/certs/ingress.crt"));
        expected.push((INGRESS_KEY_SECRET, "/etc/traefik/certs/ingress.key"));
    }
    let mut issues = Vec::new();
    for (source, target) in &expected {
        let actual = config_map.get(*source).or_else(|| secret_map.get(*source));
        match actual {
            Some(found) if found == target => {}
            Some(found) => {
                issues.push(format!("{source} is mounted at {found}, expected {target}"))
            }
            None => issues.push(format!("{source} is not mounted")),
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "ingress",
            "service mounts",
            "mounted",
            format!(
                "{} objects at the documented config and cert targets",
                expected.len()
            ),
        )
    } else {
        CheckRow::failed("ingress", "service mounts", "mount", issues.join("; "))
    }
}

fn mount_map(entries: Option<&serde_json::Value>) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(array) = entries.and_then(serde_json::Value::as_array) else {
        return map;
    };
    for entry in array {
        let name = entry
            .get("ConfigName")
            .or_else(|| entry.get("SecretName"))
            .or_else(|| entry.get("Source"))
            .or_else(|| entry.get("Name"))
            .and_then(serde_json::Value::as_str);
        let target = entry
            .pointer("/File/Name")
            .or_else(|| entry.get("Target"))
            .and_then(serde_json::Value::as_str);
        if let (Some(name), Some(target)) = (name, target) {
            map.insert(name.to_owned(), target.to_owned());
        }
    }
    map
}

#[cfg(test)]
fn temp_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("qr-deploy-ingress-{}-{name}", std::process::id()))
}

#[cfg(test)]
mod tests;
