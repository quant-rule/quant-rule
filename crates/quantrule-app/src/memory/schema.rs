//! The v1 schema and its numbered migration.
//!
//! The schema is additive. `PRAGMA user_version` selects the migration, and the
//! `meta` table records the contract identity. A database with a newer version
//! is refused rather than opened and silently modified.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension};

use super::types::MemoryError;

/// The research-memory contract identity.
pub const SCHEMA: &str = "quantrule.research-memory/v1";
/// The schema version this build writes.
pub const CURRENT_VERSION: i64 = 1;

const MIGRATION_V1: &str = r#"
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    root TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE datasets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    data_version TEXT NOT NULL,
    instrument TEXT NOT NULL,
    candles INTEGER NOT NULL,
    first_open_time INTEGER,
    last_open_time INTEGER,
    created_at INTEGER NOT NULL,
    UNIQUE (workspace_id, path)
) STRICT;

CREATE TABLE configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    config_hash TEXT NOT NULL,
    schema_version TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    UNIQUE (workspace_id, path)
) STRICT;

CREATE TABLE runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    config_id INTEGER REFERENCES configs(id) ON DELETE SET NULL,
    run_key TEXT NOT NULL UNIQUE,
    data_version TEXT NOT NULL,
    config_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    finished_at INTEGER,
    summary_json TEXT NOT NULL
) STRICT;

CREATE TABLE metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('real', 'integer', 'text', 'null')),
    value REAL,
    text_value TEXT,
    UNIQUE (run_id, name)
) STRICT;

CREATE TABLE claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
    claim_type TEXT NOT NULL,
    statement TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    run_id INTEGER REFERENCES runs(id) ON DELETE SET NULL,
    body TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_runs_workspace ON runs(workspace_id);
CREATE INDEX idx_claims_run ON claims(run_id);
CREATE INDEX idx_notes_workspace ON notes(workspace_id);

CREATE TRIGGER runs_config_same_workspace_insert
BEFORE INSERT ON runs
WHEN NEW.config_id IS NOT NULL
 AND (SELECT workspace_id FROM configs WHERE id = NEW.config_id) <> NEW.workspace_id
BEGIN
    SELECT RAISE(ABORT, 'run config must belong to the run workspace');
END;

CREATE TRIGGER runs_config_same_workspace_update
BEFORE UPDATE ON runs
WHEN NEW.config_id IS NOT NULL
 AND (SELECT workspace_id FROM configs WHERE id = NEW.config_id) <> NEW.workspace_id
BEGIN
    SELECT RAISE(ABORT, 'run config must belong to the run workspace');
END;

CREATE TRIGGER notes_run_same_workspace_insert
BEFORE INSERT ON notes
WHEN NEW.run_id IS NOT NULL
 AND (SELECT workspace_id FROM runs WHERE id = NEW.run_id) <> NEW.workspace_id
BEGIN
    SELECT RAISE(ABORT, 'note run must belong to the note workspace');
END;

CREATE TRIGGER notes_run_same_workspace_update
BEFORE UPDATE ON notes
WHEN NEW.run_id IS NOT NULL
 AND (SELECT workspace_id FROM runs WHERE id = NEW.run_id) <> NEW.workspace_id
BEGIN
    SELECT RAISE(ABORT, 'note run must belong to the note workspace');
END;

INSERT INTO meta (key, value) VALUES ('schema', 'quantrule.research-memory/v1');
"#;

/// Applies every pending migration and verifies the stored identity.
///
/// A database that already contains user tables is only accepted when it
/// already carries the research-memory schema. The version alone cannot prove
/// identity: every SQLite database starts at version 0.
pub(super) fn apply(connection: &mut Connection, path: &Path) -> Result<(), MemoryError> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let version = user_version(connection)?;
    if version > CURRENT_VERSION {
        return Err(MemoryError::UnsupportedVersion {
            found: version,
            expected: CURRENT_VERSION,
        });
    }
    let existing = check_existing(connection, path)?;
    if version == 0 {
        if existing {
            return Err(MemoryError::ForeignDatabase {
                path: path.to_path_buf(),
                detail: String::from("it carries the research-memory schema but reports version 0"),
            });
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(MIGRATION_V1)?;
        transaction.pragma_update(None, "user_version", CURRENT_VERSION)?;
        transaction.commit()?;
    }
    verify(connection)
}

/// Checks whether a connection is fresh or already a research memory.
///
/// A fresh database has no user tables. An existing research memory has the
/// `meta.schema` identity. Any other database is refused before a migration
/// can add tables or change `user_version`.
pub(super) fn check_existing(connection: &Connection, path: &Path) -> Result<bool, MemoryError> {
    let tables: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master
          WHERE type IN ('table', 'view')
            AND name NOT LIKE 'sqlite_%'",
        [],
        |row| row.get(0),
    )?;
    if tables == 0 {
        return Ok(false);
    }
    let stored: Option<String> = connection
        .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get(0)
        })
        .optional()?;
    match stored {
        Some(schema) if schema == SCHEMA => Ok(true),
        Some(schema) => Err(MemoryError::UnsupportedSchema {
            found: schema,
            expected: SCHEMA.to_owned(),
        }),
        None => Err(MemoryError::ForeignDatabase {
            path: path.to_path_buf(),
            detail: String::from("it contains tables but no research-memory schema"),
        }),
    }
}

/// Verifies that the stored schema identity is the one this build writes.
pub(super) fn verify(connection: &Connection) -> Result<(), MemoryError> {
    let stored: String =
        connection.query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
            row.get(0)
        })?;
    if stored != SCHEMA {
        return Err(MemoryError::UnsupportedSchema {
            found: stored,
            expected: SCHEMA.to_owned(),
        });
    }
    Ok(())
}

/// Reads the migration version.
pub(super) fn user_version(connection: &Connection) -> Result<i64, MemoryError> {
    Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use rusqlite::Connection;

    use super::{CURRENT_VERSION, SCHEMA, apply, user_version};

    #[test]
    fn a_fresh_database_reaches_the_current_version() {
        let mut connection = Connection::open_in_memory().expect("the database opens");
        apply(&mut connection, Path::new(":memory:")).expect("the migration applies");
        assert_eq!(
            user_version(&connection).expect("the version is readable"),
            CURRENT_VERSION
        );
        let schema: String = connection
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
                row.get(0)
            })
            .expect("the schema is stored");
        assert_eq!(schema, SCHEMA);
    }

    #[test]
    fn reapplying_the_migration_is_idempotent() {
        let mut connection = Connection::open_in_memory().expect("the database opens");
        apply(&mut connection, Path::new(":memory:")).expect("the first migration applies");
        apply(&mut connection, Path::new(":memory:")).expect("the second migration is a no-op");
        assert_eq!(
            user_version(&connection).expect("the version is readable"),
            CURRENT_VERSION
        );
    }

    #[test]
    fn a_newer_version_is_refused() {
        let mut connection = Connection::open_in_memory().expect("the database opens");
        connection
            .pragma_update(None, "user_version", CURRENT_VERSION + 1)
            .expect("the version is written");
        assert!(apply(&mut connection, Path::new(":memory:")).is_err());
    }
}
