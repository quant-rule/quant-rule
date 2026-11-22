//! The Postgres run-store implementation.
//!
//! The store type and its row helpers live in `postgres`; this module holds the
//! `RunStore` impl so the file stays below the project's size limit.

use quantrule_runtime::adapters::BoxFuture;

use crate::postgres::{PgRunStore, load_run, load_run_rows, run_state};
use crate::runs::{RunArchiveOutcome, RunDatasetReference, RunEraseOutcome, RunEraseReceipt};

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
                      config_hash, model_version, image_digest, model_artifact_id)
                 VALUES ($1, $2, $3, $4, 'queued', $5::timestamptz, $6, $7, $8, $9)
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
            .bind(run.model_artifact_id.as_ref().map(ToString::to_string))
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
            // The notification commits with the run row. A crash after this
            // commit leaves a durable intent, not a queued run nobody sees.
            sqlx::query(
                "INSERT INTO runs.outbox (run_id, routing_key)
                 SELECT $1, 'run.requested'
                  WHERE EXISTS (SELECT 1 FROM runs.runs
                                 WHERE run_id = $1 AND status = 'queued')
                 ON CONFLICT (run_id, routing_key) DO UPDATE
                     SET published_at = NULL, last_error = NULL
                     WHERE runs.outbox.published_at IS NOT NULL",
            )
            .bind(run.run_id.to_string())
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
                           AND s.archived_at IS NULL
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
                         WHERE s.profile_id = $1 AND s.archived_at IS NULL
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
                        archived_at: None,
                    })
                })
                .collect()
        })
    }

    fn list_including_archived(
        &self,
        profile: quantrule_api::ProfileId,
        status: Option<quantrule_api::RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<quantrule_api::RunSummary>, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows: Vec<(String, String, String, String, String, Option<String>)> =
                match status {
                    Some(status) => {
                        sqlx::query_as(
                            "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                to_char(r.submitted_at AT TIME ZONE 'UTC',
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                                to_char(s.archived_at AT TIME ZONE 'UTC',
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
                                        'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                                to_char(s.archived_at AT TIME ZONE 'UTC',
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
                .map(
                    |(run_id, status, config_id, data_version, submitted_at, archived_at)| {
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
                            archived_at,
                        })
                    },
                )
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

    fn archive(
        &self,
        profile: quantrule_api::ProfileId,
        run_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<RunArchiveOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE runs.submissions SET archived_at = $3::timestamptz
                  WHERE profile_id = $1 AND run_id = $2 AND archived_at IS NULL",
            )
            .bind(profile.as_str())
            .bind(&run_id)
            .bind(&archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT 1::text FROM runs.submissions
                      WHERE profile_id = $1 AND run_id = $2",
                )
                .bind(profile.as_str())
                .bind(&run_id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return Ok(if exists.is_some() {
                    RunArchiveOutcome::AlreadyArchived
                } else {
                    RunArchiveOutcome::Missing
                });
            }
            sqlx::query(
                "INSERT INTO runs.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'run', $2, 'archive', 'operator.archive', 0)",
            )
            .bind(profile.as_str())
            .bind(&run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let row: Option<(String, String, String, String, String, Option<String>)> =
                sqlx::query_as(
                    "SELECT r.run_id, r.status, r.config_id, r.data_version,
                            to_char(r.submitted_at AT TIME ZONE 'UTC',
                                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                            to_char(s.archived_at AT TIME ZONE 'UTC',
                                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                     FROM runs.runs r
                     JOIN runs.submissions s ON s.run_id = r.run_id
                     WHERE s.profile_id = $1 AND r.run_id = $2",
                )
                .bind(profile.as_str())
                .bind(&run_id)
                .fetch_optional(&pool)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let row = row.ok_or_else(|| {
                crate::runs::RunStoreError("the run vanished after archive".to_owned())
            })?;
            Ok(RunArchiveOutcome::Archived(quantrule_api::RunSummary {
                run_id: quantrule_api::RunId(quantrule_api::Digest::parse(&row.0).map_err(
                    |error| {
                        crate::runs::RunStoreError(format!("a stored run id is malformed: {error}"))
                    },
                )?),
                status: run_state(&row.1)?,
                config_id: row.2,
                data_version: quantrule_api::Digest::parse(&row.3).map_err(|error| {
                    crate::runs::RunStoreError(format!(
                        "a stored data version is malformed: {error}"
                    ))
                })?,
                submitted_at: row.4,
                elapsed_ms: None,
                archived_at: row.5,
            }))
        })
    }

    fn erase(
        &self,
        profile: quantrule_api::ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunEraseOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let row: Option<(Option<String>, String, String)> = sqlx::query_as(
                "SELECT to_char(s.archived_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        r.status, r.data_version
                   FROM runs.submissions s
                   JOIN runs.runs r ON r.run_id = s.run_id
                  WHERE s.profile_id = $1 AND s.run_id = $2",
            )
            .bind(profile.as_str())
            .bind(&run_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let Some((archived_at, status_word, data_version)) = row else {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return Ok(RunEraseOutcome::Missing);
            };
            if archived_at.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return Ok(RunEraseOutcome::NotArchived);
            }
            let status = run_state(&status_word)?;
            if !matches!(
                status,
                quantrule_api::RunState::Failed | quantrule_api::RunState::Cancelled
            ) {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                return Ok(RunEraseOutcome::Protected(status));
            }
            // The release commits with the delete and survives it. A failed
            // data-service call is retried from this tombstone instead of
            // leaving an orphan reference that blocks the dataset forever.
            sqlx::query(
                "INSERT INTO runs.dataset_reference_releases
                     (profile_id, data_version, reference_id, kind)
                 VALUES ($1, $2, $3, 'run')
                 ON CONFLICT (profile_id, data_version, reference_id) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(&data_version)
            .bind(format!("run:{run_id}"))
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            sqlx::query("DELETE FROM runs.submissions WHERE profile_id = $1 AND run_id = $2")
                .bind(profile.as_str())
                .bind(&run_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let remaining: (i64,) =
                sqlx::query_as("SELECT count(*) FROM runs.submissions WHERE run_id = $1")
                    .bind(&run_id)
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let mut blobs = Vec::new();
            let mut bytes = 0u64;
            if remaining.0 == 0 {
                let artifacts: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT sha256, bytes FROM runs.artifacts WHERE run_id = $1 ORDER BY sha256",
                )
                .bind(&run_id)
                .fetch_all(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                sqlx::query("DELETE FROM runs.runs WHERE run_id = $1")
                    .bind(&run_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                let mut seen = std::collections::BTreeSet::new();
                for (sha, size) in artifacts {
                    if !seen.insert(sha.clone()) {
                        continue;
                    }
                    let references: (i64,) =
                        sqlx::query_as("SELECT count(*) FROM runs.artifacts WHERE sha256 = $1")
                            .bind(&sha)
                            .fetch_one(&mut *transaction)
                            .await
                            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
                    if references.0 == 0 {
                        bytes = bytes.saturating_add(u64::try_from(size).unwrap_or(0));
                        blobs.push(quantrule_api::Digest::parse(&sha).map_err(|error| {
                            crate::runs::RunStoreError(format!(
                                "a stored artifact hash is malformed: {error}"
                            ))
                        })?);
                    }
                }
            }
            sqlx::query(
                "INSERT INTO runs.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'run', $2, 'erase', 'operator.erase', $3)",
            )
            .bind(profile.as_str())
            .bind(&run_id)
            .bind(i64::try_from(bytes).unwrap_or(i64::MAX))
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            Ok(RunEraseOutcome::Erased(RunEraseReceipt {
                run_id: quantrule_api::RunId(quantrule_api::Digest::parse(&run_id).map_err(
                    |error| crate::runs::RunStoreError(format!("the run id is malformed: {error}")),
                )?),
                data_version: quantrule_api::Digest::parse(&data_version).map_err(|error| {
                    crate::runs::RunStoreError(format!(
                        "a stored data version is malformed: {error}"
                    ))
                })?,
                bytes,
                blobs,
                removed_run: remaining.0 == 0,
            }))
        })
    }

    fn submitted_datasets(
        &self,
    ) -> BoxFuture<Result<Vec<RunDatasetReference>, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT s.profile_id, r.run_id, r.data_version
                   FROM runs.submissions s
                   JOIN runs.runs r ON r.run_id = s.run_id
                  ORDER BY s.profile_id, r.run_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            rows.into_iter()
                .map(|(profile, run_id, data_version)| {
                    Ok(RunDatasetReference {
                        profile: quantrule_api::ProfileId::parse(&profile).map_err(|error| {
                            crate::runs::RunStoreError(format!(
                                "a stored profile is malformed: {error}"
                            ))
                        })?,
                        run_id: quantrule_api::RunId(
                            quantrule_api::Digest::parse(&run_id).map_err(|error| {
                                crate::runs::RunStoreError(format!(
                                    "a stored run id is malformed: {error}"
                                ))
                            })?,
                        ),
                        data_version: quantrule_api::Digest::parse(&data_version).map_err(
                            |error| {
                                crate::runs::RunStoreError(format!(
                                    "a stored data version is malformed: {error}"
                                ))
                            },
                        )?,
                    })
                })
                .collect()
        })
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
            // The status predicate is the compare-and-swap: a complete that
            // committed after the read above must win, and a second concurrent
            // cancel must be a conflict, not a second 202.
            let updated = sqlx::query(
                "UPDATE runs.runs
                    SET status = 'cancelled', finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL,
                        lease_term = lease_term + 1
                  WHERE run_id = $1 AND status IN ('queued', 'running')",
            )
            .bind(&run_id)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 1 {
                sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
                    .bind(&run_id)
                    .execute(&pool)
                    .await
                    .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            }
            let record = load_run(&pool, profile.as_str(), &run_id)
                .await?
                .ok_or_else(|| {
                    crate::runs::RunStoreError("the run vanished after cancel".to_owned())
                })?;
            if updated.rows_affected() == 1 {
                Ok(crate::runs::CancelOutcome::Cancelled(record))
            } else {
                Ok(crate::runs::CancelOutcome::Conflict(record))
            }
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
        holder: String,
        continuation: Option<u64>,
    ) -> BoxFuture<Result<crate::runs::ClaimOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        let lease_seconds = self.lease_seconds;
        Box::pin(async move {
            let continuation = continuation
                .map(i64::try_from)
                .transpose()
                .map_err(|_| crate::runs::RunStoreError("a claim term does not fit".to_owned()))?;
            // One conditional statement is the claim. The lease is the claim
            // right: a queued run, an absent/expired lease, or the same
            // holder's own live continuation. A live lease held by another
            // attempt matches no branch and is never stolen.
            let claimed = sqlx::query(
                "UPDATE runs.runs
                 SET status = 'running', started_at = now(), attempts = attempts + 1,
                     lease_holder = $2, lease_term = lease_term + 1,
                     lease_expires_at = now() + make_interval(secs => $3)
                 WHERE run_id = $1
                   AND attempts < $4
                   AND (
                       status = 'queued'
                       OR (status = 'running' AND digest IS NULL
                           AND (lease_expires_at IS NULL
                                OR lease_expires_at < now()
                                OR (lease_holder = $2 AND lease_term = $5)))
                   )",
            )
            .bind(&run_id)
            .bind(&holder)
            .bind(lease_seconds as f64)
            .bind(i64::from(crate::runs::MAX_ATTEMPTS))
            .bind(continuation)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let Some(record) = load_run_rows(&pool, None, &run_id).await? else {
                return Ok(crate::runs::ClaimOutcome::Missing);
            };
            if claimed.rows_affected() == 1 {
                return Ok(crate::runs::ClaimOutcome::Claimed(record));
            }
            if record.status != quantrule_api::RunState::Running {
                return Ok(crate::runs::ClaimOutcome::Terminal(record));
            }
            if record.digest.is_some() {
                return Ok(crate::runs::ClaimOutcome::Running(record));
            }
            let (live, spent): (bool, bool) = sqlx::query_as(
                "SELECT lease_expires_at IS NOT NULL AND lease_expires_at >= now(),
                        attempts >= $2
                   FROM runs.runs WHERE run_id = $1",
            )
            .bind(&run_id)
            .bind(i64::from(crate::runs::MAX_ATTEMPTS))
            .fetch_one(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if live || !spent {
                Ok(crate::runs::ClaimOutcome::Running(record))
            } else {
                Ok(crate::runs::ClaimOutcome::Exhausted(record))
            }
        })
    }

    fn heartbeat(
        &self,
        run_id: String,
        holder: String,
        term: u64,
    ) -> BoxFuture<Result<crate::runs::HeartbeatOutcome, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        let lease_seconds = self.lease_seconds;
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| crate::runs::RunStoreError("a lease term does not fit".to_owned()))?;
            let renewed = sqlx::query(
                "UPDATE runs.runs
                 SET lease_expires_at = now() + make_interval(secs => $4)
                 WHERE run_id = $1 AND status = 'running'
                   AND lease_holder = $2 AND lease_term = $3
                   AND lease_expires_at IS NOT NULL
                   AND lease_expires_at >= now()",
            )
            .bind(&run_id)
            .bind(&holder)
            .bind(term)
            .bind(lease_seconds as f64)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let Some(record) = load_run_rows(&pool, None, &run_id).await? else {
                return Ok(crate::runs::HeartbeatOutcome::Missing);
            };
            if renewed.rows_affected() == 1 {
                Ok(crate::runs::HeartbeatOutcome::Renewed(Box::new(record)))
            } else {
                Ok(crate::runs::HeartbeatOutcome::Stale)
            }
        })
    }

    fn computed(
        &self,
        run_id: String,
        term: u64,
        digest: quantrule_api::Digest,
        artifacts: Vec<quantrule_api::ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| crate::runs::RunStoreError("a lease term does not fit".to_owned()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            // The term is the fence: a stale worker cannot write the digest or
            // the artifact set of a later attempt.
            let updated = sqlx::query(
                "UPDATE runs.runs SET digest = $2
                  WHERE run_id = $1 AND status = 'running' AND lease_term = $3",
            )
            .bind(&run_id)
            .bind(digest.to_string())
            .bind(term)
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
            // The computed notification is written in the same transaction as
            // the digest. A lost worker publish is then repaired by evidence's
            // outbox instead of leaving the run running forever.
            sqlx::query(
                "INSERT INTO runs.outbox (run_id, routing_key)
                 VALUES ($1, 'run.computed')
                 ON CONFLICT (run_id, routing_key) DO NOTHING",
            )
            .bind(&run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
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
        term: u64,
        digest: quantrule_api::Digest,
        artifacts: Vec<quantrule_api::ArtifactRef>,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| crate::runs::RunStoreError("a lease term does not fit".to_owned()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE runs.runs
                    SET status = 'succeeded', digest = $2, finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL
                  WHERE run_id = $1 AND status = 'running' AND lease_term = $3",
            )
            .bind(&run_id)
            .bind(digest.to_string())
            .bind(term)
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
            // A terminal run needs no further notification. Deleting the rows
            // keeps the outbox bounded; a run row that is erased later removes
            // any remaining row through its cascade.
            sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
                .bind(&run_id)
                .execute(&mut *transaction)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
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
        term: u64,
        error: String,
        stages: Vec<quantrule_api::StageProgress>,
    ) -> BoxFuture<Result<crate::runs::RunRecord, crate::runs::RunStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = i64::try_from(term)
                .map_err(|_| crate::runs::RunStoreError("a lease term does not fit".to_owned()))?;
            let updated = sqlx::query(
                "UPDATE runs.runs
                    SET status = 'failed', error = $2, finished_at = now(),
                        lease_holder = NULL, lease_expires_at = NULL
                  WHERE run_id = $1 AND status = 'running' AND lease_term = $3",
            )
            .bind(&run_id)
            .bind(&error)
            .bind(term)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return load_run_rows(&pool, None, &run_id)
                    .await?
                    .ok_or_else(|| crate::runs::RunStoreError(format!("run {run_id} is unknown")));
            }
            sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
                .bind(&run_id)
                .execute(&pool)
                .await
                .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
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
