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

/// A dataset catalog backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgDatasetCatalog {
    pool: PgPool,
}

impl PgDatasetCatalog {
    /// Build the catalog over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type DatasetRow = (String, String, i64, i64, i64, i64, String, String, String);

fn dataset_count(value: i64, field: &str) -> Result<u64, crate::data::CatalogError> {
    u64::try_from(value).map_err(|_| {
        crate::data::CatalogError::Store(format!("the catalog holds a negative {field}"))
    })
}

fn dataset_record(
    row: DatasetRow,
) -> Result<quantrule_api::DatasetRecord, crate::data::CatalogError> {
    let summary = quantrule_api::DatasetSummary {
        data_version: quantrule_api::Digest::parse(&row.0).map_err(|error| {
            crate::data::CatalogError::Store(format!("a stored version is malformed: {error}"))
        })?,
        name: row.1,
        candles: dataset_count(row.2, "candle count")?,
        first_bar: dataset_count(row.3, "first bar")?,
        last_bar: dataset_count(row.4, "last bar")?,
        bytes: dataset_count(row.5, "byte count")?,
    };
    Ok(quantrule_api::DatasetRecord {
        summary,
        object_key: row.6,
        sha256: quantrule_api::Digest::parse(&row.7).map_err(|error| {
            crate::data::CatalogError::Store(format!("a stored hash is malformed: {error}"))
        })?,
        created_at: row.8,
    })
}

impl crate::data::DatasetCatalog for PgDatasetCatalog {
    fn insert(
        &self,
        profile: quantrule_api::ProfileId,
        record: quantrule_api::DatasetRecord,
    ) -> BoxFuture<Result<(), crate::data::CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let summary = &record.summary;
            let candles = i64::try_from(summary.candles).map_err(|_| {
                crate::data::CatalogError::Store("the candle count does not fit".to_owned())
            })?;
            let first_bar = i64::try_from(summary.first_bar).map_err(|_| {
                crate::data::CatalogError::Store("the first bar does not fit".to_owned())
            })?;
            let last_bar = i64::try_from(summary.last_bar).map_err(|_| {
                crate::data::CatalogError::Store("the last bar does not fit".to_owned())
            })?;
            let bytes = i64::try_from(summary.bytes).map_err(|_| {
                crate::data::CatalogError::Store("the byte count does not fit".to_owned())
            })?;
            let inserted = sqlx::query(
                "INSERT INTO catalog.datasets
                     (profile_id, data_version, name, candles, first_bar, last_bar, bytes,
                      object_key, sha256, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::timestamptz)
                 ON CONFLICT (profile_id, data_version) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(summary.data_version.to_string())
            .bind(&summary.name)
            .bind(candles)
            .bind(first_bar)
            .bind(last_bar)
            .bind(bytes)
            .bind(&record.object_key)
            .bind(record.sha256.as_hex())
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map_err(|error| crate::data::CatalogError::Store(error.to_string()))?;
            if inserted.rows_affected() == 0 {
                return Err(crate::data::CatalogError::AlreadyExists);
            }
            Ok(())
        })
    }

    fn get(
        &self,
        profile: quantrule_api::ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<quantrule_api::DatasetRecord>, crate::data::CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.datasets
                 WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(version)
            .fetch_optional(&pool)
            .await
            .map_err(|error| crate::data::CatalogError::Store(error.to_string()))?;
            row.map(dataset_record).transpose()
        })
    }

    fn list(
        &self,
        profile: quantrule_api::ProfileId,
    ) -> BoxFuture<Result<Vec<quantrule_api::DatasetRecord>, crate::data::CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.datasets
                 WHERE profile_id = $1 ORDER BY data_version",
            )
            .bind(profile.as_str())
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::data::CatalogError::Store(error.to_string()))?;
            rows.into_iter().map(dataset_record).collect()
        })
    }
}
