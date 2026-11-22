//! The data service's Postgres repositories.
//!
//! The pool and the client-library boundary live in `quantrule-runtime`. These
//! rows belong to the `catalog` schema, and the data service is their only
//! writer and reader. A marker keeps the SQL in one module so a second backend
//! implements the same trait operations rather than a guessed subset.

use quantrule_api::{ConfigOrigin, DatasetReference, DatasetReferenceKind, ProfileId};
use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use crate::catalog::{CatalogError, DatasetInsert, ObjectGuard};
use crate::configs::{ConfigCatalog, ConfigRecord, ConfigStoreError};
use crate::rows::{ConfigRow, DatasetRow, config_record, dataset_count, dataset_record};

const INSERT_DATASET_SQL: &str = "INSERT INTO catalog.datasets
         (profile_id, data_version, name, candles, first_bar, last_bar, bytes,
          object_key, sha256, quality, created_at, archived_at)
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::timestamptz,
             $12::timestamptz)
     ON CONFLICT (profile_id, data_version) DO NOTHING
     RETURNING data_version, name, candles, first_bar, last_bar, bytes, object_key,
               sha256,
               to_char(created_at AT TIME ZONE 'UTC',
                       'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
               quality::text,
               to_char(archived_at AT TIME ZONE 'UTC',
                       'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')";

const SELECT_DATASET_SQL: &str = "SELECT data_version, name, candles, first_bar, last_bar, bytes,
            object_key, sha256,
            to_char(created_at AT TIME ZONE 'UTC',
                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
            quality::text,
            to_char(archived_at AT TIME ZONE 'UTC',
                    'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
       FROM catalog.datasets
      WHERE profile_id = $1 AND data_version = $2";

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

/// Insert a row and return it, or `None` when the content already exists.
async fn insert_dataset_row(
    pool: &PgPool,
    profile: &ProfileId,
    record: &quantrule_api::DatasetRecord,
) -> Result<Option<DatasetRow>, CatalogError> {
    let summary = &record.summary;
    let candles = i64::try_from(summary.candles)
        .map_err(|_| CatalogError::Store("the candle count does not fit".to_owned()))?;
    let first_bar = i64::try_from(summary.first_bar)
        .map_err(|_| CatalogError::Store("the first bar does not fit".to_owned()))?;
    let last_bar = i64::try_from(summary.last_bar)
        .map_err(|_| CatalogError::Store("the last bar does not fit".to_owned()))?;
    let bytes = i64::try_from(summary.bytes)
        .map_err(|_| CatalogError::Store("the byte count does not fit".to_owned()))?;
    sqlx::query_as(INSERT_DATASET_SQL)
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
        .bind(&record.summary.archived_at)
        .fetch_optional(pool)
        .await
        .map_err(|error| CatalogError::Store(error.to_string()))
}

/// Read one row for a profile.
async fn load_dataset_row(
    pool: &PgPool,
    profile: &ProfileId,
    version: &str,
) -> Result<Option<DatasetRow>, CatalogError> {
    sqlx::query_as(SELECT_DATASET_SQL)
        .bind(profile.as_str())
        .bind(version)
        .fetch_optional(pool)
        .await
        .map_err(|error| CatalogError::Store(error.to_string()))
}

/// A transaction-level advisory lock for one object key.
struct PgObjectGuard {
    transaction: Option<sqlx::Transaction<'static, sqlx::Postgres>>,
}

impl ObjectGuard for PgObjectGuard {
    fn release(mut self: Box<Self>) -> BoxFuture<Result<(), CatalogError>> {
        Box::pin(async move {
            if let Some(transaction) = self.transaction.take() {
                transaction
                    .commit()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
            }
            Ok(())
        })
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
            if insert_dataset_row(&pool, &profile, &record)
                .await?
                .is_none()
            {
                return Err(CatalogError::AlreadyExists);
            }
            Ok(())
        })
    }

    fn insert_or_get(
        &self,
        profile: ProfileId,
        record: quantrule_api::DatasetRecord,
    ) -> BoxFuture<Result<DatasetInsert, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let version = record.summary.data_version.to_string();
            // The conflict branch waits for the winning transaction to commit,
            // so the follow-up read sees it. One retry covers an erase that
            // removed the row in that instant.
            for _ in 0..2 {
                if let Some(row) = insert_dataset_row(&pool, &profile, &record).await? {
                    return Ok(DatasetInsert {
                        record: dataset_record(row)?,
                        created: true,
                    });
                }
                if let Some(row) = load_dataset_row(&pool, &profile, &version).await? {
                    return Ok(DatasetInsert {
                        record: dataset_record(row)?,
                        created: false,
                    });
                }
            }
            Err(CatalogError::Store(
                "the catalog refused a duplicate but holds no record".to_owned(),
            ))
        })
    }

    fn object_guard(
        &self,
        object_key: String,
    ) -> BoxFuture<Result<Box<dyn ObjectGuard>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
                .bind(&object_key)
                .execute(&mut *transaction)
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            Ok(Box::new(PgObjectGuard {
                transaction: Some(transaction),
            }) as Box<dyn ObjectGuard>)
        })
    }

    fn object_referenced(&self, object_key: String) -> BoxFuture<Result<bool, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: (bool,) = sqlx::query_as(
                "SELECT EXISTS (
                     SELECT 1 FROM catalog.datasets WHERE object_key = $1
                 )",
            )
            .bind(&object_key)
            .fetch_one(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            Ok(row.0)
        })
    }

    fn get(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Option<quantrule_api::DatasetRecord>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            load_dataset_row(&pool, &profile, &version)
                .await?
                .map(dataset_record)
                .transpose()
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
                        quality::text,
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.datasets
                 WHERE profile_id = $1 AND archived_at IS NULL ORDER BY data_version",
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
            let row: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.datasets
                      WHERE profile_id = $1 AND archived_at IS NULL",
            )
            .bind(profile.as_str())
            .fetch_one(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            dataset_count(row.0, "dataset count")
        })
    }

    fn list_including_archived(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<quantrule_api::DatasetRecord>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let rows: Vec<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        quality::text,
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
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

    fn archive(
        &self,
        profile: ProfileId,
        version: String,
        archived_at: String,
    ) -> BoxFuture<Result<quantrule_api::DatasetRecord, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let updated = sqlx::query(
                "UPDATE catalog.datasets
                    SET archived_at = $3::timestamptz
                  WHERE profile_id = $1 AND data_version = $2 AND archived_at IS NULL",
            )
            .bind(profile.as_str())
            .bind(&version)
            .bind(&archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if updated.rows_affected() == 0 {
                let exists: Option<(String,)> = sqlx::query_as(
                    "SELECT data_version FROM catalog.datasets
                      WHERE profile_id = $1 AND data_version = $2",
                )
                .bind(profile.as_str())
                .bind(&version)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(if exists.is_some() {
                    CatalogError::Archived
                } else {
                    CatalogError::NotFound
                });
            }
            let row: DatasetRow = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        quality::text,
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.datasets
                 WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            sqlx::query(
                "INSERT INTO catalog.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'dataset', $2, 'archive', 'operator.archive', $3)",
            )
            .bind(profile.as_str())
            .bind(&version)
            .bind(row.5)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            dataset_record(row)
        })
    }

    fn erase(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<crate::catalog::EraseOutcome, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let row: Option<DatasetRow> = sqlx::query_as(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"'),
                        quality::text,
                        to_char(archived_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.datasets
                 WHERE profile_id = $1 AND data_version = $2
                 FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            let Some(row) = row else {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(CatalogError::NotFound);
            };
            if row.10.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(CatalogError::NotArchived);
            }
            let references: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.dataset_references
                  WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if references.0 > 0 {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(CatalogError::Referenced(
                    u64::try_from(references.0).unwrap_or(u64::MAX),
                ));
            }
            sqlx::query(
                "DELETE FROM catalog.dataset_references
                  WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            sqlx::query("DELETE FROM catalog.datasets WHERE profile_id = $1 AND data_version = $2")
                .bind(profile.as_str())
                .bind(&version)
                .execute(&mut *transaction)
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let shared: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM catalog.datasets
                  WHERE object_key = $1
                    AND NOT (profile_id = $2 AND data_version = $3)
                  LIMIT 1",
            )
            .bind(&row.6)
            .bind(profile.as_str())
            .bind(&version)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            sqlx::query(
                "INSERT INTO catalog.retention_events
                     (profile_id, kind, record_id, action, reason, bytes)
                 VALUES ($1, 'dataset', $2, 'erase', 'operator.erase', $3)",
            )
            .bind(profile.as_str())
            .bind(&version)
            .bind(row.5)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            Ok(crate::catalog::EraseOutcome {
                record: dataset_record(row)?,
                object_shared: shared.is_some(),
            })
        })
    }

    fn add_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference: DatasetReference,
    ) -> BoxFuture<Result<u64, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT data_version FROM catalog.datasets
                  WHERE profile_id = $1 AND data_version = $2
                  FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if exists.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(CatalogError::NotFound);
            }
            sqlx::query(
                "INSERT INTO catalog.dataset_references
                     (profile_id, data_version, reference_id, kind, created_at)
                 VALUES ($1, $2, $3, $4, $5::timestamptz)
                 ON CONFLICT (profile_id, data_version, reference_id) DO NOTHING",
            )
            .bind(profile.as_str())
            .bind(&version)
            .bind(&reference.reference_id)
            .bind(reference.kind.as_str())
            .bind(&reference.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            let count: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.dataset_references
                  WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            Ok(u64::try_from(count.0).unwrap_or(u64::MAX))
        })
    }

    fn release_reference(
        &self,
        profile: ProfileId,
        version: String,
        reference_id: String,
    ) -> BoxFuture<Result<u64, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let mut transaction = pool
                .begin()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT data_version FROM catalog.datasets
                  WHERE profile_id = $1 AND data_version = $2
                  FOR UPDATE",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if exists.is_none() {
                transaction
                    .rollback()
                    .await
                    .map_err(|error| CatalogError::Store(error.to_string()))?;
                return Err(CatalogError::NotFound);
            }
            sqlx::query(
                "DELETE FROM catalog.dataset_references
                  WHERE profile_id = $1 AND data_version = $2 AND reference_id = $3",
            )
            .bind(profile.as_str())
            .bind(&version)
            .bind(&reference_id)
            .execute(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            let count: (i64,) = sqlx::query_as(
                "SELECT count(*) FROM catalog.dataset_references
                  WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_one(&mut *transaction)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            transaction
                .commit()
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            Ok(u64::try_from(count.0).unwrap_or(u64::MAX))
        })
    }

    fn references(
        &self,
        profile: ProfileId,
        version: String,
    ) -> BoxFuture<Result<Vec<DatasetReference>, CatalogError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT data_version FROM catalog.datasets
                  WHERE profile_id = $1 AND data_version = $2",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_optional(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            if exists.is_none() {
                return Err(CatalogError::NotFound);
            }
            let rows: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT reference_id, kind,
                        to_char(created_at AT TIME ZONE 'UTC', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')
                 FROM catalog.dataset_references
                 WHERE profile_id = $1 AND data_version = $2
                 ORDER BY reference_id",
            )
            .bind(profile.as_str())
            .bind(&version)
            .fetch_all(&pool)
            .await
            .map_err(|error| CatalogError::Store(error.to_string()))?;
            rows.into_iter()
                .map(|(reference_id, kind, created_at)| {
                    let kind = DatasetReferenceKind::parse(&kind).ok_or_else(|| {
                        CatalogError::Store(format!(
                            "a stored dataset reference has an unknown kind: {kind}"
                        ))
                    })?;
                    Ok(DatasetReference {
                        reference_id,
                        kind,
                        created_at,
                    })
                })
                .collect()
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
