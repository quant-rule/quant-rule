//! The Postgres stage request-outbox implementation.
//!
//! It reads and writes the `stage` schema the stage store already owns. A
//! submission inserts the intent in the same transaction as the stage row; the
//! publisher marks it published; the reconciler inserts one that predates the
//! outbox. Terminal rows are excluded, so a re-arm can never publish a stage
//! that already finished.

use quantrule_api::{Digest, StageRunId};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use super::outbox::{StageOutbox, StageOutboxNotification};
use super::postgres::PgStageStore;
use crate::outbox::{EnsureOutcome, OutboxError, ReconcileMode};

fn stage_run_id(value: &str) -> Result<StageRunId, OutboxError> {
    Digest::parse(value)
        .map(StageRunId)
        .map_err(|error| OutboxError(format!("a stored stage run id is malformed: {error}")))
}

impl StageOutbox for PgStageStore {
    fn wanted(&self) -> BoxFuture<Result<Vec<StageRunId>, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<String> = sqlx::query_scalar(
                "SELECT stage_run_id FROM stage.runs
                  WHERE status = 'queued'
                     OR (status = 'running'
                         AND (lease_expires_at IS NULL OR lease_expires_at < now()))
                  ORDER BY stage_run_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            rows.into_iter().map(|row| stage_run_id(&row)).collect()
        })
    }

    fn ensure(
        &self,
        wanted: &[StageRunId],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        let wanted = wanted.to_vec();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| OutboxError(error.to_string()))?;
            let mut outcome = EnsureOutcome::default();
            for stage in &wanted {
                let inserted = sqlx::query(
                    "INSERT INTO stage.outbox (stage_run_id, routing_key)
                     VALUES ($1, 'stage.requested')
                     ON CONFLICT (stage_run_id, routing_key) DO NOTHING",
                )
                .bind(stage.to_string())
                .execute(&mut *transaction)
                .await
                .map_err(|error| OutboxError(error.to_string()))?;
                outcome.inserted = outcome.inserted.saturating_add(inserted.rows_affected());
                if inserted.rows_affected() == 1 {
                    continue;
                }
                let rearmed = match mode {
                    ReconcileMode::Startup => {
                        sqlx::query(
                            "UPDATE stage.outbox SET published_at = NULL, last_error = NULL
                              WHERE stage_run_id = $1 AND routing_key = 'stage.requested'
                                AND published_at IS NOT NULL",
                        )
                        .bind(stage.to_string())
                        .execute(&mut *transaction)
                        .await
                    }
                    ReconcileMode::Periodic => {
                        // Only an expired-lease request is re-armed, and only
                        // after the lease window has passed since its last
                        // confirmed publish. A live lease never matches.
                        sqlx::query(
                            "UPDATE stage.outbox o
                                SET published_at = NULL, last_error = NULL
                               FROM stage.runs r
                              WHERE o.stage_run_id = $1
                                AND o.routing_key = 'stage.requested'
                                AND o.published_at IS NOT NULL
                                AND r.stage_run_id = o.stage_run_id
                                AND r.status = 'running' AND r.artifact_id IS NULL
                                AND (r.lease_expires_at IS NULL
                                     OR r.lease_expires_at < now())
                                AND o.published_at
                                    < now() - make_interval(secs => $2)",
                        )
                        .bind(stage.to_string())
                        .bind(quantrule_service_api::STAGE_LEASE_TTL_SECONDS as f64)
                        .execute(&mut *transaction)
                        .await
                    }
                }
                .map_err(|error| OutboxError(error.to_string()))?;
                outcome.rearmed = outcome.rearmed.saturating_add(rearmed.rows_affected());
            }
            transaction
                .commit()
                .await
                .map_err(|error| OutboxError(error.to_string()))?;
            Ok(outcome)
        })
    }

    fn pending(
        &self,
        limit: usize,
    ) -> BoxFuture<Result<Vec<StageOutboxNotification>, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows: Vec<(i64, String, i32)> = sqlx::query_as(
                "SELECT event_id, stage_run_id, attempts
                   FROM stage.outbox
                  WHERE published_at IS NULL
                  ORDER BY event_id
                  LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            rows.into_iter()
                .map(|(event_id, stage, attempts)| {
                    Ok(StageOutboxNotification {
                        event_id,
                        stage_run_id: stage_run_id(&stage)?,
                        attempts: u32::try_from(attempts).unwrap_or(u32::MAX),
                    })
                })
                .collect()
        })
    }

    fn published(&self, event_id: i64) -> BoxFuture<Result<(), OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE stage.outbox
                    SET published_at = now(), attempts = attempts + 1, last_error = NULL
                  WHERE event_id = $1",
            )
            .bind(event_id)
            .execute(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return Err(OutboxError(format!(
                    "stage outbox event {event_id} is unknown"
                )));
            }
            Ok(())
        })
    }

    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE stage.outbox
                    SET attempts = attempts + 1, last_error = $2
                  WHERE event_id = $1",
            )
            .bind(event_id)
            .bind(&error)
            .execute(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return Err(OutboxError(format!(
                    "stage outbox event {event_id} is unknown"
                )));
            }
            Ok(())
        })
    }

    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let row: (i64,) =
                sqlx::query_as("SELECT count(*) FROM stage.outbox WHERE published_at IS NULL")
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| OutboxError(error.to_string()))?;
            Ok(u64::try_from(row.0).unwrap_or(u64::MAX))
        })
    }
}
