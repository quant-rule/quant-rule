//! The server's Postgres repositories.
//!
//! The pool and the client library boundary live in `quantrule-runtime`. These
//! rows belong to the modules that own them and move to their service crates in
//! P3 and P5; until then they stay together so the server has one place that
//! names SQL.

mod runs;

pub use runs::PgRunStore;

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

type DatasetRow = (
    String,
    String,
    i64,
    i64,
    i64,
    i64,
    String,
    String,
    String,
    Option<String>,
);

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
        quality: row
            .9
            .as_deref()
            .map(serde_json::from_str)
            .transpose()
            .map_err(|error| {
                crate::data::CatalogError::Store(format!(
                    "a stored quality report is malformed: {error}"
                ))
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
                      object_key, sha256, quality, created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::timestamptz)
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
            .bind(
                record
                    .quality
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .map_err(|error| crate::data::CatalogError::Store(error.to_string()))?,
            )
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
                        sha256, quality::text,
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
                        sha256, quality::text,
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
        files: serde_json::from_str(&row.7).map_err(|error| {
            crate::configs::ConfigStoreError(format!("stored config files are malformed: {error}"))
        })?,
        created_at: row.8,
    })
}

impl crate::configs::ConfigCatalog for PgConfigStore {
    fn upsert(
        &self,
        profile: quantrule_api::ProfileId,
        record: crate::configs::ConfigRecord,
    ) -> BoxFuture<Result<(), crate::configs::ConfigStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let origin = match record.origin {
                quantrule_api::ConfigOrigin::Repository => "repository",
                quantrule_api::ConfigOrigin::Upload => "upload",
            };
            let files = serde_json::to_string(&record.files)
                .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            sqlx::query(
                "INSERT INTO catalog.config_bundles (config_id, files) VALUES ($1, $2::jsonb)
                 ON CONFLICT (config_id) DO UPDATE SET files = EXCLUDED.files",
            )
            .bind(&record.config_id)
            .bind(files)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            sqlx::query(
                "INSERT INTO catalog.configs
                     (profile_id, config_id, name, origin, valid, message, config_hash, body,
                      created_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::timestamptz)
                 ON CONFLICT (profile_id, config_id) DO UPDATE SET
                     name = EXCLUDED.name,
                     origin = EXCLUDED.origin,
                     valid = EXCLUDED.valid,
                     message = EXCLUDED.message,
                     config_hash = EXCLUDED.config_hash,
                     body = EXCLUDED.body",
            )
            .bind(profile.as_str())
            .bind(&record.config_id)
            .bind(&record.name)
            .bind(origin)
            .bind(record.valid)
            .bind(&record.message)
            .bind(record.config_hash.map(|digest| digest.to_wire()))
            .bind(&record.body)
            .bind(&record.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            Ok(())
        })
    }

    fn list(
        &self,
        profile: quantrule_api::ProfileId,
    ) -> BoxFuture<Result<Vec<crate::configs::ConfigRecord>, crate::configs::ConfigStoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<ConfigRow> = sqlx::query_as(
                "SELECT c.config_id, c.name, c.origin, c.valid, c.message, c.config_hash, c.body,
                        b.files::text,
                        to_char(c.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.configs c
                 JOIN catalog.config_bundles b ON b.config_id = c.config_id
                 WHERE c.profile_id = $1 ORDER BY c.config_id",
            )
            .bind(profile.as_str())
            .fetch_all(&pool)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            rows.into_iter().map(config_record).collect()
        })
    }

    fn get(
        &self,
        profile: quantrule_api::ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<crate::configs::ConfigRecord>, crate::configs::ConfigStoreError>>
    {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<ConfigRow> = sqlx::query_as(
                "SELECT c.config_id, c.name, c.origin, c.valid, c.message, c.config_hash, c.body,
                        b.files::text,
                        to_char(c.created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.configs c
                 JOIN catalog.config_bundles b ON b.config_id = c.config_id
                 WHERE c.profile_id = $1 AND c.config_id = $2",
            )
            .bind(profile.as_str())
            .bind(&config_id)
            .fetch_optional(&pool)
            .await
            .map_err(|error| crate::configs::ConfigStoreError(error.to_string()))?;
            row.map(config_record).transpose()
        })
    }
}
