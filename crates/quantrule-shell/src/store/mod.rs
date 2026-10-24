//! The SQLite store.
//!
//! The store persists one dataset per ingest: the wide per-bar record, the
//! pivots, levels, events, legs, consensus, and the materialized Layer 3. The
//! module owns the connection, the PRAGMAs, and the schema migration; the
//! command layer owns the file path and the rendering.
//!
//! The schema is the ADR's version 1. A migration runs in one transaction, and
//! a failed migration rolls back to the previous version.

mod error;
mod schema;
#[cfg(test)]
mod tests;

pub(crate) use error::StoreError;

use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

use schema::{SCHEMA_V1, SCHEMA_VERSION};

/// An open store.
#[derive(Debug)]
pub(crate) struct Store {
    connection: Connection,
    path: Option<PathBuf>,
}

impl Store {
    /// Opens or creates a database at `path` and applies the schema.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the directory or file cannot be created, the
    /// file is not a database, the schema is newer than this binary, or a
    /// migration fails.
    pub(crate) fn open(path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| StoreError::Directory {
                path: path.to_path_buf(),
                source,
            })?;
        }

        let mut connection = Connection::open(path).map_err(|source| StoreError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        configure(&connection).map_err(|source| map_error(source, path))?;
        enable_wal(&connection).map_err(|source| map_error(source, path))?;
        init(&mut connection, Some(path))?;

        Ok(Self {
            connection,
            path: Some(path.to_path_buf()),
        })
    }

    /// Opens an in-memory database and applies the schema.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the schema cannot be applied.
    #[cfg(test)]
    pub(crate) fn open_in_memory() -> Result<Self, StoreError> {
        let mut connection = Connection::open_in_memory().map_err(|source| StoreError::Open {
            path: PathBuf::from(":memory:"),
            source,
        })?;
        configure(&connection).map_err(|source| StoreError::Query { source })?;
        init(&mut connection, None)?;

        Ok(Self {
            connection,
            path: None,
        })
    }

    /// The schema version in the file.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError`] when the version cannot be read.
    pub(crate) fn schema_version(&self) -> Result<i64, StoreError> {
        read_version(&self.connection, self.path.as_deref())
    }

    /// Runs `PRAGMA integrity_check`.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Corrupt`] when SQLite reports a problem, and
    /// [`StoreError::Query`] when the check cannot be executed.
    #[cfg(test)]
    pub(crate) fn integrity_check(&self) -> Result<(), StoreError> {
        let result: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(|source| StoreError::Query { source })?;
        if result == "ok" {
            Ok(())
        } else {
            Err(StoreError::Corrupt {
                path: self.path.clone().unwrap_or_default(),
                detail: result,
            })
        }
    }

    /// The connection, for the writers and tests.
    #[cfg(test)]
    pub(crate) fn connection(&self) -> &Connection {
        &self.connection
    }
}

fn configure(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch(
        "PRAGMA foreign_keys=ON;\nPRAGMA synchronous=NORMAL;\nPRAGMA busy_timeout=5000;",
    )
}

fn enable_wal(connection: &Connection) -> Result<(), rusqlite::Error> {
    connection.execute_batch("PRAGMA journal_mode=WAL;")
}

fn map_error(source: rusqlite::Error, path: &Path) -> StoreError {
    if is_corrupt(&source) {
        StoreError::Corrupt {
            path: path.to_path_buf(),
            detail: source.to_string(),
        }
    } else {
        StoreError::Query { source }
    }
}

fn init(connection: &mut Connection, path: Option<&Path>) -> Result<(), StoreError> {
    let version = read_version(connection, path)?;
    if version > SCHEMA_VERSION {
        return Err(StoreError::UnsupportedVersion {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }
    if version == 0 {
        migrate(connection, path, SCHEMA_V1)?;
    }
    Ok(())
}

fn read_version(connection: &Connection, path: Option<&Path>) -> Result<i64, StoreError> {
    connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|source| match path {
            Some(path) if is_corrupt(&source) => StoreError::Corrupt {
                path: path.to_path_buf(),
                detail: source.to_string(),
            },
            _ => StoreError::Query { source },
        })
}

fn is_corrupt(error: &rusqlite::Error) -> bool {
    matches!(
        error,
        rusqlite::Error::SqliteFailure(_, Some(message))
            if message.contains("not a database") || message.contains("malformed")
    )
}

fn migrate(connection: &mut Connection, _path: Option<&Path>, ddl: &str) -> Result<(), StoreError> {
    let transaction = connection
        .transaction()
        .map_err(|source| StoreError::Migrate { source })?;
    transaction
        .execute_batch(ddl)
        .map_err(|source| StoreError::Migrate { source })?;
    transaction
        .pragma_update(None, "user_version", SCHEMA_VERSION)
        .map_err(|source| StoreError::Migrate { source })?;
    transaction
        .commit()
        .map_err(|source| StoreError::Migrate { source })?;
    Ok(())
}
