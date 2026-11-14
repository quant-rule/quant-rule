//! The gateway's Postgres profile repository.
//!
//! The pool and the client-library boundary live in `quantrule-runtime`. The
//! catalog repository moved to `quantrule-data-service` in P3; the run and
//! hypothesis repositories moved to `quantrule-evidence-service` in P5. The
//! gateway owns only `gateway.profiles`.

use quantrule_api::ProfileId;
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

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
