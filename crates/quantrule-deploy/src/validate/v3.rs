//! `quantrule.deploy/v3` structural and cross-field validation.
//!
//! The v3 document is closed by `deny_unknown_fields`; this module adds the
//! rules a typed parse cannot express: exactly seven services, exactly four
//! databases, one 443-only ingress, profile-specific failure domains and
//! anti-affinity, pinned stateful units, TLS endpoints, and the required
//! secret references. File contents are checked only by
//! [`validate_files`], so `validate` stays usable before secrets are copied.

use std::collections::BTreeSet;
use std::net::IpAddr;

use crate::descriptor::v3::{
    BackupCipher, CONTRACT, Database, Descriptor, ImagePin, Observability, Profile, Service,
};
use crate::error::Error;

use super::network;
use super::placement;
use super::secrets;
use super::stateful;
use super::v2::is_private_ip;

/// The seven stateless services every v3 deployment places.
pub const REQUIRED_SERVICES: [&str; 7] = [
    "gateway",
    "data",
    "model",
    "strategy",
    "backtest",
    "evidence",
    "simulation",
];

/// The four logical databases every v3 deployment declares.
pub const REQUIRED_DATABASES: [&str; 4] = ["gateway", "catalog", "evidence", "simulation"];

/// Every infrastructure unit the v3 renderer places must be pinned.
pub const REQUIRED_IMAGES: [&str; 16] = [
    "traefik",
    "postgres",
    "patroni",
    "etcd",
    "haproxy",
    "keepalived",
    "rabbitmq",
    "minio",
    "minio_client",
    "prometheus",
    "grafana",
    "loki",
    "alertmanager",
    "node_exporter",
    "promtail",
    "blackbox_exporter",
];

/// The fixed database facts for one logical database.
pub const DATABASE_FACTS: [(&str, &str, &str, &str); 4] = [
    (
        "gateway",
        "qr_gateway",
        "qr_gateway_owner",
        "qr_gateway_app",
    ),
    (
        "catalog",
        "qr_catalog",
        "qr_catalog_owner",
        "qr_catalog_app",
    ),
    (
        "evidence",
        "qr_evidence",
        "qr_evidence_owner",
        "qr_evidence_app",
    ),
    (
        "simulation",
        "qr_simulation",
        "qr_simulation_owner",
        "qr_simulation_app",
    ),
];

/// Every labelled fixture digest. Release mode refuses all of them, at the
/// exact `image.digest` or `images.<name>.digest` field, so reverting a pin to
/// an old example is never silently accepted. The list is a denylist, not an
/// allowlist: it does not prove a new digest exists, only that a known example
/// or blocker cannot ship.
pub const FIXTURE_IMAGE_DIGESTS: [(&str, &str); 19] = [
    (
        "sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe",
        "labelled application fixture",
    ),
    (
        crate::descriptor::v3::PGBACKREST_FIXTURE_DIGEST,
        "derived Patroni pgBackRest blocker; deploy/v3/images/patroni/build.sh prints the real digest",
    ),
    (
        crate::descriptor::v3::PATRONI_BASE_DIGEST,
        "Patroni base image without pgBackRest; use the derived image",
    ),
    (
        "sha256:8330e8d28853641532609f63676dc2eec8d19e2bc7fadd5505a1d0c8c254d042",
        "retired traefik fixture",
    ),
    (
        "sha256:dc184ed86a2b6421fd5ea1bf2ffa3d6e9f1df86796bf0830a7476eb064a186ff",
        "retired postgres fixture",
    ),
    (
        "sha256:e78141e6fe0807c466f17d8343f99249c7480a94d603bc03f91b10e79e7b94fd",
        "retired patroni fixture",
    ),
    (
        "sha256:a6cec115044c30857d8458d120facbaa2251f21fd4ad39296a99017468d482dc",
        "retired etcd fixture",
    ),
    (
        "sha256:2a88fcd69de33b07362a9b98f822f2b31f7607932356829cee9a5d44d851a2db",
        "retired haproxy fixture",
    ),
    (
        "sha256:67a5f8c8ca46f83c69a30376f47eaa4704f29a0495675433812d5216b0171682",
        "retired keepalived fixture",
    ),
    (
        "sha256:605a57a95a166325381f132867163eafc20cb0ece6039504e45681136550e6b7",
        "retired rabbitmq fixture",
    ),
    (
        "sha256:bdf0781acceb93983c15eec9e56bfbd59dcadc98ea57467022d61aeee074a2cc",
        "MinIO blocker; anonymous registry metadata is denied here",
    ),
    (
        "sha256:5c5b1f8f2a4e3d6c9b7a1e0f8d2c4b6a9e7f1d3c5b8a0e2f4d6c8b1a3e5f7d9c",
        "MinIO client blocker; anonymous registry metadata is denied here",
    ),
    (
        "sha256:c4bfb1e3ef91709eca17ae08b3676ac008939765ad9cf6c2b4617f363b1354be",
        "retired prometheus fixture",
    ),
    (
        "sha256:2e28e87d9b98ca68880a966df73ad63f90db6b4df6ae86637e130f52f48264ba",
        "retired grafana fixture",
    ),
    (
        "sha256:e13e67451cba9b2074bf13220303cb12e4ef2316709cd6f0fbd6f8f76b59e2d5",
        "retired loki fixture",
    ),
    (
        "sha256:62187e72213c4a4a0d1e1634c2350b73f3a1320777fb2274827a1f8c524f0ff2",
        "retired alertmanager fixture",
    ),
    (
        "sha256:c26a6b6c79c6a303387b01b3dcfc34824e4cf03fe9c2e9899a762b601e46a680",
        "retired node_exporter fixture",
    ),
    (
        "sha256:60b24aa368eea19a4fe8d35e294777963ba4a21f908a293de6f9e90d409ca919",
        "retired promtail fixture",
    ),
    (
        "sha256:a4e65b27bff6ae4a3491998b70df242bafffc87b0ddacbd7f9d0a4132395732a",
        "retired blackbox_exporter fixture",
    ),
];

/// Validate a v3 descriptor without opening local files.
///
/// # Errors
///
/// Returns the first [`Error::Rule`] with its stable `deploy.*` code and field
/// dot path.
pub fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    validate_version(descriptor)?;
    validate_name(descriptor)?;
    validate_image(&descriptor.image, "image")?;
    validate_images(descriptor)?;
    validate_hosts(descriptor)?;
    validate_swarm(descriptor)?;
    network::validate(descriptor)?;
    validate_auth(descriptor)?;
    secrets::validate_structure(descriptor)?;
    validate_databases(descriptor)?;
    validate_services(descriptor)?;
    validate_stateful(descriptor)?;
    validate_observability(descriptor)?;
    super::backup::validate(descriptor)?;
    placement::validate(descriptor)?;
    Ok(())
}

/// Refuse any labelled fixture digest in a release-mode descriptor.
///
/// # Errors
///
/// Returns `deploy.image.fixture` at the exact field that still carries a
/// fixture or blocker digest: `image.digest`, `images.minio.digest`,
/// `images.minio_client.digest`, or a reverted old infrastructure pin.
pub fn validate_release(descriptor: &Descriptor) -> Result<(), Error> {
    if let Some(reason) = fixture_reason(&descriptor.image.digest) {
        return Err(rule(
            "deploy.image.fixture",
            "image.digest",
            format!("is the {reason}; pin the published release digest with tools/release.sh pin"),
        ));
    }
    for (name, pin) in &descriptor.images {
        if let Some(reason) = fixture_reason(&pin.digest) {
            return Err(rule(
                "deploy.image.fixture",
                format!("images.{name}.digest"),
                format!("is the {reason}; pin a reviewed registry digest in images.{name}.digest"),
            ));
        }
    }
    if descriptor.backup.cipher == BackupCipher::None {
        return Err(rule(
            "deploy.backup.cipher",
            "backup.cipher",
            "must be aes-256-cbc in release mode; an unencrypted repository is refused",
        ));
    }
    Ok(())
}

fn fixture_reason(digest: &str) -> Option<&'static str> {
    FIXTURE_IMAGE_DIGESTS
        .iter()
        .find(|(fixture, _)| *fixture == digest)
        .map(|(_, reason)| *reason)
}

/// Validate a v3 descriptor and every local secret and ingress file.
///
/// # Errors
///
/// Returns the first structural rule failure, an unreadable or unsafe file, a
/// default credential, or a duplicate role credential.
pub fn validate_files(descriptor: &Descriptor) -> Result<(), Error> {
    load_files(descriptor).map(|_| ())
}

/// Every secret and ingress value, read once for validation and rendering.
pub(crate) struct LoadedFiles {
    /// The source `secrets.files` values by logical name.
    pub secrets: std::collections::BTreeMap<String, String>,
    /// The public ingress certificate chain, when `tls` is `cert`.
    pub ingress_certificate: Option<String>,
    /// The ingress private key, when `tls` is `cert`.
    pub ingress_key: Option<String>,
    /// The Postgres and etcd server leaves read once for rendering.
    pub stateful_pki: stateful::StatefulFiles,
}

/// Validate a v3 descriptor and return the one read of every secret file.
///
/// The renderer uses this so the value that was checked is the value it
/// writes; no second reader opens the source file.
pub(crate) fn load_files(descriptor: &Descriptor) -> Result<LoadedFiles, Error> {
    validate(descriptor)?;
    let secrets = secrets::load_values(descriptor)?;
    secrets::validate_values(descriptor, &secrets)?;
    let ingress = network::load_files(descriptor)?;
    let stateful_pki = stateful::load(descriptor)?;
    Ok(LoadedFiles {
        secrets,
        ingress_certificate: ingress.certificate,
        ingress_key: ingress.key,
        stateful_pki,
    })
}

fn validate_version(descriptor: &Descriptor) -> Result<(), Error> {
    if descriptor.v != CONTRACT {
        return Err(rule("deploy.version", "v", format!("must be {CONTRACT}")));
    }
    Ok(())
}

fn validate_name(descriptor: &Descriptor) -> Result<(), Error> {
    if let Some(name) = &descriptor.name
        && !is_identifier(name)
    {
        return Err(rule(
            "deploy.name",
            "name",
            "must be a lowercase DNS label of at most 63 characters",
        ));
    }
    Ok(())
}

fn validate_image(pin: &ImagePin, prefix: &str) -> Result<(), Error> {
    if pin.reference.trim().is_empty() || pin.reference.chars().any(char::is_whitespace) {
        return Err(rule(
            "deploy.image.reference",
            format!("{prefix}.reference"),
            "must be a non-empty image reference without whitespace",
        ));
    }
    if prefix == "image" && !pin.reference.contains('/') {
        return Err(rule(
            "deploy.image.reference",
            format!("{prefix}.reference"),
            "must name a registry/repository path, not a bare image name",
        ));
    }
    validate_digest(&format!("{prefix}.digest"), &pin.digest)
}

fn validate_digest(field: &str, digest: &str) -> Result<(), Error> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(rule(
            "deploy.image.digest",
            field,
            "must start with sha256:",
        ));
    };
    if hex.len() != 64 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(rule(
            "deploy.image.digest",
            field,
            "must be a 64-character hex digest",
        ));
    }
    if hex.chars().any(|character| character.is_ascii_uppercase()) {
        return Err(rule("deploy.image.digest", field, "must use lowercase hex"));
    }
    let first = hex.chars().next().unwrap_or('0');
    if hex.chars().all(|character| character == first) {
        return Err(rule(
            "deploy.image.digest",
            field,
            "must not be a placeholder made of one repeated character",
        ));
    }
    Ok(())
}

fn validate_images(descriptor: &Descriptor) -> Result<(), Error> {
    for name in REQUIRED_IMAGES {
        if !descriptor.images.contains_key(name) {
            return Err(rule(
                "deploy.images.missing",
                format!("images.{name}"),
                format!("must pin the {name} image with a sha256 digest"),
            ));
        }
    }
    for name in descriptor.images.keys() {
        if !REQUIRED_IMAGES.contains(&name.as_str()) {
            return Err(rule(
                "deploy.images.unknown",
                format!("images.{name}"),
                format!("{name} is not a rendered infrastructure unit"),
            ));
        }
    }
    for (name, pin) in &descriptor.images {
        validate_image(pin, &format!("images.{name}"))?;
    }
    Ok(())
}

fn validate_hosts(descriptor: &Descriptor) -> Result<(), Error> {
    let mut addresses = BTreeSet::new();
    let mut public_addresses = BTreeSet::new();
    for (name, host) in &descriptor.hosts {
        if !is_identifier(name) {
            return Err(rule(
                "deploy.host.name",
                format!("hosts.{name}"),
                "a host key must be a lowercase DNS label",
            ));
        }
        let address = host.address.parse::<IpAddr>().map_err(|_| {
            rule(
                "deploy.host.address",
                format!("hosts.{name}.address"),
                format!("{} is not an IP address", host.address),
            )
        })?;
        if address.is_unspecified() {
            return Err(rule(
                "deploy.host.address",
                format!("hosts.{name}.address"),
                "must not be 0.0.0.0 or ::",
            ));
        }
        if !is_private_ip(address) {
            return Err(rule(
                "deploy.host.address",
                format!("hosts.{name}.address"),
                "must be a private overlay or LAN address; put the public address in public_address",
            ));
        }
        if !addresses.insert(address) {
            return Err(rule(
                "deploy.host.address",
                format!("hosts.{name}.address"),
                "a second host uses the same address",
            ));
        }
        if !is_identifier(&host.failure_domain) {
            return Err(rule(
                "deploy.host.failure_domain",
                format!("hosts.{name}.failure_domain"),
                "must be a lowercase failure-domain label",
            ));
        }
        if let Some(public) = &host.public_address {
            let public_address = public.parse::<IpAddr>().map_err(|_| {
                rule(
                    "deploy.host.public_address",
                    format!("hosts.{name}.public_address"),
                    format!("{public} is not an IP address"),
                )
            })?;
            if public_address.is_unspecified() {
                return Err(rule(
                    "deploy.host.public_address",
                    format!("hosts.{name}.public_address"),
                    "must not be 0.0.0.0 or ::",
                ));
            }
            if !public_addresses.insert(public_address) {
                return Err(rule(
                    "deploy.host.public_address",
                    format!("hosts.{name}.public_address"),
                    "a second ingress host uses the same public address",
                ));
            }
            if !descriptor.ingress.hosts.contains(name) {
                return Err(rule(
                    "deploy.host.public_address",
                    format!("hosts.{name}.public_address"),
                    "only an ingress host may declare a public address",
                ));
            }
        }
    }
    Ok(())
}

fn validate_swarm(descriptor: &Descriptor) -> Result<(), Error> {
    let managers = &descriptor.swarm.managers;
    if managers.is_empty() {
        return Err(rule(
            "deploy.swarm.managers",
            "swarm.managers",
            "must name the odd Swarm manager hosts",
        ));
    }
    if managers.len().is_multiple_of(2) {
        return Err(rule(
            "deploy.swarm.managers",
            "swarm.managers",
            format!(
                "must list an odd number of managers; found {}",
                managers.len()
            ),
        ));
    }
    if managers.len() < 3 {
        return Err(rule(
            "deploy.swarm.managers",
            "swarm.managers",
            "must list at least three managers so one loss keeps quorum",
        ));
    }
    let mut seen = BTreeSet::new();
    for manager in managers {
        if !seen.insert(manager.as_str()) {
            return Err(rule(
                "deploy.swarm.managers",
                "swarm.managers",
                format!("{manager} is listed twice"),
            ));
        }
        if !descriptor.hosts.contains_key(manager) {
            return Err(rule(
                "deploy.swarm.managers",
                "swarm.managers",
                format!("{manager} is not declared in hosts"),
            ));
        }
    }
    Ok(())
}

fn validate_auth(descriptor: &Descriptor) -> Result<(), Error> {
    if descriptor.secrets.files.is_empty() {
        return Err(rule(
            "deploy.secrets.empty",
            "secrets.files",
            "must map at least the application secrets to absolute host paths",
        ));
    }
    let secret = &descriptor.auth.gateway_token_secret;
    if !is_secret_name(secret) {
        return Err(rule(
            "deploy.auth.gateway_token_secret",
            "auth.gateway_token_secret",
            "must be a lowercase secret name using letters, digits and underscores",
        ));
    }
    if !descriptor.secrets.files.contains_key(secret) {
        return Err(rule(
            "deploy.auth.gateway_token_secret",
            "auth.gateway_token_secret",
            format!("{secret} is not present in secrets.files"),
        ));
    }
    Ok(())
}

fn validate_databases(descriptor: &Descriptor) -> Result<(), Error> {
    for name in REQUIRED_DATABASES {
        if !descriptor.databases.contains_key(name) {
            return Err(rule(
                "deploy.database.missing",
                format!("databases.{name}"),
                format!("the {name} database is required"),
            ));
        }
    }
    for name in descriptor.databases.keys() {
        if !REQUIRED_DATABASES.contains(&name.as_str()) {
            return Err(rule(
                "deploy.database.unknown",
                format!("databases.{name}"),
                format!("{name} is not one of the four logical databases"),
            ));
        }
    }
    for (name, database, owner_role, app_role) in DATABASE_FACTS {
        let value = &descriptor.databases[name];
        validate_database(name, value, database, owner_role, app_role)?;
    }
    Ok(())
}

fn validate_database(
    name: &str,
    value: &Database,
    database: &str,
    owner_role: &str,
    app_role: &str,
) -> Result<(), Error> {
    if value.database != database {
        return Err(rule(
            "deploy.database.name",
            format!("databases.{name}.database"),
            format!("must be {database}"),
        ));
    }
    if value.owner_role != owner_role {
        return Err(rule(
            "deploy.database.owner_role",
            format!("databases.{name}.owner_role"),
            format!("must be {owner_role}"),
        ));
    }
    if value.app_role != app_role {
        return Err(rule(
            "deploy.database.app_role",
            format!("databases.{name}.app_role"),
            format!("must be {app_role}"),
        ));
    }
    Ok(())
}

fn validate_services(descriptor: &Descriptor) -> Result<(), Error> {
    for name in REQUIRED_SERVICES {
        if !descriptor.services.contains_key(name) {
            return Err(rule(
                "deploy.service.missing",
                format!("services.{name}"),
                format!("the {name} service is required"),
            ));
        }
    }
    for name in descriptor.services.keys() {
        if !REQUIRED_SERVICES.contains(&name.as_str()) {
            return Err(rule(
                "deploy.service.unknown",
                format!("services.{name}"),
                format!("{name} is not one of the seven services"),
            ));
        }
    }
    for (name, service) in &descriptor.services {
        validate_service(descriptor, name, service)?;
    }
    Ok(())
}

fn validate_service(descriptor: &Descriptor, name: &str, service: &Service) -> Result<(), Error> {
    if service.replicas == 0 {
        return Err(rule(
            "deploy.service.replicas",
            format!("services.{name}.replicas"),
            "must be at least one",
        ));
    }
    if service.hosts.is_empty() {
        return Err(rule(
            "deploy.service.hosts",
            format!("services.{name}.hosts"),
            "must name at least one host",
        ));
    }
    let mut hosts = BTreeSet::new();
    for host in &service.hosts {
        if !hosts.insert(host.as_str()) {
            return Err(rule(
                "deploy.service.hosts",
                format!("services.{name}.hosts"),
                format!("{host} is listed twice"),
            ));
        }
        host_by_name(descriptor, host, &format!("services.{name}.hosts"))?;
    }
    if service.port == 0 {
        return Err(rule(
            "deploy.service.port",
            format!("services.{name}.port"),
            "must be between 1 and 65535",
        ));
    }
    if service.port == 443 {
        return Err(rule(
            "deploy.service.public_port",
            format!("services.{name}.port"),
            "443 belongs to the one ingress; services use an internal port",
        ));
    }
    Ok(())
}

fn validate_stateful(descriptor: &Descriptor) -> Result<(), Error> {
    validate_nodes(
        descriptor,
        "stateful.postgres.nodes",
        &descriptor.stateful.postgres.nodes,
        3,
    )?;
    let postgres = &descriptor.stateful.postgres;
    if postgres.synchronous_standby != 1 {
        return Err(rule(
            "deploy.stateful.postgres.synchronous_standby",
            "stateful.postgres.synchronous_standby",
            "must be exactly one so committed rows keep RPO 0 and writes fail closed without a synchronous standby",
        ));
    }
    if !postgres.pitr {
        return Err(rule(
            "deploy.stateful.postgres.pitr",
            "stateful.postgres.pitr",
            "must be true; PITR covers all four databases",
        ));
    }
    validate_nodes(
        descriptor,
        "stateful.etcd.nodes",
        &descriptor.stateful.etcd.nodes,
        3,
    )?;
    validate_nodes(
        descriptor,
        "stateful.haproxy.nodes",
        &descriptor.stateful.haproxy.nodes,
        2,
    )?;
    validate_nodes(
        descriptor,
        "stateful.rabbitmq.nodes",
        &descriptor.stateful.rabbitmq.nodes,
        3,
    )?;
    let rabbitmq = &descriptor.stateful.rabbitmq;
    if rabbitmq.nodes.len().is_multiple_of(2) {
        return Err(rule(
            "deploy.stateful.rabbitmq.nodes",
            "stateful.rabbitmq.nodes",
            format!(
                "must list an odd number of brokers so one loss keeps quorum; found {}",
                rabbitmq.nodes.len()
            ),
        ));
    }
    if !rabbitmq.quorum {
        return Err(rule(
            "deploy.stateful.rabbitmq.quorum",
            "stateful.rabbitmq.quorum",
            "must be true; the HA profile uses quorum queues",
        ));
    }
    if !(256..=10_485_760).contains(&rabbitmq.disk_free_limit_mib) {
        return Err(rule(
            "deploy.stateful.rabbitmq.disk_free_limit_mib",
            "stateful.rabbitmq.disk_free_limit_mib",
            "must be between 256 and 10485760 MiB",
        ));
    }
    if !(10..=90).contains(&rabbitmq.memory_high_watermark_percent) {
        return Err(rule(
            "deploy.stateful.rabbitmq.memory_high_watermark_percent",
            "stateful.rabbitmq.memory_high_watermark_percent",
            "must be between 10 and 90 percent",
        ));
    }
    let minio_minimum = match descriptor.profile {
        Profile::SwarmHa | Profile::SingleAz => 4,
        Profile::LabSingleHost => 3,
    };
    validate_nodes(
        descriptor,
        "stateful.minio.nodes",
        &descriptor.stateful.minio.nodes,
        minio_minimum,
    )?;
    validate_minio_drives(descriptor)?;
    Ok(())
}

fn validate_nodes(
    descriptor: &Descriptor,
    field: &str,
    nodes: &[String],
    minimum: usize,
) -> Result<(), Error> {
    if nodes.len() < minimum {
        return Err(rule(
            "deploy.stateful.nodes",
            field,
            format!("must list at least {minimum} nodes; found {}", nodes.len()),
        ));
    }
    let mut seen = BTreeSet::new();
    for node in nodes {
        if !seen.insert(node.as_str()) {
            return Err(rule(
                "deploy.stateful.nodes",
                field,
                format!("{node} is listed twice"),
            ));
        }
        host_by_name(descriptor, node, field)?;
    }
    Ok(())
}

fn validate_minio_drives(descriptor: &Descriptor) -> Result<(), Error> {
    let minio = &descriptor.stateful.minio;
    if minio.drives.len() != minio.nodes.len() {
        return Err(rule(
            "deploy.stateful.minio.drives",
            "stateful.minio.drives",
            format!(
                "must name one drive count per node; {} nodes and {} counts",
                minio.nodes.len(),
                minio.drives.len()
            ),
        ));
    }
    if minio.drives.contains(&0) {
        return Err(rule(
            "deploy.stateful.minio.drives",
            "stateful.minio.drives",
            "every node must contribute at least one drive",
        ));
    }
    let total: u64 = minio.drives.iter().map(|drives| u64::from(*drives)).sum();
    if total < 4 {
        return Err(rule(
            "deploy.stateful.minio.drives",
            "stateful.minio.drives",
            format!("must total at least four drives for erasure coding; found {total}"),
        ));
    }
    if descriptor.profile != Profile::LabSingleHost
        && minio.drives.iter().any(|drives| *drives != 1)
    {
        return Err(rule(
            "deploy.stateful.minio.drives",
            "stateful.minio.drives",
            "the HA profiles use one drive on each of four or more nodes",
        ));
    }
    minio_parity(descriptor)?;
    Ok(())
}

/// The erasure-coding parity and the write-quorum rule.
///
/// One node loss must leave the write quorum: `total - max_drives >=
/// total - parity + 1`, so `parity > max_drives`. MinIO also needs the data
/// blocks to be at least the parity blocks: `2 * parity <= total`. `auto`
/// derives `total / 2`, which gives EC:2 for four nodes × one drive and EC:3
/// for the lab's three nodes × two drives.
pub(crate) fn minio_parity(descriptor: &Descriptor) -> Result<u32, Error> {
    let minio = &descriptor.stateful.minio;
    let total: u64 = minio.drives.iter().map(|drives| u64::from(*drives)).sum();
    let max_drives = minio.drives.iter().copied().max().unwrap_or(0);
    let parity = if minio.parity == "auto" {
        u32::try_from(total / 2).unwrap_or(u32::MAX)
    } else {
        let Some(value) = minio.parity.strip_prefix("EC:") else {
            return Err(rule(
                "deploy.stateful.minio.parity",
                "stateful.minio.parity",
                "must be auto or EC:<n>",
            ));
        };
        let Ok(value) = value.parse::<u32>() else {
            return Err(rule(
                "deploy.stateful.minio.parity",
                "stateful.minio.parity",
                "must be auto or EC:<n> with n a positive integer",
            ));
        };
        value
    };
    if parity == 0 {
        return Err(rule(
            "deploy.stateful.minio.parity",
            "stateful.minio.parity",
            "must be at least one parity drive",
        ));
    }
    if u64::from(parity) * 2 > total {
        return Err(rule(
            "deploy.stateful.minio.parity",
            "stateful.minio.parity",
            format!(
                "EC:{parity} needs at least {} drives so data blocks are not fewer than parity blocks; found {total}",
                u64::from(parity) * 2
            ),
        ));
    }
    if parity <= max_drives {
        return Err(rule(
            "deploy.stateful.minio.parity",
            "stateful.minio.parity",
            format!(
                "EC:{parity} does not keep the write quorum after one node loss; a node with {max_drives} drives needs parity greater than {max_drives}"
            ),
        ));
    }
    Ok(parity)
}

fn validate_observability(descriptor: &Descriptor) -> Result<(), Error> {
    let Observability {
        prometheus,
        grafana,
        loki,
        alertmanager,
    } = &descriptor.observability;
    for (unit, value) in [
        ("prometheus", prometheus),
        ("grafana", grafana),
        ("loki", loki),
        ("alertmanager", alertmanager),
    ] {
        host_by_name(
            descriptor,
            &value.host,
            &format!("observability.{unit}.host"),
        )?;
    }
    // The two alerting-path units must survive one host loss independently:
    // Prometheus evaluates and Alertmanager routes, so a shared host would lose
    // both. This is a placement rule, not a new descriptor field.
    let hosts: BTreeSet<&str> = [prometheus, grafana, loki, alertmanager]
        .iter()
        .map(|unit| unit.host.as_str())
        .collect();
    if hosts.len() == 1 {
        return Err(rule(
            "deploy.observability.placement",
            "observability",
            "must not place all four observability units on one host; spread them so one host loss does not remove metrics, logs, dashboards, and alerting together",
        ));
    }
    if prometheus.host == alertmanager.host {
        return Err(rule(
            "deploy.observability.placement",
            "observability.alertmanager.host",
            format!(
                "must differ from observability.prometheus.host; {} hosts both Prometheus and Alertmanager",
                prometheus.host
            ),
        ));
    }
    Ok(())
}

/// Look up a declared host.
pub(super) fn host_by_name<'a>(
    descriptor: &'a Descriptor,
    name: &str,
    field: &str,
) -> Result<&'a crate::descriptor::v3::Host, Error> {
    descriptor.hosts.get(name).ok_or_else(|| {
        rule(
            "deploy.host.undeclared",
            field,
            format!("{name} is not declared in hosts"),
        )
    })
}

/// The distinct failure domains across every host.
pub(crate) fn distinct_domains(descriptor: &Descriptor) -> BTreeSet<&str> {
    descriptor
        .hosts
        .values()
        .map(|host| host.failure_domain.as_str())
        .collect()
}

/// Build a v3 rule error.
pub(super) fn rule(
    code: &'static str,
    field: impl Into<String>,
    message: impl Into<String>,
) -> Error {
    Error::rule(code, field, message)
}

/// A lowercase DNS label.
pub(super) fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
}

/// A lowercase secret name.
pub(super) fn is_secret_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests;
