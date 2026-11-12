//! The runtime error.
//!
//! One error type for startup and serving. A dependency that is merely
//! unreachable is a health report, not a startup error: the process stays up so
//! the operator can see what is down.

/// A failure that stops the process or a request.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// Invalid or missing configuration.
    #[error("configuration: {0}")]
    Config(#[from] crate::runtime::config::ConfigError),
    /// The database client failed.
    #[error("postgres: {0}")]
    Postgres(#[from] sqlx::Error),
    /// The object store client failed.
    #[error("object store: {0}")]
    ObjectStore(String),
    /// The queue client failed.
    #[error("queue: {0}")]
    Queue(#[from] lapin::Error),
    /// The listener could not bind or the server failed.
    #[error("serve: {0}")]
    Serve(String),
    /// Standard I/O failed.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
