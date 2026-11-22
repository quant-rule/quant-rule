//! The Postgres outbox and release-tombstone implementations.
//!
//! Both read and write the `runs` schema the evidence store already owns. The
//! submission transaction inserts a notification; the publisher marks it
//! published; the reconciler inserts one that predates the outbox. The release
//! tombstone commits with the consumer delete, so a crash between the databases
//! leaves a retry, not an orphan reference.

use quantrule_api::{DatasetReferenceKind, Digest, ProfileId, RunId};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use crate::outbox::{EnsureOutcome, OutboxError, OutboxNotification, ReconcileMode, RunOutbox};
use crate::postgres::PgRunStore;
use crate::releases::{ReferenceReleaseLog, ReleaseLogError};
use crate::runs::{DatasetReferenceRelease, NotificationKind, RunNotification};

fn run_id(value: &str) -> Result<RunId, OutboxError> {
    Digest::parse(value)
        .map(RunId)
        .map_err(|error| OutboxError(format!("a stored run id is malformed: {error}")))
}

fn kind(value: &str) -> Result<NotificationKind, OutboxError> {
    NotificationKind::parse(value)
        .ok_or_else(|| OutboxError(format!("a stored notification is unknown: {value}")))
}

impl RunOutbox for PgRunStore {
    fn wanted_notifications(&self) -> BoxFuture<Result<Vec<RunNotification>, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<(String, String)> = sqlx::query_as(
                "SELECT run_id, 'run.requested' FROM runs.runs
                  WHERE status = 'queued'
                     OR (status = 'running' AND digest IS NULL
                         AND (lease_expires_at IS NULL
                              OR lease_expires_at < now()))
                 UNION ALL
                 SELECT run_id, 'run.computed' FROM runs.runs
                  WHERE status = 'running' AND digest IS NOT NULL
                 ORDER BY run_id, 2",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            rows.into_iter()
                .map(|(run, routing_key)| {
                    Ok(RunNotification {
                        run_id: run_id(&run)?,
                        kind: kind(&routing_key)?,
                    })
                })
                .collect()
        })
    }

    fn ensure(
        &self,
        wanted: &[RunNotification],
        mode: ReconcileMode,
    ) -> BoxFuture<Result<EnsureOutcome, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        let lease_seconds = self.lease_seconds;
        let wanted = wanted.to_vec();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| OutboxError(error.to_string()))?;
            let mut outcome = EnsureOutcome::default();
            for notification in &wanted {
                let inserted = sqlx::query(
                    "INSERT INTO runs.outbox (run_id, routing_key)
                     VALUES ($1, $2)
                     ON CONFLICT (run_id, routing_key) DO NOTHING",
                )
                .bind(notification.run_id.to_string())
                .bind(notification.kind.routing_key())
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
                            "UPDATE runs.outbox SET published_at = NULL, last_error = NULL
                              WHERE run_id = $1 AND routing_key = $2
                                AND published_at IS NOT NULL",
                        )
                        .bind(notification.run_id.to_string())
                        .bind(notification.kind.routing_key())
                        .execute(&mut *transaction)
                        .await
                    }
                    ReconcileMode::Periodic => {
                        // Only an expired-lease request is re-armed, and only
                        // after the lease window has passed since its last
                        // confirmed publish. A live lease never matches.
                        sqlx::query(
                            "UPDATE runs.outbox o
                                SET published_at = NULL, last_error = NULL
                               FROM runs.runs r
                              WHERE o.run_id = $1 AND o.routing_key = $2
                                AND o.published_at IS NOT NULL
                                AND r.run_id = o.run_id
                                AND r.status = 'running' AND r.digest IS NULL
                                AND (r.lease_expires_at IS NULL
                                     OR r.lease_expires_at < now())
                                AND o.published_at
                                    < now() - make_interval(secs => $3)",
                        )
                        .bind(notification.run_id.to_string())
                        .bind(notification.kind.routing_key())
                        .bind(lease_seconds as f64)
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

    fn pending(&self, limit: usize) -> BoxFuture<Result<Vec<OutboxNotification>, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let limit = i64::try_from(limit).unwrap_or(i64::MAX);
            let rows: Vec<(i64, String, String, i32)> = sqlx::query_as(
                "SELECT event_id, run_id, routing_key, attempts
                   FROM runs.outbox
                  WHERE published_at IS NULL
                  ORDER BY event_id
                  LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            rows.into_iter()
                .map(|(event_id, run, routing_key, attempts)| {
                    Ok(OutboxNotification {
                        event_id,
                        run_id: run_id(&run)?,
                        kind: kind(&routing_key)?,
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
                "UPDATE runs.outbox
                    SET published_at = now(), attempts = attempts + 1, last_error = NULL
                  WHERE event_id = $1",
            )
            .bind(event_id)
            .execute(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return Err(OutboxError(format!("outbox event {event_id} is unknown")));
            }
            Ok(())
        })
    }

    fn failed(&self, event_id: i64, error: String) -> BoxFuture<Result<(), OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE runs.outbox
                    SET attempts = attempts + 1, last_error = $2
                  WHERE event_id = $1",
            )
            .bind(event_id)
            .bind(&error)
            .execute(&pool)
            .await
            .map_err(|error| OutboxError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return Err(OutboxError(format!("outbox event {event_id} is unknown")));
            }
            Ok(())
        })
    }

    fn outstanding(&self) -> BoxFuture<Result<u64, OutboxError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let row: (i64,) =
                sqlx::query_as("SELECT count(*) FROM runs.outbox WHERE published_at IS NULL")
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| OutboxError(error.to_string()))?;
            Ok(u64::try_from(row.0).unwrap_or(u64::MAX))
        })
    }
}

impl ReferenceReleaseLog for PgRunStore {
    fn record(&self, release: DatasetReferenceRelease) -> BoxFuture<Result<(), ReleaseLogError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            sqlx::query(
                "INSERT INTO runs.dataset_reference_releases
                     (profile_id, data_version, reference_id, kind)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (profile_id, data_version, reference_id) DO NOTHING",
            )
            .bind(release.profile.as_str())
            .bind(release.data_version.to_string())
            .bind(&release.reference_id)
            .bind(release.kind.as_str())
            .execute(&pool)
            .await
            .map_err(|error| ReleaseLogError(error.to_string()))?;
            Ok(())
        })
    }

    fn pending_releases(&self) -> BoxFuture<Result<Vec<DatasetReferenceRelease>, ReleaseLogError>> {
        let pool: PgPool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<(String, String, String, String)> = sqlx::query_as(
                "SELECT profile_id, data_version, reference_id, kind
                   FROM runs.dataset_reference_releases
                  WHERE released_at IS NULL
                  ORDER BY created_at, profile_id, data_version, reference_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| ReleaseLogError(error.to_string()))?;
            rows.into_iter()
                .map(|(profile, data_version, reference_id, kind)| {
                    Ok(DatasetReferenceRelease {
                        profile: ProfileId::parse(&profile).map_err(|error| {
                            ReleaseLogError(format!("a stored profile is malformed: {error}"))
                        })?,
                        data_version: Digest::parse(&data_version).map_err(|error| {
                            ReleaseLogError(format!("a stored data version is malformed: {error}"))
                        })?,
                        reference_id,
                        kind: DatasetReferenceKind::parse(&kind).ok_or_else(|| {
                            ReleaseLogError(format!("a stored reference kind is unknown: {kind}"))
                        })?,
                    })
                })
                .collect()
        })
    }

    fn recorded(
        &self,
        release: &DatasetReferenceRelease,
        error: Option<&str>,
    ) -> BoxFuture<Result<(), ReleaseLogError>> {
        let pool: PgPool = self.pool.clone();
        let release = release.clone();
        let error = error.map(str::to_owned);
        Box::pin(async move {
            let updated = sqlx::query(
                "UPDATE runs.dataset_reference_releases
                    SET attempts = attempts + 1,
                        released_at = CASE WHEN $5 THEN now() ELSE released_at END,
                        last_error = $6
                  WHERE profile_id = $1 AND data_version = $2 AND reference_id = $3
                    AND kind = $4",
            )
            .bind(release.profile.as_str())
            .bind(release.data_version.to_string())
            .bind(&release.reference_id)
            .bind(release.kind.as_str())
            .bind(error.is_none())
            .bind(error.as_deref())
            .execute(&pool)
            .await
            .map_err(|error| ReleaseLogError(error.to_string()))?;
            if updated.rows_affected() == 0 {
                return Err(ReleaseLogError(format!(
                    "the release tombstone {} is unknown",
                    release.reference_id
                )));
            }
            Ok(())
        })
    }
}
