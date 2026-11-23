//! Network, ingress and TLS endpoint rules for `quantrule.deploy/v3`.
//!
//! The profile has one public port and one public name. Every stateful client
//! endpoint is TLS with an absolute CA path; a plaintext or userinfo-bearing
//! endpoint is refused before a service starts.

use std::collections::BTreeSet;
use std::net::Ipv4Addr;
use std::path::Path;

use crate::descriptor::v3::{Descriptor, IngressTls};
use crate::error::Error;

use super::secrets;
use super::v3::{host_by_name, is_identifier, rule};

/// Validate the overlay, the one ingress, and the three stateful endpoints.
///
/// # Errors
///
/// Returns [`Error::Rule`] with a stable code.
pub fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    validate_network(descriptor)?;
    validate_ingress(descriptor)?;
    validate_endpoints(descriptor)?;
    Ok(())
}

fn validate_network(descriptor: &Descriptor) -> Result<(), Error> {
    if !is_identifier(&descriptor.network.overlay) {
        return Err(rule(
            "deploy.network.overlay",
            "network.overlay",
            "must be a lowercase overlay name",
        ));
    }
    if !descriptor.network.encrypted {
        return Err(rule(
            "deploy.network.encrypted",
            "network.encrypted",
            "must be true; the overlay is the internal trust boundary",
        ));
    }
    Ok(())
}

fn validate_ingress(descriptor: &Descriptor) -> Result<(), Error> {
    let ingress = &descriptor.ingress;
    if ingress.dns != "quantrule.dev" {
        return Err(rule(
            "deploy.ingress.dns",
            "ingress.dns",
            "must be quantrule.dev",
        ));
    }
    if ingress.port != 443 {
        return Err(rule(
            "deploy.ingress.port",
            "ingress.port",
            "must be 443; no other public port is supported",
        ));
    }
    let vip = validate_vip(ingress.vip.as_deref())?;
    validate_interface(&ingress.interface)?;
    match ingress.tls {
        IngressTls::Acme => {
            return Err(rule(
                "deploy.ingress.tls",
                "ingress.tls",
                "v1.5.10 requires tls: cert; acme is reserved for a future path and is not implemented",
            ));
        }
        IngressTls::Cert => {
            let names: BTreeSet<&str> = ingress.files.keys().map(String::as_str).collect();
            if names != BTreeSet::from(["certificate", "key"]) {
                return Err(rule(
                    "deploy.ingress.files",
                    "ingress.files",
                    "cert mode needs exactly the certificate and key files",
                ));
            }
            for (name, path) in &ingress.files {
                validate_file_path(path, &format!("ingress.files.{name}"))?;
            }
        }
    }
    if ingress.hosts.is_empty() {
        return Err(rule(
            "deploy.ingress.hosts",
            "ingress.hosts",
            "must name at least one host",
        ));
    }
    let mut seen = BTreeSet::new();
    for host_name in &ingress.hosts {
        if !seen.insert(host_name.as_str()) {
            return Err(rule(
                "deploy.ingress.hosts",
                "ingress.hosts",
                format!("{host_name} is listed twice"),
            ));
        }
        let host = host_by_name(descriptor, host_name, "ingress.hosts")?;
        if host.public_address.is_none() {
            return Err(rule(
                "deploy.ingress.public_address",
                format!("hosts.{host_name}.public_address"),
                "an ingress host must declare the public address DNS resolves to",
            ));
        }
        if host
            .public_address
            .as_deref()
            .and_then(|public| public.parse::<Ipv4Addr>().ok())
            == Some(vip)
        {
            return Err(rule(
                "deploy.ingress.vip",
                "ingress.vip",
                format!("must not equal the public address of ingress host {host_name}"),
            ));
        }
    }
    Ok(())
}

/// Parse the floating IPv4 address and refuse addresses that cannot be a VIP.
fn validate_vip(value: Option<&str>) -> Result<Ipv4Addr, Error> {
    let Some(value) = value else {
        return Err(rule(
            "deploy.ingress.vip",
            "ingress.vip",
            "must declare the floating IPv4 address",
        ));
    };
    let vip = value.parse::<Ipv4Addr>().map_err(|_| {
        rule(
            "deploy.ingress.vip",
            "ingress.vip",
            format!("{value} is not an IPv4 address"),
        )
    })?;
    if vip.is_unspecified() {
        return Err(rule(
            "deploy.ingress.vip",
            "ingress.vip",
            "must not be 0.0.0.0",
        ));
    }
    if vip.is_loopback() {
        return Err(rule(
            "deploy.ingress.vip",
            "ingress.vip",
            "must not be a loopback address",
        ));
    }
    if vip.is_multicast() {
        return Err(rule(
            "deploy.ingress.vip",
            "ingress.vip",
            "must not be a multicast address",
        ));
    }
    if vip == Ipv4Addr::BROADCAST {
        return Err(rule(
            "deploy.ingress.vip",
            "ingress.vip",
            "must not be the limited broadcast address",
        ));
    }
    Ok(vip)
}

/// A simple Linux interface name: 1–15 ASCII letters, digits, `_`, `-`, or `.`.
fn validate_interface(value: &str) -> Result<(), Error> {
    if value.is_empty()
        || value.len() > 15
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(rule(
            "deploy.ingress.interface",
            "ingress.interface",
            "must be 1-15 ASCII letters, digits, underscores, hyphens, or dots",
        ));
    }
    Ok(())
}

fn validate_endpoints(descriptor: &Descriptor) -> Result<(), Error> {
    let database = &descriptor.endpoints.database;
    validate_endpoint_host(&database.host, "endpoints.database.host")?;
    if database.port != 5432 {
        return Err(rule(
            "deploy.endpoint.database.port",
            "endpoints.database.port",
            "must be 5432",
        ));
    }
    if database.sslmode != "verify-full" {
        return Err(rule(
            "deploy.endpoint.database.sslmode",
            "endpoints.database.sslmode",
            "must be verify-full; prefer, require and verify-ca are refused",
        ));
    }
    validate_ca(&database.tls_ca, "endpoints.database.tls_ca")?;

    let queue = &descriptor.endpoints.queue;
    validate_endpoint_host(&queue.host, "endpoints.queue.host")?;
    if queue.host != "rabbitmq" {
        return Err(rule(
            "deploy.endpoint.queue.host",
            "endpoints.queue.host",
            "must be the shared rabbitmq alias served by the HAProxy pair",
        ));
    }
    if queue.port != 5671 {
        return Err(rule(
            "deploy.endpoint.queue.port",
            "endpoints.queue.port",
            "must be 5671; plaintext 5672 is refused",
        ));
    }
    if queue.scheme != "amqps" {
        return Err(rule(
            "deploy.endpoint.queue.scheme",
            "endpoints.queue.scheme",
            "must be amqps",
        ));
    }
    validate_ca(&queue.tls_ca, "endpoints.queue.tls_ca")?;

    let object_store = &descriptor.endpoints.object_store;
    validate_endpoint_host(&object_store.host, "endpoints.object_store.host")?;
    if object_store.port != 9000 {
        return Err(rule(
            "deploy.endpoint.object_store.port",
            "endpoints.object_store.port",
            "must be 9000",
        ));
    }
    if object_store.scheme != "https" {
        return Err(rule(
            "deploy.endpoint.object_store.scheme",
            "endpoints.object_store.scheme",
            "must be https",
        ));
    }
    validate_ca(&object_store.tls_ca, "endpoints.object_store.tls_ca")?;
    Ok(())
}

fn validate_endpoint_host(host: &str, field: &str) -> Result<(), Error> {
    if host.trim().is_empty() {
        return Err(rule(
            "deploy.endpoint.host",
            field,
            "must name a host or service",
        ));
    }
    if host.chars().any(char::is_whitespace)
        || host.contains('@')
        || host.contains('/')
        || host.contains("://")
    {
        return Err(rule(
            "deploy.endpoint.host",
            field,
            "must be a host or service name without userinfo or a scheme",
        ));
    }
    Ok(())
}

fn validate_ca(path: &str, field: &str) -> Result<(), Error> {
    validate_file_path(path, field)?;
    if path.contains("${") {
        return Err(rule(
            "deploy.endpoint.tls_ca",
            field,
            "must be a literal path; CA bundles are not placeholders",
        ));
    }
    Ok(())
}

fn validate_file_path(path: &str, field: &str) -> Result<(), Error> {
    if path.trim().is_empty() || !Path::new(path).is_absolute() {
        return Err(rule(
            "deploy.file.absolute",
            field,
            "must be an absolute path",
        ));
    }
    if path.chars().any(char::is_whitespace) {
        return Err(rule(
            "deploy.file.whitespace",
            field,
            "must not contain whitespace",
        ));
    }
    Ok(())
}

/// The ingress certificate material, read once for validation and rendering.
#[derive(Debug, Default)]
pub struct IngressFiles {
    /// The public certificate chain, when `tls` is `cert`.
    pub certificate: Option<String>,
    /// The private key, when `tls` is `cert`.
    pub key: Option<String>,
}

/// Check the ingress certificate files, when cert mode is selected.
///
/// # Errors
///
/// Returns [`Error::Rule`] for a missing, non-regular, oversized or unsafe
/// file, and for a private key that is not mode 600.
#[cfg(test)]
pub fn validate_files(descriptor: &Descriptor) -> Result<(), Error> {
    load_files(descriptor).map(|_| ())
}

/// Read the ingress certificate material once.
///
/// The renderer writes the value this function returns; no second reader opens
/// the source file.
///
/// # Errors
///
/// Returns the same failures as the local file validation.
pub fn load_files(descriptor: &Descriptor) -> Result<IngressFiles, Error> {
    if descriptor.ingress.tls != IngressTls::Cert {
        return Ok(IngressFiles::default());
    }
    let mut material = IngressFiles::default();
    for (name, path) in &descriptor.ingress.files {
        let field = format!("ingress.files.{name}");
        if name == "key" {
            material.key = Some(secrets::read_pem_file(path, &field, true)?);
        } else {
            material.certificate = Some(secrets::read_public_file(path, &field)?);
        }
    }
    Ok(material)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::v3::Descriptor;
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qr-deploy-v3-network-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("temp dir");
        path
    }

    fn write(path: &Path, content: &str, mode: u32) {
        std::fs::write(path, content).expect("write file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
                .expect("set mode");
        }
        #[cfg(not(unix))]
        let _ = mode;
    }

    fn cert_descriptor(directory: &Path) -> Descriptor {
        let mut value: Value =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                .expect("sample");
        value["ingress"]["tls"] = serde_json::json!("cert");
        value["ingress"]["files"] = serde_json::json!({
            "certificate": directory.join("fullchain.pem").display().to_string(),
            "key": directory.join("privkey.pem").display().to_string(),
        });
        serde_json::from_value(value).expect("descriptor")
    }

    #[test]
    fn cert_mode_checks_the_certificate_and_private_key_mode() {
        let directory = temp_dir();
        write(
            &directory.join("fullchain.pem"),
            "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
            0o644,
        );
        write(&directory.join("privkey.pem"), "key-fixture\n", 0o600);
        let descriptor = cert_descriptor(&directory);
        validate_files(&descriptor).expect("a cert and a mode-600 key");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                directory.join("privkey.pem"),
                std::fs::Permissions::from_mode(0o644),
            )
            .expect("mode 644");
        }
        let error = validate_files(&descriptor).expect_err("a readable key");
        assert!(error.to_string().contains("deploy.secret.mode"), "{error}");
        let _ = std::fs::remove_dir_all(directory);
    }
}
