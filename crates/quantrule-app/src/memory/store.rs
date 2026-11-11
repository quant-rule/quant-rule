//! SQLite-backed CRUD for the research memory.
//!
//! Every write validates its cross-record references before it reaches SQLite,
//! and the schema's foreign keys and triggers enforce the same rules again. All
//! reads are ordered by the stable row ID, so callers never depend on SQLite's
//! physical order.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::schema;
use super::types::{
    ClaimRecord, ConfigRecord, DatasetRecord, MemoryError, MetricRecord, MetricValue, NewClaim,
    NewConfig, NewDataset, NewNote, NewRun, NewWorkspace, NoteRecord, RunRecord, WorkspaceRecord,
};

/// One open local research memory.
#[derive(Debug)]
pub struct ResearchMemory {
    connection: Connection,
    path: Option<PathBuf>,
}

impl ResearchMemory {
    /// Opens or creates a memory database at `path`.
    ///
    /// A missing parent directory is created. The v1 migration runs before the
    /// memory is returned.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Io`] when the parent cannot be created,
    /// [`MemoryError::Database`] when SQLite rejects the file, and
    /// [`MemoryError::UnsupportedSchema`] or
    /// [`MemoryError::UnsupportedVersion`] when the file is not compatible.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, MemoryError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).map_err(|source| MemoryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut connection = Connection::open(path)?;
        schema::apply(&mut connection)?;
        Ok(Self {
            connection,
            path: Some(path.to_path_buf()),
        })
    }

    /// Opens an empty in-memory memory. It exists for tests and short-lived
    /// sessions; it is never written to a file.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite cannot create the schema.
    pub fn open_in_memory() -> Result<Self, MemoryError> {
        let mut connection = Connection::open_in_memory()?;
        schema::apply(&mut connection)?;
        Ok(Self {
            connection,
            path: None,
        })
    }

    /// The database file, or `None` for an in-memory memory.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// The stored schema identity.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the value cannot be read.
    pub fn schema(&self) -> Result<String, MemoryError> {
        Ok(self
            .connection
            .query_row("SELECT value FROM meta WHERE key = 'schema'", [], |row| {
                row.get(0)
            })?)
    }

    /// The migration version.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the value cannot be read.
    pub fn version(&self) -> Result<i64, MemoryError> {
        schema::user_version(&self.connection)
    }

    /// Inserts or updates a workspace and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty field, and
    /// [`MemoryError::Database`] when SQLite rejects the write.
    pub fn upsert_workspace(&self, workspace: NewWorkspace<'_>) -> Result<i64, MemoryError> {
        require_non_empty("workspace.root", workspace.root)?;
        require_non_empty("workspace.name", workspace.name)?;
        Ok(self.connection.query_row(
            "INSERT INTO workspaces (root, name, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(root) DO UPDATE SET name = excluded.name
             RETURNING id",
            params![workspace.root, workspace.name, workspace.created_at],
            |row| row.get(0),
        )?)
    }

    /// Inserts or updates a dataset and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty field or a negative
    /// candle count, and [`MemoryError::Database`] when SQLite rejects the write.
    pub fn upsert_dataset(&self, dataset: NewDataset<'_>) -> Result<i64, MemoryError> {
        require_non_empty("dataset.name", dataset.name)?;
        require_non_empty("dataset.path", dataset.path)?;
        require_non_empty("dataset.data_version", dataset.data_version)?;
        require_non_empty("dataset.instrument", dataset.instrument)?;
        if dataset.candles < 0 {
            return Err(MemoryError::InvalidRecord(String::from(
                "dataset.candles must not be negative",
            )));
        }
        if let (Some(first), Some(last)) = (dataset.first_open_time, dataset.last_open_time)
            && first > last
        {
            return Err(MemoryError::InvalidRecord(String::from(
                "dataset.first_open_time must not be after last_open_time",
            )));
        }
        Ok(self.connection.query_row(
            "INSERT INTO datasets
                 (workspace_id, name, path, data_version, instrument, candles,
                  first_open_time, last_open_time, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(workspace_id, path) DO UPDATE SET
                 name = excluded.name,
                 data_version = excluded.data_version,
                 instrument = excluded.instrument,
                 candles = excluded.candles,
                 first_open_time = excluded.first_open_time,
                 last_open_time = excluded.last_open_time
             RETURNING id",
            params![
                dataset.workspace_id,
                dataset.name,
                dataset.path,
                dataset.data_version,
                dataset.instrument,
                dataset.candles,
                dataset.first_open_time,
                dataset.last_open_time,
                dataset.created_at
            ],
            |row| row.get(0),
        )?)
    }

    /// Inserts or updates a configuration and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty field, and
    /// [`MemoryError::Database`] when SQLite rejects the write.
    pub fn upsert_config(&self, config: NewConfig<'_>) -> Result<i64, MemoryError> {
        require_non_empty("config.name", config.name)?;
        require_non_empty("config.path", config.path)?;
        require_non_empty("config.config_hash", config.config_hash)?;
        require_non_empty("config.schema_version", config.schema_version)?;
        Ok(self.connection.query_row(
            "INSERT INTO configs
                 (workspace_id, name, path, config_hash, schema_version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(workspace_id, path) DO UPDATE SET
                 name = excluded.name,
                 config_hash = excluded.config_hash,
                 schema_version = excluded.schema_version
             RETURNING id",
            params![
                config.workspace_id,
                config.name,
                config.path,
                config.config_hash,
                config.schema_version,
                config.created_at
            ],
            |row| row.get(0),
        )?)
    }

    /// Inserts a run and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty field, invalid JSON,
    /// or a configuration from another workspace, and
    /// [`MemoryError::Database`] when SQLite rejects the write.
    pub fn insert_run(&self, run: NewRun<'_>) -> Result<i64, MemoryError> {
        require_non_empty("run.run_key", run.run_key)?;
        require_non_empty("run.data_version", run.data_version)?;
        require_non_empty("run.config_hash", run.config_hash)?;
        require_non_empty("run.status", run.status)?;
        serde_json::from_str::<serde_json::Value>(run.summary_json).map_err(|error| {
            MemoryError::InvalidRecord(format!("run.summary_json is not JSON: {error}"))
        })?;
        if let Some(finished_at) = run.finished_at
            && finished_at < run.started_at
        {
            return Err(MemoryError::InvalidRecord(String::from(
                "run.finished_at must not be before started_at",
            )));
        }
        if let Some(config_id) = run.config_id {
            let owner: Option<i64> = self
                .connection
                .query_row(
                    "SELECT workspace_id FROM configs WHERE id = ?1",
                    [config_id],
                    |row| row.get(0),
                )
                .optional()?;
            match owner {
                Some(workspace_id) if workspace_id == run.workspace_id => {}
                Some(_) => {
                    return Err(MemoryError::InvalidRecord(String::from(
                        "run.config_id belongs to another workspace",
                    )));
                }
                None => {
                    return Err(MemoryError::InvalidRecord(format!(
                        "run.config_id {config_id} does not exist"
                    )));
                }
            }
        }
        Ok(self.connection.query_row(
            "INSERT INTO runs
                 (workspace_id, config_id, run_key, data_version, config_hash,
                  status, started_at, finished_at, summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             RETURNING id",
            params![
                run.workspace_id,
                run.config_id,
                run.run_key,
                run.data_version,
                run.config_hash,
                run.status,
                run.started_at,
                run.finished_at,
                run.summary_json
            ],
            |row| row.get(0),
        )?)
    }

    /// Inserts or updates a metric and returns its ID.
    ///
    /// A null value stores no number. A real value must be finite. An integer
    /// is stored as its decimal text so it is never rounded through a float.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty name or a non-finite
    /// real, and [`MemoryError::Database`] when SQLite rejects the write.
    pub fn upsert_metric(
        &self,
        run_id: i64,
        name: &str,
        value: &MetricValue,
    ) -> Result<i64, MemoryError> {
        require_non_empty("metric.name", name)?;
        let (kind, real, text) = match value {
            MetricValue::Real(number) => {
                if !number.is_finite() {
                    return Err(MemoryError::InvalidRecord(String::from(
                        "metric real value must be finite",
                    )));
                }
                ("real", Some(*number), None)
            }
            MetricValue::Integer(number) => ("integer", None, Some(number.to_string())),
            MetricValue::Text(text) => ("text", None, Some(text.clone())),
            MetricValue::Null => ("null", None, None),
        };
        Ok(self.connection.query_row(
            "INSERT INTO metrics (run_id, name, kind, value, text_value)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(run_id, name) DO UPDATE SET
                 kind = excluded.kind,
                 value = excluded.value,
                 text_value = excluded.text_value
             RETURNING id",
            params![run_id, name, kind, real, text],
            |row| row.get(0),
        )?)
    }

    /// Inserts a claim and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty field, and
    /// [`MemoryError::Database`] when SQLite rejects the write.
    pub fn insert_claim(&self, claim: NewClaim<'_>) -> Result<i64, MemoryError> {
        require_non_empty("claim.claim_type", claim.claim_type)?;
        require_non_empty("claim.statement", claim.statement)?;
        require_non_empty("claim.status", claim.status)?;
        Ok(self.connection.query_row(
            "INSERT INTO claims (run_id, claim_type, statement, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             RETURNING id",
            params![
                claim.run_id,
                claim.claim_type,
                claim.statement,
                claim.status,
                claim.created_at
            ],
            |row| row.get(0),
        )?)
    }

    /// Inserts a note and returns its ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty body or a run from
    /// another workspace, and [`MemoryError::Database`] when SQLite rejects the
    /// write.
    pub fn insert_note(&self, note: NewNote<'_>) -> Result<i64, MemoryError> {
        require_non_empty("note.body", note.body)?;
        if note.updated_at < note.created_at {
            return Err(MemoryError::InvalidRecord(String::from(
                "note.updated_at must not be before created_at",
            )));
        }
        if let Some(run_id) = note.run_id {
            let owner: Option<i64> = self
                .connection
                .query_row(
                    "SELECT workspace_id FROM runs WHERE id = ?1",
                    [run_id],
                    |row| row.get(0),
                )
                .optional()?;
            match owner {
                Some(workspace_id) if workspace_id == note.workspace_id => {}
                Some(_) => {
                    return Err(MemoryError::InvalidRecord(String::from(
                        "note.run_id belongs to another workspace",
                    )));
                }
                None => {
                    return Err(MemoryError::InvalidRecord(format!(
                        "note.run_id {run_id} does not exist"
                    )));
                }
            }
        }
        Ok(self.connection.query_row(
            "INSERT INTO notes (workspace_id, run_id, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             RETURNING id",
            params![
                note.workspace_id,
                note.run_id,
                note.body,
                note.created_at,
                note.updated_at
            ],
            |row| row.get(0),
        )?)
    }

    /// Updates one note body and timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] for an empty body, and
    /// [`MemoryError::Database`] when SQLite rejects the write.
    pub fn update_note(&self, id: i64, body: &str, updated_at: i64) -> Result<bool, MemoryError> {
        require_non_empty("note.body", body)?;
        let changed = self.connection.execute(
            "UPDATE notes SET body = ?1, updated_at = ?2 WHERE id = ?3",
            params![body, updated_at, id],
        )?;
        Ok(changed > 0)
    }

    /// Deletes a workspace and every record that depends on it.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_workspace(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM workspaces WHERE id = ?1", [id])?
            > 0)
    }

    /// Deletes a dataset.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_dataset(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM datasets WHERE id = ?1", [id])?
            > 0)
    }

    /// Deletes a configuration. A run that referenced it keeps a null link.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_config(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM configs WHERE id = ?1", [id])?
            > 0)
    }

    /// Deletes a run and its metrics and claims. A note that referenced it
    /// keeps a null link.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_run(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM runs WHERE id = ?1", [id])?
            > 0)
    }

    /// Deletes one metric.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_metric(&self, run_id: i64, name: &str) -> Result<bool, MemoryError> {
        Ok(self.connection.execute(
            "DELETE FROM metrics WHERE run_id = ?1 AND name = ?2",
            params![run_id, name],
        )? > 0)
    }

    /// Deletes one claim.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_claim(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM claims WHERE id = ?1", [id])?
            > 0)
    }

    /// Deletes one note.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when SQLite rejects the write.
    pub fn delete_note(&self, id: i64) -> Result<bool, MemoryError> {
        Ok(self
            .connection
            .execute("DELETE FROM notes WHERE id = ?1", [id])?
            > 0)
    }

    /// Every workspace, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn workspaces(&self) -> Result<Vec<WorkspaceRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, root, name, created_at FROM workspaces ORDER BY id",
            workspace_from_row,
            [],
        )
    }

    /// One workspace, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn workspace(&self, id: i64) -> Result<Option<WorkspaceRecord>, MemoryError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, root, name, created_at FROM workspaces WHERE id = ?1",
                [id],
                workspace_from_row,
            )
            .optional()?)
    }

    /// Every dataset in one workspace, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn datasets(&self, workspace_id: i64) -> Result<Vec<DatasetRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, workspace_id, name, path, data_version, instrument, candles,
                    first_open_time, last_open_time, created_at
             FROM datasets WHERE workspace_id = ?1 ORDER BY id",
            dataset_from_row,
            [workspace_id],
        )
    }

    /// Every configuration in one workspace, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn configs(&self, workspace_id: i64) -> Result<Vec<ConfigRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, workspace_id, name, path, config_hash, schema_version, created_at
             FROM configs WHERE workspace_id = ?1 ORDER BY id",
            config_from_row,
            [workspace_id],
        )
    }

    /// Every run in one workspace, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn runs(&self, workspace_id: i64) -> Result<Vec<RunRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, workspace_id, config_id, run_key, data_version, config_hash,
                    status, started_at, finished_at, summary_json
             FROM runs WHERE workspace_id = ?1 ORDER BY id",
            run_from_row,
            [workspace_id],
        )
    }

    /// One run, or `None`.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn run(&self, id: i64) -> Result<Option<RunRecord>, MemoryError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, workspace_id, config_id, run_key, data_version, config_hash,
                        status, started_at, finished_at, summary_json
                 FROM runs WHERE id = ?1",
                [id],
                run_from_row,
            )
            .optional()?)
    }

    /// Every metric for one run, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails, and
    /// [`MemoryError::InvalidRecord`] when a stored kind and its value disagree.
    pub fn metrics(&self, run_id: i64) -> Result<Vec<MetricRecord>, MemoryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, run_id, name, kind, value, text_value
             FROM metrics WHERE run_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map([run_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<f64>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut metrics = Vec::new();
        for row in rows {
            let (id, run_id, name, kind, value, text) = row?;
            metrics.push(MetricRecord {
                id,
                run_id,
                name,
                value: decode_metric(&kind, value, text)?,
            });
        }
        Ok(metrics)
    }

    /// Every claim for one run, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn claims(&self, run_id: i64) -> Result<Vec<ClaimRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, run_id, claim_type, statement, status, created_at
             FROM claims WHERE run_id = ?1 ORDER BY id",
            claim_from_row,
            [run_id],
        )
    }

    /// Every note in one workspace, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the query fails.
    pub fn notes(&self, workspace_id: i64) -> Result<Vec<NoteRecord>, MemoryError> {
        query_all(
            &self.connection,
            "SELECT id, workspace_id, run_id, body, created_at, updated_at
             FROM notes WHERE workspace_id = ?1 ORDER BY id",
            note_from_row,
            [workspace_id],
        )
    }

    /// Whether every data table is empty. The `meta` table is ignored.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::Database`] when the check fails.
    pub fn is_empty(&self) -> Result<bool, MemoryError> {
        let count: i64 = self.connection.query_row(
            "SELECT (SELECT COUNT(*) FROM workspaces)
                  + (SELECT COUNT(*) FROM datasets)
                  + (SELECT COUNT(*) FROM configs)
                  + (SELECT COUNT(*) FROM runs)
                  + (SELECT COUNT(*) FROM metrics)
                  + (SELECT COUNT(*) FROM claims)
                  + (SELECT COUNT(*) FROM notes)",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Runs SQLite's integrity check and foreign-key check.
    ///
    /// # Errors
    ///
    /// Returns [`MemoryError::InvalidRecord`] when either check reports a
    /// problem, and [`MemoryError::Database`] when the check cannot run.
    pub fn verify_integrity(&self) -> Result<(), MemoryError> {
        let result: String = self
            .connection
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        if result != "ok" {
            return Err(MemoryError::InvalidRecord(format!(
                "integrity check failed: {result}"
            )));
        }
        let mut statement = self.connection.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        if rows.next()?.is_some() {
            return Err(MemoryError::InvalidRecord(String::from(
                "foreign-key check failed",
            )));
        }
        Ok(())
    }

    /// The underlying connection, for the transfer module.
    pub(super) fn connection(&self) -> &Connection {
        &self.connection
    }

    /// The mutable underlying connection, for the import transaction.
    pub(super) fn connection_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}

fn require_non_empty(field: &str, value: &str) -> Result<(), MemoryError> {
    if value.trim().is_empty() {
        return Err(MemoryError::InvalidRecord(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn decode_metric(
    kind: &str,
    value: Option<f64>,
    text: Option<String>,
) -> Result<MetricValue, MemoryError> {
    match kind {
        "real" => value
            .map(MetricValue::Real)
            .ok_or_else(|| MemoryError::InvalidRecord(String::from("real metric has no value"))),
        "integer" => {
            let text = text.ok_or_else(|| {
                MemoryError::InvalidRecord(String::from("integer metric has no text"))
            })?;
            let number = text.parse::<i64>().map_err(|error| {
                MemoryError::InvalidRecord(format!("integer metric is not an integer: {error}"))
            })?;
            Ok(MetricValue::Integer(number))
        }
        "text" => text
            .map(MetricValue::Text)
            .ok_or_else(|| MemoryError::InvalidRecord(String::from("text metric has no text"))),
        "null" => Ok(MetricValue::Null),
        other => Err(MemoryError::InvalidRecord(format!(
            "metric kind {other} is not supported"
        ))),
    }
}

fn query_all<T, P, F>(
    connection: &Connection,
    sql: &str,
    map: F,
    params: P,
) -> Result<Vec<T>, MemoryError>
where
    P: rusqlite::Params,
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params, map)?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values)
}

fn workspace_from_row(row: &Row<'_>) -> rusqlite::Result<WorkspaceRecord> {
    Ok(WorkspaceRecord {
        id: row.get(0)?,
        root: row.get(1)?,
        name: row.get(2)?,
        created_at: row.get(3)?,
    })
}

fn dataset_from_row(row: &Row<'_>) -> rusqlite::Result<DatasetRecord> {
    Ok(DatasetRecord {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        path: row.get(3)?,
        data_version: row.get(4)?,
        instrument: row.get(5)?,
        candles: row.get(6)?,
        first_open_time: row.get(7)?,
        last_open_time: row.get(8)?,
        created_at: row.get(9)?,
    })
}

fn config_from_row(row: &Row<'_>) -> rusqlite::Result<ConfigRecord> {
    Ok(ConfigRecord {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        name: row.get(2)?,
        path: row.get(3)?,
        config_hash: row.get(4)?,
        schema_version: row.get(5)?,
        created_at: row.get(6)?,
    })
}

fn run_from_row(row: &Row<'_>) -> rusqlite::Result<RunRecord> {
    Ok(RunRecord {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        config_id: row.get(2)?,
        run_key: row.get(3)?,
        data_version: row.get(4)?,
        config_hash: row.get(5)?,
        status: row.get(6)?,
        started_at: row.get(7)?,
        finished_at: row.get(8)?,
        summary_json: row.get(9)?,
    })
}

fn claim_from_row(row: &Row<'_>) -> rusqlite::Result<ClaimRecord> {
    Ok(ClaimRecord {
        id: row.get(0)?,
        run_id: row.get(1)?,
        claim_type: row.get(2)?,
        statement: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn note_from_row(row: &Row<'_>) -> rusqlite::Result<NoteRecord> {
    Ok(NoteRecord {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        run_id: row.get(2)?,
        body: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}
