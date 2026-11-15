//! The SQLite adapter.
//!
//! The embedded desktop profile runs the same six services against one SQLite
//! file. SQLite has no schemas, so [`crate::adapters::Schema`] is a table prefix;
//! a JSON document is `TEXT` and an instant is RFC 3339 UTC `TEXT`. The mapping
//! lives in [`crate::adapters::Dialect`], so a repository names one relation and
//! the backend spells it.
//!
//! `rusqlite` is synchronous. Every repository call goes through
//! [`SqliteDatabase::call`], which clones the guarded connection and runs the
//! closure on [`tokio::task::spawn_blocking`], so the async route shape is
//! unchanged and a local query never blocks the reactor.
//!
//! The migration runner keeps one shared `schema_migrations` table with a
//! `ledger` column. Each service owns its own baseline and its own ledger name;
//! the runtime owns the table. The runner is forward-only: it reuses
//! [`crate::migrations::plan`], records a checksum per ledger row, and refuses a
//! changed body, an unknown applied version, or a database ahead of the binary.
//! The ledger, the plan, the SQL, and the record share one immediate
//! transaction, so a crash leaves the ledger unchanged and a second apply
//! returns 0.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};

use crate::adapters::{Backend, BoxFuture, Database, DatabaseError};
use crate::migrations::{self, Applied, Migration, MigrationError};

/// The database file the desktop profile uses under `QR_DATA_DIR`.
pub const DATABASE_FILE: &str = "quantrule.sqlite";

/// A SQLite handle.
///
/// The connection sits behind a mutex because SQLite serializes writers and
/// `rusqlite::Connection` is not `Sync`. Cloning the handle shares one
/// connection, which is what the embedded profile wants: six services in six
/// processes each open the file once, and the calls within one process queue
/// behind the mutex while the blocking pool keeps the async runtime free.
#[derive(Clone)]
pub struct SqliteDatabase {
    connection: Arc<Mutex<Connection>>,
    path: Option<PathBuf>,
}

impl std::fmt::Debug for SqliteDatabase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SqliteDatabase")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteDatabase {
    /// Open or create a SQLite file.
    ///
    /// A missing parent directory is created. A file database runs in WAL mode
    /// with foreign keys enforced, a five-second busy timeout, and a full
    /// synchronous commit, because the embedded profile opens the same file
    /// from more than one process.
    ///
    /// # Errors
    ///
    /// Returns [`DatabaseError`] when the parent cannot be created, SQLite
    /// rejects the file, or the filesystem does not support WAL.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        let path = path.as_ref();
        if path == Path::new(":memory:") {
            return Self::open_in_memory();
        }
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| {
                DatabaseError(format!("cannot create {}: {error}", parent.display()))
            })?;
        }
        let connection = Connection::open(path).map_err(database_error)?;
        configure(&connection, true)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path: Some(path.to_path_buf()),
        })
    }

    /// Open an empty in-memory database.
    ///
    /// It exists for tests and short-lived sessions. It is never written to a
    /// file, so it cannot be shared with another process.
    ///
    /// # Errors
    ///
    /// Returns [`DatabaseError`] when SQLite cannot create the database or
    /// configure it.
    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let connection = Connection::open_in_memory().map_err(database_error)?;
        configure(&connection, false)?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            path: None,
        })
    }

    /// The database file, or `None` for an in-memory database.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Run one repository operation on the blocking pool.
    ///
    /// The closure runs while the handle's mutex is held, so SQLite work stays
    /// serialized and a second call queues rather than opening a second
    /// connection. The closure must not await or call back into the same
    /// handle; it is a short local operation by contract.
    ///
    /// # Errors
    ///
    /// Returns [`DatabaseError`] when the blocking task panics, the lock is
    /// poisoned, or the closure returns an error.
    pub async fn call<T, E>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, E> + Send + 'static,
    ) -> Result<T, DatabaseError>
    where
        T: Send + 'static,
        E: std::fmt::Display + Send + 'static,
    {
        let connection = Arc::clone(&self.connection);
        let joined = tokio::task::spawn_blocking(move || {
            let mut connection = connection
                .lock()
                .map_err(|_| DatabaseError("the SQLite connection lock is poisoned".to_owned()))?;
            f(&mut connection).map_err(|error| DatabaseError(error.to_string()))
        })
        .await;
        match joined {
            Ok(result) => result,
            Err(error) => Err(DatabaseError(format!(
                "the SQLite blocking task failed: {error}"
            ))),
        }
    }
}

impl Database for SqliteDatabase {
    fn backend(&self) -> Backend {
        Backend::Sqlite
    }

    fn ping(&self) -> BoxFuture<Result<(), DatabaseError>> {
        let database = self.clone();
        Box::pin(async move {
            database
                .call(|connection| {
                    connection
                        .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))
                        .map(|_| ())
                })
                .await
        })
    }
}

/// Apply one service's SQLite baseline under the shared ledger.
///
/// `ledger` is the service name (`gateway`, `catalog`, `runs`, `research`,
/// `simulation`, or `queue`). The runner filters the ledger rows by that name
/// and records each applied body with its checksum. The ledger table is created
/// inside the same immediate transaction as the plan, so a crash leaves neither
/// a partial schema nor a partial ledger.
///
/// # Errors
///
/// Returns [`MigrationError`] when the ledger name is invalid, the plan is
/// refused, a statement fails, or the transaction cannot be committed.
pub async fn apply_migrations_in(
    database: &SqliteDatabase,
    ledger: &str,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    if !super::is_identifier(ledger) {
        return Err(MigrationError::Sql(format!(
            "ledger {ledger:?} is not a schema name"
        )));
    }
    let connection = Arc::clone(&database.connection);
    let ledger = ledger.to_owned();
    let available = available.to_vec();
    let joined = tokio::task::spawn_blocking(move || {
        let mut connection = connection.lock().map_err(|_| {
            MigrationError::Sql("the SQLite connection lock is poisoned".to_owned())
        })?;
        apply_locked(&mut connection, &ledger, &available)
    })
    .await;
    match joined {
        Ok(result) => result,
        Err(error) => Err(MigrationError::Sql(format!(
            "the SQLite migration task failed: {error}"
        ))),
    }
}

fn apply_locked(
    connection: &mut Connection,
    ledger: &str,
    available: &[Migration],
) -> Result<usize, MigrationError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(migration_error)?;

    transaction
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                 ledger     TEXT NOT NULL,
                 version    INTEGER NOT NULL,
                 name       TEXT NOT NULL,
                 checksum   TEXT NOT NULL,
                 applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
                 PRIMARY KEY (ledger, version)
             ) STRICT",
        )
        .map_err(migration_error)?;

    let mut applied = Vec::new();
    {
        let mut statement = transaction
            .prepare(
                "SELECT version, name, checksum
                   FROM schema_migrations
                  WHERE ledger = ?1
                  ORDER BY version",
            )
            .map_err(migration_error)?;
        let rows = statement
            .query_map(rusqlite::params![ledger], |row| {
                Ok(Applied {
                    version: row.get(0)?,
                    name: row.get(1)?,
                    checksum: row.get(2)?,
                })
            })
            .map_err(migration_error)?;
        for row in rows {
            applied.push(row.map_err(migration_error)?);
        }
    }

    let pending = migrations::plan(&applied, available)?;
    for migration in &pending {
        transaction
            .execute_batch(migration.sql)
            .map_err(migration_error)?;
        transaction
            .execute(
                "INSERT INTO schema_migrations (ledger, version, name, checksum)
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    ledger,
                    migration.version,
                    migration.name,
                    migrations::checksum(migration.sql)
                ],
            )
            .map_err(migration_error)?;
    }

    transaction.commit().map_err(migration_error)?;
    Ok(pending.len())
}

fn configure(connection: &Connection, file: bool) -> Result<(), DatabaseError> {
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(database_error)?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(database_error)?;
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(database_error)?;
    if file {
        let mode: String = connection
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))
            .map_err(database_error)?;
        if !mode.eq_ignore_ascii_case("wal") {
            return Err(DatabaseError(format!(
                "SQLite refused WAL (journal_mode is {mode}); the embedded profile needs a local filesystem"
            )));
        }
    }
    Ok(())
}

fn database_error(error: rusqlite::Error) -> DatabaseError {
    DatabaseError(error.to_string())
}

fn migration_error(error: rusqlite::Error) -> MigrationError {
    MigrationError::Sql(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::SqliteDatabase;

    #[test]
    fn the_handle_is_send_and_sync() {
        fn assert<T: Send + Sync>() {}
        assert::<SqliteDatabase>();
    }

    #[test]
    fn an_in_memory_database_has_no_path() {
        let database = SqliteDatabase::open_in_memory().expect("the database opens");
        assert_eq!(database.path(), None);
    }
}
