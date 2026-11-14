//! The gateway and evidence Postgres repositories.
//!
//! The pool and the client-library boundary live in `quantrule-runtime`. The
//! catalog and config repositories moved to `quantrule-data-service` in P3;
//! these rows are the gateway's profile table and the evidence hypothesis store
//! until P5 moves them.

mod runs;

pub use runs::PgRunStore;

use quantrule_api::ProfileId;
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

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

/// A profile store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgProfileStore {
    pool: PgPool,
}

impl PgProfileStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type ProfileRow = (String, String, String);

fn profile_record(
    row: ProfileRow,
) -> Result<crate::profile::ProfileRecord, crate::profile::ProfileError> {
    Ok(crate::profile::ProfileRecord {
        profile_id: ProfileId::parse(&row.0).map_err(|error| {
            crate::profile::ProfileError::Store(format!(
                "a stored profile id is malformed: {error}"
            ))
        })?,
        name: row.1,
        created_at: row.2,
    })
}

impl crate::profile::ProfileStore for PgProfileStore {
    fn create(
        &self,
        record: crate::profile::ProfileRecord,
    ) -> BoxFuture<Result<(), crate::profile::ProfileError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let inserted = sqlx::query(
                "INSERT INTO gateway.profiles (profile_id, name, created_at)
                 VALUES ($1, $2, $3::timestamptz)
                 ON CONFLICT (profile_id) DO NOTHING",
            )
            .bind(record.profile_id.as_str())
            .bind(&record.name)
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map_err(|error| crate::profile::ProfileError::Store(error.to_string()))?;
            if inserted.rows_affected() == 0 {
                return Err(crate::profile::ProfileError::AlreadyExists);
            }
            Ok(())
        })
    }

    fn get(
        &self,
        profile_id: ProfileId,
    ) -> BoxFuture<Result<Option<crate::profile::ProfileRecord>, crate::profile::ProfileError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<ProfileRow> = sqlx::query_as(
                "SELECT profile_id, name,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM gateway.profiles WHERE profile_id = $1",
            )
            .bind(profile_id.as_str())
            .fetch_optional(&pool)
            .await
            .map_err(|error| crate::profile::ProfileError::Store(error.to_string()))?;
            row.map(profile_record).transpose()
        })
    }

    fn list(
        &self,
    ) -> BoxFuture<Result<Vec<crate::profile::ProfileRecord>, crate::profile::ProfileError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<ProfileRow> = sqlx::query_as(
                "SELECT profile_id, name,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM gateway.profiles ORDER BY profile_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::profile::ProfileError::Store(error.to_string()))?;
            rows.into_iter().map(profile_record).collect()
        })
    }
}
