//! The Postgres adapter.
//!
//! The only module allowed to name the `sqlx` client. A service asks the trait.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::runtime::adapters::{Backend, BoxFuture, Database, DatabaseError};
use crate::runtime::migrations::{self, Applied, MigrationError};

/// A Postgres handle.
#[derive(Debug, Clone)]
pub struct PgDatabase {
    pool: PgPool,
}

impl PgDatabase {
    /// Connect with a small pool.
    ///
    /// # Errors
    ///
    /// Returns the driver message when the server is unreachable.
    pub async fn connect(url: &str) -> Result<Self, DatabaseError> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(url)
            .await
            .map_err(|error| DatabaseError(error.to_string()))?;
        Ok(Self { pool })
    }

    /// The pool, for a module that owns a repository.
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl Database for PgDatabase {
    fn backend(&self) -> Backend {
        Backend::Postgres
    }

    fn ping(&self) -> BoxFuture<Result<(), DatabaseError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            sqlx::query("SELECT 1")
                .execute(&pool)
                .await
                .map(|_| ())
                .map_err(|error| DatabaseError(error.to_string()))
        })
    }
}

/// The advisory lock key that serializes migration runs.
const MIGRATION_LOCK: i64 = 0x5152_4d49_4752_4154;

/// Apply every pending migration under an advisory lock.
///
/// # Errors
///
/// Returns [`MigrationError`] when the plan is refused or a statement fails.
pub async fn apply_migrations(pool: &PgPool) -> Result<usize, MigrationError> {
    sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(MIGRATION_LOCK)
        .execute(pool)
        .await
        .map_err(|error| MigrationError::Sql(error.to_string()))?;

    let result = apply_locked(pool).await;

    let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(MIGRATION_LOCK)
        .execute(pool)
        .await;
    result
}

async fn apply_locked(pool: &PgPool) -> Result<usize, MigrationError> {
    let failed = |error: sqlx::Error| MigrationError::Sql(error.to_string());
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS public.schema_migrations (
             version    bigint PRIMARY KEY,
             name       text NOT NULL,
             checksum   text NOT NULL,
             applied_at timestamptz NOT NULL DEFAULT now()
         )",
    )
    .execute(pool)
    .await
    .map_err(failed)?;

    let rows: Vec<(i64, String, String)> = sqlx::query_as(
        "SELECT version, name, checksum FROM public.schema_migrations ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .map_err(failed)?;
    let applied: Vec<Applied> = rows
        .into_iter()
        .map(|(version, name, checksum)| Applied {
            version,
            name,
            checksum,
        })
        .collect();

    let available = migrations::available();
    let pending = migrations::plan(&applied, &available)?;

    for migration in &pending {
        let mut transaction = pool.begin().await.map_err(failed)?;
        sqlx::raw_sql(migration.sql)
            .execute(&mut *transaction)
            .await
            .map_err(failed)?;
        sqlx::query(
            "INSERT INTO public.schema_migrations (version, name, checksum) VALUES ($1, $2, $3)",
        )
        .bind(migration.version)
        .bind(migration.name)
        .bind(migrations::checksum(migration.sql))
        .execute(&mut *transaction)
        .await
        .map_err(failed)?;
        transaction.commit().await.map_err(failed)?;
    }

    Ok(pending.len())
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
                     (fingerprint, premise, rule, dataset, costs, split, criteria, sealed_at, body)
                 VALUES ($1, $2, $3, $4, $5::jsonb, $6::jsonb, $7, $8::timestamptz, $9::jsonb)",
            )
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
        fingerprint: String,
    ) -> BoxFuture<Result<Option<quantrule_research::SealedHypothesis>, crate::research::StoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT body::text FROM research.hypotheses WHERE fingerprint = $1")
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
