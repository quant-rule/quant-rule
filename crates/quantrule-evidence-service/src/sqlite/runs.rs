//! The SQLite run repository.
//!
//! The state machine is the one the Postgres and memory stores implement:
//! `queued -> running -> succeeded | failed | cancelled`, with a redelivered,
//! uncomputed `running` run reclaimed up to `MAX_ATTEMPTS`. Every transition
//! runs inside `BEGIN IMMEDIATE` and re-reads the row inside that transaction.
//! SQLite has no `FOR UPDATE`; the immediate write lock is what makes the
//! read-then-decide sequence atomic, so two processes cannot both claim the same
//! delivery from the same attempt count. A consumer killed after a claim leaves
//! a `running` row with no digest, and the next redelivery reclaims it, so the
//! run stays recoverable without a second policy.

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunId, RunState, RunSummary, StageName, StageProgress,
    StageStatus,
};
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{Connection, OptionalExtension, params};

use super::{ARTIFACTS_TABLE, RUNS_TABLE, STAGES_TABLE, SUBMISSIONS_TABLE};
use crate::runs::{RunRecord, RunStoreError};

/// A run store backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteRunStore {
    pub(crate) database: SqliteDatabase,
}

impl SqliteRunStore {
    /// Build the store over an open SQLite database.
    #[must_use]
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    /// The database the store reads and writes.
    #[must_use]
    pub fn database(&self) -> &SqliteDatabase {
        &self.database
    }
}

type RunRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    i64,
    Option<String>,
);

pub(crate) fn store_error(error: rusqlite::Error) -> RunStoreError {
    RunStoreError(error.to_string())
}

fn run_digest(value: Option<String>, fallback: &[u8]) -> Result<Digest, RunStoreError> {
    match value {
        Some(value) => Digest::parse(&value)
            .map_err(|error| RunStoreError(format!("a stored run digest is malformed: {error}"))),
        None => Ok(Digest::sha256(fallback)),
    }
}

pub(crate) fn run_state(value: &str) -> Result<RunState, RunStoreError> {
    RunState::parse(value)
        .ok_or_else(|| RunStoreError(format!("a stored run has an unknown status: {value}")))
}

fn run_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RunRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn load_stages(connection: &Connection, run_id: &str) -> Result<Vec<StageProgress>, RunStoreError> {
    let sql = format!(
        "SELECT name, state, elapsed_ms FROM {STAGES_TABLE} WHERE run_id = ?1
         ORDER BY CASE name WHEN 'data' THEN 0 WHEN 'model' THEN 1
                            WHEN 'backtest' THEN 2 ELSE 3 END"
    );
    let mut statement = connection.prepare(&sql).map_err(store_error)?;
    let rows = statement
        .query_map(params![run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(store_error)?;
    let mut stages = Vec::new();
    for row in rows {
        let (name, state, elapsed_ms) = row.map_err(store_error)?;
        stages.push(StageProgress {
            name: StageName::parse(&name).ok_or_else(|| {
                RunStoreError(format!("a stored stage has an unknown name: {name}"))
            })?,
            state: StageStatus::parse(&state).ok_or_else(|| {
                RunStoreError(format!("a stored stage has an unknown state: {state}"))
            })?,
            elapsed_ms: elapsed_ms.and_then(|value| u64::try_from(value).ok()),
        });
    }
    Ok(stages)
}

fn load_artifacts(
    connection: &Connection,
    run_id: &str,
) -> Result<Vec<ArtifactRef>, RunStoreError> {
    let sql = format!(
        "SELECT name, bytes, sha256 FROM {ARTIFACTS_TABLE} WHERE run_id = ?1 ORDER BY name"
    );
    let mut statement = connection.prepare(&sql).map_err(store_error)?;
    let rows = statement
        .query_map(params![run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(store_error)?;
    let mut artifacts = Vec::new();
    for row in rows {
        let (name, bytes, sha256) = row.map_err(store_error)?;
        artifacts.push(ArtifactRef {
            name,
            bytes: u64::try_from(bytes)
                .map_err(|_| RunStoreError("a stored artifact size is negative".to_owned()))?,
            sha256: Digest::parse(&sha256).map_err(|error| {
                RunStoreError(format!("a stored artifact hash is malformed: {error}"))
            })?,
        });
    }
    Ok(artifacts)
}

pub(crate) fn load_run(
    connection: &Connection,
    profile: Option<&str>,
    run_id: &str,
) -> Result<Option<RunRecord>, RunStoreError> {
    let columns = "r.run_id, r.status, r.owner, r.config_id, r.data_version,
                   r.config_hash, r.model_version, r.image_digest, r.submitted_at,
                   r.digest, r.error, r.attempts, r.model_artifact_id";
    let row: Option<RunRow> = match profile {
        Some(profile) => {
            let sql = format!(
                "SELECT {columns}
                   FROM {RUNS_TABLE} r
                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                  WHERE s.profile_id = ?1 AND r.run_id = ?2"
            );
            connection
                .query_row(&sql, params![profile, run_id], run_row)
                .optional()
        }
        None => {
            let sql = format!("SELECT {columns} FROM {RUNS_TABLE} r WHERE r.run_id = ?1");
            connection
                .query_row(&sql, params![run_id], run_row)
                .optional()
        }
    }
    .map_err(store_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stages = load_stages(connection, &row.0)?;
    let artifacts = load_artifacts(connection, &row.0)?;
    Ok(Some(RunRecord {
        run_id: RunId(
            Digest::parse(&row.0)
                .map_err(|error| RunStoreError(format!("a stored run id is malformed: {error}")))?,
        ),
        status: run_state(&row.1)?,
        owner: row.2,
        config_id: row.3,
        data_version: Digest::parse(&row.4).map_err(|error| {
            RunStoreError(format!("a stored data version is malformed: {error}"))
        })?,
        config_hash: run_digest(row.5, b"unknown-config")?,
        model_version: row.6.unwrap_or_else(|| "unknown".to_owned()),
        image_digest: run_digest(row.7, b"unknown-image")?,
        submitted_at: row.8,
        stages,
        digest: row
            .9
            .map(|value| Digest::parse(&value))
            .transpose()
            .map_err(|error| RunStoreError(format!("a stored digest is malformed: {error}")))?,
        artifacts,
        error: row.10,
        attempts: u32::try_from(row.11)
            .map_err(|_| RunStoreError("a stored attempt count is negative".to_owned()))?,
        model_artifact_id: row
            .12
            .map(|value| Digest::parse(&value))
            .transpose()
            .map_err(|error| {
                RunStoreError(format!("a stored model artifact id is malformed: {error}"))
            })?,
    }))
}

pub(crate) fn update_stages(
    connection: &Connection,
    run_id: &str,
    stages: &[StageProgress],
) -> Result<(), RunStoreError> {
    let sql = format!(
        "UPDATE {STAGES_TABLE} SET state = ?3, elapsed_ms = ?4
          WHERE run_id = ?1 AND name = ?2"
    );
    for stage in stages {
        let elapsed = stage
            .elapsed_ms
            .map(i64::try_from)
            .transpose()
            .map_err(|_| RunStoreError("a stage duration does not fit".to_owned()))?;
        connection
            .execute(
                &sql,
                params![run_id, stage.name.as_str(), stage.state.as_str(), elapsed],
            )
            .map_err(store_error)?;
    }
    Ok(())
}

pub(crate) fn upsert_artifacts(
    connection: &Connection,
    run_id: &str,
    artifacts: &[ArtifactRef],
) -> Result<(), RunStoreError> {
    let sql = format!(
        "INSERT INTO {ARTIFACTS_TABLE} (run_id, name, bytes, sha256)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (run_id, name) DO UPDATE SET
             bytes = excluded.bytes,
             sha256 = excluded.sha256"
    );
    for artifact in artifacts {
        let bytes = i64::try_from(artifact.bytes)
            .map_err(|_| RunStoreError("an artifact size does not fit".to_owned()))?;
        connection
            .execute(
                &sql,
                params![run_id, artifact.name, bytes, artifact.sha256.as_hex()],
            )
            .map_err(store_error)?;
    }
    Ok(())
}

pub(crate) enum RunArchiveDbOutcome {
    Archived,
    Missing,
    AlreadyArchived,
}

pub(crate) fn load_summary(
    connection: &Connection,
    profile: &ProfileId,
    run_id: &str,
) -> Result<Option<RunSummary>, RunStoreError> {
    let sql = format!(
        "SELECT r.run_id, r.status, r.config_id, r.data_version,
                r.submitted_at, s.archived_at
           FROM {RUNS_TABLE} r
           JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
          WHERE s.profile_id = ?1 AND r.run_id = ?2"
    );
    let row: Option<(String, String, String, String, String, Option<String>)> = connection
        .query_row(&sql, params![profile.as_str(), run_id], summary_row)
        .optional()
        .map_err(store_error)?;
    row.map(|row| {
        Ok(RunSummary {
            run_id: RunId(Digest::parse(&row.0).map_err(|error| {
                RunStoreError(format!("a stored run id is malformed: {error}"))
            })?),
            status: run_state(&row.1)?,
            config_id: row.2,
            data_version: Digest::parse(&row.3).map_err(|error| {
                RunStoreError(format!("a stored data version is malformed: {error}"))
            })?,
            submitted_at: row.4,
            elapsed_ms: None,
            archived_at: row.5,
        })
    })
    .transpose()
}

pub(crate) fn summary_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, String, String, Option<String>)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}
