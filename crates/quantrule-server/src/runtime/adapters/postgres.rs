//! The Postgres adapter.
//!
//! The only module allowed to name the `sqlx` client. A service asks the trait.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::runtime::adapters::{Backend, BoxFuture, Database, DatabaseError};

/// A Postgres handle.
#[derive(Debug, Clone)]
pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    /// Connect with a small pool.
    ///
    /// # Errors
    ///
    /// Returns the driver message when the server is unreachable.
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(url)
            .await
            .map_err(|error| DatabaseError(error.to_string()))?;
        Ok(Self { pool })
    }

    /// The pool, for a module that owns a repository.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl Database for PgDatabase {
    fn backend(&self) -> Backend {
        Backend::Postgres
    }

    fn ping(&self) -> BoxFuture<Result<(), DatabaseError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("SELECT 1")
                .execute(&pool)
                .await
                .map(|_| ())
                .map_err(|error| DatabaseError(error.to_string()))
        })
    }
}
