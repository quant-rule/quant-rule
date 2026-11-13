//! The Postgres run store.

use sqlx::PgPool;

use crate::runtime::adapters::BoxFuture;

/// A run store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgRunStore {
    pool: PgPool,
}

impl PgRunStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
    i32,
);

fn run_digest(
    value: Option<String>,
    fallback: &[u8],
) -> Result<quantrule_api::Digest, crate::runs::RunStoreError> {
    match value {
        Some(value) => quantrule_api::Digest::parse(&value).map_err(|error| {
            crate::runs::RunStoreError(format!("a stored run digest is malformed: {error}"))
        }),
        None => Ok(quantrule_api::Digest::sha256(fallback)),
    }
}

fn run_state(value: &str) -> Result<quantrule_api::RunState, crate::runs::RunStoreError> {
    quantrule_api::RunState::parse(value).ok_or_else(|| {
        crate::runs::RunStoreError(format!("a stored run has an unknown status: {value}"))
    })
}

fn stage_name(value: &str) -> Result<quantrule_api::StageName, crate::runs::RunStoreError> {
    quantrule_api::StageName::parse(value).ok_or_else(|| {
        crate::runs::RunStoreError(format!("a stored stage has an unknown name: {value}"))
    })
}

fn stage_status(value: &str) -> Result<quantrule_api::StageStatus, crate::runs::RunStoreError> {
    quantrule_api::StageStatus::parse(value).ok_or_else(|| {
        crate::runs::RunStoreError(format!("a stored stage has an unknown state: {value}"))
    })
}

async fn load_run_rows(
    pool: &PgPool,
    profile: Option<&str>,
    run_id: &str,
) -> Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError> {
    let row: Option<RunRow> = match profile {
        Some(profile) => {
            sqlx::query_as(
                "SELECT r.run_id, r.status, r.owner, r.config_id, r.data_version,
                        r.config_hash, r.model_version, r.image_digest,
                        to_char(r.submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        r.digest, r.error, r.attempts
                 FROM runs.runs r
                 JOIN runs.submissions s ON s.run_id = r.run_id
                 WHERE s.profile_id = $1 AND r.run_id = $2",
            )
            .bind(profile)
            .bind(run_id)
            .fetch_optional(pool)
            .await
        }
        None => {
            sqlx::query_as(
                "SELECT r.run_id, r.status, r.owner, r.config_id, r.data_version,
                        r.config_hash, r.model_version, r.image_digest,
                        to_char(r.submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        r.digest, r.error, r.attempts
                 FROM runs.runs r
                 WHERE r.run_id = $1",
            )
            .bind(run_id)
            .fetch_optional(pool)
            .await
        }
    }
    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let stages: Vec<(String, String, Option<i64>)> = sqlx::query_as(
        "SELECT name, state, elapsed_ms FROM runs.stages WHERE run_id = $1
         ORDER BY CASE name WHEN 'data' THEN 0 WHEN 'model' THEN 1
                            WHEN 'backtest' THEN 2 ELSE 3 END",
    )
    .bind(&row.0)
    .fetch_all(pool)
    .await
    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
    let mut progress = Vec::with_capacity(stages.len());
    for (name, state, elapsed_ms) in stages {
        progress.push(quantrule_api::StageProgress {
            name: stage_name(&name)?,
            state: stage_status(&state)?,
            elapsed_ms: elapsed_ms.and_then(|value| u64::try_from(value).ok()),
        });
    }
    let stored_artifacts: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT name, bytes, sha256 FROM runs.artifacts WHERE run_id = $1 ORDER BY name",
    )
    .bind(&row.0)
    .fetch_all(pool)
    .await
    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
    let mut artifacts = Vec::with_capacity(stored_artifacts.len());
    for (name, bytes, sha256) in stored_artifacts {
        artifacts.push(quantrule_api::ArtifactRef {
            name,
            bytes: u64::try_from(bytes).map_err(|_| {
                crate::runs::RunStoreError("a stored artifact size is negative".to_owned())
            })?,
            sha256: quantrule_api::Digest::parse(&sha256).map_err(|error| {
                crate::runs::RunStoreError(format!("a stored artifact hash is malformed: {error}"))
            })?,
        });
    }
    Ok(Some(crate::runs::RunRecord {
        run_id: quantrule_api::RunId(quantrule_api::Digest::parse(&row.0).map_err(|error| {
            crate::runs::RunStoreError(format!("a stored run id is malformed: {error}"))
        })?),
        status: run_state(&row.1)?,
        owner: row.2,
        config_id: row.3,
        data_version: quantrule_api::Digest::parse(&row.4).map_err(|error| {
            crate::runs::RunStoreError(format!("a stored data version is malformed: {error}"))
        })?,
        config_hash: run_digest(row.5, b"unknown-config")?,
        model_version: row.6.unwrap_or_else(|| "unknown".to_owned()),
        image_digest: run_digest(row.7, b"unknown-image")?,
        submitted_at: row.8,
        stages: progress,
        digest: row
            .9
            .map(|value| quantrule_api::Digest::parse(&value))
            .transpose()
            .map_err(|error| {
                crate::runs::RunStoreError(format!("a stored digest is malformed: {error}"))
            })?,
        artifacts,
        error: row.10,
        attempts: u32::try_from(row.11).map_err(|_| {
            crate::runs::RunStoreError("a stored attempt count is negative".to_owned())
        })?,
    }))
}

async fn load_run(
    pool: &PgPool,
    profile: &str,
    run_id: &str,
) -> Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError> {
    load_run_rows(pool, Some(profile), run_id).await
}

impl crate::runs::RunStore for PgRunStore {
    fn submit(
        &self,
        profile: quantrule_api::ProfileId,
        run: crate::runs::NewRun,
    ) -> BoxFuture<Result<crate::runs::SubmitOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let inserted = sqlx::query(
                "INSERT INTO runs.runs
                     (run_id, config_id, data_version, owner, status, submitted_at,
                      config_hash, model_version, image_digest)
                 VALUES ($1, $2, $3, $4, 'queued', $5::timestamptz, $6, $7, $8)
                 ON CONFLICT (run_id) DO NOTHING",
            )
            .bind(run.run_id.to_string())
            .bind(&run.config_id)
            .bind(run.data_version.to_string())
            .bind(&run.owner)
            .bind(&run.submitted_at)
            .bind(run.config_hash.to_string())
            .bind(&run.model_version)
            .bind(run.image_digest.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let created = inserted.rows_affected() == 1;
            if created {
                for name in ["data", "model", "backtest", "evidence"] {
                    sqlx::query(
                        "INSERT INTO runs.stages (run_id, name, state)
                         VALUES ($1, $2, 'pending') ON CONFLICT DO NOTHING",
                    )
                    .bind(run.run_id.to_string())
                    .bind(name)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                }
            }
            sqlx::query(
                "INSERT INTO runs.submissions (profile_id, run_id, submitted_at)
                 VALUES ($1, $2, $3::timestamptz) ON CONFLICT DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(run.run_id.to_string())
            .bind(&run.submitted_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let record = load_run(&pool, profile.as_str(), &run.run_id.to_string())
                .await?
                .ok_or_else(|| {
                    crate::runs::RunStoreError("the run vanished after submission".to_owned())
                })?;
            Ok(if created {
                crate::runs::SubmitOutcome::Created(record)
            } else {
                crate::runs::SubmitOutcome::Existing(record)
            })
        })
    }

    fn list(
        &self,
        profile: quantrule_api::ProfileId,
        status: Option<quantrule_api::RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<quantrule_api::RunSummary>, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows: Vec<(String, String, String, String, String)> = match status {
                Some(status) => {
                    sqlx::query_as(
                        "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                to_char(r.submitted_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                         FROM runs.runs r
                         JOIN runs.submissions s ON s.run_id = r.run_id
                         WHERE s.profile_id = $1 AND r.status = $2
                         ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT $3",
                    )
                    .bind(profile.as_str())
                    .bind(status.as_str())
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
                None => {
                    sqlx::query_as(
                        "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                to_char(r.submitted_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                         FROM runs.runs r
                         JOIN runs.submissions s ON s.run_id = r.run_id
                         WHERE s.profile_id = $1
                         ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT $2",
                    )
                    .bind(profile.as_str())
                    .bind(limit)
                    .fetch_all(&pool)
                    .await
                }
            }
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            rows.into_iter()
                .map(|(run_id, status, config_id, data_version, submitted_at)| {
                    Ok(quantrule_api::RunSummary {
                        run_id: quantrule_api::RunId(
                            quantrule_api::Digest::parse(&run_id).map_err(|error| {
                                crate::runs::RunStoreError(format!(
                                    "a stored run id is malformed: {error}"
                                ))
                            })?,
                        ),
                        status: run_state(&status)?,
                        config_id,
                        data_version: quantrule_api::Digest::parse(&data_version).map_err(
                            |error| {
                                crate::runs::RunStoreError(format!(
                                    "a stored data version is malformed: {error}"
                                ))
                            },
                        )?,
                        submitted_at,
                        elapsed_ms: None,
                    })
                })
                .collect()
        })
    }

    fn get(
        &self,
        profile: quantrule_api::ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_run(&pool, profile.as_str(), &run_id).await })
    }

    fn cancel(
        &self,
        profile: quantrule_api::ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<crate::runs::CancelOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let Some(record) = load_run(&pool, profile.as_str(), &run_id).await? else {
                return Ok(crate::runs::CancelOutcome::Missing);
            };
            if !record.status.is_cancellable() {
                return Ok(crate::runs::CancelOutcome::Conflict(record));
            }
            sqlx::query(
                "UPDATE runs.runs SET status = 'cancelled', finished_at = now() WHERE run_id = $1",
            )
            .bind(&run_id)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let record = load_run(&pool, profile.as_str(), &run_id)
                .await?
                .ok_or_else(|| {
                    crate::runs::RunStoreError("the run vanished after cancel".to_owned())
                })?;
            Ok(crate::runs::CancelOutcome::Cancelled(record))
        })
    }

    fn get_any(
        &self,
        run_id: String,
    ) -> BoxFuture<Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move { load_run_rows(&pool, None, &run_id).await })
    }

    fn claim(
        &self,
        run_id: String,
        redelivered: bool,
    ) -> BoxFuture<Result<crate::runs::ClaimOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let started = sqlx::query(
                "UPDATE runs.runs
                 SET status = 'running', started_at = now(), attempts = attempts + 1
                 WHERE run_id = $1 AND status = 'queued'",
            )
            .bind(&run_id)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let Some(record) = load_run_rows(&pool, None, &run_id).await? else {
                return Ok(crate::runs::ClaimOutcome::Missing);
            };
            if started.rows_affected() == 1 {
                return Ok(crate::runs::ClaimOutcome::Claimed(record));
            }
            if record.status != quantrule_api::RunState::Running {
                return Ok(crate::runs::ClaimOutcome::Terminal(record));
            }
            if !redelivered || record.digest.is_some() {
                return Ok(crate::runs::ClaimOutcome::Running(record));
            }
            if record.attempts >= crate::runs::MAX_ATTEMPTS {
                return Ok(crate::runs::ClaimOutcome::Exhausted(record));
            }
            // The conditional update is what makes two redeliveries race
            // safely: only one increments, the other reloads and skips.
            let reclaimed = sqlx::query(
                "UPDATE runs.runs SET attempts = attempts + 1, started_at = now()
                 WHERE run_id = $1 AND status = 'running' AND digest IS NULL
                   AND attempts < $2",
            )
            .bind(&run_id)
            .bind(i64::from(crate::runs::MAX_ATTEMPTS))
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let Some(record) = load_run_rows(&pool, None, &run_id).await? else {
                return Ok(crate::runs::ClaimOutcome::Missing);
            };
            if reclaimed.rows_affected() == 1 {
                Ok(crate::runs::ClaimOutcome::Claimed(record))
            } else if record.status == quantrule_api::RunState::Running {
                Ok(crate::runs::ClaimOutcome::Running(record))
            } else {
                Ok(crate::runs::ClaimOutcome::Terminal(record))
            }
        })
    }

    fn computed(
        &self,
        run_id: String,
        digest: quantrule_api::Digest,
        artifacts: Vec<quantrule_api::ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE runs.runs SET digest = $2 WHERE run_id = $1 AND status = 'running'",
            )
            .bind(&run_id)
            .bind(digest.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return load_run_rows(&pool, None, &run_id)
                    .await?
                    .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")));
            }
            for stage in stages {
                let elapsed = stage
                    .elapsed_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        crate::runs::RunStoreError("a stage duration does not fit".to_owned())
                    })?;
                sqlx::query(
                    "UPDATE runs.stages SET state = $3, elapsed_ms = $4
                     WHERE run_id = $1 AND name = $2",
                )
                .bind(&run_id)
                .bind(stage.name.as_str())
                .bind(stage.state.as_str())
                .bind(elapsed)
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            for artifact in &artifacts {
                let bytes = i64::try_from(artifact.bytes).map_err(|_| {
                    crate::runs::RunStoreError("an artifact size does not fit".to_owned())
                })?;
                sqlx::query(
                    "INSERT INTO runs.artifacts (run_id, name, bytes, sha256)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (run_id, name) DO UPDATE SET
                         bytes = EXCLUDED.bytes,
                         sha256 = EXCLUDED.sha256",
                )
                .bind(&run_id)
                .bind(&artifact.name)
                .bind(bytes)
                .bind(artifact.sha256.as_hex())
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")))
        })
    }

    fn complete(
        &self,
        run_id: String,
        digest: quantrule_api::Digest,
        artifacts: Vec<quantrule_api::ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE runs.runs SET status = 'succeeded', digest = $2, finished_at = now()
                 WHERE run_id = $1 AND status = 'running'",
            )
            .bind(&run_id)
            .bind(digest.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return load_run_rows(&pool, None, &run_id)
                    .await?
                    .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")));
            }
            for stage in stages {
                let elapsed = stage
                    .elapsed_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        crate::runs::RunStoreError("a stage duration does not fit".to_owned())
                    })?;
                sqlx::query(
                    "UPDATE runs.stages SET state = $3, elapsed_ms = $4
                     WHERE run_id = $1 AND name = $2",
                )
                .bind(&run_id)
                .bind(stage.name.as_str())
                .bind(stage.state.as_str())
                .bind(elapsed)
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            for artifact in &artifacts {
                let bytes = i64::try_from(artifact.bytes).map_err(|_| {
                    crate::runs::RunStoreError("an artifact size does not fit".to_owned())
                })?;
                sqlx::query(
                    "INSERT INTO runs.artifacts (run_id, name, bytes, sha256)
                     VALUES ($1, $2, $3, $4)
                     ON CONFLICT (run_id, name) DO UPDATE SET
                         bytes = EXCLUDED.bytes,
                         sha256 = EXCLUDED.sha256",
                )
                .bind(&run_id)
                .bind(&artifact.name)
                .bind(bytes)
                .bind(artifact.sha256.as_hex())
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            transaction
                .commit()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")))
        })
    }

    fn fail(
        &self,
        run_id: String,
        error: String,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE runs.runs SET status = 'failed', error = $2, finished_at = now()
                 WHERE run_id = $1 AND status = 'running'",
            )
            .bind(&run_id)
            .bind(&error)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return load_run_rows(&pool, None, &run_id)
                    .await?
                    .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")));
            }
            for stage in stages {
                let elapsed = stage
                    .elapsed_ms
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                        crate::runs::RunStoreError("a stage duration does not fit".to_owned())
                    })?;
                sqlx::query(
                    "UPDATE runs.stages SET state = $3, elapsed_ms = $4
                     WHERE run_id = $1 AND name = $2",
                )
                .bind(&run_id)
                .bind(stage.name.as_str())
                .bind(stage.state.as_str())
                .bind(elapsed)
                .execute(&pool)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")))
        })
    }
}
