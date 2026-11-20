//! The SQLite stage repository.
//!
//! This is the embedded-profile backend. The state machine is the one the
//! Postgres and memory stores implement: `queued -> running -> succeeded |
//! failed | cancelled`, with a redelivered, uncomputed `running` stage
//! reclaimed up to [`MAX_ATTEMPTS`]. Every transition runs inside
//! `BEGIN IMMEDIATE` and re-reads the row inside that transaction. SQLite has no
//! `FOR UPDATE`; the immediate write lock is what makes the read-then-decide
//! sequence atomic, so two processes cannot both claim the same delivery.
//!
//! The tables are `stage__`-prefixed, matching `stage.runs`, and live in a
//! different table family from `runs__runs`, so the run store cannot read a
//! stage row even when both stores open the same file.

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageRunSummary,
    StageState,
};
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

use super::store::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageCreateOutcome,
    StageDeletion, StageExpiry, StageRecord, StageStore, StageStoreError, validate_artifact,
    validate_new_stage,
};

const STAGE_RUNS_TABLE: &str = "stage__runs";
const STAGE_SUBMISSIONS_TABLE: &str = "stage__submissions";
const STAGE_ARTIFACTS_TABLE: &str = "stage__artifacts";

/// A stage store backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteStageStore {
    database: SqliteDatabase,
}

impl SqliteStageStore {
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

type StageRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    Option<String>,
    i64,
);

fn store_error(error: rusqlite::Error) -> StageStoreError {
    StageStoreError(error.to_string())
}

fn digest(value: &str, what: &str) -> Result<Digest, StageStoreError> {
    Digest::parse(value)
        .map_err(|error| StageStoreError(format!("a stored stage {what} is malformed: {error}")))
}

fn state(value: &str) -> Result<StageState, StageStoreError> {
    StageState::parse(value)
        .ok_or_else(|| StageStoreError(format!("a stored stage has an unknown status: {value}")))
}

fn kind(value: &str) -> Result<StageKind, StageStoreError> {
    StageKind::parse(value)
        .ok_or_else(|| StageStoreError(format!("a stored stage has an unknown kind: {value}")))
}

fn input(input_kind: &str, input_digest: &str) -> Result<StageInput, StageStoreError> {
    let digest = digest(input_digest, "input")?;
    match input_kind {
        "data_version" => Ok(StageInput::DataVersion {
            data_version: digest,
        }),
        "model_artifact" => Ok(StageInput::ModelArtifact {
            model_artifact_id: digest,
        }),
        "decisions_artifact" => Ok(StageInput::DecisionsArtifact {
            decisions_artifact_id: digest,
        }),
        other => Err(StageStoreError(format!(
            "a stored stage has an unknown input kind: {other}"
        ))),
    }
}

fn stage_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StageRow> {
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
        row.get(13)?,
        row.get(14)?,
    ))
}

fn load_files(
    connection: &Connection,
    stage_run_id: &str,
) -> Result<Vec<ArtifactRef>, StageStoreError> {
    let sql = format!(
        "SELECT name, bytes, sha256 FROM {STAGE_ARTIFACTS_TABLE}
         WHERE stage_run_id = ?1 ORDER BY name"
    );
    let mut statement = connection.prepare(&sql).map_err(store_error)?;
    let rows = statement
        .query_map(params![stage_run_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(store_error)?;
    let mut files = Vec::new();
    for row in rows {
        let (name, bytes, sha256) = row.map_err(store_error)?;
        files.push(ArtifactRef {
            name,
            bytes: u64::try_from(bytes).map_err(|_| {
                StageStoreError("a stored stage artifact size is negative".to_owned())
            })?,
            sha256: digest(&sha256, "artifact hash")?,
        });
    }
    Ok(files)
}

fn record(row: StageRow, files: Vec<ArtifactRef>) -> Result<StageRecord, StageStoreError> {
    let stage_run_id = quantrule_api::StageRunId(digest(&row.0, "id")?);
    let artifact = match &row.11 {
        Some(value) => {
            let artifact_id = digest(value, "artifact id")?;
            let artifact = StageArtifact::from_files(files)
                .map_err(|error| StageStoreError(error.to_string()))?;
            if artifact.artifact_id != artifact_id {
                return Err(StageStoreError(
                    "a stored stage artifact id does not match its files".to_owned(),
                ));
            }
            Some(artifact)
        }
        None => {
            if !files.is_empty() {
                return Err(StageStoreError(
                    "a stored stage has artifact files but no artifact id".to_owned(),
                ));
            }
            None
        }
    };
    let record = StageRecord {
        stage_run_id,
        kind: kind(&row.1)?,
        state: state(&row.2)?,
        owner: row.3,
        config_id: row.4,
        input: input(&row.5, &row.6)?,
        provenance: quantrule_api::StageProvenance {
            data_version: digest(&row.7, "data version")?,
            config_hash: digest(&row.8, "config hash")?,
            model_version: row.9,
            image_digest: digest(&row.10, "image digest")?,
        },
        artifact,
        submitted_at: row.12,
        error: row.13,
        attempts: u32::try_from(row.14)
            .map_err(|_| StageStoreError("a stored stage attempt count is negative".to_owned()))?,
    };
    if !record.has_derived_id() {
        return Err(StageStoreError(
            "a stored stage id is not the one its identity derives".to_owned(),
        ));
    }
    Ok(record)
}

fn load_stage(
    connection: &Connection,
    profile: Option<&str>,
    stage_run_id: &str,
) -> Result<Option<StageRecord>, StageStoreError> {
    let columns = "r.stage_run_id, r.kind, r.status, r.owner, r.config_id,
                   r.input_kind, r.input_digest, r.data_version, r.config_hash,
                   r.model_version, r.image_digest, r.artifact_id, r.submitted_at,
                   r.error, r.attempts";
    let row: Option<StageRow> = match profile {
        Some(profile) => {
            let sql = format!(
                "SELECT {columns}
                   FROM {STAGE_RUNS_TABLE} r
                   JOIN {STAGE_SUBMISSIONS_TABLE} s ON s.stage_run_id = r.stage_run_id
                  WHERE s.profile_id = ?1 AND r.stage_run_id = ?2"
            );
            connection
                .query_row(&sql, params![profile, stage_run_id], stage_row)
                .optional()
        }
        None => {
            let sql =
                format!("SELECT {columns} FROM {STAGE_RUNS_TABLE} r WHERE r.stage_run_id = ?1");
            connection
                .query_row(&sql, params![stage_run_id], stage_row)
                .optional()
        }
    }
    .map_err(store_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let files = load_files(connection, &row.0)?;
    record(row, files).map(Some)
}

fn insert_artifacts(
    connection: &Connection,
    stage_run_id: &str,
    artifact: &StageArtifact,
) -> Result<(), StageStoreError> {
    let sql = format!(
        "INSERT INTO {STAGE_ARTIFACTS_TABLE} (stage_run_id, name, bytes, sha256)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (stage_run_id, name) DO UPDATE SET
             bytes = excluded.bytes,
             sha256 = excluded.sha256"
    );
    for file in &artifact.files {
        let bytes = i64::try_from(file.bytes)
            .map_err(|_| StageStoreError("a stage artifact size does not fit".to_owned()))?;
        connection
            .execute(
                &sql,
                params![stage_run_id, file.name, bytes, file.sha256.as_hex()],
            )
            .map_err(store_error)?;
    }
    Ok(())
}

impl StageStore for SqliteStageStore {
    fn create(
        &self,
        profile: ProfileId,
        run: NewStageRun,
    ) -> BoxFuture<Result<StageCreateOutcome, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            validate_new_stage(&run)?;
            let profile_id = profile.as_str().to_owned();
            let stage_run_id = run.stage_run_id.to_string();
            let kind = run.kind.as_str().to_owned();
            let owner = run.owner.clone();
            let config_id = run.config_id.clone();
            let input_kind = run.input.label().to_owned();
            let input_digest = run.input.digest().to_string();
            let data_version = run.provenance.data_version.to_string();
            let config_hash = run.provenance.config_hash.to_string();
            let model_version = run.provenance.model_version.clone();
            let image_digest = run.provenance.image_digest.to_string();
            let submitted_at = run.submitted_at.clone();
            let outcome = database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let insert_run = format!(
                        "INSERT INTO {STAGE_RUNS_TABLE}
                             (stage_run_id, kind, status, owner, config_id, input_kind,
                              input_digest, data_version, config_hash, model_version,
                              image_digest, submitted_at)
                         VALUES (?1, ?2, 'queued', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                         ON CONFLICT (stage_run_id) DO NOTHING"
                    );
                    let inserted = transaction
                        .execute(
                            &insert_run,
                            params![
                                stage_run_id,
                                kind,
                                owner,
                                config_id,
                                input_kind,
                                input_digest,
                                data_version,
                                config_hash,
                                model_version,
                                image_digest,
                                submitted_at
                            ],
                        )
                        .map_err(store_error)?;
                    let created = inserted == 1;
                    let insert_submission = format!(
                        "INSERT INTO {STAGE_SUBMISSIONS_TABLE}
                             (profile_id, stage_run_id, submitted_at)
                         VALUES (?1, ?2, ?3)
                         ON CONFLICT DO NOTHING"
                    );
                    transaction
                        .execute(
                            &insert_submission,
                            params![profile_id, stage_run_id, submitted_at],
                        )
                        .map_err(store_error)?;
                    transaction.commit().map_err(store_error)?;
                    let record = load_stage(connection, Some(&profile_id), &stage_run_id)?
                        .ok_or_else(|| {
                            StageStoreError("the stage vanished after submission".to_owned())
                        })?;
                    Ok::<(bool, StageRecord), StageStoreError>((created, record))
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            Ok(if outcome.0 {
                StageCreateOutcome::Created(outcome.1)
            } else {
                StageCreateOutcome::Existing(outcome.1)
            })
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        kind_filter: Option<StageKind>,
        status: Option<StageState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageRunSummary>, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let kind_filter = kind_filter.map(|kind| kind.as_str().to_owned());
            let status = status.map(|status| status.as_str().to_owned());
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows = database
                .call(move |connection| {
                    let sql = format!(
                        "SELECT r.stage_run_id, r.kind, r.status, r.config_id,
                                r.submitted_at, r.artifact_id
                           FROM {STAGE_RUNS_TABLE} r
                           JOIN {STAGE_SUBMISSIONS_TABLE} s
                             ON s.stage_run_id = r.stage_run_id
                          WHERE s.profile_id = ?1
                            AND (?2 IS NULL OR r.kind = ?2)
                            AND (?3 IS NULL OR r.status = ?3)
                          ORDER BY r.submitted_at DESC, r.stage_run_id ASC LIMIT ?4"
                    );
                    let mut statement = connection.prepare(&sql).map_err(store_error)?;
                    let mapped = statement
                        .query_map(params![profile_id, kind_filter, status, limit], summary_row)
                        .map_err(store_error)?;
                    mapped
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(store_error)
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            rows.into_iter()
                .map(
                    |(
                        stage_run_id,
                        kind_value,
                        state_value,
                        config_id,
                        submitted_at,
                        artifact_id,
                    )| {
                        Ok(StageRunSummary {
                            stage_run_id: quantrule_api::StageRunId(digest(&stage_run_id, "id")?),
                            kind: kind(&kind_value)?,
                            state: state(&state_value)?,
                            config_id,
                            submitted_at,
                            artifact_id: artifact_id
                                .map(|value| digest(&value, "artifact id"))
                                .transpose()?,
                            evidence: false,
                        })
                    },
                )
                .collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            database
                .call(move |connection| load_stage(connection, Some(&profile_id), &stage_run_id))
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn get_any(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| load_stage(connection, None, &stage_run_id))
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn find_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let kind = kind.as_str().to_owned();
            let artifact_id = artifact_id.to_string();
            database
                .call(move |connection| {
                    let sql = format!(
                        "SELECT r.stage_run_id
                           FROM {STAGE_RUNS_TABLE} r
                           JOIN {STAGE_SUBMISSIONS_TABLE} s
                             ON s.stage_run_id = r.stage_run_id
                          WHERE s.profile_id = ?1
                            AND r.kind = ?2
                            AND r.status = 'succeeded'
                            AND r.artifact_id = ?3
                          ORDER BY r.submitted_at DESC, r.stage_run_id ASC
                          LIMIT 1"
                    );
                    let id: Option<String> = connection
                        .query_row(&sql, params![profile_id, kind, artifact_id], |row| {
                            row.get(0)
                        })
                        .optional()
                        .map_err(store_error)?;
                    match id {
                        Some(id) => load_stage(connection, Some(&profile_id), &id),
                        None => Ok(None),
                    }
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn claim(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_stage(&transaction, None, &stage_run_id)? else {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageClaimOutcome, StageStoreError>(
                            StageClaimOutcome::Missing,
                        );
                    };
                    if record.state == StageState::Queued {
                        let sql = format!(
                            "UPDATE {STAGE_RUNS_TABLE}
                                SET status = 'running',
                                    started_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now'),
                                    attempts = attempts + 1
                              WHERE stage_run_id = ?1"
                        );
                        transaction
                            .execute(&sql, params![stage_run_id])
                            .map_err(store_error)?;
                        let record =
                            load_stage(&transaction, None, &stage_run_id)?.ok_or_else(|| {
                                StageStoreError("the stage vanished after a claim".to_owned())
                            })?;
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageClaimOutcome, StageStoreError>(
                            StageClaimOutcome::Claimed(record),
                        );
                    }
                    if record.state != StageState::Running {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageClaimOutcome, StageStoreError>(
                            StageClaimOutcome::Terminal(record),
                        );
                    }
                    // A live worker still holds the delivery; only a broker
                    // redelivery proves that holder is gone. A stage has no
                    // computed-but-unfinalized state: completion is atomic.
                    if !redelivered {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageClaimOutcome, StageStoreError>(
                            StageClaimOutcome::Running(record),
                        );
                    }
                    if record.attempts >= MAX_ATTEMPTS {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageClaimOutcome, StageStoreError>(
                            StageClaimOutcome::Exhausted(record),
                        );
                    }
                    let sql = format!(
                        "UPDATE {STAGE_RUNS_TABLE}
                            SET attempts = attempts + 1,
                                started_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE stage_run_id = ?1"
                    );
                    transaction
                        .execute(&sql, params![stage_run_id])
                        .map_err(store_error)?;
                    let record =
                        load_stage(&transaction, None, &stage_run_id)?.ok_or_else(|| {
                            StageStoreError("the stage vanished after a reclaim".to_owned())
                        })?;
                    transaction.commit().map_err(store_error)?;
                    Ok(StageClaimOutcome::Claimed(record))
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn complete(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            validate_artifact(&artifact)?;
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_stage(&transaction, None, &stage_run_id)? else {
                        transaction.commit().map_err(store_error)?;
                        return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
                    };
                    // Only a running stage can finish. The Postgres store
                    // enforces this with a `status = 'running'` predicate; the
                    // SQLite store must agree with it.
                    if record.state != StageState::Running {
                        transaction.commit().map_err(store_error)?;
                        return Ok(record);
                    }
                    let sql = format!(
                        "UPDATE {STAGE_RUNS_TABLE}
                            SET status = 'succeeded',
                                artifact_id = ?2,
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE stage_run_id = ?1 AND status = 'running'"
                    );
                    transaction
                        .execute(
                            &sql,
                            params![stage_run_id, artifact.artifact_id.to_string()],
                        )
                        .map_err(store_error)?;
                    insert_artifacts(&transaction, &stage_run_id, &artifact)?;
                    let record =
                        load_stage(&transaction, None, &stage_run_id)?.ok_or_else(|| {
                            StageStoreError("the stage vanished after completion".to_owned())
                        })?;
                    transaction.commit().map_err(store_error)?;
                    Ok(record)
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn fail(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_stage(&transaction, None, &stage_run_id)? else {
                        transaction.commit().map_err(store_error)?;
                        return Err(StageStoreError(format!("stage {stage_run_id} is unknown")));
                    };
                    if record.state != StageState::Running {
                        transaction.commit().map_err(store_error)?;
                        return Ok(record);
                    }
                    let sql = format!(
                        "UPDATE {STAGE_RUNS_TABLE}
                            SET status = 'failed',
                                error = ?2,
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE stage_run_id = ?1 AND status = 'running'"
                    );
                    transaction
                        .execute(&sql, params![stage_run_id, error])
                        .map_err(store_error)?;
                    let record =
                        load_stage(&transaction, None, &stage_run_id)?.ok_or_else(|| {
                            StageStoreError("the stage vanished after a failure".to_owned())
                        })?;
                    transaction.commit().map_err(store_error)?;
                    Ok(record)
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn cancel(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<StageCancelOutcome, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let Some(record) = load_stage(&transaction, Some(&profile_id), &stage_run_id)?
                    else {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageCancelOutcome, StageStoreError>(
                            StageCancelOutcome::Missing,
                        );
                    };
                    if !record.state.is_cancellable() {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<StageCancelOutcome, StageStoreError>(
                            StageCancelOutcome::Conflict(record),
                        );
                    }
                    let sql = format!(
                        "UPDATE {STAGE_RUNS_TABLE}
                            SET status = 'cancelled',
                                finished_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
                          WHERE stage_run_id = ?1 AND status IN ('queued', 'running')"
                    );
                    transaction
                        .execute(&sql, params![stage_run_id])
                        .map_err(store_error)?;
                    let record = load_stage(&transaction, Some(&profile_id), &stage_run_id)?
                        .ok_or_else(|| {
                            StageStoreError("the stage vanished after cancellation".to_owned())
                        })?;
                    transaction.commit().map_err(store_error)?;
                    Ok(StageCancelOutcome::Cancelled(record))
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn delete(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageDeletion>, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let exists: bool = transaction
                        .query_row(
                            &format!(
                                "SELECT EXISTS(SELECT 1 FROM {STAGE_SUBMISSIONS_TABLE}
                                  WHERE profile_id = ?1 AND stage_run_id = ?2)"
                            ),
                            params![profile_id, stage_run_id],
                            |row| row.get(0),
                        )
                        .map_err(store_error)?;
                    if !exists {
                        transaction.commit().map_err(store_error)?;
                        return Ok::<Option<StageDeletion>, StageStoreError>(None);
                    }
                    let record = load_stage(&transaction, Some(&profile_id), &stage_run_id)?
                        .ok_or_else(|| {
                            StageStoreError("the stage vanished after submission".to_owned())
                        })?;
                    let hashes = artifact_hashes_in(&transaction, &stage_run_id)?;
                    transaction
                        .execute(
                            &format!("DELETE FROM {STAGE_RUNS_TABLE} WHERE stage_run_id = ?1"),
                            params![stage_run_id],
                        )
                        .map_err(store_error)?;
                    transaction
                        .execute(
                            "INSERT INTO stage__retention_events
                                 (profile_id, kind, record_id, action, reason, bytes)
                             VALUES (?1, 'stage', ?2, 'erase', 'operator.erase', 0)",
                            params![profile_id, stage_run_id],
                        )
                        .map_err(store_error)?;
                    transaction.commit().map_err(store_error)?;
                    Ok(Some(StageDeletion {
                        stage_run_id: record.stage_run_id,
                        artifact_hashes: hashes,
                    }))
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn purge_expired(&self, cutoff: &str) -> BoxFuture<Result<StageExpiry, StageStoreError>> {
        let database = self.database.clone();
        let cutoff = cutoff.to_owned();
        Box::pin(async move {
            database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let mut ids = transaction
                        .prepare(&format!(
                            "SELECT stage_run_id FROM {STAGE_RUNS_TABLE}
                              WHERE submitted_at < ?1 ORDER BY stage_run_id"
                        ))
                        .map_err(store_error)?
                        .query_map(params![cutoff], |row| row.get::<_, String>(0))
                        .map_err(store_error)?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(store_error)?;
                    let mut hashes = transaction
                        .prepare(&format!(
                            "SELECT DISTINCT a.sha256 FROM {STAGE_ARTIFACTS_TABLE} a
                               JOIN {STAGE_RUNS_TABLE} r ON r.stage_run_id = a.stage_run_id
                              WHERE r.submitted_at < ?1 ORDER BY a.sha256"
                        ))
                        .map_err(store_error)?
                        .query_map(params![cutoff], |row| row.get::<_, String>(0))
                        .map_err(store_error)?
                        .collect::<rusqlite::Result<Vec<_>>>()
                        .map_err(store_error)?;
                    transaction
                        .execute(
                            &format!("DELETE FROM {STAGE_RUNS_TABLE} WHERE submitted_at < ?1"),
                            params![cutoff],
                        )
                        .map_err(store_error)?;
                    transaction.commit().map_err(store_error)?;
                    ids.sort();
                    hashes.sort();
                    let stage_run_ids = ids
                        .into_iter()
                        .map(|id| Digest::parse(&id).map(quantrule_api::StageRunId))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|error| {
                            StageStoreError(format!("a stored stage id is malformed: {error}"))
                        })?;
                    let artifact_hashes = hashes
                        .into_iter()
                        .map(|hash| {
                            Digest::parse(&hash).map_err(|error| {
                                StageStoreError(format!(
                                    "a stored stage artifact hash is malformed: {error}"
                                ))
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok::<StageExpiry, StageStoreError>(StageExpiry {
                        stage_run_ids,
                        artifact_hashes,
                    })
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }

    fn artifact_referenced(&self, sha256: Digest) -> BoxFuture<Result<bool, StageStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(move |connection| {
                    connection
                        .query_row(
                            &format!(
                                "SELECT EXISTS(SELECT 1 FROM {STAGE_ARTIFACTS_TABLE}
                                  WHERE sha256 = ?1)"
                            ),
                            params![sha256.as_hex()],
                            |row| row.get::<_, bool>(0),
                        )
                        .map_err(store_error)
                })
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }
}

fn artifact_hashes_in(
    connection: &Connection,
    stage_run_id: &str,
) -> Result<Vec<Digest>, StageStoreError> {
    let mut statement = connection
        .prepare(&format!(
            "SELECT sha256 FROM {STAGE_ARTIFACTS_TABLE}
              WHERE stage_run_id = ?1 ORDER BY sha256"
        ))
        .map_err(store_error)?;
    let rows = statement
        .query_map(params![stage_run_id], |row| row.get::<_, String>(0))
        .map_err(store_error)?;
    rows.map(|row| {
        let hash = row.map_err(store_error)?;
        Digest::parse(&hash).map_err(|error| {
            StageStoreError(format!(
                "a stored stage artifact hash is malformed: {error}"
            ))
        })
    })
    .collect()
}

fn summary_row(
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

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::adapters::{Dialect, Schema};

    #[test]
    fn the_tables_are_the_stage_prefix() {
        for (table, name) in [
            (STAGE_RUNS_TABLE, "runs"),
            (STAGE_SUBMISSIONS_TABLE, "submissions"),
            (STAGE_ARTIFACTS_TABLE, "artifacts"),
        ] {
            assert_eq!(
                table,
                Dialect::Sqlite
                    .table(Schema::Stage, name)
                    .expect("the stage table")
            );
            assert!(
                table.starts_with("stage__"),
                "{table} is outside the stage prefix"
            );
        }
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Stage, "runs")
                .expect("the Postgres name"),
            "stage.runs"
        );
    }

    #[test]
    fn a_stage_store_does_not_name_the_run_tables() {
        for table in [
            STAGE_RUNS_TABLE,
            STAGE_SUBMISSIONS_TABLE,
            STAGE_ARTIFACTS_TABLE,
        ] {
            assert!(!table.starts_with("runs__"), "{table} is a run table");
        }
    }
}
