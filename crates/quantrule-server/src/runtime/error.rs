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
    /// The object store client failed.
    #[error("object store: {0}")]
    ObjectStore(String),
    /// The database could not be reached.
    #[error("database: {0}")]
    Database(String),
    /// The broker could not be reached.
    #[error("queue: {0}")]
    Queue(String),
    /// The listener could not bind or the server failed.
    #[error("serve: {0}")]
    Serve(String),
    /// The schema could not be prepared.
    #[error("migrations: {0}")]
    Migrations(String),
    /// Standard I/O failed.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
