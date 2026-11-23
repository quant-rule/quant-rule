//! The gateway bearer-token source and rotation.
//!
//! The only production token source is `QR_BEARER_TOKEN_FILE`. The file is
//! read once at startup, then every `QR_TOKEN_RELOAD_SECONDS` and on `SIGHUP`.
//! A successful read swaps the current value behind one lock; an in-flight
//! request keeps the value it started with, and the next request sees the new
//! one. A missing or invalid file keeps the last good token: a bad edit must
//! never silently disable authentication.
//!
//! The token value is never logged, fingerprinted, hashed or exposed through
//! `/version`. Failures are counted by a stable reason and the age of the
//! current token is a gauge.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use quantrule_runtime::config::{
    ConfigError, FileSecretReader, MIN_BEARER_TOKEN_BYTES, SecretError, SecretFileReader,
    parse_secret_contents, validate_bearer_token,
};
use quantrule_runtime::metrics::Metrics;
use quantrule_runtime::{Config, RuntimeError, Shutdown};

/// One immutable token value and when it became current.
#[derive(Clone)]
struct Snapshot {
    token: Arc<str>,
    loaded_at: Instant,
}

struct Inner {
    current: RwLock<Arc<Snapshot>>,
    path: Option<PathBuf>,
    reader: Arc<dyn SecretFileReader>,
    reload: Duration,
    metrics: Option<Metrics>,
}

/// The gateway's current bearer token.
///
/// Cloning shares one cell, so the middleware and the reload task see the same
/// swap. `Debug` reports only whether a token exists.
#[derive(Clone)]
pub struct TokenSource {
    inner: Arc<Inner>,
}

impl fmt::Debug for TokenSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TokenSource")
            .field("configured", &true)
            .field("path", &self.inner.path)
            .field("reload", &self.inner.reload)
            .finish_non_exhaustive()
    }
}

impl TokenSource {
    /// Build the production source from the resolved configuration.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Config`] when the configured token file is
    /// outside the secret-file contract or the token is invalid.
    pub fn start(config: &Config, metrics: Metrics) -> Result<Option<Self>, RuntimeError> {
        Self::with_reader(
            config.bearer_token_file.clone(),
            config.bearer_token.clone(),
            Arc::new(FileSecretReader),
            config.bearer_token_reload,
            Some(metrics),
        )
        .map_err(RuntimeError::Config)
    }

    /// Build a source with an explicit reader, for tests.
    ///
    /// `None` means authentication is disabled: there is no path and no
    /// initial value. A path with no initial value is read through `reader`; an
    /// initial value is validated and used, and the path is re-read on reload.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Secret`] naming `QR_BEARER_TOKEN_FILE` when the
    /// file cannot be read or the token is invalid.
    pub fn with_reader(
        path: Option<PathBuf>,
        initial: Option<String>,
        reader: Arc<dyn SecretFileReader>,
        reload: Duration,
        metrics: Option<Metrics>,
    ) -> Result<Option<Self>, ConfigError> {
        if path.is_none() && initial.is_none() {
            return Ok(None);
        }
        let value = match initial {
            Some(value) => value,
            None => load_token(reader.as_ref(), path.as_deref()).map_err(TokenError::config)?,
        };
        validate(value.as_str()).map_err(TokenError::config)?;
        let source = Self {
            inner: Arc::new(Inner {
                current: RwLock::new(Arc::new(Snapshot {
                    token: Arc::from(value),
                    loaded_at: Instant::now(),
                })),
                path,
                reader,
                reload,
                metrics,
            }),
        };
        source.publish_age();
        Ok(Some(source))
    }

    /// Build a static source from a token the configuration already validated.
    ///
    /// This is the `AppState` fallback for a test or a hand-built `Config`; the
    /// gateway binary uses [`Self::start`], which reads and validates the file
    /// and starts the reload task.
    pub(crate) fn from_trusted(token: &str) -> Self {
        Self {
            inner: Arc::new(Inner {
                current: RwLock::new(Arc::new(Snapshot {
                    token: Arc::from(token),
                    loaded_at: Instant::now(),
                })),
                path: None,
                reader: Arc::new(FileSecretReader),
                reload: Duration::from_secs(0),
                metrics: None,
            }),
        }
    }

    /// The token a request must present.
    ///
    /// The returned `Arc` is one immutable snapshot: a swap after this call
    /// cannot change an in-flight request's comparison.
    #[must_use]
    pub fn current(&self) -> Arc<str> {
        read(&self.inner.current).token.clone()
    }

    /// Whether this source re-reads a file.
    #[must_use]
    pub fn is_reloadable(&self) -> bool {
        self.inner.path.is_some()
    }

    /// Re-read the token file and swap it in.
    ///
    /// Returns `true` when the file was read and accepted, and `false` when the
    /// last good token was kept. A failure increments
    /// `quantrule_auth_reload_failures_total{reason}` and never logs a value.
    pub fn reload(&self) -> bool {
        let Some(path) = self.inner.path.as_deref() else {
            return true;
        };
        let value = match load_token(self.inner.reader.as_ref(), Some(path))
            .and_then(|value| validate(value.as_str()).map(|()| value))
        {
            Ok(value) => value,
            Err(error) => {
                let reason = error.reason();
                if let Some(metrics) = &self.inner.metrics {
                    metrics.auth_reload_failure(reason);
                }
                self.publish_age();
                tracing::warn!(
                    reason,
                    "gateway bearer token reload failed; keeping the last good token"
                );
                return false;
            }
        };
        let next = Arc::new(Snapshot {
            token: Arc::from(value),
            loaded_at: Instant::now(),
        });
        let changed = {
            let mut current = write(&self.inner.current);
            if current.token == next.token {
                false
            } else {
                *current = next;
                true
            }
        };
        self.publish_age();
        if changed {
            tracing::info!("gateway bearer token reloaded");
        } else {
            tracing::debug!("gateway bearer token unchanged");
        }
        true
    }

    /// Spawn the interval and `SIGHUP` reload task.
    ///
    /// Returns `None` for a source with no file. The task stops with the same
    /// shutdown signal as the server.
    #[must_use]
    pub fn spawn_reload(&self, shutdown: Shutdown) -> Option<tokio::task::JoinHandle<()>> {
        self.inner.path.as_ref()?;
        let source = self.clone();
        let interval = self.inner.reload;
        Some(tokio::spawn(async move {
            let mut shutdown = shutdown;
            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            // The startup read already ran; consume the immediate first tick.
            ticker.tick().await;
            #[cfg(unix)]
            {
                use tokio::signal::unix::{SignalKind, signal};
                let mut hangup = signal(SignalKind::hangup()).ok();
                loop {
                    tokio::select! {
                        _ = shutdown.wait() => break,
                        _ = ticker.tick() => {
                            source.reload();
                        }
                        _ = wait_hangup(&mut hangup) => {
                            source.reload();
                        }
                    }
                }
            }
            #[cfg(not(unix))]
            {
                loop {
                    tokio::select! {
                        _ = shutdown.wait() => break,
                        _ = ticker.tick() => {
                            source.reload();
                        }
                    }
                }
            }
        }))
    }

    fn publish_age(&self) {
        if let Some(metrics) = &self.inner.metrics {
            let age = read(&self.inner.current).loaded_at.elapsed().as_secs();
            metrics.set_auth_token_age(age);
        }
    }
}

/// Wait for one `SIGHUP`, or forever when the signal stream is unavailable.
#[cfg(unix)]
async fn wait_hangup(hangup: &mut Option<tokio::signal::unix::Signal>) -> Option<()> {
    match hangup {
        Some(stream) => stream.recv().await,
        None => std::future::pending().await,
    }
}

/// Why a token file or value was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenError {
    Read,
    Empty,
    Format,
    Short,
}

impl TokenError {
    /// A stable reason for a log and a metric label.
    fn reason(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Empty => "empty",
            Self::Format => "format",
            Self::Short => "short",
        }
    }

    /// The startup error, which names the variable but never the value.
    fn config(self) -> ConfigError {
        let reason = match self {
            Self::Read => SecretError::Unreadable.reason(),
            Self::Empty => SecretError::Empty.reason(),
            Self::Format => "must be a valid token file value",
            Self::Short => "must be at least 32 characters",
        };
        ConfigError::Secret {
            name: "QR_BEARER_TOKEN_FILE",
            reason,
        }
    }
}

/// Read and validate one token file through the shared secret-file contract.
fn load_token(reader: &dyn SecretFileReader, path: Option<&Path>) -> Result<String, TokenError> {
    let Some(path) = path else {
        return Err(TokenError::Read);
    };
    if !path.is_absolute() {
        return Err(TokenError::Format);
    }
    let contents = reader.read(path).map_err(|_| TokenError::Read)?;
    let value = parse_secret_contents(&contents).map_err(|error| match error {
        SecretError::Empty | SecretError::Whitespace => TokenError::Empty,
        _ => TokenError::Format,
    })?;
    validate(value.as_str())?;
    Ok(value)
}

/// Validate one token and name the stable reload reason.
fn validate(value: &str) -> Result<(), TokenError> {
    if value.len() < MIN_BEARER_TOKEN_BYTES {
        return Err(TokenError::Short);
    }
    validate_bearer_token(value).map_err(|_| TokenError::Format)
}

fn read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn write<T>(lock: &RwLock<T>) -> std::sync::RwLockWriteGuard<'_, T> {
    lock.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use super::*;

    const OLD: &str = "old-0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
    const NEW: &str = "new-0123456789abcdef0123456789abcdef0123456789abcdef0123456789";

    #[derive(Default)]
    struct SharedReader {
        files: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
    }

    impl SharedReader {
        fn set(&self, path: &str, value: &str) {
            lock_files(self).insert(PathBuf::from(path), value.as_bytes().to_vec());
        }

        fn remove(&self, path: &str) {
            lock_files(self).remove(Path::new(path));
        }
    }

    impl SecretFileReader for SharedReader {
        fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
            lock_files(self)
                .get(path)
                .cloned()
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }

    fn lock_files(reader: &SharedReader) -> std::sync::MutexGuard<'_, BTreeMap<PathBuf, Vec<u8>>> {
        reader
            .files
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn source(reader: Arc<SharedReader>, metrics: Option<Metrics>) -> TokenSource {
        TokenSource::with_reader(
            Some(PathBuf::from("/run/secrets/bearer_token")),
            None,
            reader,
            Duration::from_secs(5),
            metrics,
        )
        .expect("a token source")
        .expect("configured")
    }

    #[test]
    fn a_configured_file_is_the_startup_token() {
        let reader = Arc::new(SharedReader::default());
        reader.set("/run/secrets/bearer_token", OLD);
        let source = source(reader, None);
        assert_eq!(&*source.current(), OLD);
        assert!(source.is_reloadable());
    }

    #[test]
    fn no_path_and_no_token_disables_authentication() {
        let disabled = TokenSource::with_reader(
            None,
            None,
            Arc::new(SharedReader::default()),
            Duration::from_secs(5),
            None,
        )
        .expect("disabled");
        assert!(disabled.is_none());
    }

    #[test]
    fn an_invalid_startup_file_fails_closed() {
        for contents in ["", "\n", "short", "two words"] {
            let reader = Arc::new(SharedReader::default());
            reader.set("/run/secrets/bearer_token", contents);
            let error = TokenSource::with_reader(
                Some(PathBuf::from("/run/secrets/bearer_token")),
                None,
                reader,
                Duration::from_secs(5),
                None,
            )
            .expect_err("an invalid file must fail startup");
            assert!(
                error.to_string().contains("QR_BEARER_TOKEN_FILE"),
                "{error}"
            );
            if !contents.is_empty() {
                assert!(!error.to_string().contains(contents), "{error}");
            }
        }
    }

    #[test]
    fn a_reload_swaps_the_token_and_keeps_the_last_good_one() {
        let reader = Arc::new(SharedReader::default());
        reader.set("/run/secrets/bearer_token", OLD);
        let metrics = Metrics::new("gateway", "1.6.0", None);
        let source = source(Arc::clone(&reader), Some(metrics.clone()));

        reader.set("/run/secrets/bearer_token", NEW);
        assert!(source.reload());
        assert_eq!(&*source.current(), NEW);

        reader.set("/run/secrets/bearer_token", "short");
        assert!(!source.reload());
        assert_eq!(&*source.current(), NEW, "the last good token must survive");

        reader.remove("/run/secrets/bearer_token");
        assert!(!source.reload());
        assert_eq!(&*source.current(), NEW);

        let text = metrics.encode();
        assert!(
            text.contains("quantrule_auth_reload_failures_total{reason=\"short\"} 1"),
            "{text}"
        );
        assert!(
            text.contains("quantrule_auth_reload_failures_total{reason=\"read\"} 1"),
            "{text}"
        );
    }

    #[test]
    fn debug_never_renders_the_token() {
        let reader = Arc::new(SharedReader::default());
        reader.set("/run/secrets/bearer_token", OLD);
        let source = source(reader, None);
        let rendered = format!("{source:?}");
        assert!(!rendered.contains(OLD), "{rendered}");
    }
}
