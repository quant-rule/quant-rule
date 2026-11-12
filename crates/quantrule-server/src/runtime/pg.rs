//! Postgres.
//!
//! The pool is created once per process. A check is a round trip, not a socket
//! inspection, so a half-open connection is reported as down.

pub use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::runtime::error::RuntimeError;

/// Open a pool.
///
/// # Errors
///
/// Returns [`RuntimeError::Postgres`] when the server is unreachable or the URL
/// is malformed.
pub async fn connect(url: &str) -> Result<PgPool, RuntimeError> {
    PgPoolOptions::new()
        .max_connections(8)
        .connect(url)
        .await
        .map_err(RuntimeError::from)
}

/// Round-trip the pool.
///
/// # Errors
///
/// Returns the driver message when the query fails.
pub async fn check(pool: &PgPool) -> Result<(), String> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}
