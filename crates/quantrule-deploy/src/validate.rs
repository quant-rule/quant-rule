//! Descriptor validation and placeholder expansion.
//!
//! Validation is deliberately strict: a descriptor that reaches a host has
//! already been checked for one image digest, one public service, private
//! internal binds, unique host ports, and required secrets.

use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;

use crate::descriptor::{CONTRACT, Descriptor, PROFILE, REQUIRED_SERVICES, Service, Tls};
use crate::error::Error;

/// Validate the descriptor without expanding secrets.
///
/// # Errors
///
/// Returns the first violation as [`Error::Invalid`].
pub fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    if descriptor.v != CONTRACT {
        return Err(invalid("v", format!("must be {CONTRACT}")));
    }
    if descriptor.profile != PROFILE {
        return Err(invalid("profile", format!("must be {PROFILE}")));
    }
    validate_image(descriptor)?;
    validate_hosts(descriptor)?;
    validate_services(descriptor)?;
    validate_endpoints(descriptor)?;
    validate_auth(descriptor)?;
    match descriptor.exposure.internal {
        crate::descriptor::InternalExposure::Private => {}
    }
    if descriptor.config_root != "/srv/config" {
        return Err(invalid(
            "config_root",
            "must be /srv/config; the data Compose mount is fixed",
        ));
    }
    if descriptor.secrets_file.trim().is_empty() {
        return Err(invalid("secrets_file", "must not be empty"));
    }
    if !descriptor.hosts.contains_key(&descriptor.infra_host) {
        return Err(invalid(
            "infra_host",
            format!("{} is not a declared host", descriptor.infra_host),
        ));
    }
    let infra_address = host_address(descriptor, &descriptor.infra_host)?;
    if !is_private_ip(infra_address) {
        return Err(invalid(
            "infra_host",
            "the infrastructure host must use a private address",
        ));
    }
    let core = &descriptor.services["data"].host;
    for name in ["evidence", "simulation"] {
        if &descriptor.services[name].host != core {
            return Err(invalid(
                format!("services.{name}.host"),
                "data, evidence, and simulation must share the core host",
            ));
        }
    }
    let compute = &descriptor.services["model"].host;
    if &descriptor.services["backtest"].host != compute {
        return Err(invalid(
            "services.backtest.host",
            "model and backtest must share the compute host",
        ));
    }
    let gateway_host = &descriptor.services["gateway"].host;
    if gateway_host == core || gateway_host == compute || gateway_host == &descriptor.infra_host {
        return Err(invalid(
            "services.gateway.host",
            "the gateway must not share a role host",
        ));
    }
    for name in ["data", "evidence", "simulation", "model", "backtest"] {
        if descriptor.services[name].host == descriptor.infra_host {
            return Err(invalid(
                format!("services.{name}.host"),
                "a service must not run on the infrastructure host",
            ));
        }
    }
    if let Some(value) = descriptor.backtest_concurrency
        && value == 0
    {
        return Err(invalid("backtest_concurrency", "must be at least one"));
    }
    if let Some(value) = descriptor.stage_retention_days
        && value == 0
    {
        return Err(invalid("stage_retention_days", "must be at least one"));
    }
    if let Some(bucket) = &descriptor.endpoints.object_bucket
        && (bucket.trim().is_empty()
            || !bucket
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character)))
    {
        return Err(invalid("endpoints.object_bucket", "must be a bucket name"));
    }
    Ok(())
}

/// Validate the descriptor after expanding its placeholders.
///
/// # Errors
///
/// Returns [`Error::MissingSecret`] for an undefined placeholder and
/// [`Error::Invalid`] for a default or malformed expanded value.
pub fn validate_with_secrets(
    descriptor: &Descriptor,
    secrets: &BTreeMap<String, String>,
) -> Result<(), Error> {
    validate(descriptor)?;
    let database = expand(&descriptor.endpoints.database, secrets)?;
    let queue = expand(&descriptor.endpoints.queue, secrets)?;
    let object_store = expand(&descriptor.endpoints.object_store, secrets)?;
    check_default_credentials("endpoints.database", &database)?;
    check_default_credentials("endpoints.queue", &queue)?;
    check_default_credentials("endpoints.object_store", &object_store)?;
    check_endpoint_targets(descriptor, &database, &queue, &object_store)?;
    check_secret_quality(secrets)?;
    if descriptor.auth.gateway_token
        && secrets
            .get("QR_BEARER_TOKEN")
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(Error::MissingSecret {
            name: "QR_BEARER_TOKEN".to_owned(),
        });
    }
    for key in [
        "QR_POSTGRES_USER",
        "QR_POSTGRES_PASSWORD",
        "QR_POSTGRES_DB",
        "QR_OBJECT_STORE_USER",
        "QR_OBJECT_STORE_PASSWORD",
        "QR_QUEUE_USER",
        "QR_QUEUE_PASSWORD",
        "QR_QUEUE_VHOST",
    ] {
        if secrets.get(key).is_none_or(|value| value.trim().is_empty()) {
            return Err(Error::MissingSecret {
                name: key.to_owned(),
            });
        }
    }
    Ok(())
}

/// Expand `${NAME}` placeholders from a secrets map.
///
/// # Errors
///
/// Returns [`Error::MissingSecret`] when a placeholder has no value and
/// [`Error::Invalid`] when a placeholder is malformed.
pub fn expand(value: &str, secrets: &BTreeMap<String, String>) -> Result<String, Error> {
    let mut result = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            return Err(invalid("endpoints", "unclosed ${ placeholder"));
        };
        let name = &after[..end];
        if name.is_empty()
            || !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(invalid(
                "endpoints",
                format!("invalid placeholder ${{{name}}}"),
            ));
        }
        let Some(replacement) = secrets.get(name) else {
            return Err(Error::MissingSecret {
                name: name.to_owned(),
            });
        };
        result.push_str(replacement);
        rest = &after[end + 1..];
    }
    result.push_str(rest);
    Ok(result)
}

/// The private, loopback, link-local, CGNAT, and unique-local check shared with
/// the runtime bind guard.
#[must_use]
pub fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => {
            let octets = address.octets();
            address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        }
        IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.to_ipv4_mapped().is_some_and(|mapped| {
                    let octets = mapped.octets();
                    mapped.is_private()
                        || mapped.is_loopback()
                        || mapped.is_link_local()
                        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                })
        }
    }
}

fn validate_image(descriptor: &Descriptor) -> Result<(), Error> {
    if descriptor.image.reference.trim().is_empty()
        || descriptor.image.reference.chars().any(char::is_whitespace)
    {
        return Err(invalid(
            "image.reference",
            "must be a non-empty image reference",
        ));
    }
    let digest = &descriptor.image.digest;
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(invalid("image.digest", "must start with sha256:"));
    };
    if hex.len() != 64 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(invalid("image.digest", "must be a 64-character hex digest"));
    }
    if hex.chars().any(|character| character.is_ascii_uppercase()) {
        return Err(invalid("image.digest", "must use lowercase hex"));
    }
    let first = hex.chars().next().unwrap_or('0');
    if hex.chars().all(|character| character == first) {
        return Err(invalid(
            "image.digest",
            "must not be a placeholder made of one repeated character",
        ));
    }
    Ok(())
}

fn validate_hosts(descriptor: &Descriptor) -> Result<(), Error> {
    if descriptor.hosts.is_empty() {
        return Err(invalid("hosts", "must declare at least one host"));
    }
    let mut addresses = BTreeSet::new();
    for (name, host) in &descriptor.hosts {
        if name.trim().is_empty() {
            return Err(invalid("hosts", "a host key is empty"));
        }
        let address = host.address.parse::<IpAddr>().map_err(|_| {
            invalid(
                format!("hosts.{name}.address"),
                format!("{} is not an IP address", host.address),
            )
        })?;
        if address.is_unspecified() {
            return Err(invalid(
                format!("hosts.{name}.address"),
                "must not be 0.0.0.0 or ::",
            ));
        }
        if address.is_ipv6() {
            return Err(invalid(
                format!("hosts.{name}.address"),
                "IPv6 is not supported in deploy/v2; use an IPv4 overlay",
            ));
        }
        if !addresses.insert(address) {
            return Err(invalid(
                format!("hosts.{name}.address"),
                "a second host uses the same address",
            ));
        }
    }
    Ok(())
}

/// Parse a declared host address.
fn host_address(descriptor: &Descriptor, name: &str) -> Result<IpAddr, Error> {
    descriptor.hosts[name]
        .address
        .parse::<IpAddr>()
        .map_err(|_| invalid(format!("hosts.{name}.address"), "host address is not an IP"))
}

fn validate_services(descriptor: &Descriptor) -> Result<(), Error> {
    let names: BTreeSet<&str> = descriptor.services.keys().map(String::as_str).collect();
    for required in REQUIRED_SERVICES {
        if !names.contains(required) {
            return Err(invalid("services", format!("missing {required}")));
        }
    }
    for name in &names {
        if !REQUIRED_SERVICES.contains(name) {
            return Err(invalid("services", format!("unknown service {name}")));
        }
    }
    let mut ports = BTreeSet::new();
    for (name, service) in &descriptor.services {
        validate_service(descriptor, name, service)?;
        if let Some(port) = service.port
            && !ports.insert((service.host.clone(), port))
        {
            return Err(invalid(
                format!("services.{name}.port"),
                "another service on this host already uses the port",
            ));
        }
    }
    Ok(())
}

fn validate_service(descriptor: &Descriptor, name: &str, service: &Service) -> Result<(), Error> {
    if !descriptor.hosts.contains_key(&service.host) {
        return Err(invalid(
            format!("services.{name}.host"),
            format!("{} is not a declared host", service.host),
        ));
    }
    if service.replicas == 0 {
        return Err(invalid(
            format!("services.{name}.replicas"),
            "must be at least one",
        ));
    }
    if service.port == Some(0) {
        return Err(invalid(
            format!("services.{name}.port"),
            "must be between 1 and 65535",
        ));
    }
    let host = &descriptor.hosts[&service.host];
    let address = host
        .address
        .parse::<IpAddr>()
        .map_err(|_| invalid(format!("services.{name}.host"), "host address is not an IP"))?;
    match name {
        "gateway" => {
            if service.port.is_none() {
                return Err(invalid("services.gateway.port", "must be present"));
            }
            if service.replicas != 1 {
                return Err(invalid("services.gateway.replicas", "must be one"));
            }
            let published = matches!(
                descriptor.exposure.gateway,
                crate::descriptor::GatewayExposure::Lan
                    | crate::descriptor::GatewayExposure::Public
            );
            if service.public != published {
                return Err(invalid(
                    "services.gateway.public",
                    "must match exposure.gateway",
                ));
            }
            if descriptor.exposure.gateway != crate::descriptor::GatewayExposure::Public
                && !is_private_ip(address)
            {
                return Err(invalid(
                    "services.gateway.host",
                    "a LAN or private gateway must use a private address",
                ));
            }
        }
        "backtest" => {
            if service.port.is_some() {
                return Err(invalid("services.backtest.port", "must be absent"));
            }
            if service.public {
                return Err(invalid("services.backtest.public", "must be false"));
            }
            if !is_private_ip(address) {
                return Err(invalid(
                    "services.backtest.host",
                    "a backtest host must use a private address",
                ));
            }
        }
        _ => {
            if service.port.is_none() {
                return Err(invalid(format!("services.{name}.port"), "must be present"));
            }
            if service.public {
                return Err(invalid(
                    format!("services.{name}.public"),
                    "must be false; only the gateway is published",
                ));
            }
            if !is_private_ip(address) {
                return Err(invalid(
                    format!("services.{name}.host"),
                    "an internal service must use a private address",
                ));
            }
            if service.replicas != 1 {
                return Err(invalid(format!("services.{name}.replicas"), "must be one"));
            }
        }
    }
    Ok(())
}

fn validate_endpoints(descriptor: &Descriptor) -> Result<(), Error> {
    check_scheme(
        "endpoints.database",
        &descriptor.endpoints.database,
        &["postgres://", "postgresql://"],
    )?;
    check_scheme(
        "endpoints.object_store",
        &descriptor.endpoints.object_store,
        &["https://"],
    )?;
    check_scheme(
        "endpoints.queue",
        &descriptor.endpoints.queue,
        &["amqps://"],
    )?;
    if !descriptor
        .endpoints
        .database
        .contains("sslmode=verify-full")
    {
        return Err(invalid(
            "endpoints.database",
            "must carry sslmode=verify-full; prefer, require and verify-ca are refused",
        ));
    }
    for (field, value) in [
        ("endpoints.database", &descriptor.endpoints.database),
        ("endpoints.object_store", &descriptor.endpoints.object_store),
        ("endpoints.queue", &descriptor.endpoints.queue),
    ] {
        if value.chars().any(char::is_whitespace) {
            return Err(invalid(field, "must not contain whitespace"));
        }
        check_placeholders(field, value)?;
    }
    for (field, value) in [
        (
            "endpoints.database_tls_ca",
            &descriptor.endpoints.database_tls_ca,
        ),
        (
            "endpoints.object_store_tls_ca",
            &descriptor.endpoints.object_store_tls_ca,
        ),
        ("endpoints.queue_tls_ca", &descriptor.endpoints.queue_tls_ca),
    ] {
        if value.trim().is_empty() || !std::path::Path::new(value).is_absolute() {
            return Err(invalid(
                field,
                "must be an absolute path to a PEM CA bundle",
            ));
        }
        if value.chars().any(char::is_whitespace) {
            return Err(invalid(field, "must not contain whitespace"));
        }
        if value.contains("${") {
            return Err(invalid(
                field,
                "must be a literal path; CA files are not placeholders",
            ));
        }
    }
    Ok(())
}

fn validate_auth(descriptor: &Descriptor) -> Result<(), Error> {
    let exposed = matches!(
        descriptor.exposure.gateway,
        crate::descriptor::GatewayExposure::Lan | crate::descriptor::GatewayExposure::Public
    );
    if exposed && !descriptor.auth.gateway_token {
        return Err(invalid(
            "auth.gateway_token",
            "must be true for a LAN or public gateway",
        ));
    }
    if descriptor.exposure.gateway != crate::descriptor::GatewayExposure::Private
        && descriptor.auth.tls != Tls::ReverseProxy
    {
        return Err(invalid(
            "auth.tls",
            "must be reverse-proxy for a LAN or public gateway",
        ));
    }
    Ok(())
}

fn check_scheme(field: &str, value: &str, prefixes: &[&str]) -> Result<(), Error> {
    if prefixes.iter().any(|prefix| value.starts_with(prefix)) {
        Ok(())
    } else {
        Err(invalid(
            field,
            format!("must start with one of {}", prefixes.join(", ")),
        ))
    }
}

fn check_placeholders(field: &str, value: &str) -> Result<(), Error> {
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else {
            return Err(invalid(field, "unclosed ${ placeholder"));
        };
        let name = &after[..end];
        if name.is_empty()
            || !name
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(invalid(field, format!("invalid placeholder ${{{name}}}")));
        }
        rest = &after[end + 1..];
    }
    Ok(())
}

fn check_default_credentials(field: &str, value: &str) -> Result<(), Error> {
    let lower = value.to_ascii_lowercase();
    for marker in [
        ":quantrule@",
        ":change-me@",
        ":password@",
        "password=quantrule",
        "password=change-me",
    ] {
        if lower.contains(marker) {
            return Err(invalid(field, "uses a documented default credential"));
        }
    }
    Ok(())
}

fn check_endpoint_targets(
    descriptor: &Descriptor,
    database: &str,
    queue: &str,
    object_store: &str,
) -> Result<(), Error> {
    let infra = &descriptor.hosts[&descriptor.infra_host].address;
    if !database.contains(&format!("@{infra}:5432/")) {
        return Err(invalid(
            "endpoints.database",
            format!("must target {infra}:5432 on the infrastructure host"),
        ));
    }
    if !queue.contains(&format!("@{infra}:5671/")) {
        return Err(invalid(
            "endpoints.queue",
            format!("must target {infra}:5671 on the infrastructure host"),
        ));
    }
    if !object_store.contains(&format!("://{infra}:9000")) {
        return Err(invalid(
            "endpoints.object_store",
            format!("must target https://{infra}:9000"),
        ));
    }
    Ok(())
}

fn check_secret_quality(secrets: &BTreeMap<String, String>) -> Result<(), Error> {
    for key in [
        "QR_POSTGRES_PASSWORD",
        "QR_OBJECT_STORE_PASSWORD",
        "QR_QUEUE_PASSWORD",
    ] {
        let value = secrets.get(key).map(String::as_str).unwrap_or_default();
        let lower = value.to_ascii_lowercase();
        if value.len() < 16
            || lower.contains("replace-with")
            || lower.contains("change-me")
            || lower.contains("example")
            || lower.contains("password")
        {
            return Err(invalid(
                key,
                "must be at least 16 characters and not a template placeholder",
            ));
        }
    }
    let token = secrets
        .get("QR_BEARER_TOKEN")
        .map(String::as_str)
        .unwrap_or_default();
    let lower = token.to_ascii_lowercase();
    let repeated = token
        .chars()
        .next()
        .is_some_and(|first| token.chars().all(|character| character == first));
    if token.len() < 32
        || lower.contains("replace-with")
        || lower.contains("change-me")
        || lower.contains("example")
        || repeated
    {
        return Err(invalid(
            "QR_BEARER_TOKEN",
            "must be at least 32 characters and not a template placeholder",
        ));
    }
    Ok(())
}

fn invalid(field: impl Into<String>, message: impl Into<String>) -> Error {
    Error::Invalid {
        field: field.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::Descriptor;

    fn example() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../deploy/multi-host/deploy.example.json"
        ))
        .expect("the example parses")
    }

    fn valid_example() -> serde_json::Value {
        let mut value = example();
        value["image"]["digest"] = serde_json::json!(
            "sha256:3f5a9c1e7b2d4f608a1c3e5d7b9f0a2c4e6d8f1b3a5c7e9d0f2a4c6e8b1d3f5a"
        );
        value
    }

    fn parse(value: serde_json::Value) -> Descriptor {
        serde_json::from_value(value).expect("the mutated example stays structurally valid")
    }

    fn secrets() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("QR_POSTGRES_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_POSTGRES_PASSWORD".to_owned(),
                "db-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_POSTGRES_DB".to_owned(), "quantrule".to_owned()),
            ("QR_OBJECT_STORE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_OBJECT_STORE_PASSWORD".to_owned(),
                "object-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_QUEUE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_QUEUE_PASSWORD".to_owned(),
                "queue-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_QUEUE_VHOST".to_owned(), "/".to_owned()),
            (
                "QR_BEARER_TOKEN".to_owned(),
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
            ),
        ])
    }

    #[test]
    fn the_example_is_valid() {
        validate(&parse(valid_example())).expect("the example validates");
    }

    #[test]
    fn the_example_is_valid_with_secrets() {
        validate_with_secrets(&parse(valid_example()), &secrets()).expect("the example validates");
    }

    #[test]
    fn the_example_placeholder_digest_is_rejected() {
        let error = validate(&parse(example())).expect_err("the placeholder digest");
        assert!(error.to_string().contains("placeholder"), "{error}");
    }

    #[test]
    fn a_template_secret_is_rejected() {
        let mut secrets = secrets();
        secrets.insert(
            "QR_OBJECT_STORE_PASSWORD".to_owned(),
            "replace-with-a-long-random-value".to_owned(),
        );
        let error = validate_with_secrets(&parse(valid_example()), &secrets)
            .expect_err("a template secret");
        assert!(
            error.to_string().contains("QR_OBJECT_STORE_PASSWORD"),
            "{error}"
        );
    }

    #[test]
    fn a_public_infra_host_is_rejected() {
        let mut value = valid_example();
        value["hosts"]["infra"]["address"] = serde_json::json!("8.8.8.8");
        let error = validate(&parse(value)).expect_err("a public infra host");
        assert!(error.to_string().contains("infrastructure host"), "{error}");
    }

    #[test]
    fn an_unspecified_or_ipv6_host_is_rejected() {
        for address in ["0.0.0.0", "::", "fd00::10"] {
            let mut value = valid_example();
            value["hosts"]["core"]["address"] = serde_json::json!(address);
            assert!(validate(&parse(value)).is_err(), "{address} was accepted");
        }
    }

    #[test]
    fn a_port_zero_is_rejected() {
        let mut value = valid_example();
        value["services"]["data"]["port"] = serde_json::json!(0);
        let error = validate(&parse(value)).expect_err("port zero");
        assert!(error.to_string().contains("port"), "{error}");
    }

    #[test]
    fn an_external_endpoint_is_rejected() {
        let mut value = valid_example();
        value["endpoints"]["database"] = serde_json::json!(
            "postgres://quantrule:db-secret@8.8.8.8:5432/quantrule?sslmode=verify-full"
        );
        let error =
            validate_with_secrets(&parse(value), &secrets()).expect_err("an external database");
        assert!(error.to_string().contains("infrastructure host"), "{error}");
    }

    #[test]
    fn a_plaintext_endpoint_is_rejected() {
        for (field, endpoint) in [
            (
                "database",
                "postgres://quantrule:secret@10.10.0.20:5432/quantrule",
            ),
            (
                "database",
                "postgres://quantrule:secret@10.10.0.20:5432/quantrule?sslmode=require",
            ),
            ("object_store", "http://10.10.0.20:9000"),
            ("queue", "amqp://quantrule:secret@10.10.0.20:5672/%2f"),
        ] {
            let mut value = valid_example();
            value["endpoints"][field] = serde_json::json!(endpoint);
            assert!(
                validate(&parse(value)).is_err(),
                "{field}={endpoint} was accepted"
            );
        }
    }

    #[test]
    fn a_relative_ca_path_is_rejected() {
        let mut value = valid_example();
        value["endpoints"]["queue_tls_ca"] = serde_json::json!("pki/ca.crt");
        let error = validate(&parse(value)).expect_err("a relative CA path");
        assert!(error.to_string().contains("absolute path"), "{error}");
    }

    #[test]
    fn the_endpoint_targets_use_the_tls_ports() {
        let mut value = valid_example();
        value["endpoints"]["queue"] =
            serde_json::json!("amqps://quantrule:secret@10.10.0.20:5672/%2f");
        let error =
            validate_with_secrets(&parse(value), &secrets()).expect_err("a plaintext AMQP port");
        assert!(error.to_string().contains("5671"), "{error}");
    }

    #[test]
    fn a_second_public_service_is_rejected() {
        let mut value = valid_example();
        value["services"]["data"]["public"] = serde_json::json!(true);
        let error = validate(&parse(value)).expect_err("a public internal service");
        assert!(error.to_string().contains("only the gateway"), "{error}");
    }

    #[test]
    fn a_public_internal_host_is_rejected() {
        let mut value = valid_example();
        value["hosts"]["core"]["address"] = serde_json::json!("8.8.8.8");
        let error = validate(&parse(value)).expect_err("a public core host");
        assert!(error.to_string().contains("private address"), "{error}");
    }

    #[test]
    fn a_lan_gateway_needs_a_token() {
        let mut value = valid_example();
        value["auth"]["gateway_token"] = serde_json::json!(false);
        let error = validate(&parse(value)).expect_err("a tokenless LAN gateway");
        assert!(error.to_string().contains("gateway_token"), "{error}");
    }

    #[test]
    fn an_empty_token_is_a_missing_secret() {
        let mut secrets = secrets();
        secrets.insert("QR_BEARER_TOKEN".to_owned(), "  ".to_owned());
        let error =
            validate_with_secrets(&parse(valid_example()), &secrets).expect_err("empty token");
        assert!(error.to_string().contains("QR_BEARER_TOKEN"), "{error}");
    }

    #[test]
    fn a_public_gateway_needs_tls() {
        let mut value = valid_example();
        value["exposure"]["gateway"] = serde_json::json!("public");
        value["auth"]["tls"] = serde_json::json!("none");
        let error = validate(&parse(value)).expect_err("a public gateway without TLS");
        assert!(error.to_string().contains("auth.tls"), "{error}");
    }

    #[test]
    fn a_second_model_replica_is_rejected() {
        let mut value = valid_example();
        value["services"]["model"]["replicas"] = serde_json::json!(2);
        let error = validate(&parse(value)).expect_err("two model replicas");
        assert!(error.to_string().contains("replicas"), "{error}");
    }

    #[test]
    fn a_second_data_replica_is_rejected_by_v2() {
        let mut value = valid_example();
        value["services"]["data"]["replicas"] = serde_json::json!(2);
        let error = validate(&parse(value)).expect_err("two data replicas");
        assert!(
            error.to_string().contains("services.data.replicas"),
            "{error}"
        );
        assert!(
            error.to_string().contains("must be one"),
            "the refusal must name the v2 replica rule: {error}"
        );
    }

    #[test]
    fn a_duplicate_host_port_is_rejected() {
        let mut value = valid_example();
        value["services"]["evidence"]["port"] = serde_json::json!(18081);
        let error = validate(&parse(value)).expect_err("a duplicate port");
        assert!(
            error.to_string().contains("already uses the port"),
            "{error}"
        );
    }

    #[test]
    fn a_default_password_is_rejected_after_expansion() {
        let mut value = valid_example();
        value["endpoints"]["database"] = serde_json::json!(
            "postgres://quantrule:quantrule@10.10.0.20:5432/quantrule?sslmode=verify-full"
        );
        let error =
            validate_with_secrets(&parse(value), &secrets()).expect_err("the default password");
        assert!(error.to_string().contains("default credential"), "{error}");
    }

    #[test]
    fn a_missing_secret_is_reported() {
        let mut value = valid_example();
        value["endpoints"]["database"] = serde_json::json!(
            "postgres://${MISSING_PASSWORD}@10.10.0.20:5432/quantrule?sslmode=verify-full"
        );
        let error =
            validate_with_secrets(&parse(value), &secrets()).expect_err("the missing secret");
        assert!(error.to_string().contains("MISSING_PASSWORD"), "{error}");
    }

    #[test]
    fn an_unknown_descriptor_field_is_rejected() {
        let mut value = valid_example();
        value["unexpected"] = serde_json::json!(true);
        let error = serde_json::from_value::<Descriptor>(value).expect_err("an unknown field");
        assert!(error.to_string().contains("unknown field"), "{error}");
    }

    #[test]
    fn the_private_address_set_is_the_overlay_set() {
        for value in [
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "127.0.0.1",
            "100.64.0.1",
            "169.254.1.1",
            "fd00::1",
            "fe80::1",
            "::1",
        ] {
            assert!(
                is_private_ip(value.parse().expect("ip")),
                "{value} should be private"
            );
        }
        for value in ["0.0.0.0", "8.8.8.8", "1.1.1.1", "2001:4860:4860::8888"] {
            assert!(
                !is_private_ip(value.parse().expect("ip")),
                "{value} should be public"
            );
        }
    }

    #[test]
    fn placeholders_expand_and_malformed_placeholders_fail() {
        let secrets = BTreeMap::from([("NAME".to_owned(), "value".to_owned())]);
        assert_eq!(
            expand("x-${NAME}-y", &secrets).expect("expand"),
            "x-value-y"
        );
        assert!(expand("x-${NAME", &secrets).is_err());
        assert!(expand("x-${}", &secrets).is_err());
        assert!(expand("x-${MISSING}", &secrets).is_err());
    }

    #[test]
    fn the_dotenv_parser_handles_comments_quotes_and_duplicates() {
        let env = crate::descriptor::parse_env("# comment\nA=one\n\nB='two words'\nC=\"three\"\n")
            .expect("dotenv");
        assert_eq!(env["A"], "one");
        assert_eq!(env["B"], "two words");
        assert_eq!(env["C"], "three");
        assert!(crate::descriptor::parse_env("A=1\nA=2\n").is_err());
        assert!(crate::descriptor::parse_env("no-equals\n").is_err());
    }
}
