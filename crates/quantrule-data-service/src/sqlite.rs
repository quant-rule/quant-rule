//! The data service's SQLite repositories.
//!
//! The embedded desktop profile runs the same catalog routes against one SQLite
//! file. SQLite has no schemas, so `catalog.datasets` is `catalog__datasets`;
//! a JSON document is `TEXT`, an instant is RFC 3339 UTC `TEXT`, and a boolean
//! is `0` or `1`. The row decoders are shared with the Postgres repository in
//! the crate's `rows` module, so the two backends cannot drift apart silently.
//!
//! `rusqlite` is synchronous. Every operation runs on the runtime's blocking
//! pool through [`SqliteDatabase::call`], so the async trait shape and the
//! routes above it are unchanged. A config upsert writes the shared bundle and
//! the profile-scoped row in one immediate transaction, because a row without
//! its files is not a readable config.
//!
//! The tables live in the `catalog` ledger of the shared `schema_migrations`
//! table. This module names only its own prefix; the baseline is
//! `migrations/sqlite/0001_catalog.sql`.

use quantrule_api::ProfileId;
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{OptionalExtension, TransactionBehavior, params};

use crate::catalog::CatalogError;
use crate::configs::{ConfigCatalog, ConfigRecord, ConfigStoreError};
use crate::rows::{ConfigRow, DatasetRow, config_record, dataset_count, dataset_record};

/// The dataset table, from the shared `Schema` mapping.
const DATASETS_TABLE: &str = "catalog__datasets";

/// The profile-scoped config row table.
const CONFIGS_TABLE: &str = "catalog__configs";

/// The config file bundle, shared by content id across profiles.
const BUNDLES_TABLE: &str = "catalog__config_bundles";

/// A dataset catalog backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteDatasetCatalog {
    database: SqliteDatabase,
}

impl SqliteDatasetCatalog {
    /// Build the catalog over an open SQLite database.
    #[must_use]
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }
}

impl crate::catalog::DatasetCatalog for SqliteDatasetCatalog {
    fn insert(
        &self,
        profile: ProfileId,
        record: quantrule_api::DatasetRecord,
    ) -> BoxFuture<Result<(), CatalogError>> {
        let database = self.database.clone();
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
            let quality = record
                .quality
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            let profile_id = profile.as_str().to_owned();
            let data_version = summary.data_version.to_string();
            let name = summary.name.clone();
            let object_key = record.object_key;
            let sha256 = record.sha256.as_hex().to_owned();
            let created_at = record.created_at;
            let sql = format!(
                "INSERT INTO {DATASETS_TABLE}
                     (profile_id, data_version, name, candles, first_bar, last_bar, bytes,
                      object_key, sha256, quality, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                 ON CONFLICT (profile_id, data_version) DO NOTHING"
            );
            let inserted = database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![
                            profile_id,
                            data_version,
                            name,
                            candles,
                            first_bar,
                            last_bar,
                            bytes,
                            object_key,
                            sha256,
                            quality,
                            created_at,
                        ],
                    )
                })
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            if inserted == 0 {
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
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256, created_at, quality
                   FROM {DATASETS_TABLE}
                  WHERE profile_id = ?1 AND data_version = ?2"
            );
            let profile_id = profile.as_str().to_owned();
            let row: Option<DatasetRow> = database
                .call(move |connection| {
                    connection
                        .query_row(&sql, params![profile_id, version], |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                                row.get(5)?,
                                row.get(6)?,
                                row.get(7)?,
                                row.get(8)?,
                                row.get(9)?,
                            ))
                        })
                        .optional()
                })
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            row.map(dataset_record).transpose()
        })
    }

    fn list(
        &self,
        profile: ProfileId,
    ) -> BoxFuture<Result<Vec<quantrule_api::DatasetRecord>, CatalogError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT data_version, name, candles, first_bar, last_bar, bytes, object_key,
                        sha256, created_at, quality
                   FROM {DATASETS_TABLE}
                  WHERE profile_id = ?1
                  ORDER BY data_version"
            );
            let profile_id = profile.as_str().to_owned();
            let rows: Vec<DatasetRow> = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![profile_id], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                            row.get(9)?,
                        ))
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            rows.into_iter().map(dataset_record).collect()
        })
    }

    fn count(&self, profile: ProfileId) -> BoxFuture<Result<u64, CatalogError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!("SELECT count(*) FROM {DATASETS_TABLE} WHERE profile_id = ?1");
            let profile_id = profile.as_str().to_owned();
            let count: i64 = database
                .call(move |connection| {
                    connection.query_row(&sql, params![profile_id], |row| row.get(0))
                })
                .await
                .map_err(|error| CatalogError::Store(error.to_string()))?;
            dataset_count(count, "dataset count")
        })
    }
}

/// A configuration catalog backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteConfigStore {
    database: SqliteDatabase,
}

impl SqliteConfigStore {
    /// Build the catalog over an open SQLite database.
    #[must_use]
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }
}

impl ConfigCatalog for SqliteConfigStore {
    fn upsert(
        &self,
        profile: ProfileId,
        record: ConfigRecord,
    ) -> BoxFuture<Result<(), ConfigStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let origin = match record.origin {
                quantrule_api::ConfigOrigin::Repository => "repository",
                quantrule_api::ConfigOrigin::Upload => "upload",
            };
            let files = serde_json::to_string(&record.files)
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            let profile_id = profile.as_str().to_owned();
            let config_id = record.config_id;
            let name = record.name;
            let valid = record.valid;
            let message = record.message;
            let config_hash = record.config_hash.map(|digest| digest.to_wire());
            let body = record.body;
            let created_at = record.created_at;
            let bundle_sql = format!(
                "INSERT INTO {BUNDLES_TABLE} (config_id, files) VALUES (?1, ?2)
                 ON CONFLICT (config_id) DO UPDATE SET files = excluded.files"
            );
            let row_sql = format!(
                "INSERT INTO {CONFIGS_TABLE}
                     (profile_id, config_id, name, origin, valid, message, config_hash, body,
                      created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT (profile_id, config_id) DO UPDATE SET
                     name = excluded.name,
                     origin = excluded.origin,
                     valid = excluded.valid,
                     message = excluded.message,
                     config_hash = excluded.config_hash,
                     body = excluded.body"
            );
            database
                .call(move |connection| {
                    // The bundle and the row are one logical write: a row whose
                    // files were never stored is not a readable config.
                    let transaction =
                        connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
                    transaction.execute(&bundle_sql, params![config_id, files])?;
                    transaction.execute(
                        &row_sql,
                        params![
                            profile_id,
                            config_id,
                            name,
                            origin,
                            valid,
                            message,
                            config_hash,
                            body,
                            created_at,
                        ],
                    )?;
                    transaction.commit()
                })
                .await
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            Ok(())
        })
    }

    fn list(&self, profile: ProfileId) -> BoxFuture<Result<Vec<ConfigRecord>, ConfigStoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT c.config_id, c.name, c.origin, c.valid, c.message, c.config_hash, c.body,
                        b.files, c.created_at
                   FROM {CONFIGS_TABLE} c
                   JOIN {BUNDLES_TABLE} b ON b.config_id = c.config_id
                  WHERE c.profile_id = ?1
                  ORDER BY c.config_id"
            );
            let profile_id = profile.as_str().to_owned();
            let rows: Vec<ConfigRow> = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement.query_map(params![profile_id], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                            row.get(8)?,
                        ))
                    })?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
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
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT c.config_id, c.name, c.origin, c.valid, c.message, c.config_hash, c.body,
                        b.files, c.created_at
                   FROM {CONFIGS_TABLE} c
                   JOIN {BUNDLES_TABLE} b ON b.config_id = c.config_id
                  WHERE c.profile_id = ?1 AND c.config_id = ?2"
            );
            let profile_id = profile.as_str().to_owned();
            let row: Option<ConfigRow> = database
                .call(move |connection| {
                    connection
                        .query_row(&sql, params![profile_id, config_id], |row| {
                            Ok((
                                row.get(0)?,
                                row.get(1)?,
                                row.get(2)?,
                                row.get(3)?,
                                row.get(4)?,
                                row.get(5)?,
                                row.get(6)?,
                                row.get(7)?,
                                row.get(8)?,
                            ))
                        })
                        .optional()
                })
                .await
                .map_err(|error| ConfigStoreError(error.to_string()))?;
            row.map(config_record).transpose()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{BUNDLES_TABLE, CONFIGS_TABLE, DATASETS_TABLE};
    use quantrule_runtime::adapters::{Dialect, Schema};

    #[test]
    fn every_table_is_the_catalog_prefix() {
        for (table, name) in [
            (DATASETS_TABLE, "datasets"),
            (CONFIGS_TABLE, "configs"),
            (BUNDLES_TABLE, "config_bundles"),
        ] {
            assert_eq!(
                table,
                Dialect::Sqlite
                    .table(Schema::Catalog, name)
                    .expect("the SQLite name")
            );
            assert_eq!(
                Dialect::Postgres
                    .table(Schema::Catalog, name)
                    .expect("the Postgres name"),
                format!("catalog.{name}")
            );
        }
    }
}
