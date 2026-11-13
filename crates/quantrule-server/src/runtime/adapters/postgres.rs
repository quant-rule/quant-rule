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
        profile: quantrule_api::ProfileId,
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
        profile: quantrule_api::ProfileId,
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

    fn count(
        &self,
        profile: quantrule_api::ProfileId,
    ) -> BoxFuture<Result<u64, crate::data::CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: (i64,) =
                sqlx::query_as("SELECT count(*) FROM catalog.datasets WHERE profile_id = $1")
                    .bind(profile.as_str())
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| crate::data::CatalogError::Store(error.to_string()))?;
            dataset_count(row.0, "dataset count")
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
        profile_id: quantrule_api::ProfileId::parse(&row.0).map_err(|error| {
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
                "INSERT INTO catalog.profiles (profile_id, name, created_at)
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
        profile_id: quantrule_api::ProfileId,
    ) -> BoxFuture<Result<Option<crate::profile::ProfileRecord>, crate::profile::ProfileError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<ProfileRow> = sqlx::query_as(
                "SELECT profile_id, name,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.profiles WHERE profile_id = $1",
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
                 FROM catalog.profiles ORDER BY profile_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::profile::ProfileError::Store(error.to_string()))?;
            rows.into_iter().map(profile_record).collect()
        })
    }
}

/// A configuration catalog backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgConfigStore {
    pool: PgPool,
}

impl PgConfigStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

type ConfigRow = (
    String,
    String,
    String,
    bool,
    Option<String>,
    Option<String>,
    String,
    String,
);

fn config_origin(
    value: &str,
) -> Result<quantrule_api::ConfigOrigin, crate::configs::ConfigStoreError> {
    match value {
        "repository" => Ok(quantrule_api::ConfigOrigin::Repository),
        "upload" => Ok(quantrule_api::ConfigOrigin::Upload),
        other => Err(crate::configs::ConfigStoreError(format!(
            "a stored config has an unknown origin: {other}"
        ))),
    }
}

fn config_record(
    row: ConfigRow,
) -> Result<crate::configs::ConfigRecord, crate::configs::ConfigStoreError> {
    let config_hash = row
        .5
        .map(|value| quantrule_api::Digest::parse(&value))
        .transpose()
        .map_err(|error| {
            crate::configs::ConfigStoreError(format!("a stored config hash is malformed: {error}"))
        })?;
    Ok(crate::configs::ConfigRecord {
        config_id: row.0,
        name: row.1,
        origin: config_origin(&row.2)?,
        valid: row.3,
        message: row.4,
        config_hash,
        body: row.6,
        created_at: row.7,
    })
}

impl crate::configs::ConfigCatalog for PgConfigStore {
    fn upsert(
        &self,
        record: crate::configs::ConfigRecord,
    ) -> BoxFuture<Result<(), crate::configs::ConfigStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let origin = match record.origin {
                quantrule_api::ConfigOrigin::Repository => "repository",
                quantrule_api::ConfigOrigin::Upload => "upload",
            };
            sqlx::query(
                "INSERT INTO catalog.configs
                     (config_id, name, origin, valid, message, config_hash, body, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz)
                 ON CONFLICT (config_id) DO UPDATE SET
                     name = EXCLUDED.name,
                     origin = EXCLUDED.origin,
                     valid = EXCLUDED.valid,
                     message = EXCLUDED.message,
                     config_hash = EXCLUDED.config_hash,
                     body = EXCLUDED.body",
            )
            .bind(&record.config_id)
            .bind(&record.name)
            .bind(origin)
            .bind(record.valid)
            .bind(&record.message)
            .bind(record.config_hash.map(|digest| digest.to_wire()))
            .bind(&record.body)
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map(|_| ())
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))
        })
    }

    fn list(
        &self,
    ) -> BoxFuture<Result<Vec<crate::configs::ConfigRecord>, crate::configs::ConfigStoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<ConfigRow> = sqlx::query_as(
                "SELECT config_id, name, origin, valid, message, config_hash, body,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.configs ORDER BY config_id",
            )
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            rows.into_iter().map(config_record).collect()
        })
    }

    fn get(
        &self,
        config_id: String,
    ) -> BoxFuture<Result<Option<crate::configs::ConfigRecord>, crate::configs::ConfigStoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<ConfigRow> = sqlx::query_as(
                "SELECT config_id, name, origin, valid, message, config_hash, body,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.configs WHERE config_id = $1",
            )
            .bind(&config_id)
            .fetch_optional(&pool)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            row.map(config_record).transpose()
        })
    }
}

/// A run store backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgRunStore {
    pool: PgPool,
}

impl PgRunStore {
    /// Build the store over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

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

fn run_state(value: &str) -> Result<quantrule_api::RunState, crate::runs::RunStoreError> {
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

async fn load_run(
    pool: &PgPool,
    profile: &str,
    run_id: &str,
) -> Result<Option<crate::runs::RunRecord>, crate::runs::RunStoreError> {
    let row: Option<RunRow> = sqlx::query_as(
        "SELECT r.run_id, r.status, r.owner, r.config_id, r.data_version,
                r.config_hash, r.model_version, r.image_digest,
                to_char(r.submitted_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                r.digest, r.error
         FROM runs.runs r
         JOIN runs.submissions s ON s.run_id = r.run_id
         WHERE s.profile_id = $1 AND r.run_id = $2",
    )
    .bind(profile)
    .bind(run_id)
    .fetch_optional(pool)
    .await
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
        error: row.10,
    }))
}

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
                      config_hash, model_version, image_digest)
                 VALUES ($1, $2, $3, $4, 'queued', $5::timestamptz, $6, $7, $8)
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
                    })
                })
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
            sqlx::query(
                "UPDATE runs.runs SET status = 'cancelled', finished_at = now() WHERE run_id = $1",
            )
            .bind(&run_id)
            .execute(&pool)
            .await
            .map_err(|error| crate::runs::RunStoreError(error.to_string()))?;
            let record = load_run(&pool, profile.as_str(), &run_id)
                .await?
                .ok_or_else(|| {
                    crate::runs::RunStoreError("the run vanished after cancel".to_owned())
                })?;
            Ok(crate::runs::CancelOutcome::Cancelled(record))
        })
    }
}
