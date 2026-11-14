//! The Postgres adapter.
//!
//! The runtime owns the pool, the liveness probe, and the migration runner. A
//! service owns its repositories: they take the pool from [`PgDatabase::pool`]
//! and live in the crate that owns the rows, so this module never names a domain
//! type.
//!
//! The runner pins one pooled connection for the whole operation, so the
//! advisory lock and its release share a session. Releasing through the pool
//! could land on another connection and leave the lock held by an idle session,
//! which is what deadlocked three migrators in #772.

use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool};

use crate::adapters::{Backend, BoxFuture, Database, DatabaseError};
use crate::migrations::{self, Applied, Migration, MigrationError};

/// The advisory lock key that serializes migration runs.
pub const MIGRATION_LOCK: i64 = 0x5152_4d49_4752_4154;

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

/// Apply every pending migration under one session's advisory lock.
///
/// One [`PgPool::acquire`] connection carries the lock, the ledger, the
/// migrations, and the release. If the connection dies, Postgres releases the
/// session lock; if the release fails while the session is alive, the session is
/// closed rather than returned to the pool with the lock held.
///
/// # Errors
///
/// Returns [`MigrationError`] when the plan is refused, a statement fails, or the
/// lock cannot be released.
pub async fn apply_migrations(
    pool: &PgPool,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    let mut connection = pool
        .acquire()
        .await
        .map_err(|error| MigrationError::Sql(error.to_string()))?;

    lock(&mut connection).await?;

    let result = apply_locked(&mut connection, available).await;

    match release(&mut connection).await {
        Ok(()) => result,
        Err(release_error) => {
            tracing::error!(
                error = %release_error,
                "the migration lock could not be released; closing the session"
            );
            let _ = connection.close().await;
            match result {
                Err(error) => Err(error),
                Ok(_) => Err(release_error),
            }
        }
    }
}

async fn lock(connection: &mut PgConnection) -> Result<(), MigrationError> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(MIGRATION_LOCK)
        .execute(&mut *connection)
        .await
        .map(|_| ())
        .map_err(|error| MigrationError::Sql(error.to_string()))
}

async fn release(connection: &mut PgConnection) -> Result<(), MigrationError> {
    let unlocked: bool = sqlx::query_scalar("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK)
        .fetch_one(&mut *connection)
        .await
        .map_err(|error| MigrationError::Sql(error.to_string()))?;
    if unlocked {
        Ok(())
    } else {
        Err(MigrationError::Sql(
            "the migration session did not hold the advisory lock".to_owned(),
        ))
    }
}

async fn apply_locked(
    connection: &mut PgConnection,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    let failed = |error: sqlx::Error| MigrationError::Sql(error.to_string());
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS public.schema_migrations (
             version    bigint PRIMARY KEY,
             name       text NOT NULL,
             checksum   text NOT NULL,
             applied_at timestamptz NOT NULL DEFAULT now()
         )",
    )
    .execute(&mut *connection)
    .await
    .map_err(failed)?;

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT version, name, checksum FROM public.schema_migrations ORDER BY version",
    )
    .fetch_all(&mut *connection)
    .await
    .map_err(failed)?;
    let applied: Vec<Applied> = rows
        .into_iter()
        .map(|(version, name, checksum)| Applied {
            version,
            name,
            checksum,
        })
        .collect();

    let pending = migrations::plan(&applied, available)?;

    for migration in &pending {
        let mut transaction = connection.begin().await.map_err(failed)?;
        sqlx::raw_sql(migration.sql)
            .execute(&mut *transaction)
            .await
            .map_err(failed)?;
        sqlx::query(
            "INSERT INTO public.schema_migrations (version, name, checksum) VALUES ($1, $2, $3)",
        )
        .bind(migration.version)
        .bind(migration.name)
        .bind(migrations::checksum(migration.sql))
        .execute(&mut *transaction)
        .await
        .map_err(failed)?;
        transaction.commit().await.map_err(failed)?;
    }

    Ok(pending.len())
}
