//! The `quantrule.deploy/v3` Swarm HA descriptor.
//!
//! The schema is closed: every struct denies unknown fields, every image is
//! pinned by a `sha256:` digest, and a secret is a path in
//! [`Secrets::files`], never an inline value. The descriptor places the seven
//! stateless services and the ingress; the rendered stateful and observability
//! units stay pinned to declared hosts. Validation rules live in
//! [`crate::validate::v3`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The descriptor contract version.
pub const CONTRACT: &str = "quantrule.deploy/v3";

/// The derived Patroni image that adds pgBackRest to the pinned base.
pub const PGBACKREST_IMAGE_REFERENCE: &str = "ghcr.io/quant-rule/patroni-pgbackrest";

/// The labelled fixture digest for the derived image, until the operator builds
/// and pushes it. Release mode refuses it.
pub const PGBACKREST_FIXTURE_DIGEST: &str =
    "sha256:ffbf6f5298ec083505b31ebe962c70c32d791050d21c328a401f31c1cbe927a5";

/// The pinned `batonogov/patroni-docker` base the derived image builds from.
pub const PATRONI_BASE_DIGEST: &str =
    "sha256:0de7dc8bc5919c76721bee1475ed30155b1848c0a6bac7298605bf39eef1a7eb";

/// The application object bucket; an object archive must not target it.
pub const APP_OBJECT_BUCKET: &str = "quantrule";

/// The default dedicated posix repository path on the backup host.
pub const DEFAULT_BACKUP_PATH: &str = "/srv/quantrule/backup/pgbackrest";

/// The supported HA profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Three or more failure domains with strict anti-affinity.
    SwarmHa,
    /// One availability zone with host anti-affinity but no domain spread.
    SingleAz,
    /// Three VMs on one physical host; simulated HA only.
    LabSingleHost,
}

impl Profile {
    /// The descriptor spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SwarmHa => "swarm-ha",
            Self::SingleAz => "single-az",
            Self::LabSingleHost => "lab-single-host",
        }
    }
}

/// How the ingress terminates TLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum IngressTls {
    /// ACME issues and renews the certificate (PLAT-002).
    Acme,
    /// The operator supplies a certificate and key file.
    Cert,
}

/// How Patroni uses a watchdog device.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Watchdog {
    /// Patroni does not touch a watchdog.
    #[default]
    Off,
    /// Patroni configures the device when it is present and keeps running without it.
    Automatic,
    /// Patroni requires the device; a missing or unopenable device is a startup error.
    Required,
}

impl Watchdog {
    /// The descriptor spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Automatic => "automatic",
            Self::Required => "required",
        }
    }
}

/// One image reference and its digest pin.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ImagePin {
    /// The image reference without a mutable tag dependency.
    pub reference: String,
    /// The `sha256:` digest every host must run.
    pub digest: String,
}

/// The encrypted overlay network.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Network {
    /// The overlay network name.
    pub overlay: String,
    /// Whether the overlay is encrypted; must be true.
    pub encrypted: bool,
}

/// The one public HTTPS ingress.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ingress {
    /// The public DNS name; `quantrule.dev` in the reference topology.
    pub dns: String,
    /// The only public port; must be 443.
    pub port: u16,
    /// The floating IPv4 address keepalived assigns to the healthy edge host.
    #[serde(default)]
    pub vip: Option<String>,
    /// The Linux interface that carries the VIP; `eth0` unless the host says otherwise.
    #[serde(default = "default_ingress_interface")]
    pub interface: String,
    /// ACME or operator-supplied certificate files.
    pub tls: IngressTls,
    /// Certificate files by logical name; empty for ACME.
    #[serde(default)]
    pub files: BTreeMap<String, String>,
    /// The number of ingress replicas.
    pub replicas: u32,
    /// The host keys that run the ingress.
    pub hosts: Vec<String>,
}

/// The default VIP interface for the reference topology.
fn default_ingress_interface() -> String {
    "eth0".to_owned()
}

/// The Swarm control plane.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Swarm {
    /// The declared hosts that must be Swarm managers; three odd managers.
    pub managers: Vec<String>,
}

/// The gateway token secret reference.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    /// The key in [`Secrets::files`] that carries the gateway bearer token.
    pub gateway_token_secret: String,
}

/// One host and its failure domain.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// The private overlay or LAN address.
    pub address: String,
    /// The failure domain; a zone, rack, or lab VM group.
    pub failure_domain: String,
    /// The public address DNS resolves to; ingress hosts only.
    #[serde(default)]
    pub public_address: Option<String>,
}

/// The PostgreSQL endpoint reached through HAProxy.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseEndpoint {
    /// The endpoint host or service name; no userinfo.
    pub host: String,
    /// The TLS port; must be 5432.
    pub port: u16,
    /// The libpq SSL mode; must be `verify-full`.
    pub sslmode: String,
    /// The absolute host path of the CA bundle.
    pub tls_ca: String,
}

/// The RabbitMQ endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueueEndpoint {
    /// The endpoint host or service name; no userinfo.
    pub host: String,
    /// The TLS port; must be 5671.
    pub port: u16,
    /// The URL scheme; must be `amqps`.
    pub scheme: String,
    /// The absolute host path of the CA bundle.
    pub tls_ca: String,
}

/// The MinIO endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectStoreEndpoint {
    /// The endpoint host or service name; no userinfo.
    pub host: String,
    /// The TLS port; must be 9000.
    pub port: u16,
    /// The URL scheme; must be `https`.
    pub scheme: String,
    /// The absolute host path of the CA bundle.
    pub tls_ca: String,
}

/// The three stateful endpoints.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoints {
    /// PostgreSQL through the HAProxy pair.
    pub database: DatabaseEndpoint,
    /// RabbitMQ.
    pub queue: QueueEndpoint,
    /// MinIO.
    pub object_store: ObjectStoreEndpoint,
}

/// One logical database and its fixed roles.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Database {
    /// The database name.
    pub database: String,
    /// The owner role used only by the migration job.
    pub owner_role: String,
    /// The application role used by the long-running service.
    pub app_role: String,
}

/// One stateless service placement.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// The number of replicas.
    pub replicas: u32,
    /// The host keys the replicas may use; never a public bind.
    pub hosts: Vec<String>,
    /// The internal container port; no host port is published.
    pub port: u16,
}

/// The PostgreSQL stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PostgresStateful {
    /// The Patroni member hosts.
    pub nodes: Vec<String>,
    /// The number of synchronous standbys; must be one.
    pub synchronous_standby: u32,
    /// Whether Patroni uses a watchdog device.
    #[serde(default)]
    pub watchdog: Watchdog,
    /// Whether pgBackRest PITR is enabled; must be true.
    pub pitr: bool,
}

/// The etcd stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EtcdStateful {
    /// The etcd member hosts.
    pub nodes: Vec<String>,
}

/// The HAProxy stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HaProxyStateful {
    /// The HAProxy hosts.
    pub nodes: Vec<String>,
}

/// The RabbitMQ stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RabbitMqStateful {
    /// The broker hosts; exactly three, so one loss keeps quorum.
    pub nodes: Vec<String>,
    /// Whether quorum queues are enabled; must be true.
    pub quorum: bool,
    /// The absolute disk-free alarm in MiB.
    #[serde(default = "default_disk_free_limit_mib")]
    pub disk_free_limit_mib: u64,
    /// The memory high-watermark alarm as a percentage of RAM.
    #[serde(default = "default_memory_high_watermark_percent")]
    pub memory_high_watermark_percent: u32,
}

/// The documented default disk-free alarm: 5 GiB.
fn default_disk_free_limit_mib() -> u64 {
    5120
}

/// The documented default memory high-watermark: 60% of RAM.
fn default_memory_high_watermark_percent() -> u32 {
    60
}

/// The MinIO stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MinioStateful {
    /// The MinIO node hosts.
    pub nodes: Vec<String>,
    /// The drives per node, in node order; the total must be at least four.
    pub drives: Vec<u32>,
    /// The erasure-coding parity: `auto` or `EC:<n>`.
    ///
    /// `auto` derives half the total drive count, which keeps the write quorum
    /// after one node loss for the reference and lab layouts. An explicit value
    /// is validated against the same quorum rule.
    #[serde(default = "default_minio_parity")]
    pub parity: String,
}

/// The documented parity default: derive it from the drive layout.
fn default_minio_parity() -> String {
    "auto".to_owned()
}

/// The pinned stateful units. There is no reschedule field by design.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Stateful {
    /// Patroni/PostgreSQL members.
    pub postgres: PostgresStateful,
    /// etcd members.
    pub etcd: EtcdStateful,
    /// HAProxy members.
    pub haproxy: HaProxyStateful,
    /// RabbitMQ members.
    pub rabbitmq: RabbitMqStateful,
    /// MinIO members and drives.
    pub minio: MinioStateful,
}

/// One observability unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilityUnit {
    /// The host key that runs the unit.
    pub host: String,
}

/// The four observability units.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observability {
    /// Metrics collection.
    pub prometheus: ObservabilityUnit,
    /// Dashboards.
    pub grafana: ObservabilityUnit,
    /// Log aggregation.
    pub loki: ObservabilityUnit,
    /// Alert routing.
    pub alertmanager: ObservabilityUnit,
}

/// Where the pgBackRest repository lives.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupRepoKind {
    /// An operator-provided off-host S3-compatible endpoint.
    S3,
    /// A dedicated encrypted path on the backup host.
    #[default]
    Posix,
}

impl BackupRepoKind {
    /// The descriptor spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::S3 => "s3",
            Self::Posix => "posix",
        }
    }
}

/// How pgBackRest encrypts the repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackupCipher {
    /// AES-256-CBC with the passphrase file.
    #[default]
    #[serde(rename = "aes-256-cbc")]
    Aes256Cbc,
    /// No repository encryption; refused by release mode.
    None,
}

impl BackupCipher {
    /// The descriptor spelling and the pgBackRest `repo1-cipher-type` value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aes256Cbc => "aes-256-cbc",
            Self::None => "none",
        }
    }
}

/// The pgBackRest repository selection.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupRepo {
    /// The repository kind.
    #[serde(default)]
    pub kind: BackupRepoKind,
    /// The S3 host and port, without a scheme or userinfo.
    #[serde(default)]
    pub endpoint: Option<String>,
    /// The S3 bucket.
    #[serde(default)]
    pub bucket: Option<String>,
    /// The pgBackRest repository path. The posix default is applied by the
    /// renderer and checker, so an S3 repository without a prefix is not given
    /// the posix path.
    #[serde(default)]
    pub path: Option<String>,
    /// The absolute CA bundle path used to verify the S3 endpoint.
    #[serde(default)]
    pub tls_ca: Option<String>,
}

impl Default for BackupRepo {
    fn default() -> Self {
        Self {
            kind: BackupRepoKind::Posix,
            endpoint: None,
            bucket: None,
            path: None,
            tls_ca: None,
        }
    }
}

/// The UTC backup schedule. The operator installs the timer; no scheduler runs
/// inside a container.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupSchedule {
    /// The full-backup time, `HH:MM` UTC.
    #[serde(default = "default_full_at")]
    pub full_at: String,
    /// The differential-backup time, `HH:MM` UTC.
    #[serde(default = "default_diff_at")]
    pub diff_at: String,
}

impl Default for BackupSchedule {
    fn default() -> Self {
        Self {
            full_at: default_full_at(),
            diff_at: default_diff_at(),
        }
    }
}

fn default_full_at() -> String {
    "02:00".to_owned()
}

fn default_diff_at() -> String {
    "14:00".to_owned()
}

/// The object-archive contract. Phase 1a renders and checks it; the archive tool
/// and restore drill are phase 3.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectArchive {
    /// Whether the phase-3 archive is part of the deployment.
    #[serde(default)]
    pub enabled: bool,
    /// The archive target; it must differ from the application bucket.
    #[serde(default)]
    pub target: Option<String>,
    /// The archive retention window in days.
    #[serde(default)]
    pub retention_days: Option<u64>,
}

/// The backup contract.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    /// Whether point-in-time recovery is enabled; must be true.
    pub pitr: bool,
    /// The retention window in days.
    pub retention_days: u64,
    /// Whether a second-site object copy exists.
    ///
    /// False in v1.5.10: PLAT-005 owns local erasure and versioning, and HA-013
    /// owns off-host/archive copies, restore drills, and any second-site
    /// replication. A true value would be a replication claim without a site.
    pub object_replication: bool,
    /// The pgBackRest repository.
    #[serde(default)]
    pub repo: BackupRepo,
    /// The repository cipher.
    #[serde(default)]
    pub cipher: BackupCipher,
    /// The UTC schedule the operator's timer follows.
    #[serde(default)]
    pub schedule: BackupSchedule,
    /// The host that runs the backup script; defaults to `swarm.managers[0]`.
    #[serde(default)]
    pub host: Option<String>,
    /// The phase-3 object archive contract.
    #[serde(default)]
    pub object_archive: ObjectArchive,
}

impl Backup {
    /// The backup host: the explicit field or the first Swarm manager.
    #[must_use]
    pub fn host<'a>(&'a self, descriptor: &'a Descriptor) -> Option<&'a str> {
        self.host
            .as_deref()
            .or_else(|| descriptor.swarm.managers.first().map(String::as_str))
    }
}

/// The secret files the deployment consumes.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Secrets {
    /// Logical secret name to absolute host path.
    pub files: BTreeMap<String, String>,
}

/// The `quantrule.deploy/v3` document.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// The contract version.
    pub v: String,
    /// The HA profile.
    pub profile: Profile,
    /// An optional deployment name.
    #[serde(default)]
    pub name: Option<String>,
    /// The application image shared by the seven services.
    pub image: ImagePin,
    /// The image pins for every rendered infrastructure unit.
    pub images: BTreeMap<String, ImagePin>,
    /// The encrypted overlay.
    pub network: Network,
    /// The one HTTPS ingress.
    pub ingress: Ingress,
    /// The gateway token secret reference.
    pub auth: Auth,
    /// The hosts by key.
    pub hosts: BTreeMap<String, Host>,
    /// The Swarm control plane.
    pub swarm: Swarm,
    /// The stateful endpoints.
    pub endpoints: Endpoints,
    /// The four logical databases.
    pub databases: BTreeMap<String, Database>,
    /// The seven stateless services.
    pub services: BTreeMap<String, Service>,
    /// The pinned stateful units.
    pub stateful: Stateful,
    /// The four observability units.
    pub observability: Observability,
    /// The backup contract.
    pub backup: Backup,
    /// The secret files.
    pub secrets: Secrets,
}
