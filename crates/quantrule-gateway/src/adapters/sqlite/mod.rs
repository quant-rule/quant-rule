//! The gateway's SQLite profile repository.
//!
//! The store implements the existing [`crate::profile::ProfileStore`] trait; no
//! route or use case changes between the server and desktop profiles. Every
//! operation runs on the runtime's blocking pool through
//! [`SqliteDatabase::call`], so the async route shape stays the same.
//!
//! The table is `gateway__profiles`, the double-prefix spelling of the Postgres
//! `gateway.profiles` relation. The gateway names only its own prefix; the
//! shared ledger and the other service baselines live in the runtime and their
//! owning crates.

use quantrule_api::ProfileId;
use quantrule_runtime::adapters::BoxFuture;
use quantrule_runtime::adapters::sqlite::SqliteDatabase;
use rusqlite::{OptionalExtension, params};

use crate::profile::{ProfileError, ProfileRecord};

/// The gateway profile table, from the shared `Schema` mapping.
const PROFILES_TABLE: &str = "gateway__profiles";

/// A profile store backed by the embedded SQLite database.
#[derive(Debug, Clone)]
pub struct SqliteProfileStore {
    database: SqliteDatabase,
}

impl SqliteProfileStore {
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

type ProfileRow = (String, String, String);

fn profile_record(row: ProfileRow) -> Result<ProfileRecord, ProfileError> {
    Ok(ProfileRecord {
        profile_id: ProfileId::parse(&row.0).map_err(|error| {
            ProfileError::Store(format!("a stored profile id is malformed: {error}"))
        })?,
        name: row.1,
        created_at: row.2,
    })
}

impl crate::profile::ProfileStore for SqliteProfileStore {
    fn create(&self, record: ProfileRecord) -> BoxFuture<Result<(), ProfileError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let profile_id = record.profile_id.as_str().to_owned();
            let name = record.name;
            let created_at = record.created_at;
            let sql = format!(
                "INSERT INTO {PROFILES_TABLE} (profile_id, name, created_at)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT (profile_id) DO NOTHING"
            );
            let inserted = database
                .call(move |connection| {
                    connection.execute(&sql, params![profile_id, name, created_at])
                })
                .await
                .map_err(|error| ProfileError::Store(error.to_string()))?;
            if inserted == 0 {
                return Err(ProfileError::AlreadyExists);
            }
            Ok(())
        })
    }

    fn get(&self, profile_id: ProfileId) -> BoxFuture<Result<Option<ProfileRecord>, ProfileError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT profile_id, name, created_at
                   FROM {PROFILES_TABLE}
                  WHERE profile_id = ?1"
            );
            let row: Option<ProfileRow> = database
                .call(move |connection| {
                    connection
                        .query_row(&sql, params![profile_id.as_str()], |row| {
                            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                        })
                        .optional()
                })
                .await
                .map_err(|error| ProfileError::Store(error.to_string()))?;
            row.map(profile_record).transpose()
        })
    }

    fn list(&self) -> BoxFuture<Result<Vec<ProfileRecord>, ProfileError>> {
        let database = self.database.clone();
        Box::pin(async move {
            let sql = format!(
                "SELECT profile_id, name, created_at
                   FROM {PROFILES_TABLE}
                  ORDER BY profile_id"
            );
            let rows: Vec<ProfileRow> = database
                .call(move |connection| {
                    let mut statement = connection.prepare(&sql)?;
                    let rows = statement
                        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
                    rows.collect::<rusqlite::Result<Vec<_>>>()
                })
                .await
                .map_err(|error| ProfileError::Store(error.to_string()))?;
            rows.into_iter().map(profile_record).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::PROFILES_TABLE;
    use quantrule_runtime::adapters::{Dialect, Schema};

    #[test]
    fn the_table_is_the_gateway_prefix() {
        assert_eq!(
            PROFILES_TABLE,
            Dialect::Sqlite
                .table(Schema::Gateway, "profiles")
                .expect("the table name")
        );
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Gateway, "profiles")
                .expect("the Postgres name"),
            "gateway.profiles"
        );
    }
}
