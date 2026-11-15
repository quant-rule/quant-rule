//! The catalog SQLite baseline: table shape, stored encodings, and constraints.
//!
//! The fast tier has no Postgres. This suite proves the SQLite relation names,
//! the column set and mapped types, the TEXT/INTEGER spellings, the JSON and
//! boolean checks, and the decode error a malformed stored value produces. The
//! live method parity is `catalog_conformance.rs`; the opt-in Postgres case
//! there is the same body.

use std::collections::BTreeMap;

use quantrule_api::{
    ConfigOrigin, DatasetRecord, DatasetSummary, Digest, ProfileId, QualityReport,
};
use quantrule_data_service::migrations;
use quantrule_data_service::sqlite::{SqliteConfigStore, SqliteDatasetCatalog};
use quantrule_data_service::{ConfigCatalog, ConfigRecord, DatasetCatalog};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("the profile id")
}

fn dataset() -> DatasetRecord {
    let version = Digest::sha256(b"sqlite-catalog-row");
    DatasetRecord {
        summary: DatasetSummary {
            data_version: version.clone(),
            name: "candles.csv".to_owned(),
            candles: 10,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_260,
            bytes: 128,
        },
        object_key: format!("datasets/{}", version.as_hex()),
        sha256: version,
        quality: Some(QualityReport {
            rows_read: 2,
            accepted: 2,
            rejected: 0,
            duplicates: 0,
            out_of_order: 0,
            invalid: 0,
            skipped: Vec::new(),
        }),
        created_at: "2026-11-15T09:00:00Z".to_owned(),
    }
}

fn config() -> ConfigRecord {
    ConfigRecord {
        config_id: "sqlite_cfg".to_owned(),
        name: "sqlite/run.json".to_owned(),
        origin: ConfigOrigin::Repository,
        valid: true,
        message: None,
        config_hash: Some(Digest::sha256(b"sqlite-config")),
        body: "{\"run\":\"sqlite\"}".to_owned(),
        files: BTreeMap::from([("run.json".to_owned(), "{\"run\":\"sqlite\"}".to_owned())]),
        created_at: "2026-11-15T09:00:00Z".to_owned(),
    }
}

async fn migrated() -> SqliteDatabase {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the catalog baseline");
    database
}

async fn table_info(database: &SqliteDatabase, table: &'static str) -> Vec<(String, String, i64)> {
    database
        .call(move |connection| {
            let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the table shape")
}

#[tokio::test]
async fn the_baseline_tables_carry_the_final_catalog_columns_and_types() {
    let database = migrated().await;

    assert_eq!(
        table_info(&database, "catalog__datasets").await,
        vec![
            ("profile_id".to_owned(), "TEXT".to_owned(), 1),
            ("data_version".to_owned(), "TEXT".to_owned(), 1),
            ("name".to_owned(), "TEXT".to_owned(), 1),
            ("candles".to_owned(), "INTEGER".to_owned(), 1),
            ("first_bar".to_owned(), "INTEGER".to_owned(), 1),
            ("last_bar".to_owned(), "INTEGER".to_owned(), 1),
            ("bytes".to_owned(), "INTEGER".to_owned(), 1),
            ("object_key".to_owned(), "TEXT".to_owned(), 1),
            ("sha256".to_owned(), "TEXT".to_owned(), 1),
            ("quality".to_owned(), "TEXT".to_owned(), 0),
            ("created_at".to_owned(), "TEXT".to_owned(), 1),
        ]
    );
    assert_eq!(
        table_info(&database, "catalog__configs").await,
        vec![
            ("profile_id".to_owned(), "TEXT".to_owned(), 1),
            ("config_id".to_owned(), "TEXT".to_owned(), 1),
            ("name".to_owned(), "TEXT".to_owned(), 1),
            ("origin".to_owned(), "TEXT".to_owned(), 1),
            ("valid".to_owned(), "INTEGER".to_owned(), 1),
            ("message".to_owned(), "TEXT".to_owned(), 0),
            ("config_hash".to_owned(), "TEXT".to_owned(), 0),
            ("body".to_owned(), "TEXT".to_owned(), 1),
            ("created_at".to_owned(), "TEXT".to_owned(), 1),
        ]
    );
    assert_eq!(
        table_info(&database, "catalog__config_bundles").await,
        vec![
            ("config_id".to_owned(), "TEXT".to_owned(), 1),
            ("files".to_owned(), "TEXT".to_owned(), 1),
        ]
    );
}

#[tokio::test]
async fn the_database_holds_only_the_catalog_prefix() {
    let database = migrated().await;
    let tables: Vec<String> = database
        .call(|connection| {
            let mut statement = connection.prepare(
                "SELECT name FROM sqlite_master
                  WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                  ORDER BY name",
            )?;
            let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the table list");
    assert_eq!(
        tables,
        vec![
            "catalog__config_bundles".to_owned(),
            "catalog__configs".to_owned(),
            "catalog__datasets".to_owned(),
            "schema_migrations".to_owned(),
        ]
    );
    for table in &tables {
        for forbidden in ["gateway", "runs", "research", "simulation", "queue"] {
            if table == "schema_migrations" {
                continue;
            }
            assert!(
                !table.starts_with(forbidden),
                "the catalog baseline creates the foreign table {table}"
            );
        }
        assert!(!table.contains('.'), "{table} is a dotted Postgres name");
    }
}

#[tokio::test]
async fn a_dataset_row_stores_text_json_and_the_wire_identity() {
    let database = migrated().await;
    let catalog = SqliteDatasetCatalog::new(database.clone());
    let record = dataset();
    catalog
        .insert(profile("default"), record.clone())
        .await
        .expect("the dataset inserts");

    let raw: (String, String, String, String, i64, String) = database
        .call(|connection| {
            connection.query_row(
                "SELECT profile_id, data_version, sha256, quality, json_valid(quality),
                        typeof(candles)
                   FROM catalog__datasets
                  WHERE profile_id = 'default'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
        })
        .await
        .expect("the raw row");

    assert_eq!(raw.0, "default");
    assert_eq!(raw.1, record.summary.data_version.to_string());
    assert!(raw.1.starts_with("sha256:"));
    assert_eq!(raw.2, record.sha256.as_hex());
    assert!(!raw.2.starts_with("sha256:"));
    let quality: QualityReport = serde_json::from_str(&raw.3).expect("the stored quality json");
    assert_eq!(Some(quality), record.quality);
    assert_eq!(raw.4, 1, "the quality column is not valid JSON");
    assert_eq!(raw.5, "integer", "a count is not an INTEGER");

    let created_at: String = database
        .call(|connection| {
            connection.query_row(
                "SELECT created_at FROM catalog__datasets WHERE profile_id = 'default'",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("the stored instant");
    assert_eq!(created_at, "2026-11-15T09:00:00Z");
    assert!(created_at.ends_with('Z') && created_at.len() == 20);
}

#[tokio::test]
async fn a_config_row_stores_integer_booleans_and_a_shared_bundle() {
    let database = migrated().await;
    let store = SqliteConfigStore::new(database.clone());
    let record = config();
    store
        .upsert(profile("default"), record.clone())
        .await
        .expect("the config inserts");

    let raw: (i64, String, String, String, i64) = database
        .call(|connection| {
            connection.query_row(
                "SELECT valid, typeof(valid), origin, files, json_valid(files)
                   FROM catalog__configs c
                   JOIN catalog__config_bundles b ON b.config_id = c.config_id
                  WHERE c.profile_id = 'default'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
        })
        .await
        .expect("the raw row");
    assert_eq!(raw.0, 1);
    assert_eq!(raw.1, "integer");
    assert_eq!(raw.2, "repository");
    let files: BTreeMap<String, String> =
        serde_json::from_str(&raw.3).expect("the stored files json");
    assert_eq!(files, record.files);
    assert_eq!(raw.4, 1, "the files column is not valid JSON");

    // The first-seen instant is not moved by a refresh, matching the Postgres
    // upsert, which does not assign created_at on conflict.
    let mut refreshed = record.clone();
    refreshed.created_at = "2026-11-16T00:00:00Z".to_owned();
    store
        .upsert(profile("default"), refreshed)
        .await
        .expect("the refresh");
    let created_at: String = database
        .call(|connection| {
            connection.query_row(
                "SELECT created_at FROM catalog__configs WHERE profile_id = 'default'",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("the stored instant");
    assert_eq!(created_at, "2026-11-15T09:00:00Z");
}

#[tokio::test]
async fn a_malformed_stored_config_hash_is_a_decode_error() {
    let database = migrated().await;
    let store = SqliteConfigStore::new(database.clone());
    store
        .upsert(profile("default"), config())
        .await
        .expect("the config inserts");
    database
        .call(|connection| {
            connection.execute(
                "UPDATE catalog__configs SET config_hash = 'not-a-digest'
                  WHERE profile_id = 'default'",
                [],
            )
        })
        .await
        .expect("the raw corruption");

    let error = store
        .get(profile("default"), "sqlite_cfg".to_owned())
        .await
        .expect_err("a malformed stored hash was accepted");
    assert!(
        error.0.contains("config hash is malformed"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn the_baseline_refuses_invalid_json_and_a_non_boolean() {
    let database = migrated().await;
    let catalog = SqliteDatasetCatalog::new(database.clone());
    catalog
        .insert(profile("default"), dataset())
        .await
        .expect("the dataset inserts");
    SqliteConfigStore::new(database.clone())
        .upsert(profile("default"), config())
        .await
        .expect("the config inserts");

    let bad_quality = database
        .call(|connection| {
            connection.execute(
                "UPDATE catalog__datasets SET quality = 'not json' WHERE profile_id = 'default'",
                [],
            )
        })
        .await;
    assert!(bad_quality.is_err(), "json_valid accepted a text body");

    let bad_files = database
        .call(|connection| {
            connection.execute(
                "UPDATE catalog__config_bundles SET files = 'not json' WHERE config_id = 'sqlite_cfg'",
                [],
            )
        })
        .await;
    assert!(bad_files.is_err(), "json_valid accepted a text body");

    let bad_boolean = database
        .call(|connection| {
            connection.execute(
                "UPDATE catalog__configs SET valid = 2 WHERE profile_id = 'default'",
                [],
            )
        })
        .await;
    assert!(bad_boolean.is_err(), "the valid column accepted 2");
}
