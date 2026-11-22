//! Fenced finalisation writes for the Postgres run store.
//!
//! A finalisation takes the leader lease row lock before it writes a terminal
//! state. Acquisition takes the same lock, so a new leader cannot increment the
//! term until this write commits, and a stale leader sees the new term and is
//! refused. Keeping this seam separate leaves the run store's claim, list, and
//! archive code readable.

use quantrule_api::{ArtifactRef, Digest, StageProgress};

use crate::postgres::{PgRunStore, load_run_rows};
use crate::runs::{RunRecord, RunStoreError};

impl PgRunStore {
    /// Finish a computed run under a live leader term.
    pub(crate) async fn finalize_succeeded(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        digest: Digest,
        artifacts: Vec<ArtifactRef>,
        stages: Vec<StageProgress>,
    ) -> Result<RunRecord, RunStoreError> {
        let pool = self.pool.clone();
        let term = i64::try_from(term)
            .map_err(|_| RunStoreError("a lease term does not fit".to_owned()))?;
        let leader_term = i64::try_from(leader_term)
            .map_err(|_| RunStoreError("a leader term does not fit".to_owned()))?;
        let mut transaction = pool
            .begin()
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        if !live_leader(&mut transaction, leader_term).await? {
            transaction
                .rollback()
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            return load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
        }
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
        .map_err(|error| RunStoreError(error.to_string()))?;
        if updated.rows_affected() == 0 {
            transaction
                .rollback()
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            return load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
        }
        // A terminal run needs no further notification. Deleting the rows keeps
        // the outbox bounded; a later erase removes any remaining row through
        // its cascade.
        sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
            .bind(&run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        for stage in stages {
            update_stage(&mut transaction, &run_id, &stage).await?;
        }
        for artifact in &artifacts {
            insert_artifact(&mut transaction, &run_id, artifact).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        load_run_rows(&pool, None, &run_id)
            .await?
            .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")))
    }

    /// Fail a computed run under a live leader term.
    pub(crate) async fn finalize_failed(
        &self,
        run_id: String,
        term: u64,
        leader_term: u64,
        error: String,
        stages: Vec<StageProgress>,
    ) -> Result<RunRecord, RunStoreError> {
        let pool = self.pool.clone();
        let term = i64::try_from(term)
            .map_err(|_| RunStoreError("a lease term does not fit".to_owned()))?;
        let leader_term = i64::try_from(leader_term)
            .map_err(|_| RunStoreError("a leader term does not fit".to_owned()))?;
        let mut transaction = pool
            .begin()
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        if !live_leader(&mut transaction, leader_term).await? {
            transaction
                .rollback()
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            return load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
        }
        let updated = sqlx::query(
            "UPDATE runs.runs
                SET status = 'failed', error = $2, finished_at = now(),
                    lease_holder = NULL, lease_expires_at = NULL
              WHERE run_id = $1 AND status = 'running' AND lease_term = $3",
        )
        .bind(&run_id)
        .bind(&error)
        .bind(term)
        .execute(&mut *transaction)
        .await
        .map_err(|error| RunStoreError(error.to_string()))?;
        if updated.rows_affected() == 0 {
            transaction
                .rollback()
                .await
                .map_err(|error| RunStoreError(error.to_string()))?;
            return load_run_rows(&pool, None, &run_id)
                .await?
                .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")));
        }
        sqlx::query("DELETE FROM runs.outbox WHERE run_id = $1")
            .bind(&run_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        for stage in stages {
            update_stage(&mut transaction, &run_id, &stage).await?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| RunStoreError(error.to_string()))?;
        load_run_rows(&pool, None, &run_id)
            .await?
            .ok_or_else(|| RunStoreError(format!("run {run_id} is unknown")))
    }
}

/// Lock the leader row and confirm the term is the live one.
async fn live_leader(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    leader_term: i64,
) -> Result<bool, RunStoreError> {
    let row: Option<(i64, bool)> = sqlx::query_as(
        "SELECT term, expires_at >= now()
           FROM runs.leader_lease WHERE singleton FOR UPDATE",
    )
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|error| RunStoreError(error.to_string()))?;
    Ok(row.is_some_and(|(term, live)| term == leader_term && live))
}

async fn update_stage(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    stage: &StageProgress,
) -> Result<(), RunStoreError> {
    let elapsed = stage
        .elapsed_ms
        .map(i64::try_from)
        .transpose()
        .map_err(|_| RunStoreError("a stage duration does not fit".to_owned()))?;
    sqlx::query(
        "UPDATE runs.stages SET state = $3, elapsed_ms = $4
         WHERE run_id = $1 AND name = $2",
    )
    .bind(run_id)
    .bind(stage.name.as_str())
    .bind(stage.state.as_str())
    .bind(elapsed)
    .execute(&mut **transaction)
    .await
    .map_err(|error| RunStoreError(error.to_string()))?;
    Ok(())
}

async fn insert_artifact(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    run_id: &str,
    artifact: &ArtifactRef,
) -> Result<(), RunStoreError> {
    let bytes = i64::try_from(artifact.bytes)
        .map_err(|_| RunStoreError("an artifact size does not fit".to_owned()))?;
    sqlx::query(
        "INSERT INTO runs.artifacts (run_id, name, bytes, sha256)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (run_id, name) DO UPDATE SET
             bytes = EXCLUDED.bytes,
             sha256 = EXCLUDED.sha256",
    )
    .bind(run_id)
    .bind(&artifact.name)
    .bind(bytes)
    .bind(artifact.sha256.as_hex())
    .execute(&mut **transaction)
    .await
    .map_err(|error| RunStoreError(error.to_string()))?;
    Ok(())
}
