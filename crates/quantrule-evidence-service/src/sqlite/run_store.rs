//! The SQLite run-store implementation.
//!
//! The store type and its row helpers live in `sqlite::runs`; this module holds
//! the `RunStore` impl so the file stays below the project's size limit.

use quantrule_api::{ArtifactRef, Digest, ProfileId, RunId, RunState, RunSummary, StageProgress};
use quantrule_runtime::adapters::BoxFuture;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::runs::{
    CancelOutcome, ClaimOutcome, MAX_ATTEMPTS, NewRun, RunArchiveOutcome, RunDatasetReference,
    RunEraseOutcome, RunEraseReceipt, RunRecord, RunStore, RunStoreError, SubmitOutcome,
};
use crate::sqlite::SqliteRunStore;
use crate::sqlite::runs::{
    RunArchiveDbOutcome, load_run, load_summary, run_state, store_error, summary_row,
    update_stages, upsert_artifacts,
};
use crate::sqlite::{
    ARTIFACTS_TABLE, RUN_RETENTION_TABLE, RUNS_TABLE, STAGES_TABLE, SUBMISSIONS_TABLE,
};

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
            let model_artifact_id = run.model_artifact_id.as_ref().map(ToString::to_string);
            let outcome = database
                .call(move |connection| {
                    let transaction = connection
                        .transaction_with_behavior(TransactionBehavior::Immediate)
                        .map_err(store_error)?;
                    let insert_run = format!(
                        "INSERT INTO {RUNS_TABLE}
                             (run_id, config_id, data_version, owner, status, submitted_at,
                              config_hash, model_version, image_digest, model_artifact_id)
                         VALUES (?1, ?2, ?3, ?4, 'queued', ?5, ?6, ?7, ?8, ?9)
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
                                image_digest,
                                model_artifact_id
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
            let rows =
                database
                    .call(move |connection| {
                        let rows = match status {
                            Some(status) => {
                                let sql = format!(
                                    "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at, s.archived_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1 AND r.status = ?2
                                    AND s.archived_at IS NULL
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?3"
                                );
                                let mut statement =
                                    connection.prepare(&sql).map_err(store_error)?;
                                let mapped = statement
                                    .query_map(
                                        params![profile_id, status.as_str(), limit],
                                        summary_row,
                                    )
                                    .map_err(store_error)?;
                                mapped
                                    .collect::<rusqlite::Result<Vec<_>>>()
                                    .map_err(store_error)?
                            }
                            None => {
                                let sql = format!(
                                    "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at, s.archived_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1 AND s.archived_at IS NULL
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?2"
                                );
                                let mut statement =
                                    connection.prepare(&sql).map_err(store_error)?;
                                let mapped = statement
                                    .query_map(params![profile_id, limit], summary_row)
                                    .map_err(store_error)?;
                                mapped
                                    .collect::<rusqlite::Result<Vec<_>>>()
                                    .map_err(store_error)?
                            }
                        };
                        Ok::<
                            Vec<(String, String, String, String, String, Option<String>)>,
                            RunStoreError,
                        >(rows)
                    })
                    .await
                    .map_err(|error| RunStoreError(error.to_string()))?;
            rows.into_iter()
                .map(
                    |(run_id, status, config_id, data_version, submitted_at, archived_at)| {
                        Ok(RunSummary {
                            run_id: RunId(Digest::parse(&run_id).map_err(|error| {
                                RunStoreError(format!("a stored run id is malformed: {error}"))
                            })?),
                            status: run_state(&status)?,
                            config_id,
                            data_version: Digest::parse(&data_version).map_err(|error| {
                                RunStoreError(format!(
                                    "a stored data version is malformed: {error}"
                                ))
                            })?,
                            submitted_at,
                            elapsed_ms: None,
                            archived_at,
                        })
                    },
                )
                .collect()
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
        status: Option<RunState>,
        limit: usize,
    ) -> BoxFuture<Result<Vec<RunSummary>, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows =
                database
                    .call(move |connection| {
                        let rows = match status {
                            Some(status) => {
                                let sql = format!(
                                    "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at, s.archived_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1 AND r.status = ?2
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?3"
                                );
                                let mut statement =
                                    connection.prepare(&sql).map_err(store_error)?;
                                let mapped = statement
                                    .query_map(
                                        params![profile_id, status.as_str(), limit],
                                        summary_row,
                                    )
                                    .map_err(store_error)?;
                                mapped
                                    .collect::<rusqlite::Result<Vec<_>>>()
                                    .map_err(store_error)?
                            }
                            None => {
                                let sql = format!(
                                    "SELECT r.run_id, r.status, r.config_id, r.data_version,
                                        r.submitted_at, s.archived_at
                                   FROM {RUNS_TABLE} r
                                   JOIN {SUBMISSIONS_TABLE} s ON s.run_id = r.run_id
                                  WHERE s.profile_id = ?1
                                  ORDER BY r.submitted_at DESC, r.run_id ASC LIMIT ?2"
                                );
                                let mut statement =
                                    connection.prepare(&sql).map_err(store_error)?;
                                let mapped = statement
                                    .query_map(params![profile_id, limit], summary_row)
                                    .map_err(store_error)?;
                                mapped
                                    .collect::<rusqlite::Result<Vec<_>>>()
                                    .map_err(store_error)?
                            }
                        };
                        Ok::<
                            Vec<(String, String, String, String, String, Option<String>)>,
                            RunStoreError,
                        >(rows)
                    })
                    .await
                    .map_err(|error| RunStoreError(error.to_string()))?;
            rows.into_iter()
                .map(
                    |(run_id, status, config_id, data_version, submitted_at, archived_at)| {
                        Ok(RunSummary {
                            run_id: RunId(Digest::parse(&run_id).map_err(|error| {
                                RunStoreError(format!("a stored run id is malformed: {error}"))
                            })?),
                            status: run_state(&status)?,
                            config_id,
                            data_version: Digest::parse(&data_version).map_err(|error| {
                                RunStoreError(format!(
                                    "a stored data version is malformed: {error}"
                                ))
                            })?,
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

    fn archive(
        &self,
        profile: ProfileId,
        run_id: String,
        archived_at: String,
    ) -> BoxFuture<Result<RunArchiveOutcome, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let read_profile = profile.clone();
            let read_run = run_id.clone();
            let outcome = database
                .clone()
                .call(move |connection| -> rusqlite::Result<RunArchiveDbOutcome> {
                    let transaction =
                        connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                    let updated = transaction.execute(
                        &format!(
                            "UPDATE {SUBMISSIONS_TABLE} SET archived_at = ?3
                                  WHERE profile_id = ?1 AND run_id = ?2 AND archived_at IS NULL"
                        ),
                        params![profile.as_str(), run_id, archived_at],
                    )?;
                    if updated == 0 {
                        let exists: Option<i64> = transaction
                            .query_row(
                                &format!(
                                    "SELECT 1 FROM {SUBMISSIONS_TABLE}
                                      WHERE profile_id = ?1 AND run_id = ?2"
                                ),
                                params![profile.as_str(), run_id],
                                |row| row.get(0),
                            )
                            .optional()?;
                        return Ok(if exists.is_some() {
                            RunArchiveDbOutcome::AlreadyArchived
                        } else {
                            RunArchiveDbOutcome::Missing
                        });
                    }
                    transaction.execute(
                        &format!(
                            "INSERT INTO {RUN_RETENTION_TABLE}
                                     (profile_id, kind, record_id, action, reason, bytes)
                                 VALUES (?1, 'run', ?2, 'archive', 'operator.archive', 0)"
                        ),
                        params![profile.as_str(), run_id],
                    )?;
                    transaction.commit()?;
                    Ok(RunArchiveDbOutcome::Archived)
                })
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            match outcome {
                RunArchiveDbOutcome::Missing => Ok(RunArchiveOutcome::Missing),
                RunArchiveDbOutcome::AlreadyArchived => Ok(RunArchiveOutcome::AlreadyArchived),
                RunArchiveDbOutcome::Archived => {
                    let summary = database
                        .call(
                            move |connection| -> Result<Option<RunSummary>, RunStoreError> {
                                load_summary(connection, &read_profile, &read_run)
                            },
                        )
                        .await
                        .map_err(|error| RunStoreError(error.to_string()))?
                        .ok_or_else(|| {
                            RunStoreError("the run vanished after archive".to_owned())
                        })?;
                    Ok(RunArchiveOutcome::Archived(summary))
                }
            }
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        run_id: String,
    ) -> BoxFuture<Result<RunEraseOutcome, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(
                    move |connection| -> Result<RunEraseOutcome, RunStoreError> {
                        let transaction = connection
                            .transaction_with_behavior(TransactionBehavior::Immediate)
                            .map_err(store_error)?;
                        let row: Option<(Option<String>, String, String)> = transaction
                            .query_row(
                                &format!(
                                    "SELECT s.archived_at, r.status, r.data_version
                                   FROM {SUBMISSIONS_TABLE} s
                                   JOIN {RUNS_TABLE} r ON r.run_id = s.run_id
                                  WHERE s.profile_id = ?1 AND s.run_id = ?2"
                                ),
                                params![profile.as_str(), run_id],
                                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                            )
                            .optional()
                            .map_err(store_error)?;
                        let Some((archived_at, status_word, data_version)) = row else {
                            return Ok(RunEraseOutcome::Missing);
                        };
                        if archived_at.is_none() {
                            return Ok(RunEraseOutcome::NotArchived);
                        }
                        let status = run_state(&status_word)?;
                        if !matches!(status, RunState::Failed | RunState::Cancelled) {
                            return Ok(RunEraseOutcome::Protected(status));
                        }
                        transaction
                            .execute(
                                &format!(
                                    "DELETE FROM {SUBMISSIONS_TABLE}
                                  WHERE profile_id = ?1 AND run_id = ?2"
                                ),
                                params![profile.as_str(), run_id],
                            )
                            .map_err(store_error)?;
                        let remaining: i64 = transaction
                            .query_row(
                                &format!(
                                    "SELECT count(*) FROM {SUBMISSIONS_TABLE} WHERE run_id = ?1"
                                ),
                                params![run_id],
                                |row| row.get(0),
                            )
                            .map_err(store_error)?;
                        let mut blobs = Vec::new();
                        let mut bytes = 0u64;
                        if remaining == 0 {
                            let artifacts: Vec<(String, i64)> = {
                                let mut statement = transaction
                                    .prepare(&format!(
                                        "SELECT sha256, bytes FROM {ARTIFACTS_TABLE}
                                      WHERE run_id = ?1 ORDER BY sha256"
                                    ))
                                    .map_err(store_error)?;
                                let mapped = statement
                                    .query_map(params![run_id], |row| {
                                        Ok((row.get(0)?, row.get(1)?))
                                    })
                                    .map_err(store_error)?;
                                mapped
                                    .collect::<rusqlite::Result<Vec<_>>>()
                                    .map_err(store_error)?
                            };
                            transaction
                                .execute(
                                    &format!("DELETE FROM {RUNS_TABLE} WHERE run_id = ?1"),
                                    params![run_id],
                                )
                                .map_err(store_error)?;
                            let mut seen = std::collections::BTreeSet::new();
                            for (sha, size) in artifacts {
                                if !seen.insert(sha.clone()) {
                                    continue;
                                }
                                let references: i64 = transaction
                                .query_row(
                                    &format!(
                                        "SELECT count(*) FROM {ARTIFACTS_TABLE} WHERE sha256 = ?1"
                                    ),
                                    params![sha],
                                    |row| row.get(0),
                                )
                                .map_err(store_error)?;
                                if references == 0 {
                                    bytes = bytes.saturating_add(u64::try_from(size).unwrap_or(0));
                                    blobs.push(Digest::parse(&sha).map_err(|error| {
                                        RunStoreError(format!(
                                            "a stored artifact hash is malformed: {error}"
                                        ))
                                    })?);
                                }
                            }
                        }
                        transaction
                            .execute(
                                &format!(
                                    "INSERT INTO {RUN_RETENTION_TABLE}
                                     (profile_id, kind, record_id, action, reason, bytes)
                                 VALUES (?1, 'run', ?2, 'erase', 'operator.erase', ?3)"
                                ),
                                params![
                                    profile.as_str(),
                                    run_id,
                                    i64::try_from(bytes).unwrap_or(i64::MAX)
                                ],
                            )
                            .map_err(store_error)?;
                        transaction.commit().map_err(store_error)?;
                        Ok(RunEraseOutcome::Erased(RunEraseReceipt {
                            run_id: RunId(Digest::parse(&run_id).map_err(|error| {
                                RunStoreError(format!("the run id is malformed: {error}"))
                            })?),
                            data_version: Digest::parse(&data_version).map_err(|error| {
                                RunStoreError(format!(
                                    "a stored data version is malformed: {error}"
                                ))
                            })?,
                            bytes,
                            blobs,
                            removed_run: remaining == 0,
                        }))
                    },
                )
                .await
                .map_err(|error| RunStoreError(error.to_string()))
        })
    }

    fn submitted_datasets(&self) -> BoxFuture<Result<Vec<RunDatasetReference>, RunStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            database
                .call(
                    move |connection| -> Result<Vec<RunDatasetReference>, RunStoreError> {
                        let sql = format!(
                            "SELECT s.profile_id, r.run_id, r.data_version
                           FROM {SUBMISSIONS_TABLE} s
                           JOIN {RUNS_TABLE} r ON r.run_id = s.run_id
                          ORDER BY s.profile_id, r.run_id"
                        );
                        let mut statement = connection.prepare(&sql).map_err(store_error)?;
                        let mapped = statement
                            .query_map([], |row| {
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, String>(1)?,
                                    row.get::<_, String>(2)?,
                                ))
                            })
                            .map_err(store_error)?;
                        let mut rows = Vec::new();
                        for row in mapped {
                            let (profile, run_id, data_version) = row.map_err(store_error)?;
                            rows.push(RunDatasetReference {
                                profile: ProfileId::parse(&profile).map_err(|error| {
                                    RunStoreError(format!("a stored profile is malformed: {error}"))
                                })?,
                                run_id: RunId(Digest::parse(&run_id).map_err(|error| {
                                    RunStoreError(format!("a stored run id is malformed: {error}"))
                                })?),
                                data_version: Digest::parse(&data_version).map_err(|error| {
                                    RunStoreError(format!(
                                        "a stored data version is malformed: {error}"
                                    ))
                                })?,
                            });
                        }
                        Ok(rows)
                    },
                )
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
