//! The Postgres stage repository.
//!
//! Every statement reads or writes only the `stage` schema. The run store has
//! no query that names a `stage.` relation, so a stage id cannot resolve to a
//! run even if a client tries. A succeeded row, its artifact rows, and the
//! artifact id are written in one transaction, so a killed worker leaves no
//! visible artifact.

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageRunSummary,
    StageState,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::store::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageCreateOutcome,
    StageRecord, StageStore, StageStoreError, validate_artifact, validate_new_stage,
};

/// A stage store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgStageStore {
    pool: PgPool,
}

impl PgStageStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
    i32,
);

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

async fn load_files(
    pool: &PgPool,
    stage_run_id: &str,
) -> Result<Vec<ArtifactRef>, StageStoreError> {
    let rows: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT name, bytes, sha256 FROM stage.artifacts
         WHERE stage_run_id = $1 ORDER BY name",
    )
    .bind(stage_run_id)
    .fetch_all(pool)
    .await
    .map_err(|error| StageStoreError(error.to_string()))?;
    rows.into_iter()
        .map(|(name, bytes, sha256)| {
            Ok(ArtifactRef {
                name,
                bytes: u64::try_from(bytes).map_err(|_| {
                    StageStoreError("a stored stage artifact size is negative".to_owned())
                })?,
                sha256: digest(&sha256, "artifact hash")?,
            })
        })
        .collect()
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

async fn load_stage(
    pool: &PgPool,
    profile: Option<&str>,
    stage_run_id: &str,
) -> Result<Option<StageRecord>, StageStoreError> {
    let row: Option<StageRow> = match profile {
        Some(profile) => {
            sqlx::query_as(
                "SELECT r.stage_run_id, r.kind, r.status, r.owner, r.config_id,
                        r.input_kind, r.input_digest, r.data_version, r.config_hash,
                        r.model_version, r.image_digest, r.artifact_id,
                        to_char(r.submitted_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        r.error, r.attempts
                   FROM stage.runs r
                   JOIN stage.submissions s ON s.stage_run_id = r.stage_run_id
                  WHERE s.profile_id = $1 AND r.stage_run_id = $2",
            )
            .bind(profile)
            .bind(stage_run_id)
            .fetch_optional(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT r.stage_run_id, r.kind, r.status, r.owner, r.config_id,
                        r.input_kind, r.input_digest, r.data_version, r.config_hash,
                        r.model_version, r.image_digest, r.artifact_id,
                        to_char(r.submitted_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        r.error, r.attempts
                   FROM stage.runs r
                  WHERE r.stage_run_id = $1",
            )
            .bind(stage_run_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|error| StageStoreError(error.to_string()))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let files = load_files(pool, &row.0).await?;
    record(row, files).map(Some)
}

async fn insert_artifacts(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    stage_run_id: &str,
    artifact: &StageArtifact,
) -> Result<(), StageStoreError> {
    for file in &artifact.files {
        let bytes = i64::try_from(file.bytes)
            .map_err(|_| StageStoreError("a stage artifact size does not fit".to_owned()))?;
        sqlx::query(
            "INSERT INTO stage.artifacts (stage_run_id, name, bytes, sha256)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (stage_run_id, name) DO UPDATE SET
                 bytes = EXCLUDED.bytes,
                 sha256 = EXCLUDED.sha256",
        )
        .bind(stage_run_id)
        .bind(&file.name)
        .bind(bytes)
        .bind(file.sha256.as_hex())
        .execute(&mut **transaction)
        .await
        .map_err(|error| StageStoreError(error.to_string()))?;
    }
    Ok(())
}

impl StageStore for PgStageStore {
    fn create(
        &self,
        profile: ProfileId,
        run: NewStageRun,
    ) -> BoxFuture<Result<StageCreateOutcome, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            validate_new_stage(&run)?;
            let stage_run_id = run.stage_run_id.to_string();
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let inserted = sqlx::query(
                "INSERT INTO stage.runs
                     (stage_run_id, kind, status, owner, config_id, input_kind,
                      input_digest, data_version, config_hash, model_version,
                      image_digest, submitted_at)
                 VALUES ($1, $2, 'queued', $3, $4, $5, $6, $7, $8, $9, $10,
                         $11::timestamptz)
                 ON CONFLICT (stage_run_id) DO NOTHING",
            )
            .bind(&stage_run_id)
            .bind(run.kind.as_str())
            .bind(&run.owner)
            .bind(&run.config_id)
            .bind(run.input.label())
            .bind(run.input.digest().to_string())
            .bind(run.provenance.data_version.to_string())
            .bind(run.provenance.config_hash.to_string())
            .bind(&run.provenance.model_version)
            .bind(run.provenance.image_digest.to_string())
            .bind(&run.submitted_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let created = inserted.rows_affected() == 1;
            sqlx::query(
                "INSERT INTO stage.submissions (profile_id, stage_run_id, submitted_at)
                 VALUES ($1, $2, $3::timestamptz) ON CONFLICT DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(&stage_run_id)
            .bind(&run.submitted_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let record = load_stage(&pool, Some(profile.as_str()), &stage_run_id)
                .await?
                .ok_or_else(|| StageStoreError("the stage vanished after submission".to_owned()))?;
            Ok(if created {
                StageCreateOutcome::Created(record)
            } else {
                StageCreateOutcome::Existing(record)
            })
        })
    }

    fn list(
        &self,
        profile: ProfileId,
        status: Option<StageState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageRunSummary>, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows: Vec<(String, String, String, String, String, Option<String>)> =
                match status {
                    Some(status) => {
                        sqlx::query_as(
                            "SELECT r.stage_run_id, r.kind, r.status, r.config_id,
                                to_char(r.submitted_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                                r.artifact_id
                           FROM stage.runs r
                           JOIN stage.submissions s ON s.stage_run_id = r.stage_run_id
                          WHERE s.profile_id = $1 AND r.status = $2
                          ORDER BY r.submitted_at DESC, r.stage_run_id ASC LIMIT $3",
                        )
                        .bind(profile.as_str())
                        .bind(status.as_str())
                        .bind(limit)
                        .fetch_all(&pool)
                        .await
                    }
                    None => {
                        sqlx::query_as(
                            "SELECT r.stage_run_id, r.kind, r.status, r.config_id,
                                to_char(r.submitted_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                                r.artifact_id
                           FROM stage.runs r
                           JOIN stage.submissions s ON s.stage_run_id = r.stage_run_id
                          WHERE s.profile_id = $1
                          ORDER BY r.submitted_at DESC, r.stage_run_id ASC LIMIT $2",
                        )
                        .bind(profile.as_str())
                        .bind(limit)
                        .fetch_all(&pool)
                        .await
                    }
                }
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
        let pool = self.pool.clone();
        Box::pin(async move { load_stage(&pool, Some(profile.as_str()), &stage_run_id).await })
    }

    fn get_any(
        &self,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_stage(&pool, None, &stage_run_id).await })
    }

    fn claim(
        &self,
        stage_run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let started = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'running', started_at = now(), attempts = attempts + 1
                  WHERE stage_run_id = $1 AND status = 'queued'",
            )
            .bind(&stage_run_id)
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let Some(record) = load_stage(&pool, None, &stage_run_id).await? else {
                return Ok(StageClaimOutcome::Missing);
            };
            if started.rows_affected() == 1 {
                return Ok(StageClaimOutcome::Claimed(record));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record));
            }
            if !redelivered || record.artifact.is_some() {
                return Ok(StageClaimOutcome::Running(record));
            }
            if record.attempts >= MAX_ATTEMPTS {
                return Ok(StageClaimOutcome::Exhausted(record));
            }
            // The conditional update is what makes two redeliveries race
            // safely: only one increments, the other reloads and skips.
            let reclaimed = sqlx::query(
                "UPDATE stage.runs SET attempts = attempts + 1, started_at = now()
                  WHERE stage_run_id = $1 AND status = 'running' AND artifact_id IS NULL
                    AND attempts < $2",
            )
            .bind(&stage_run_id)
            .bind(i64::from(MAX_ATTEMPTS))
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let Some(record) = load_stage(&pool, None, &stage_run_id).await? else {
                return Ok(StageClaimOutcome::Missing);
            };
            if reclaimed.rows_affected() == 1 {
                Ok(StageClaimOutcome::Claimed(record))
            } else if record.state == StageState::Running {
                Ok(StageClaimOutcome::Running(record))
            } else {
                Ok(StageClaimOutcome::Terminal(record))
            }
        })
    }

    fn complete(
        &self,
        stage_run_id: String,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            validate_artifact(&artifact)?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'succeeded', artifact_id = $2, finished_at = now()
                  WHERE stage_run_id = $1 AND status = 'running'",
            )
            .bind(&stage_run_id)
            .bind(artifact.artifact_id.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| StageStoreError(error.to_string()))?;
                return load_stage(&pool, None, &stage_run_id)
                    .await?
                    .ok_or_else(|| StageStoreError(format!("stage {stage_run_id} is unknown")));
            }
            insert_artifacts(&mut transaction, &stage_run_id, &artifact).await?;
            transaction
                .commit()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            load_stage(&pool, None, &stage_run_id)
                .await?
                .ok_or_else(|| StageStoreError(format!("stage {stage_run_id} is unknown")))
        })
    }

    fn fail(
        &self,
        stage_run_id: String,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE stage.runs
                    SET status = 'failed', error = $2, finished_at = now()
                  WHERE stage_run_id = $1 AND status = 'running'",
            )
            .bind(&stage_run_id)
            .bind(&error)
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let record = load_stage(&pool, None, &stage_run_id)
                .await?
                .ok_or_else(|| StageStoreError(format!("stage {stage_run_id} is unknown")))?;
            Ok(record)
        })
    }

    fn cancel(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<StageCancelOutcome, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let Some(record) = load_stage(&pool, Some(profile.as_str()), &stage_run_id).await?
            else {
                return Ok(StageCancelOutcome::Missing);
            };
            if !record.state.is_cancellable() {
                return Ok(StageCancelOutcome::Conflict(record));
            }
            let updated = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'cancelled', finished_at = now()
                  WHERE stage_run_id = $1 AND status IN ('queued', 'running')",
            )
            .bind(&stage_run_id)
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let record = load_stage(&pool, Some(profile.as_str()), &stage_run_id)
                .await?
                .ok_or_else(|| {
                    StageStoreError("the stage vanished after cancellation".to_owned())
                })?;
            if updated.rows_affected() == 1 {
                Ok(StageCancelOutcome::Cancelled(record))
            } else {
                Ok(StageCancelOutcome::Conflict(record))
            }
        })
    }
}
