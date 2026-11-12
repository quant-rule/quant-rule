//! Service configuration.
//!
//! Environment only. No configuration file is required to start, and a value
//! that fails to parse is an error rather than a silent default.

use std::collections::BTreeMap;

use quantrule_api::Digest;

/// The default bind address.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";
/// The default Postgres URL used by the compose stack.
pub const DEFAULT_DATABASE_URL: &str = "postgres://quantrule:quantrule@postgres:5432/quantrule";
/// The default object-store endpoint used by the compose stack.
pub const DEFAULT_OBJECT_STORE_URL: &str = "http://minio:9000";
/// The default object-store bucket used by the compose stack.
pub const DEFAULT_OBJECT_BUCKET: &str = "quantrule";
/// The default broker URL used by the compose stack.
pub const DEFAULT_QUEUE_URL: &str = "amqp://quantrule:quantrule@rabbitmq:5672/%2f";
/// The default upload limit: 512 MiB.
pub const DEFAULT_UPLOAD_LIMIT: u64 = 512 * 1024 * 1024;

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
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Runs a backtest worker executes at once.
    pub backtest_concurrency: u32,
    /// The image digest that serves the request.
    pub image_digest: Option<Digest>,
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
            backtest_concurrency: 1,
            image_digest: None,
        }
    }
}

impl Config {
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
        if let Some(value) = values.get("QR_IMAGE_DIGEST") {
            config.image_digest = Some(Digest::parse(value).map_err(|_| ConfigError::Invalid {
                name: "QR_IMAGE_DIGEST",
                value: value.clone(),
                reason: "must be a sha256 digest",
            })?);
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn defaults_are_startable_without_any_variable() {
        let config = Config::from_map(&BTreeMap::new()).expect("defaults");
        assert_eq!(config.bind, DEFAULT_BIND);
        assert_eq!(config.upload_limit_bytes, DEFAULT_UPLOAD_LIMIT);
        assert_eq!(config.backtest_concurrency, 1);
        assert!(config.bearer_token.is_none());
    }

    #[test]
    fn a_present_value_overrides_the_default() {
        let config = Config::from_map(&map(&[("QR_BIND", "127.0.0.1:9000")])).expect("override");
        assert_eq!(config.bind, "127.0.0.1:9000");
    }

    #[test]
    fn a_bad_number_is_an_error_not_a_default() {
        let error =
            Config::from_map(&map(&[("QR_UPLOAD_LIMIT_BYTES", "big")])).expect_err("reject");
        assert_eq!(
            error,
            ConfigError::Invalid {
                name: "QR_UPLOAD_LIMIT_BYTES",
                value: "big".to_owned(),
                reason: "must be a non-negative integer",
            }
        );
    }

    #[test]
    fn concurrency_zero_is_rejected() {
        assert!(Config::from_map(&map(&[("QR_BACKTEST_CONCURRENCY", "0")])).is_err());
    }

    #[test]
    fn a_bad_image_digest_is_rejected() {
        assert!(Config::from_map(&map(&[("QR_IMAGE_DIGEST", "nope")])).is_err());
    }

    #[test]
    fn an_empty_token_means_no_token() {
        let config = Config::from_map(&map(&[("QR_BEARER_TOKEN", "")])).expect("empty");
        assert!(config.bearer_token.is_none());
    }
}
