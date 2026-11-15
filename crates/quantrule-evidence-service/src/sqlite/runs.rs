//! The SQLite run repository.
//!
//! The state machine is the one the Postgres and memory stores implement:
//! `queued -> running -> succeeded | failed | cancelled`, with a redelivered,
//! uncomputed `running` run reclaimed up to [`MAX_ATTEMPTS`]. Every transition
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
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use super::{ARTIFACTS_TABLE, RUNS_TABLE, STAGES_TABLE, SUBMISSIONS_TABLE};
use crate::runs::{
    CancelOutcome, ClaimOutcome, MAX_ATTEMPTS, NewRun, RunRecord, RunStore, RunStoreError,
    SubmitOutcome,
};

/// A run store backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteRunStore {
    database: SqliteDatabase,
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
);

fn store_error(error: rusqlite::Error) -> RunStoreError {
    RunStoreError(error.to_string())
}

fn run_digest(value: Option<String>, fallback: &[u8]) -> Result<Digest, RunStoreError> {
    match value {
        Some(value) => Digest::parse(&value)
            .map_err(|error| RunStoreError(format!("a stored run digest is malformed: {error}"))),
        None => Ok(Digest::sha256(fallback)),
    }
}

fn run_state(value: &str) -> Result<RunState, RunStoreError> {
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

fn load_run(
    connection: &Connection,
    profile: Option<&str>,
    run_id: &str,
) -> Result<Option<RunRecord>, RunStoreError> {
    let columns = "r.run_id, r.status, r.owner, r.config_id, r.data_version,
                   r.config_hash, r.model_version, r.image_digest, r.submitted_at,
                   r.digest, r.error, r.attempts";
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
    }))
}

fn update_stages(
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

fn upsert_artifacts(
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

fn summary_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<(String, String, String, String, String)> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

impl RunStore for SqliteRunStore {
    fn submit(
        &self,
        profile: ProfileId,
        run: NewRun,
    ) -> BoxFuture<Result<SubmitOutcome, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let run_id = run.run_id.to_string();
            let config_id = run.config_id.clone();
            let data_version = run.data_version.to_string();
            let owner = run.owner.clone();
            let submitted_at = run.submitted_at.clone();
            let config_hash = run.config_hash.to_string();
            let model_version = run.model_version.clone();
            let image_digest = run.image_digest.to_string();
            let outcome = database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let insert_run = format!(
                        "INSERT INTO {RUNS_TABLE}
                             (run_id, config_id, data_version, owner, status, submitted_at,
                              config_hash, model_version, image_digest)
                         VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6, ?7, ?8)
                         ON CONFLICT (run_id) DO NOTHING"
                    );
                    let inserted = transaction
                        .execute(
                            &insert_run,
                            params![
                                run_id,
                                config_id,
                                data_version,
                                owner,
                                submitted_at,
                                config_hash,
                                model_version,
                                image_digest
                            ],
                        )
                        .map_err(store_error)?;
                    let created = inserted == 1;
                    if created {
                        let insert_stage = format!(
                            "INSERT INTO {STAGES_TABLE} (run_id, name, state)
                             VALUES (?1, ?2, 'pending')
                             ON CONFLICT (run_id, name) DO NOTHING"
                        );
                        for name in ["data", "model", "backtest", "evidence"] {
                            transaction
                                .execute(&insert_stage, params![run_id, name])
                                .map_err(store_error)?;
                        }
                    }
                    let insert_submission = format!(
                        "INSERT INTO {SUBMISSIONS_TABLE} (profile_id, run_id, submitted_at)
                         VALUES (?1, ?2, ?3)
                         ON CONFLICT DO NOTHING"
                    );
                    transaction
                        .execute(
                            &insert_submission,
                            params![profile_id, run_id, submitted_at],
                        )
                        .map_err(store_error)?;
                    transaction.commit().map_err(store_error)?;
                    let record =
                        load_run(connection, Some(&profile_id), &run_id)?.ok_or_else(|| {
                            RunStoreError("the run vanished after submission".to_owned())
                        })?;
                    Ok::<(bool, RunRecord), RunStoreError>((created, record))
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            Ok(if outcome.0 {
                SubmitOutcome::Created(outcome.1)
            } else {
                SubmitOutcome::Existing(outcome.1)
            })
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows = database
                .call(move |connection| {
                    let rows = match status {
                        Some(status) => {
                            let sql = format!(
                                "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1 AND r.status = ?2
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?3"
                            );
                            let mut statement = connection.prepare(&sql).map_err(store_error)?;
                            let mapped = statement
                                .query_map(params![profile_id, status.as_str(), limit], summary_row)
                                .map_err(store_error)?;
                            mapped
                                .collect::<rusqlite::Result<Vec<_>>>()
                                .map_err(store_error)?
                        }
                        None => {
                            let sql = format!(
                                "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?2"
                            );
                            let mut statement = connection.prepare(&sql).map_err(store_error)?;
                            let mapped = statement
                                .query_map(params![profile_id, limit], summary_row)
                                .map_err(store_error)?;
                            mapped
                                .collect::<rusqlite::Result<Vec<_>>>()
                                .map_err(store_error)?
                        }
                    };
                    Ok::<Vec<(String, String, String, String, String)>, RunStoreError>(rows)
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            rows.into_iter()
                .map(|(run_id, status, config_id, data_version, submitted_at)| {
                    Ok(RunSummary {
                        run_id: RunId(Digest::parse(&run_id).map_err(|error| {
                            RunStoreError(format!("a stored run id is malformed: {error}"))
                        })?),
                        status: run_state(&status)?,
                        config_id,
                        data_version: Digest::parse(&data_version).map_err(|error| {
                            RunStoreError(format!("a stored data version is malformed: {error}"))
                        })?,
                        submitted_at,
                        elapsed_ms: None,
                    })
                })
                .collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            database
                .call(move |connection| load_run(connection, Some(&profile_id), &run_id))
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn cancel(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<CancelOutcome, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_run(&transaction, Some(&profile_id), &run_id)? else {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<CancelOutcome, RunStoreError>(CancelOutcome::Missing);
                    };
                    if !record.status.is_cancellable() {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<CancelOutcome, RunStoreError>(CancelOutcome::Conflict(record));
                    }
                    let sql = format!(
                        "UPDATE {RUNS_TABLE}
                            SET status = 'cancelled',
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE run_id = ?1"
                    );
                    transaction
                        .execute(&sql, params![run_id])
                        .map_err(store_error)?;
                    let record = load_run(&transaction, Some(&profile_id), &run_id)?
                        .ok_or_else(|| RunStoreError("the run vanished after cancel".to_owned()))?;
                    transaction.commit().map_err(store_error)?;
                    Ok(CancelOutcome::Cancelled(record))
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn get_any(&self, run_id: String) -> BoxFuture<Result<Option<RunRecord>, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| load_run(connection, None, &run_id))
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<ClaimOutcome, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_run(&transaction, None, &run_id)? else {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<ClaimOutcome, RunStoreError>(ClaimOutcome::Missing);
                    };
                    if record.status == RunState::Queued {
                        let sql = format!(
                            "UPDATE {RUNS_TABLE}
                                SET status = 'running',
                                    started_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                    attempts = attempts + 1
                              WHERE run_id = ?1"
                        );
                        transaction
                            .execute(&sql, params![run_id])
                            .map_err(store_error)?;
                        let record = load_run(&transaction, None, &run_id)?.ok_or_else(|| {
                            RunStoreError("the run vanished after a claim".to_owned())
                        })?;
                        transaction.commit().map_err(store_error)?;
                        return Ok::<ClaimOutcome, RunStoreError>(ClaimOutcome::Claimed(record));
                    }
                    if record.status != RunState::Running {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<ClaimOutcome, RunStoreError>(ClaimOutcome::Terminal(record));
                    }
                    // A live worker still holds the delivery; only a broker
                    // redelivery proves that holder is gone. A computed run
                    // waits for evidence instead of being executed twice.
                    if !redelivered || record.digest.is_some() {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<ClaimOutcome, RunStoreError>(ClaimOutcome::Running(record));
                    }
                    if record.attempts >= MAX_ATTEMPTS {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<ClaimOutcome, RunStoreError>(ClaimOutcome::Exhausted(record));
                    }
                    let sql = format!(
                        "UPDATE {RUNS_TABLE}
                            SET attempts = attempts + 1,
                                started_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE run_id = ?1"
                    );
                    transaction
                        .execute(&sql, params![run_id])
                        .map_err(store_error)?;
                    let record = load_run(&transaction, None, &run_id)?.ok_or_else(|| {
                        RunStoreError("the run vanished after a reclaim".to_owned())
                    })?;
                    transaction.commit().map_err(store_error)?;
                    Ok(ClaimOutcome::Claimed(record))
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn computed(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let sql = format!(
                        "UPDATE {RUNS_TABLE} SET digest = ?2
                          WHERE run_id = ?1 AND status = 'running'"
                    );
                    let updated = transaction
                        .execute(&sql, params![run_id, digest.to_string()])
                        .map_err(store_error)?;
                    if updated == 0 {
                        transaction.commit().map_err(store_error)?;
                        return load_run(connection, None, &run_id)?
                            .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
                    }
                    update_stages(&transaction, &run_id, &stages)?;
                    upsert_artifacts(&transaction, &run_id, &artifacts)?;
                    let record = load_run(&transaction, None, &run_id)?
                        .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")))?;
                    transaction.commit().map_err(store_error)?;
                    Ok(record)
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn complete(
        &self,
        run_id: String,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let sql = format!(
                        "UPDATE {RUNS_TABLE}
                            SET status = 'succeeded', digest = ?2,
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE run_id = ?1 AND status = 'running'"
                    );
                    let updated = transaction
                        .execute(&sql, params![run_id, digest.to_string()])
                        .map_err(store_error)?;
                    if updated == 0 {
                        transaction.commit().map_err(store_error)?;
                        return load_run(connection, None, &run_id)?
                            .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
                    }
                    update_stages(&transaction, &run_id, &stages)?;
                    upsert_artifacts(&transaction, &run_id, &artifacts)?;
                    let record = load_run(&transaction, None, &run_id)?
                        .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")))?;
                    transaction.commit().map_err(store_error)?;
                    Ok(record)
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn fail(
        &self,
        run_id: String,
        error: String,
        stages: Vec<StageProgress>,
    ) -> BoxFuture<Result<RunRecord, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let sql = format!(
                        "UPDATE {RUNS_TABLE}
                            SET status = 'failed', error = ?2,
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE run_id = ?1 AND status = 'running'"
                    );
                    let updated = transaction
                        .execute(&sql, params![run_id, error])
                        .map_err(store_error)?;
                    if updated == 0 {
                        transaction.commit().map_err(store_error)?;
                        return load_run(connection, None, &run_id)?
                            .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
                    }
                    update_stages(&transaction, &run_id, &stages)?;
                    let record = load_run(&transaction, None, &run_id)?
                        .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")))?;
                    transaction.commit().map_err(store_error)?;
                    Ok(record)
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }
}
