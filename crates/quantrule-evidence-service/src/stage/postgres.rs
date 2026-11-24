//! The Postgres stage repository.
//!
//! Every statement reads or writes only the `stage` schema. The run store has
//! no query that names a `stage.` relation, so a stage id cannot resolve to a
//! run even if a client tries. A succeeded row, its artifact rows, and the
//! artifact id are written in one transaction, so a killed worker leaves no
//! visible artifact. A claim is lease-fenced and a stage submission commits its
//! request intent in `stage.outbox`, so a rebuilt queue replays the row.

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, StageArtifact, StageInput, StageKind, StageRunSummary,
    StageState,
};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::store::{
    MAX_ATTEMPTS, NewStageRun, StageCancelOutcome, StageClaimOutcome, StageCreateOutcome,
    StageDeletion, StageExpiry, StageHeartbeatOutcome, StageRecord, StageStore, StageStoreError,
    validate_artifact, validate_new_stage,
};

/// A stage store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgStageStore {
    pub(crate) pool: PgPool,
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
    i64,
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
        lease_term: u64::try_from(row.15)
            .map_err(|_| StageStoreError("a stored stage lease term is negative".to_owned()))?,
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
                        r.error, r.attempts, r.lease_term
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
                        r.error, r.attempts, r.lease_term
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
            // The request intent commits with the stage row. A repeated
            // submission re-arms an unfinished stage so a lost publish is
            // recovered without a second execution; a terminal stage is never
            // re-armed.
            sqlx::query(
                "INSERT INTO stage.outbox (stage_run_id, routing_key)
                 SELECT $1, 'stage.requested'
                  WHERE EXISTS (
                      SELECT 1 FROM stage.runs
                       WHERE stage_run_id = $1
                         AND status IN ('queued', 'running')
                  )
                 ON CONFLICT (stage_run_id, routing_key) DO NOTHING",
            )
            .bind(&stage_run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            sqlx::query(
                "UPDATE stage.outbox SET published_at = NULL, last_error = NULL
                  WHERE stage_run_id = $1 AND routing_key = 'stage.requested'
                    AND published_at IS NOT NULL
                    AND EXISTS (
                        SELECT 1 FROM stage.runs
                         WHERE stage_run_id = $1
                           AND status IN ('queued', 'running')
                    )",
            )
            .bind(&stage_run_id)
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
        kind_filter: Option<StageKind>,
        status: Option<StageState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageRunSummary>, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let kind_filter = kind_filter.map(|kind| kind.as_str().to_owned());
            let status = status.map(|status| status.as_str().to_owned());
            let rows: Vec<(String, String, String, String, String, Option<String>)> =
                sqlx::query_as(
                    "SELECT r.stage_run_id, r.kind, r.status, r.config_id,
                            to_char(r.submitted_at AT TIME ZONE 'UTC',
                                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                            r.artifact_id
                       FROM stage.runs r
                       JOIN stage.submissions s ON s.stage_run_id = r.stage_run_id
                      WHERE s.profile_id = $1
                        AND ($2::text IS NULL OR r.kind = $2)
                        AND ($3::text IS NULL OR r.status = $3)
                      ORDER BY r.submitted_at DESC, r.stage_run_id ASC LIMIT $4",
                )
                .bind(profile.as_str())
                .bind(kind_filter)
                .bind(status)
                .bind(limit)
                .fetch_all(&pool)
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

    fn find_by_artifact(
        &self,
        profile: ProfileId,
        kind: StageKind,
        artifact_id: Digest,
    ) -> BoxFuture<Result<Option<StageRecord>, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let id: Option<String> = sqlx::query_scalar(
                "SELECT r.stage_run_id
                   FROM stage.runs r
                   JOIN stage.submissions s ON s.stage_run_id = r.stage_run_id
                  WHERE s.profile_id = $1
                    AND r.kind = $2
                    AND r.status = 'succeeded'
                    AND r.artifact_id = $3
                  ORDER BY r.submitted_at DESC, r.stage_run_id ASC
                  LIMIT 1",
            )
            .bind(profile.as_str())
            .bind(kind.as_str())
            .bind(artifact_id.to_string())
            .fetch_optional(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            match id {
                Some(id) => load_stage(&pool, Some(profile.as_str()), &id).await,
                None => Ok(None),
            }
        })
    }

    fn claim(
        &self,
        stage_run_id: String,
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<StageClaimOutcome, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let continuation = continuation
                .map(i64::try_from)
                .transpose()
                .map_err(|_| StageStoreError("a stage claim term does not fit".to_owned()))?;
            // One conditional statement is the claim. The lease is the claim
            // right: a queued stage, an absent/expired lease, or the same
            // holder's own live continuation. A live lease held by another
            // attempt matches no branch and is never stolen.
            let claimed = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'running', started_at = now(), attempts = attempts + 1,
                        lease_holder = $2, lease_term = lease_term + 1,
                        lease_expires_at = now() + make_interval(secs => $3)
                  WHERE stage_run_id = $1
                    AND attempts < $4
                    AND (
                        status = 'queued'
                        OR (status = 'running' AND artifact_id IS NULL
                            AND (lease_expires_at IS NULL
                                 OR lease_expires_at < now()
                                 OR (lease_holder = $2 AND lease_term = $5)))
                    )",
            )
            .bind(&stage_run_id)
            .bind(&holder)
            .bind(quantrule_service_api::STAGE_LEASE_TTL_SECONDS as f64)
            .bind(i64::from(MAX_ATTEMPTS))
            .bind(continuation)
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let Some(record) = load_stage(&pool, None, &stage_run_id).await? else {
                return Ok(StageClaimOutcome::Missing);
            };
            if claimed.rows_affected() == 1 {
                return Ok(StageClaimOutcome::Claimed(record));
            }
            if record.state != StageState::Running {
                return Ok(StageClaimOutcome::Terminal(record));
            }
            let (live, spent): (bool, bool) = sqlx::query_as(
                "SELECT lease_expires_at IS NOT NULL AND lease_expires_at >= now(),
                        attempts >= $2
                   FROM stage.runs WHERE stage_run_id = $1",
            )
            .bind(&stage_run_id)
            .bind(i64::from(MAX_ATTEMPTS))
            .fetch_one(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            if live || !spent {
                Ok(StageClaimOutcome::Running(record))
            } else {
                Ok(StageClaimOutcome::Exhausted(record))
            }
        })
    }

    fn renew(
        &self,
        stage_run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<StageHeartbeatOutcome, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| StageStoreError("a stage lease term does not fit".to_owned()))?;
            let renewed = sqlx::query(
                "UPDATE stage.runs
                    SET lease_expires_at = now() + make_interval(secs => $4)
                  WHERE stage_run_id = $1 AND status = 'running'
                    AND lease_holder = $2 AND lease_term = $3
                    AND lease_expires_at IS NOT NULL
                    AND lease_expires_at >= now()",
            )
            .bind(&stage_run_id)
            .bind(&holder)
            .bind(term)
            .bind(quantrule_service_api::STAGE_LEASE_TTL_SECONDS as f64)
            .execute(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let Some(record) = load_stage(&pool, None, &stage_run_id).await? else {
                return Ok(StageHeartbeatOutcome::Missing);
            };
            if renewed.rows_affected() == 1 {
                Ok(StageHeartbeatOutcome::Renewed(Box::new(record)))
            } else {
                Ok(StageHeartbeatOutcome::Stale)
            }
        })
    }

    fn complete(
        &self,
        stage_run_id: String,
        term: u64,
        artifact: StageArtifact,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            validate_artifact(&artifact)?;
            let term = i64::try_from(term)
                .map_err(|_| StageStoreError("a stage lease term does not fit".to_owned()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            // The term is the fence: a stale worker cannot write the artifact
            // set of a later attempt.
            let updated = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'succeeded', artifact_id = $2, finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL
                  WHERE stage_run_id = $1 AND status = 'running' AND lease_term = $3",
            )
            .bind(&stage_run_id)
            .bind(artifact.artifact_id.to_string())
            .bind(term)
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
            sqlx::query("DELETE FROM stage.outbox WHERE stage_run_id = $1")
                .bind(&stage_run_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
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
        term: u64,
        error: String,
    ) -> BoxFuture<Result<StageRecord, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| StageStoreError("a stage lease term does not fit".to_owned()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'failed', error = $2, finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL
                  WHERE stage_run_id = $1 AND status = 'running' AND lease_term = $3",
            )
            .bind(&stage_run_id)
            .bind(&error)
            .bind(term)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            if updated.rows_affected() == 1 {
                sqlx::query("DELETE FROM stage.outbox WHERE stage_run_id = $1")
                    .bind(&stage_run_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| StageStoreError(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            load_stage(&pool, None, &stage_run_id)
                .await?
                .ok_or_else(|| StageStoreError(format!("stage {stage_run_id} is unknown")))
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
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE stage.runs
                    SET status = 'cancelled', finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL
                  WHERE stage_run_id = $1 AND status IN ('queued', 'running')",
            )
            .bind(&stage_run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            if updated.rows_affected() == 1 {
                sqlx::query("DELETE FROM stage.outbox WHERE stage_run_id = $1")
                    .bind(&stage_run_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| StageStoreError(error.to_string()))?;
            }
            transaction
                .commit()
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

    fn delete(
        &self,
        profile: ProfileId,
        stage_run_id: String,
    ) -> BoxFuture<Result<Option<StageDeletion>, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(
                     SELECT 1 FROM stage.submissions
                      WHERE profile_id = $1 AND stage_run_id = $2
                 )",
            )
            .bind(profile.as_str())
            .bind(&stage_run_id)
            .fetch_one(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            if !exists {
                return Ok(None);
            }
            let Some(record) = load_stage(&pool, Some(profile.as_str()), &stage_run_id).await?
            else {
                return Ok(None);
            };
            let hashes: Vec<String> = sqlx::query_scalar(
                "SELECT sha256 FROM stage.artifacts
                  WHERE stage_run_id = $1 ORDER BY sha256",
            )
            .bind(&stage_run_id)
            .fetch_all(&pool)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            sqlx::query("DELETE FROM stage.runs WHERE stage_run_id = $1")
                .bind(&stage_run_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            sqlx::query(
                "INSERT INTO stage.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'stage', $2, 'erase', 'operator.erase', 0)",
            )
            .bind(profile.as_str())
            .bind(&stage_run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
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
            Ok(Some(StageDeletion {
                stage_run_id: record.stage_run_id,
                artifact_hashes,
            }))
        })
    }

    fn purge_expired(&self, cutoff: &str) -> BoxFuture<Result<StageExpiry, StageStoreError>> {
        let pool = self.pool.clone();
        let cutoff = cutoff.to_owned();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT stage_run_id FROM stage.runs
                  WHERE submitted_at < $1::timestamptz ORDER BY stage_run_id",
            )
            .bind(&cutoff)
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            let hashes: Vec<String> = sqlx::query_scalar(
                "SELECT DISTINCT a.sha256 FROM stage.artifacts a
                   JOIN stage.runs r ON r.stage_run_id = a.stage_run_id
                  WHERE r.submitted_at < $1::timestamptz ORDER BY a.sha256",
            )
            .bind(&cutoff)
            .fetch_all(&mut *transaction)
            .await
            .map_err(|error| StageStoreError(error.to_string()))?;
            sqlx::query("DELETE FROM stage.runs WHERE submitted_at < $1::timestamptz")
                .bind(&cutoff)
                .execute(&mut *transaction)
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| StageStoreError(error.to_string()))?;
            let stage_run_ids = ids
                .into_iter()
                .map(|id| {
                    Digest::parse(&id)
                        .map(quantrule_api::StageRunId)
                        .map_err(|error| {
                            StageStoreError(format!("a stored stage id is malformed: {error}"))
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
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
            Ok(StageExpiry {
                stage_run_ids,
                artifact_hashes,
            })
        })
    }

    fn artifact_referenced(&self, sha256: Digest) -> BoxFuture<Result<bool, StageStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stage.artifacts WHERE sha256 = $1)")
                .bind(sha256.as_hex())
                .fetch_one(&pool)
                .await
                .map_err(|error| StageStoreError(error.to_string()))
        })
    }
}
