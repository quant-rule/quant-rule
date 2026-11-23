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
    /// The number of synchronous standbys; at least one.
    pub synchronous_standby: u32,
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
    /// The broker hosts.
    pub nodes: Vec<String>,
    /// Whether quorum queues are enabled; must be true.
    pub quorum: bool,
}

/// The MinIO stateful unit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MinioStateful {
    /// The MinIO node hosts.
    pub nodes: Vec<String>,
    /// The drives per node, in node order; the total must be at least four.
    pub drives: Vec<u32>,
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

/// The backup contract.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Backup {
    /// Whether point-in-time recovery is enabled; must be true.
    pub pitr: bool,
    /// The retention window in days.
    pub retention_days: u64,
    /// Whether object storage is replicated.
    pub object_replication: bool,
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
