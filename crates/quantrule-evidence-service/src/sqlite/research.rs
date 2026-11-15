//! The SQLite hypothesis repository.
//!
//! A sealed hypothesis is written once and read back by fingerprint within one
//! profile. The split columns stay for inspection, and the whole sealed record
//! is stored as JSON so a reader reconstructs exactly what was written. The
//! SQLite baseline carries a `BEFORE UPDATE OR DELETE` trigger, so the same
//! immutability rule the Postgres trigger enforces holds on the embedded
//! profile as well.

use quantrule_api::ProfileId;
use quantrule_research::SealedHypothesis;
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{OptionalExtension, params};

use super::HYPOTHESES_TABLE;
use crate::research::{HypothesisStore, StoreError};

/// A hypothesis store backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteHypothesisStore {
    database: SqliteDatabase,
}

impl SqliteHypothesisStore {
    /// Build the store over an open SQLite database.
    #[must_use]
    pub fn new(database: SqliteDatabase) -> Self {
        Self { database }
    }

    /// The database the store reads and writes.
    #[must_use]
    pub fn database(&self) -> &SqliteDatabase {
        &self.database
    }
}

impl HypothesisStore for SqliteHypothesisStore {
    fn insert(
        &self,
        profile: ProfileId,
        sealed: SealedHypothesis,
    ) -> BoxFuture<Result<(), StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let fingerprint = sealed.fingerprint().to_owned();
            let sealed_at = sealed.sealed_at().to_owned();
            let body =
                serde_json::to_string(&sealed).map_err(|error| StoreError(error.to_string()))?;
            let hypothesis = sealed.hypothesis();
            let costs = serde_json::to_string(&hypothesis.costs)
                .map_err(|error| StoreError(error.to_string()))?;
            let split = serde_json::to_string(&hypothesis.split)
                .map_err(|error| StoreError(error.to_string()))?;
            let premise = hypothesis.premise.clone();
            let rule = hypothesis.rule.clone();
            let dataset = hypothesis.dataset.clone();
            let criteria = hypothesis.criteria.clone();
            let sql = format!(
                "INSERT INTO {HYPOTHESES_TABLE}
                     (profile_id, fingerprint, premise, rule, dataset, costs, split,
                      criteria, sealed_at, body)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
            );
            database
                .call(move |connection| {
                    connection.execute(
                        &sql,
                        params![
                            profile_id,
                            fingerprint,
                            premise,
                            rule,
                            dataset,
                            costs,
                            split,
                            criteria,
                            sealed_at,
                            body
                        ],
                    )
                })
                .await
                .map(|_| ())
                .map_err(|error| StoreError(error.to_string()))
        })
    }

    fn load(
        &self,
        profile: ProfileId,
        fingerprint: String,
    ) -> BoxFuture<Result<Option<SealedHypothesis>, StoreError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = profile.as_str().to_owned();
            let sql = format!(
                "SELECT body FROM {HYPOTHESES_TABLE}
                  WHERE profile_id = ?1 AND fingerprint = ?2"
            );
            let body: Option<String> = database
                .call(move |connection| {
                    connection
                        .query_row(&sql, params![profile_id, fingerprint], |row| row.get(0))
                        .optional()
                })
                .await
                .map_err(|error| StoreError(error.to_string()))?;
            match body {
                Some(body) => serde_json::from_str(&body)
                    .map(Some)
                    .map_err(|error| StoreError(error.to_string())),
                None => Ok(None),
            }
        })
    }
}
