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

use std::path::Path;
use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};
use sqlx::{Connection, PgConnection, PgPool};

use crate::adapters::{Backend, BoxFuture, Database, DatabaseError};
use crate::migrations::{self, Applied, Migration, MigrationError};

/// The advisory lock key that serializes migration runs.
pub const MIGRATION_LOCK: i64 = 0x5152_4d49_4752_4154;

/// The log filter directive for this driver.
pub const LOG_FILTER_DIRECTIVE: &str = "sqlx=warn";

/// A Postgres handle.
#[derive(Debug, Clone)]
pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    /// Connect with a small pool over the URL's plaintext mode.
    ///
    /// # Errors
    ///
    /// Returns the driver message when the server is unreachable.
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        Self::connect_with_tls(url, None).await
    }

    /// Connect with a small pool and an explicit server CA.
    ///
    /// When `ca` is set, the connection is forced to
    /// [`PgSslMode::VerifyFull`] and the private bundle replaces the platform
    /// roots for this server. A missing file, a handshake failure or a hostname
    /// mismatch returns an error: there is no plaintext retry. The caller turns
    /// that error into a failed migration job or a degraded `/ready`, never a
    /// fallback connection.
    ///
    /// # Errors
    ///
    /// Returns the driver message when the URL is malformed or the server is
    /// unreachable.
    pub async fn connect_with_tls(url: &str, ca: Option<&Path>) -> Result<Self, DatabaseError> {
        let mut options =
            PgConnectOptions::from_str(url).map_err(|error| DatabaseError(error.to_string()))?;
        if let Some(ca) = ca {
            options = options.ssl_mode(PgSslMode::VerifyFull).ssl_root_cert(ca);
        }
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options)
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
    apply_migrations_in(pool, "public", available).await
}

/// Apply every pending migration to one schema's ledger under the session lock.
///
/// Each service owns one ledger, so the catalog and gateway migration lists do
/// not collide on a version number. The ledger name is a fixed service name,
/// validated before it is interpolated into DDL.
///
/// # Errors
///
/// Returns [`MigrationError`] when the ledger name is invalid, the plan is
/// refused, a statement fails, or the lock cannot be released.
pub async fn apply_migrations_in(
    pool: &PgPool,
    ledger: &str,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    let ledger = ledger_table(ledger)?;
    let mut connection = pool
        .acquire()
        .await
        .map_err(|error| MigrationError::Sql(error.to_string()))?;

    lock(&mut connection).await?;

    let result = apply_locked(&mut connection, &ledger, available).await;

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

fn ledger_table(ledger: &str) -> Result<String, MigrationError> {
    if ledger.is_empty()
        || !ledger
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(MigrationError::Sql(format!(
            "ledger {ledger:?} is not a schema name"
        )));
    }
    Ok(format!("{ledger}.schema_migrations"))
}

async fn apply_locked(
    connection: &mut PgConnection,
    ledger: &str,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    let failed = |error: sqlx::Error| MigrationError::Sql(error.to_string());
    let schema = ledger
        .split_once('.')
        .map_or(ledger, |(schema, _table)| schema);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE SCHEMA IF NOT EXISTS {schema}"
    )))
    .execute(&mut *connection)
    .await
    .map_err(failed)?;
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "CREATE TABLE IF NOT EXISTS {ledger} (
             version    bigint PRIMARY KEY,
             name       text NOT NULL,
             checksum   text NOT NULL,
             applied_at timestamptz NOT NULL DEFAULT now()
         )"
    )))
    .execute(&mut *connection)
    .await
    .map_err(failed)?;

    let rows: Vec<(i64, String, String)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT version, name, checksum FROM {ledger} ORDER BY version"
    )))
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
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "INSERT INTO {ledger} (version, name, checksum) VALUES ($1, $2, $3)"
        )))
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

#[cfg(test)]
mod tls_tests {
    use std::path::Path;
    use std::str::FromStr;

    use sqlx::postgres::{PgConnectOptions, PgSslMode};

    use super::PgDatabase;

    #[test]
    fn a_tls_url_parses_as_verify_full() {
        let options = PgConnectOptions::from_str(
            "postgres://qr_gateway_app:secret@postgres:5432/qr_gateway?sslmode=verify-full",
        )
        .expect("the URL");
        assert!(matches!(options.get_ssl_mode(), PgSslMode::VerifyFull));
    }

    #[tokio::test]
    async fn a_tls_pool_refuses_a_malformed_url_before_connecting() {
        let error =
            PgDatabase::connect_with_tls("not-a-postgres-url", Some(Path::new("/run/pki/ca.crt")))
                .await
                .expect_err("a malformed URL must fail closed");
        assert!(!error.to_string().is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::ledger_table;

    #[test]
    fn a_ledger_name_is_one_identifier() {
        assert_eq!(
            ledger_table("catalog").expect("catalog"),
            "catalog.schema_migrations"
        );
        assert_eq!(
            ledger_table("public").expect("public"),
            "public.schema_migrations"
        );
    }

    #[test]
    fn a_ledger_name_cannot_escape_its_schema() {
        for name in ["", "catalog; drop table x", "a.b", "a-b", "../x"] {
            assert!(ledger_table(name).is_err(), "{name:?} was accepted");
        }
    }
}
