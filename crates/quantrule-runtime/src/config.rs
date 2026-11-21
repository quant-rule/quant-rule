//! Service configuration.
//!
//! Environment only. No configuration file is required to start, and a value
//! that fails to parse is an error rather than a silent default.

use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use quantrule_api::Digest;

/// The default bind address.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";
/// The default Postgres URL used by the compose stack.
///
/// It carries no password: the compose stack supplies the full URL from
/// `QR_DATABASE_URL`, and a process started without one fails closed.
pub const DEFAULT_DATABASE_URL: &str = "postgres://quantrule@postgres:5432/quantrule";
/// The default object-store endpoint used by the compose stack.
pub const DEFAULT_OBJECT_STORE_URL: &str = "http://minio:9000";
/// The default object-store bucket used by the compose stack.
pub const DEFAULT_OBJECT_BUCKET: &str = "quantrule";
/// The default broker URL used by the compose stack, without a password.
pub const DEFAULT_QUEUE_URL: &str = "amqp://quantrule@rabbitmq:5672/%2f";
/// The default upload limit: 512 MiB.
pub const DEFAULT_UPLOAD_LIMIT: u64 = 512 * 1024 * 1024;
/// The default diagnostic stage retention, in days.
pub const DEFAULT_STAGE_RETENTION_DAYS: u64 = 30;

/// Why a configuration value was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    /// A required variable was absent.
    Missing {
        /// The variable name.
        name: &'static str,
    },
    /// A variable was present but could not be used.
    Invalid {
        /// The variable name.
        name: &'static str,
        /// The value, as read.
        value: String,
        /// Why it was rejected.
        reason: &'static str,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing { name } => write!(formatter, "{name} is required"),
            Self::Invalid {
                name,
                value,
                reason,
            } => write!(formatter, "{name}={value:?} is invalid: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// The runtime configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// Where the HTTP server listens.
    pub bind: String,
    /// Postgres connection string.
    pub database_url: String,
    /// S3-compatible endpoint.
    pub object_store_url: String,
    /// Bucket that holds every object.
    pub object_store_bucket: String,
    /// AMQP connection string.
    pub queue_url: String,
    /// Upload cap in bytes.
    pub upload_limit_bytes: u64,
    /// When set, every request must carry it.
    pub bearer_token: Option<String>,
    /// Where the built client lives; absent in a client-less test or probe.
    pub static_dir: Option<PathBuf>,
    /// Where the read-only run-config mount lives.
    pub config_root: PathBuf,
    /// Runs a backtest worker executes at once.
    pub backtest_concurrency: u32,
    /// How long a diagnostic stage run and its artifacts are retained.
    pub stage_retention_days: u64,
    /// The image digest that serves the request.
    pub image_digest: Option<Digest>,
    /// Whether this process applies pending migrations at startup.
    pub migrate: bool,
    /// Whether the HTTP server must bind to a private or loopback address.
    ///
    /// The multi-host profile sets this on every internal HTTP service so a
    /// container cannot accidentally publish an unauthenticated service on a
    /// public interface.
    pub internal_only: bool,
}

impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .field("object_store_url", &"<redacted>")
            .field("object_store_bucket", &self.object_store_bucket)
            .field("queue_url", &"<redacted>")
            .field("upload_limit_bytes", &self.upload_limit_bytes)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .field("static_dir", &self.static_dir)
            .field("config_root", &self.config_root)
            .field("backtest_concurrency", &self.backtest_concurrency)
            .field("stage_retention_days", &self.stage_retention_days)
            .field("image_digest", &self.image_digest)
            .field("migrate", &self.migrate)
            .field("internal_only", &self.internal_only)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: DEFAULT_BIND.to_owned(),
            database_url: DEFAULT_DATABASE_URL.to_owned(),
            object_store_url: DEFAULT_OBJECT_STORE_URL.to_owned(),
            object_store_bucket: DEFAULT_OBJECT_BUCKET.to_owned(),
            queue_url: DEFAULT_QUEUE_URL.to_owned(),
            upload_limit_bytes: DEFAULT_UPLOAD_LIMIT,
            bearer_token: None,
            static_dir: None,
            config_root: PathBuf::from("config"),
            backtest_concurrency: 1,
            stage_retention_days: DEFAULT_STAGE_RETENTION_DAYS,
            image_digest: None,
            migrate: true,
            internal_only: false,
        }
    }
}

impl Config {
    /// Re-check the internal bind after construction.
    ///
    /// The parser already calls this when `QR_INTERNAL_ONLY` is set. The
    /// service entry points call it again so a `Config` built by code cannot
    /// bypass the guard.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Invalid`] when the bind is not private.
    pub fn check_internal_bind(&self) -> Result<(), ConfigError> {
        if self.internal_only {
            validate_internal_bind(&self.bind)?;
        }
        Ok(())
    }

    /// Read the configuration from the process environment.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a value is present but unparseable.
    pub fn from_env() -> Result<Self, ConfigError> {
        let values: BTreeMap<String, String> = std::env::vars()
            .filter(|(name, _)| name.starts_with("QR_"))
            .collect();
        Self::from_map(&values)
    }

    /// Read the configuration from a map, which is how tests avoid the
    /// environment.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a value is present but unparseable.
    pub fn from_map(values: &BTreeMap<String, String>) -> Result<Self, ConfigError> {
        let mut config = Self::default();
        if let Some(value) = values.get("QR_BIND") {
            if value.trim().is_empty() {
                return Err(ConfigError::Invalid {
                    name: "QR_BIND",
                    value: value.clone(),
                    reason: "must not be empty",
                });
            }
            config.bind.clone_from(value);
        }
        if let Some(value) = values.get("QR_DATABASE_URL") {
            config.database_url.clone_from(value);
        }
        if let Some(value) = values.get("QR_OBJECT_STORE_URL") {
            config.object_store_url.clone_from(value);
        }
        if let Some(value) = values.get("QR_OBJECT_BUCKET") {
            config.object_store_bucket.clone_from(value);
        }
        if let Some(value) = values.get("QR_QUEUE_URL") {
            config.queue_url.clone_from(value);
        }
        if let Some(value) = values.get("QR_UPLOAD_LIMIT_BYTES") {
            config.upload_limit_bytes = value.parse().map_err(|_| ConfigError::Invalid {
                name: "QR_UPLOAD_LIMIT_BYTES",
                value: value.clone(),
                reason: "must be a non-negative integer",
            })?;
        }
        if let Some(value) = values
            .get("QR_STATIC_DIR")
            .filter(|value| !value.trim().is_empty())
        {
            config.static_dir = Some(PathBuf::from(value));
        }
        if let Some(value) = values
            .get("QR_CONFIG_ROOT")
            .filter(|value| !value.trim().is_empty())
        {
            config.config_root = PathBuf::from(value);
        }
        if let Some(value) = values.get("QR_BEARER_TOKEN") {
            config.bearer_token = Some(value.clone()).filter(|token| !token.is_empty());
        }
        if let Some(value) = values.get("QR_BACKTEST_CONCURRENCY") {
            let parsed: u32 = value.parse().map_err(|_| ConfigError::Invalid {
                name: "QR_BACKTEST_CONCURRENCY",
                value: value.clone(),
                reason: "must be a positive integer",
            })?;
            if parsed == 0 {
                return Err(ConfigError::Invalid {
                    name: "QR_BACKTEST_CONCURRENCY",
                    value: value.clone(),
                    reason: "must be at least one",
                });
            }
            config.backtest_concurrency = parsed;
        }
        if let Some(value) = values.get("QR_MIGRATE") {
            config.migrate = match value.as_str() {
                "1" | "true" => true,
                "0" | "false" => false,
                _ => {
                    return Err(ConfigError::Invalid {
                        name: "QR_MIGRATE",
                        value: value.clone(),
                        reason: "must be 1, 0, true, or false",
                    });
                }
            };
        }
        if let Some(value) = values.get("QR_STAGE_RETENTION_DAYS") {
            let parsed: u64 = value.parse().map_err(|_| ConfigError::Invalid {
                name: "QR_STAGE_RETENTION_DAYS",
                value: value.clone(),
                reason: "must be a positive integer",
            })?;
            if parsed == 0 {
                return Err(ConfigError::Invalid {
                    name: "QR_STAGE_RETENTION_DAYS",
                    value: value.clone(),
                    reason: "must be at least one day",
                });
            }
            config.stage_retention_days = parsed;
        }
        if let Some(value) = values.get("QR_IMAGE_DIGEST") {
            config.image_digest = Some(Digest::parse(value).map_err(|_| ConfigError::Invalid {
                name: "QR_IMAGE_DIGEST",
                value: value.clone(),
                reason: "must be a sha256 digest",
            })?);
        }
        if let Some(value) = values.get("QR_INTERNAL_ONLY") {
            config.internal_only = match value.as_str() {
                "1" | "true" | "yes" => true,
                "0" | "false" | "no" => false,
                _ => {
                    return Err(ConfigError::Invalid {
                        name: "QR_INTERNAL_ONLY",
                        value: value.clone(),
                        reason: "must be 1, 0, true, false, yes, or no",
                    });
                }
            };
        }
        if config.internal_only {
            validate_internal_bind(&config.bind)?;
        }
        Ok(config)
    }
}

/// Refuse a bind address that is reachable from a public interface when the
/// service is marked internal.
///
/// The multi-host profile is the caller: an internal service has no application
/// authentication, so a public bind is a security defect even when the
/// descriptor validator missed it.
fn validate_internal_bind(bind: &str) -> Result<(), ConfigError> {
    let address: SocketAddr = bind.parse().map_err(|_| ConfigError::Invalid {
        name: "QR_BIND",
        value: bind.to_owned(),
        reason: "must be an IP:port when QR_INTERNAL_ONLY is set",
    })?;
    if is_private_bind(address.ip()) {
        return Ok(());
    }
    Err(ConfigError::Invalid {
        name: "QR_BIND",
        value: bind.to_owned(),
        reason: "must be a private or loopback address when QR_INTERNAL_ONLY is set",
    })
}

/// Whether an address belongs to a private, loopback, link-local, CGNAT, or
/// unique-local range.
fn is_private_bind(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(address) => is_private_v4(address),
        IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.to_ipv4_mapped().is_some_and(is_private_v4)
        }
    }
}

/// The IPv4 private set used by a multi-host overlay.
fn is_private_v4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_private()
        || address.is_loopback()
        || address.is_link_local()
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
}
