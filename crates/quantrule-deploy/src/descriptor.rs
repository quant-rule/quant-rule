//! The `quantrule.deploy/v2` descriptor and the dotenv secrets file.
//!
//! The descriptor is the single source of truth for a multi-host deployment.
//! It is JSON so the tool needs no YAML dependency and the schema is the Rust
//! type. Secrets stay in a separate dotenv file and are referenced from
//! endpoints as `${NAME}` placeholders.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::Error;

/// The descriptor contract version.
pub const CONTRACT: &str = "quantrule.deploy/v2";

/// The only supported profile name.
pub const PROFILE: &str = "multi-host";

/// The services a multi-host deployment must place.
pub const REQUIRED_SERVICES: [&str; 6] = [
    "gateway",
    "data",
    "model",
    "evidence",
    "simulation",
    "backtest",
];

/// The trusted network between hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkGate {
    /// A self-hosted WireGuard overlay.
    Wireguard,
    /// A Tailscale tailnet.
    Tailscale,
    /// A dedicated private LAN or VLAN.
    PrivateLan,
}

impl NetworkGate {
    /// The descriptor spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Wireguard => "wireguard",
            Self::Tailscale => "tailscale",
            Self::PrivateLan => "private-lan",
        }
    }
}

/// How the gateway is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GatewayExposure {
    /// Reachable only through the private overlay.
    Private,
    /// Reachable on the operator's LAN.
    Lan,
    /// Reachable from the public internet behind TLS.
    Public,
}

/// How the internal services are exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InternalExposure {
    /// Reachable only through the private overlay.
    Private,
}

/// Where TLS terminates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Tls {
    /// Plain HTTP on a trusted network.
    None,
    /// A reverse proxy terminates TLS in front of the gateway.
    ReverseProxy,
}

/// The image identity every host must run.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Image {
    /// The image reference, for example `quantrule-server:1.4.0`.
    pub reference: String,
    /// The image digest recorded as the run identity.
    pub digest: String,
}

/// One host in the deployment.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Host {
    /// The overlay or LAN address of the host.
    pub address: String,
}

/// One service placement.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    /// The host key that runs the service.
    pub host: String,
    /// The HTTP port, absent for the backtest worker.
    #[serde(default)]
    pub port: Option<u16>,
    /// The number of containers; default one.
    #[serde(default = "one")]
    pub replicas: u32,
    /// Whether the gateway is published beyond the overlay.
    #[serde(default)]
    pub public: bool,
}

/// The default replica count.
fn one() -> u32 {
    1
}

/// The shared infrastructure endpoints.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoints {
    /// The Postgres URL, with `${NAME}` placeholders allowed.
    ///
    /// It must carry `sslmode=verify-full`.
    pub database: String,
    /// The absolute host path of the PostgreSQL CA bundle.
    pub database_tls_ca: String,
    /// The S3-compatible endpoint URL; `https://` only.
    pub object_store: String,
    /// The absolute host path of the object-store CA bundle.
    pub object_store_tls_ca: String,
    /// The AMQP URL, with `${NAME}` placeholders allowed; `amqps://` only.
    pub queue: String,
    /// The absolute host path of the RabbitMQ CA bundle.
    pub queue_tls_ca: String,
    /// The object bucket; default `quantrule`.
    #[serde(default)]
    pub object_bucket: Option<String>,
}

/// How the gateway is exposed and authenticated.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exposure {
    /// The gateway exposure.
    pub gateway: GatewayExposure,
    /// The internal exposure; only `private` is valid.
    pub internal: InternalExposure,
}

/// The gateway authentication and TLS shape.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Auth {
    /// Whether the gateway requires a bearer token.
    pub gateway_token: bool,
    /// Where TLS terminates.
    pub tls: Tls,
}

/// The `quantrule.deploy/v2` document.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    /// The contract version.
    pub v: String,
    /// The deployment profile.
    pub profile: String,
    /// The one image every host runs.
    pub image: Image,
    /// The trusted network between hosts.
    pub network_gate: NetworkGate,
    /// The exposure contract.
    pub exposure: Exposure,
    /// The gateway authentication contract.
    pub auth: Auth,
    /// The hosts by key.
    pub hosts: BTreeMap<String, Host>,
    /// The service placements.
    pub services: BTreeMap<String, Service>,
    /// The shared infrastructure endpoints.
    pub endpoints: Endpoints,
    /// The host key that runs Postgres, MinIO, and RabbitMQ.
    pub infra_host: String,
    /// The read-only config root on the data host.
    pub config_root: String,
    /// The dotenv file that supplies placeholders and credentials.
    pub secrets_file: String,
    /// Runs per backtest worker; default one.
    #[serde(default)]
    pub backtest_concurrency: Option<u32>,
    /// Diagnostic stage retention in days; default 30.
    #[serde(default)]
    pub stage_retention_days: Option<u64>,
}

/// Read and parse a descriptor.
///
/// # Errors
///
/// Returns [`Error::Read`] when the file is absent and [`Error::Parse`] when it
/// is not valid JSON.
pub fn load(path: &Path) -> Result<Descriptor, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|error| Error::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

/// Read a dotenv file into a map.
///
/// Blank lines and lines whose first non-space character is `#` are ignored.
/// Values may be wrapped in matching single or double quotes.
///
/// # Errors
///
/// Returns [`Error::Read`] for an unreadable file and [`Error::Parse`] for a
/// malformed line, an empty key, or a duplicate key.
pub fn load_env(path: &Path) -> Result<BTreeMap<String, String>, Error> {
    let text = std::fs::read_to_string(path).map_err(|source| Error::Read {
        path: path.to_path_buf(),
        source,
    })?;
    parse_env(&text).map_err(|message| Error::Parse {
        path: path.to_path_buf(),
        message,
    })
}

/// Parse dotenv text.
///
/// # Errors
///
/// Returns a message when a line has no `=`, a key is empty or invalid, or a
/// key appears twice.
pub fn parse_env(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut values = BTreeMap::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {} has no '='", index + 1));
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(format!("line {} has an invalid key", index + 1));
        }
        let value = unquote(value.trim());
        if values.insert(key.to_owned(), value).is_some() {
            return Err(format!("line {} repeats {key}", index + 1));
        }
    }
    Ok(values)
}

/// Strip one pair of matching quotes.
fn unquote(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}

/// Resolve the descriptor's secrets path relative to the descriptor file.
#[must_use]
pub fn secrets_path(config: &Path, descriptor: &Descriptor) -> PathBuf {
    let path = Path::new(&descriptor.secrets_file);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config.parent().unwrap_or_else(|| Path::new(".")).join(path)
    }
}
