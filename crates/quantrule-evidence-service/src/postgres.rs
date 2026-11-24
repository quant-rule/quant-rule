//! The Postgres run and hypothesis repositories.
//!
//! The pool and the client-library boundary live in `quantrule-runtime`. These
//! rows read and write only `runs.` and `research.` tables; evidence owns both
//! schemas and never reads the `catalog`, `gateway`, or `simulation` schema.

use quantrule_api::ProfileId;
use sqlx::PgPool;

use quantrule_runtime::adapters::BoxFuture;

/// A run store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgRunStore {
    pub(crate) pool: PgPool,
    pub(crate) lease_seconds: u64,
}

impl PgRunStore {
    /// Build the store over an existing pool with the production lease TTL.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self::with_lease_seconds(pool, quantrule_service_api::RUN_LEASE_TTL_SECONDS)
    }

    /// Build the store with an explicit lease TTL.
    ///
    /// The production service uses [`Self::new`]. A composed test can pass a
    /// zero TTL to observe an expired lease without waiting.
    #[must_use]
    pub fn with_lease_seconds(pool: PgPool, lease_seconds: u64) -> Self {
        Self {
            pool,
            lease_seconds,
        }
    }

    /// Count succeeded runs whose canonical digest is still missing.
    ///
    /// The observability gauge reads this value; a non-zero value for a bounded
    /// interval is the partial-publish alert (F07). The query is read-only and
    /// uses the existing status/digest columns.
    ///
    /// # Errors
    ///
    /// Returns the database error so the caller can keep the last good value
    /// instead of publishing a false zero.
    pub async fn missing_digest_count(&self) -> Result<u64, sqlx::Error> {
        let row: (i64,) = sqlx::query_as(MISSING_DIGEST_QUERY)
            .fetch_one(&self.pool)
            .await?;
        Ok(u64::try_from(row.0).unwrap_or(0))
    }
}

/// The partial-publish query: a succeeded run without its canonical digest.
pub(crate) const MISSING_DIGEST_QUERY: &str = "SELECT count(*)::bigint FROM runs.runs
      WHERE status = 'succeeded' AND digest IS NULL";

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
    Option<String>,
    Option<String>,
    i64,
    Option<String>,
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

pub(crate) fn run_state(
    value: &str,
) -> Result<quantrule_api::RunState, crate::runs::RunStoreError> {
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

pub(crate) async fn load_run_rows(
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
                        r.digest, r.error, r.attempts, r.model_artifact_id,
                        r.lease_holder, r.lease_term,
                        to_char(r.lease_expires_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
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
                        r.digest, r.error, r.attempts, r.model_artifact_id,
                        r.lease_holder, r.lease_term,
                        to_char(r.lease_expires_at AT TIME ZONE 'UTC',
                                'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
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
        model_artifact_id: row
            .12
            .map(|value| quantrule_api::Digest::parse(&value))
            .transpose()
            .map_err(|error| {
                crate::runs::RunStoreError(format!(
                    "a stored model artifact id is malformed: {error}"
                ))
            })?,
        lease_holder: row.13,
        lease_term: u64::try_from(row.14).map_err(|_| {
            crate::runs::RunStoreError("a stored lease term is negative".to_owned())
        })?,
        lease_expires_at: row.15,
    }))
}

pub(crate) async fn load_run(
    pool: &PgPool,
    profile: &str,
    run_id: &str,
) -> Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError> {
    load_run_rows(pool, Some(profile), run_id).await
}

/// A hypothesis store backed by Postgres.
///
/// The sealed record is written whole as JSON and read back by fingerprint; the
/// split columns remain for inspection.
#[derive(Debug, Clone)]
pub struct PgHypothesisStore {
    pool: PgPool,
}

impl PgHypothesisStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl crate::research::HypothesisStore for PgHypothesisStore {
    fn insert(
        &self,
        profile: ProfileId,
        sealed: quantrule_research::SealedHypothesis,
    ) -> BoxFuture<Result<(), crate::research::StoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let encode = |value: &quantrule_research::SealedHypothesis| {
                serde_json::to_string(value)
                    .map_err(|error| crate::research::StoreError(error.to_string()))
            };
            let hypothesis = sealed.hypothesis();
            let costs = serde_json::to_string(&hypothesis.costs)
                .map_err(|error| crate::research::StoreError(error.to_string()))?;
            let split = serde_json::to_string(&hypothesis.split)
                .map_err(|error| crate::research::StoreError(error.to_string()))?;
            let body = encode(&sealed)?;
            sqlx::query(
                "INSERT INTO research.hypotheses
                     (profile_id, fingerprint, premise, rule, dataset, costs, split, criteria,
                      sealed_at, body)
                 VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8, $9::timestamptz,
                         $10::jsonb)",
            )
            .bind(profile.as_str())
            .bind(sealed.fingerprint())
            .bind(&hypothesis.premise)
            .bind(&hypothesis.rule)
            .bind(&hypothesis.dataset)
            .bind(costs)
            .bind(split)
            .bind(&hypothesis.criteria)
            .bind(sealed.sealed_at())
            .bind(body)
            .execute(&pool)
            .await
            .map(|_| ())
            .map_err(|error| crate::research::StoreError(error.to_string()))
        })
    }

    fn load(
        &self,
        profile: ProfileId,
        fingerprint: String,
    ) -> BoxFuture<Result<Option<quantrule_research::SealedHypothesis>, crate::research::StoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<(String,)> = sqlx::query_as(
                "SELECT body::text FROM research.hypotheses
                 WHERE profile_id = $1 AND fingerprint = $2",
            )
            .bind(profile.as_str())
            .bind(&fingerprint)
            .fetch_optional(&pool)
            .await
            .map_err(|error| crate::research::StoreError(error.to_string()))?;
            match row {
                Some((body,)) => serde_json::from_str(&body)
                    .map(Some)
                    .map_err(|error| crate::research::StoreError(error.to_string())),
                None => Ok(None),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MISSING_DIGEST_QUERY;

    #[test]
    fn the_missing_digest_query_counts_only_succeeded_null_digest_rows() {
        assert!(MISSING_DIGEST_QUERY.contains("status = 'succeeded'"));
        assert!(MISSING_DIGEST_QUERY.contains("digest IS NULL"));
        assert!(MISSING_DIGEST_QUERY.contains("runs.runs"));
        assert!(!MISSING_DIGEST_QUERY.contains("state"));
    }
}
