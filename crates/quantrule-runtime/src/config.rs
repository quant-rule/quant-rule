//! Service configuration.
//!
//! Environment only. No configuration file is required to start, and a value
//! that fails to parse is an error rather than a silent default.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use quantrule_api::Digest;

use crate::queue::{DEFAULT_QUEUE_GENERATION, QueueNames};

/// The argument that selects the one-shot owner migration mode.
///
/// A long-running service never applies a migration: it holds only the
/// application URL. The same binary runs once with this flag, opens the owner
/// URL, applies its ledger, and exits before any pool or HTTP listener is
/// created.
pub const MIGRATE_ONLY_ARG: &str = "--migrate-only";

/// The default bind address.
pub const DEFAULT_BIND: &str = "0.0.0.0:8080";
/// The default Postgres application-role URL.
///
/// It carries no password: the compose stack supplies the full URL from
/// `QR_DATABASE_URL`, and a process started without one fails closed. Each
/// database-owning service is pointed at its own database; the default is only
/// the value a `Config` built outside Compose carries.
pub const DEFAULT_DATABASE_URL: &str = "postgres://quantrule@postgres:5432/qr_gateway";
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
/// The default HTTP drain deadline, in seconds.
pub const DEFAULT_DRAIN_TIMEOUT_HTTP: u64 = 20;
/// The default worker drain deadline, in seconds.
pub const DEFAULT_DRAIN_TIMEOUT_WORK: u64 = 25;
/// The largest accepted drain deadline, in seconds.
pub const MAX_DRAIN_TIMEOUT_SECONDS: u64 = 300;

/// The largest accepted secret file, in bytes.
pub const MAX_SECRET_FILE_BYTES: usize = 8 * 1024;
/// The default bearer-token reload interval, in seconds.
pub const DEFAULT_TOKEN_RELOAD_SECONDS: u64 = 5;
/// The largest accepted bearer-token reload interval, in seconds.
pub const MAX_TOKEN_RELOAD_SECONDS: u64 = 3600;
/// The minimum bearer-token length, in bytes.
pub const MIN_BEARER_TOKEN_BYTES: usize = 32;

/// The secret-bearing variable and its file-backed counterpart.
///
/// The plain variable is refused in every profile: a value that reaches the
/// process environment is visible to `/proc`, `docker inspect` and a child
/// process. The `_FILE` variable names an absolute path the runtime reads once
/// at startup.
pub const SECRET_VARIABLES: [(&str, &str); 6] = [
    ("QR_DATABASE_URL", "QR_DATABASE_URL_FILE"),
    ("QR_DATABASE_OWNER_URL", "QR_DATABASE_OWNER_URL_FILE"),
    ("QR_QUEUE_URL", "QR_QUEUE_URL_FILE"),
    ("QR_BEARER_TOKEN", "QR_BEARER_TOKEN_FILE"),
    (
        "QR_OBJECT_STORE_ACCESS_KEY",
        "QR_OBJECT_STORE_ACCESS_KEY_FILE",
    ),
    (
        "QR_OBJECT_STORE_SECRET_KEY",
        "QR_OBJECT_STORE_SECRET_KEY_FILE",
    ),
];

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
    /// A secret-bearing value was absent, duplicated or malformed.
    ///
    /// The value is never included: a configuration error reaches the log, and
    /// a secret must not leak through a rejected startup.
    Secret {
        /// The variable name.
        name: &'static str,
        /// Why it was rejected.
        reason: &'static str,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing { name } => write!(formatter, "{name} is required"),
            Self::Invalid {
                name,
                value,
                reason,
            } => write!(formatter, "{name}={value:?} is invalid: {reason}"),
            Self::Secret { name, reason } => write!(formatter, "{name} is invalid: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Why a secret file or its contents were rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretError {
    /// The path is relative.
    NotAbsolute,
    /// The file cannot be read.
    Unreadable,
    /// The file is larger than [`MAX_SECRET_FILE_BYTES`].
    TooLarge,
    /// The bytes are not UTF-8.
    NotUtf8,
    /// The contents contain a NUL byte.
    Nul,
    /// The value is empty after the trailing newline is stripped.
    Empty,
    /// The value has leading or trailing whitespace.
    Whitespace,
    /// The value contains an embedded newline or carriage return.
    EmbeddedNewline,
}

impl SecretError {
    /// A stable reason, safe to log and use as a metric label.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::NotAbsolute => "must be an absolute path",
            Self::Unreadable => "cannot be read",
            Self::TooLarge => "must be at most 8 KiB",
            Self::NotUtf8 => "must be UTF-8",
            Self::Nul => "must not contain NUL",
            Self::Empty => "must not be empty",
            Self::Whitespace => "must not have leading or trailing whitespace",
            Self::EmbeddedNewline => "must not contain an embedded newline",
        }
    }
}

/// Reads the bytes of one secret file.
///
/// The production implementation reads the filesystem; tests inject a map so a
/// test never touches `/run/secrets`. The reader is a seam, not a policy: path,
/// size, encoding, newline and whitespace rules are applied by
/// [`parse_secret_contents`] and [`read_secret_file`].
pub trait SecretFileReader: Send + Sync {
    /// Read the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns the underlying I/O error.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
}

/// The production secret-file reader.
#[derive(Debug, Clone, Copy, Default)]
pub struct FileSecretReader;

impl SecretFileReader for FileSecretReader {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }
}

/// A reader used by [`Config::from_map`], which cannot read files.
#[derive(Debug, Clone, Copy, Default)]
struct UnreadableSecretFiles;

impl SecretFileReader for UnreadableSecretFiles {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        Err(io::Error::from(io::ErrorKind::NotFound))
    }
}

/// Parse one secret file's bytes under the documented file contract.
///
/// The contract strips exactly one trailing `\n` and an optional preceding
/// `\r`, then rejects an empty value, leading or trailing whitespace, an
/// embedded newline, a NUL and invalid UTF-8. The value itself is returned
/// unchanged. An error names the rule, never the value.
///
/// # Errors
///
/// Returns [`SecretError`] when the contents are outside the contract.
pub fn parse_secret_contents(contents: &[u8]) -> Result<String, SecretError> {
    if contents.len() > MAX_SECRET_FILE_BYTES {
        return Err(SecretError::TooLarge);
    }
    let text = std::str::from_utf8(contents).map_err(|_| SecretError::NotUtf8)?;
    if text.as_bytes().contains(&0) {
        return Err(SecretError::Nul);
    }
    let value = text
        .strip_suffix('\n')
        .map_or(text, |rest| rest.strip_suffix('\r').unwrap_or(rest));
    if value.trim().is_empty() {
        return Err(SecretError::Empty);
    }
    if value.trim() != value {
        return Err(SecretError::Whitespace);
    }
    if value.contains(['\n', '\r']) {
        return Err(SecretError::EmbeddedNewline);
    }
    Ok(value.to_owned())
}

/// Read and parse one secret file.
///
/// # Errors
///
/// Returns [`ConfigError::Secret`] naming the `_FILE` variable when the path is
/// relative, unreadable, or outside the file contract.
pub fn read_secret_file(
    name: &'static str,
    path: &Path,
    reader: &impl SecretFileReader,
) -> Result<String, ConfigError> {
    if !path.is_absolute() {
        return Err(ConfigError::Secret {
            name,
            reason: SecretError::NotAbsolute.reason(),
        });
    }
    let contents = reader.read(path).map_err(|_| ConfigError::Secret {
        name,
        reason: SecretError::Unreadable.reason(),
    })?;
    parse_secret_contents(&contents).map_err(|error| ConfigError::Secret {
        name,
        reason: error.reason(),
    })
}

/// Validate one bearer-token value.
///
/// The gateway compares the bytes after `Bearer `, so a token containing
/// whitespace or a control character can never match. A short value is refused
/// because the documented token is 256 bits rendered as lowercase hexadecimal.
///
/// # Errors
///
/// Returns a stable reason the value is not a token. The value itself is never
/// included.
pub fn validate_bearer_token(value: &str) -> Result<(), &'static str> {
    if value.len() < MIN_BEARER_TOKEN_BYTES {
        return Err("must be at least 32 characters");
    }
    if !value.bytes().all(|byte| (0x21..=0x7e).contains(&byte)) {
        return Err("must be visible ASCII without spaces");
    }
    Ok(())
}

/// Whether a URL authority carries userinfo.
///
/// A secret in an endpoint URL would end up in `Config` debug output, an error
/// message and a deployment descriptor. `QR_OBJECT_STORE_URL` therefore refuses
/// it and the credentials come from their own files.
fn has_url_userinfo(value: &str) -> bool {
    let authority = value
        .split_once("://")
        .map_or(value, |(_scheme, rest)| rest)
        .split('/')
        .next()
        .unwrap_or_default();
    authority.contains('@')
}

/// The runtime configuration.
#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    /// Where the HTTP server listens.
    pub bind: String,
    /// Postgres connection string for the service's application role.
    ///
    /// The application role has DML only. It cannot create a schema or a table.
    pub database_url: String,
    /// Whether the resolved map supplied the database URL.
    pub database_url_from_secret: bool,
    /// Postgres connection string for the service's owner role.
    ///
    /// Read only by the one-shot `--migrate-only` job. A long-running service
    /// must not receive it: the runtime refuses to start when the variable is
    /// present, so the process cannot keep a DDL credential in its
    /// environment.
    pub database_owner_url: Option<String>,
    /// S3-compatible endpoint.
    pub object_store_url: String,
    /// Bucket that holds every object.
    pub object_store_bucket: String,
    /// S3 access-key id, resolved from its `_FILE` variable.
    pub object_store_access_key: Option<String>,
    /// S3 secret access key, resolved from its `_FILE` variable.
    pub object_store_secret_key: Option<String>,
    /// AMQP connection string.
    pub queue_url: String,
    /// Whether the resolved map supplied the queue URL.
    pub queue_url_from_secret: bool,
    /// The queue generation appended to every declared queue name.
    ///
    /// A changed queue declaration moves to a new generation instead of
    /// redeclaring a live name with different arguments. The service never
    /// deletes the old generation; the operator drains or retires it.
    pub queue_generation: u32,
    /// Upload cap in bytes.
    pub upload_limit_bytes: u64,
    /// When set, every request must carry it.
    pub bearer_token: Option<String>,
    /// Where the gateway re-reads the bearer token.
    ///
    /// Absent keeps the documented unauthenticated profile. The path, not the
    /// value, is the configuration identity.
    pub bearer_token_file: Option<PathBuf>,
    /// How often the gateway re-reads the bearer token file.
    pub bearer_token_reload: Duration,
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
    /// Whether `QR_MIGRATE` asks the long-running service to migrate.
    ///
    /// It must be false. The service refuses a true value; the ledger belongs
    /// to the `--migrate-only` job.
    pub migrate: bool,
    /// Whether the HTTP server must bind to a private or loopback address.
    ///
    /// The multi-host profile sets this on every internal HTTP service so a
    /// container cannot accidentally publish an unauthenticated service on a
    /// public interface.
    pub internal_only: bool,
    /// Where the internal Prometheus listener binds.
    ///
    /// Unset disables the listener. When `QR_INTERNAL_ONLY` is set, the
    /// address must be private or loopback: the metrics surface is internal and
    /// is never proxied by the gateway.
    pub metrics_bind: Option<SocketAddr>,
    /// How long the HTTP server waits for in-flight requests after a drain
    /// signal. The process exits successfully at the deadline.
    pub drain_timeout_http: Duration,
    /// How long a worker waits for its in-flight run or stage after a drain
    /// signal. A run left running keeps its lease and is recovered by expiry.
    pub drain_timeout_work: Duration,
}

impl fmt::Debug for Config {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Config")
            .field("bind", &self.bind)
            .field("database_url", &"<redacted>")
            .field("database_owner_url", &"<redacted>")
            .field("object_store_url", &"<redacted>")
            .field("object_store_bucket", &self.object_store_bucket)
            .field(
                "object_store_access_key",
                &self.object_store_access_key.as_ref().map(|_| "<redacted>"),
            )
            .field(
                "object_store_secret_key",
                &self.object_store_secret_key.as_ref().map(|_| "<redacted>"),
            )
            .field("queue_url", &"<redacted>")
            .field("queue_generation", &self.queue_generation)
            .field("upload_limit_bytes", &self.upload_limit_bytes)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "<redacted>"),
            )
            .field("bearer_token_file", &self.bearer_token_file)
            .field("bearer_token_reload", &self.bearer_token_reload)
            .field("static_dir", &self.static_dir)
            .field("config_root", &self.config_root)
            .field("backtest_concurrency", &self.backtest_concurrency)
            .field("stage_retention_days", &self.stage_retention_days)
            .field("image_digest", &self.image_digest)
            .field("migrate", &self.migrate)
            .field("internal_only", &self.internal_only)
            .field("metrics_bind", &self.metrics_bind)
            .field("drain_timeout_http", &self.drain_timeout_http)
            .field("drain_timeout_work", &self.drain_timeout_work)
            .finish()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: DEFAULT_BIND.to_owned(),
            database_url: DEFAULT_DATABASE_URL.to_owned(),
            database_url_from_secret: false,
            database_owner_url: None,
            object_store_url: DEFAULT_OBJECT_STORE_URL.to_owned(),
            object_store_bucket: DEFAULT_OBJECT_BUCKET.to_owned(),
            object_store_access_key: None,
            object_store_secret_key: None,
            queue_url: DEFAULT_QUEUE_URL.to_owned(),
            queue_url_from_secret: false,
            queue_generation: DEFAULT_QUEUE_GENERATION,
            upload_limit_bytes: DEFAULT_UPLOAD_LIMIT,
            bearer_token: None,
            bearer_token_file: None,
            bearer_token_reload: Duration::from_secs(DEFAULT_TOKEN_RELOAD_SECONDS),
            static_dir: None,
            config_root: PathBuf::from("config"),
            backtest_concurrency: 1,
            stage_retention_days: DEFAULT_STAGE_RETENTION_DAYS,
            image_digest: None,
            migrate: true,
            internal_only: false,
            metrics_bind: None,
            drain_timeout_http: Duration::from_secs(DEFAULT_DRAIN_TIMEOUT_HTTP),
            drain_timeout_work: Duration::from_secs(DEFAULT_DRAIN_TIMEOUT_WORK),
        }
    }
}

impl Config {
    /// Re-check the internal binds after construction.
    ///
    /// The parser already calls this when `QR_INTERNAL_ONLY` is set. The
    /// service entry points call it again so a `Config` built by code cannot
    /// bypass the guard for either the service bind or the metrics bind.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Invalid`] when a bind is not private.
    pub fn check_internal_bind(&self) -> Result<(), ConfigError> {
        if self.internal_only {
            validate_internal_bind(&self.bind)?;
        }
        self.check_metrics_bind()
    }

    /// Re-check the metrics listener bind after construction.
    ///
    /// The metrics listener is internal. When `QR_INTERNAL_ONLY` is set, its
    /// address must be private or loopback. A process with the listener disabled
    /// has nothing to check.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Invalid`] when the metrics bind is not private.
    pub fn check_metrics_bind(&self) -> Result<(), ConfigError> {
        if self.internal_only
            && let Some(metrics) = self.metrics_bind
        {
            validate_private_address("QR_METRICS_BIND", metrics)?;
        }
        Ok(())
    }

    /// The service's application database URL, which must come from its file.
    ///
    /// A long-running database-owning service starts only when
    /// `QR_DATABASE_URL_FILE` supplied the URL. The default is a documentation
    /// value for a `Config` built by code, not a fallback credential.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Missing`] naming `QR_DATABASE_URL_FILE`.
    pub fn require_database_url(&self) -> Result<&str, ConfigError> {
        if self.database_url_from_secret {
            Ok(&self.database_url)
        } else {
            Err(ConfigError::Missing {
                name: "QR_DATABASE_URL_FILE",
            })
        }
    }

    /// The broker URL, which must come from its file.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Missing`] naming `QR_QUEUE_URL_FILE`.
    pub fn require_queue_url(&self) -> Result<&str, ConfigError> {
        if self.queue_url_from_secret {
            Ok(&self.queue_url)
        } else {
            Err(ConfigError::Missing {
                name: "QR_QUEUE_URL_FILE",
            })
        }
    }

    /// The queue names this process declares and consumes.
    ///
    /// A consumer must use these names. A base name, or another generation's
    /// name, is outside the declared topology and is refused by the adapter.
    #[must_use]
    pub fn queue_names(&self) -> QueueNames {
        QueueNames::new(self.queue_generation)
    }

    /// Read the configuration from the process environment.
    ///
    /// This is the only production entry point that reads a secret file. It
    /// resolves every `*_FILE` value, refuses a plain secret variable, and then
    /// validates the resolved map.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a secret file is missing or outside the
    /// file contract, when a plain secret variable is set, or when a value is
    /// present but unparseable.
    pub fn from_env() -> Result<Self, ConfigError> {
        let values: BTreeMap<String, String> = std::env::vars()
            .filter(|(name, _)| name.starts_with("QR_"))
            .collect();
        Self::from_values_with_reader(&values, &FileSecretReader)
    }

    /// Read the configuration from a map without reading a file.
    ///
    /// Tests use this constructor for non-secret values. A secret variable,
    /// plain or `_FILE`, is refused because this entry point cannot read the
    /// file; a test that needs a secret uses
    /// [`Self::from_values_with_reader`] with an injected reader.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a secret variable is set or a value is
    /// present but unparseable.
    pub fn from_map(values: &BTreeMap<String, String>) -> Result<Self, ConfigError> {
        Self::from_values_with_reader(values, &UnreadableSecretFiles)
    }

    /// Read the configuration from a map and an injected secret reader.
    ///
    /// This is the test constructor: the caller supplies the value map and the
    /// bytes a `_FILE` path names, so no test reaches `/run/secrets`.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] when a secret file is missing or outside the
    /// file contract, when a plain secret variable is set, or when a value is
    /// present but unparseable.
    pub fn from_values_with_reader(
        values: &BTreeMap<String, String>,
        reader: &impl SecretFileReader,
    ) -> Result<Self, ConfigError> {
        let mut resolved = values.clone();
        let mut bearer_token_file = None;
        for (plain, file) in SECRET_VARIABLES {
            let plain_set = values.contains_key(plain);
            let file_set = values.contains_key(file);
            if plain_set && file_set {
                return Err(ConfigError::Secret {
                    name: plain,
                    reason: "must not be set when its _FILE variable is set",
                });
            }
            if plain_set {
                return Err(ConfigError::Secret {
                    name: plain,
                    reason: "must come from its _FILE variable",
                });
            }
            if file_set {
                let Some(value) = values.get(file) else {
                    continue;
                };
                let path = Path::new(value);
                let secret = read_secret_file(file, path, reader)?;
                if plain == "QR_BEARER_TOKEN" {
                    bearer_token_file = Some(path.to_path_buf());
                }
                resolved.remove(file);
                resolved.insert(plain.to_owned(), secret);
            }
        }
        let mut config = Self::from_resolved_map(&resolved)?;
        config.bearer_token_file = bearer_token_file;
        if config.object_store_access_key.is_some() != config.object_store_secret_key.is_some() {
            return Err(ConfigError::Secret {
                name: "QR_OBJECT_STORE_ACCESS_KEY_FILE",
                reason: "and QR_OBJECT_STORE_SECRET_KEY_FILE must be set together",
            });
        }
        Ok(config)
    }

    /// Validate a map whose secret variables are already resolved values.
    fn from_resolved_map(values: &BTreeMap<String, String>) -> Result<Self, ConfigError> {
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
            config.database_url_from_secret = true;
        }
        if let Some(value) = values
            .get("QR_DATABASE_OWNER_URL")
            .filter(|value| !value.trim().is_empty())
        {
            config.database_owner_url = Some(value.clone());
        }
        if let Some(value) = values.get("QR_OBJECT_STORE_URL") {
            if has_url_userinfo(value) {
                return Err(ConfigError::Secret {
                    name: "QR_OBJECT_STORE_URL",
                    reason: "must not contain userinfo",
                });
            }
            config.object_store_url.clone_from(value);
        }
        if let Some(value) = values.get("QR_OBJECT_BUCKET") {
            config.object_store_bucket.clone_from(value);
        }
        if let Some(value) = values.get("QR_OBJECT_STORE_ACCESS_KEY") {
            config.object_store_access_key = Some(value.clone());
        }
        if let Some(value) = values.get("QR_OBJECT_STORE_SECRET_KEY") {
            config.object_store_secret_key = Some(value.clone());
        }
        if let Some(value) = values.get("QR_QUEUE_URL") {
            config.queue_url.clone_from(value);
            config.queue_url_from_secret = true;
        }
        if let Some(value) = values.get("QR_QUEUE_GENERATION") {
            let parsed: u32 = value.parse().map_err(|_| ConfigError::Invalid {
                name: "QR_QUEUE_GENERATION",
                value: value.clone(),
                reason: "must be a positive integer",
            })?;
            if parsed == 0 {
                return Err(ConfigError::Invalid {
                    name: "QR_QUEUE_GENERATION",
                    value: value.clone(),
                    reason: "must be at least one",
                });
            }
            config.queue_generation = parsed;
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
            validate_bearer_token(value).map_err(|reason| ConfigError::Secret {
                name: "QR_BEARER_TOKEN",
                reason,
            })?;
            config.bearer_token = Some(value.clone());
        }
        if let Some(value) = values.get("QR_TOKEN_RELOAD_SECONDS") {
            let parsed: u64 = value.parse().map_err(|_| ConfigError::Invalid {
                name: "QR_TOKEN_RELOAD_SECONDS",
                value: value.clone(),
                reason: "must be a whole number of seconds",
            })?;
            if !(1..=MAX_TOKEN_RELOAD_SECONDS).contains(&parsed) {
                return Err(ConfigError::Invalid {
                    name: "QR_TOKEN_RELOAD_SECONDS",
                    value: value.clone(),
                    reason: "must be between 1 and 3600 seconds",
                });
            }
            config.bearer_token_reload = Duration::from_secs(parsed);
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
        if let Some(value) = values.get("QR_METRICS_BIND") {
            let value = value.trim();
            if value.is_empty() {
                config.metrics_bind = None;
            } else {
                let parsed: SocketAddr = value.parse().map_err(|_| ConfigError::Invalid {
                    name: "QR_METRICS_BIND",
                    value: value.to_owned(),
                    reason: "must be an IP:port",
                })?;
                if config.internal_only {
                    validate_private_address("QR_METRICS_BIND", parsed)?;
                }
                config.metrics_bind = Some(parsed);
            }
        }
        if let Some(value) = values.get("QR_DRAIN_TIMEOUT_HTTP") {
            config.drain_timeout_http = parse_drain_timeout("QR_DRAIN_TIMEOUT_HTTP", value)?;
        }
        if let Some(value) = values.get("QR_DRAIN_TIMEOUT_WORK") {
            config.drain_timeout_work = parse_drain_timeout("QR_DRAIN_TIMEOUT_WORK", value)?;
        }
        Ok(config)
    }
}

/// Parse a positive, bounded drain deadline in seconds.
///
/// The upper bound keeps a stalled request or stage from holding a container
/// open past the orchestrator's stop grace period.
fn parse_drain_timeout(name: &'static str, value: &str) -> Result<Duration, ConfigError> {
    let seconds: u64 = value.parse().map_err(|_| ConfigError::Invalid {
        name,
        value: value.to_owned(),
        reason: "must be a whole number of seconds",
    })?;
    if !(1..=MAX_DRAIN_TIMEOUT_SECONDS).contains(&seconds) {
        return Err(ConfigError::Invalid {
            name,
            value: value.to_owned(),
            reason: "must be between 1 and 300 seconds",
        });
    }
    Ok(Duration::from_secs(seconds))
}

/// Whether this process was started as a one-shot owner migration.
///
/// The check is argument-only and is evaluated before a service builds a
/// database pool. The same image therefore keeps one migration path per
/// service binary without a separate migration crate or a second binary.
#[must_use]
pub fn migrate_only_requested() -> bool {
    migrate_only_in(std::env::args_os().skip(1))
}

fn migrate_only_in(args: impl Iterator<Item = OsString>) -> bool {
    args.into_iter()
        .any(|argument| argument == std::ffi::OsStr::new(MIGRATE_ONLY_ARG))
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
    validate_private_address("QR_BIND", address)
}

fn validate_private_address(name: &'static str, address: SocketAddr) -> Result<(), ConfigError> {
    if is_private_bind(address.ip()) {
        return Ok(());
    }
    Err(ConfigError::Invalid {
        name,
        value: address.to_string(),
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

#[cfg(test)]
mod tests;
