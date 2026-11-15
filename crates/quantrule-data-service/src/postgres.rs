//! The data service's Postgres repositories.
//!
//! The pool and the client-library boundary live in `quantrule-runtime`. These
//! rows belong to the `catalog` schema, and the data service is their only
//! writer and reader. A marker keeps the SQL in one module so a second backend
//! implements the same trait operations rather than a guessed subset.

use quantrule_api::{ConfigOrigin, ProfileId};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use crate::catalog::CatalogError;
use crate::configs::{ConfigCatalog, ConfigRecord, ConfigStoreError};
use crate::rows::{ConfigRow, DatasetRow, config_record, dataset_count, dataset_record};

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

impl crate::catalog::DatasetCatalog for PgDatasetCatalog {
    fn insert(
        &self,
        profile: ProfileId,
        record: quantrule_api::DatasetRecord,
    ) -> BoxFuture<Result<(), CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let summary = &record.summary;
            let candles = i64::try_from(summary.candles)
                .map_err(|_| CatalogError::Store("the candle count does not fit".to_owned()))?;
            let first_bar = i64::try_from(summary.first_bar)
                .map_err(|_| CatalogError::Store("the first bar does not fit".to_owned()))?;
            let last_bar = i64::try_from(summary.last_bar)
                .map_err(|_| CatalogError::Store("the last bar does not fit".to_owned()))?;
            let bytes = i64::try_from(summary.bytes)
                .map_err(|_| CatalogError::Store("the byte count does not fit".to_owned()))?;
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
                    .map_err(|error| CatalogError::Store(error.to_string()))?,
            )
            .bind(&record.created_at)
            .execute(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if inserted.rows_affected() == 0 {
                return Err(CatalogError::AlreadyExists);
            }
            Ok(())
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<quantrule_api::DatasetRecord>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        quality::text
                 FROM catalog.datasets
                 WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(version)
            .fetch_optional(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            row.map(dataset_record).transpose()
        })
    }

    fn list(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<quantrule_api::DatasetRecord>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        quality::text
                 FROM catalog.datasets
                 WHERE profile_id = $1 ORDER BY data_version",
            )
            .bind(profile.as_str())
            .fetch_all(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            rows.into_iter().map(dataset_record).collect()
        })
    }

    fn count(&self, profile: ProfileId) -> BoxFuture<Result<u64, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: (i64,) =
                sqlx::query_as("SELECT count(*) FROM catalog.datasets WHERE profile_id = $1")
                    .bind(profile.as_str())
                    .fetch_one(&pool)
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
            dataset_count(row.0, "dataset count")
        })
    }
}

/// A configuration catalog backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgConfigStore {
    pool: PgPool,
}

impl PgConfigStore {
    /// Build the catalog over an existing pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ConfigCatalog for PgConfigStore {
    fn upsert(
        &self,
        profile: ProfileId,
        record: ConfigRecord,
    ) -> BoxFuture<Result<(), ConfigStoreError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let origin = match record.origin {
                ConfigOrigin::Repository => "repository",
                ConfigOrigin::Upload => "upload",
            };
            let files = serde_json::to_string(&record.files)
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            sqlx::query(
                "INSERT INTO catalog.config_bundles (config_id, files) VALUES ($1, $2::jsonb)
                 ON CONFLICT (config_id) DO UPDATE SET files = EXCLUDED.files",
            )
            .bind(&record.config_id)
            .bind(files)
            .execute(&mut *transaction)
            .await
            .map_err(|error| ConfigStoreError(error.to_string()))?;
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
            .map_err(|error| ConfigStoreError(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            Ok(())
        })
    }

    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>> {
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
            .map_err(|error| ConfigStoreError(error.to_string()))?;
            rows.into_iter().map(config_record).collect()
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        config_id: String,
    ) -> BoxFuture<Result<Option<ConfigRecord>, ConfigStoreError>> {
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
            .map_err(|error| ConfigStoreError(error.to_string()))?;
            row.map(config_record).transpose()
        })
    }
}
