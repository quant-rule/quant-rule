use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use super::schema::{SCHEMA_V1, SCHEMA_VERSION};
use super::{Store, StoreError, migrate};

fn database(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/store-tests")
        .join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("the test directory is created");
    directory.join("store.db")
}

#[test]
fn a_fresh_database_has_the_schema_and_version() {
    let store = Store::open_in_memory().expect("the in-memory store opens");
    assert_eq!(store.schema_version().expect("the version"), SCHEMA_VERSION);

    let tables: Vec<String> = store
        .connection()
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .expect("the table query")
        .query_map([], |row| row.get(0))
        .expect("the table rows")
        .collect::<Result<_, _>>()
        .expect("the table names");
    assert_eq!(
        tables,
        vec![
            "bar".to_string(),
            "consensus".to_string(),
            "dataset".to_string(),
            "event".to_string(),
            "layer3".to_string(),
            "leg".to_string(),
            "level".to_string(),
            "pivot".to_string(),
        ]
    );
}

#[test]
fn the_schema_widths_match_the_contract() {
    let store = Store::open_in_memory().expect("the in-memory store opens");
    let connection = store.connection();
    let count = |table: &str| -> i64 {
        connection
            .query_row(
                "SELECT count(*) FROM pragma_table_info(?1)",
                [table],
                |row| row.get(0),
            )
            .expect("the table info")
    };
    assert_eq!(count("bar"), 74);
    assert_eq!(count("layer3"), 285);
    assert_eq!(
        connection
            .prepare("SELECT * FROM layer3_full LIMIT 0")
            .expect("the view query")
            .column_count(),
        357
    );
}

#[test]
fn the_bar_ddl_matches_the_research_column_order() {
    use quantrule_analytics::research::{layer1_names, layer2_names};

    let store = Store::open_in_memory().expect("the in-memory store opens");
    let connection = store.connection();
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('bar') ORDER BY cid")
        .expect("the bar column query");
    let ddl: Vec<String> = statement
        .query_map([], |row| row.get(0))
        .expect("the bar columns")
        .collect::<Result<_, _>>()
        .expect("the bar column names");

    let mut expected = vec![String::from("dataset_id")];
    expected.extend(layer1_names().into_iter().map(str::to_owned));
    expected.extend(
        layer2_names()[layer1_names().len()..]
            .iter()
            .map(|name| (*name).to_owned()),
    );
    assert_eq!(ddl, expected);
}

#[test]
fn a_file_database_reopens_idempotently() {
    let path = database("reopen");
    {
        let store = Store::open(&path).expect("the first open");
        assert_eq!(store.schema_version().expect("the version"), SCHEMA_VERSION);
    }
    let store = Store::open(&path).expect("the second open");
    assert_eq!(store.schema_version().expect("the version"), SCHEMA_VERSION);
    store.integrity_check().expect("the integrity check");
}

#[test]
fn a_newer_schema_is_rejected() {
    let path = database("newer");
    {
        let connection = Connection::open(&path).expect("the raw connection");
        connection
            .pragma_update(None, "user_version", 3)
            .expect("the version write");
    }
    let error = Store::open(&path).expect_err("the newer schema is rejected");
    assert!(matches!(
        error,
        StoreError::UnsupportedVersion {
            found: 3,
            supported: SCHEMA_VERSION
        }
    ));
}

#[test]
fn a_failed_migration_rolls_back() {
    let mut connection = Connection::open_in_memory().expect("the raw connection");
    let error = migrate(
        &mut connection,
        None,
        "CREATE TABLE ok (id INTEGER); CREATE TABLE bad (;",
        1,
    )
    .expect_err("the bad migration fails");
    assert!(matches!(error, StoreError::Migrate { .. }));

    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("the version");
    assert_eq!(version, 0);
    let tables: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'ok'",
            [],
            |row| row.get(0),
        )
        .expect("the table count");
    assert_eq!(tables, 0);

    migrate(&mut connection, None, SCHEMA_V1, 1).expect("the real migration succeeds");
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("the version");
    assert_eq!(version, 1);
}

#[test]
fn a_version_one_database_is_migrated_to_version_two() {
    let path = database("migrate-v2");
    {
        let mut connection = Connection::open(&path).expect("the raw connection");
        migrate(&mut connection, None, SCHEMA_V1, 1).expect("the version 1 schema");
    }

    let store = Store::open(&path).expect("the version 1 database opens");
    assert_eq!(store.schema_version().expect("the version"), SCHEMA_VERSION);

    let event_key: String = store
        .connection()
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'event_key'",
            [],
            |row| row.get(0),
        )
        .expect("the event key index");
    assert!(event_key.contains("WHERE NOT"));
    let threshold_key: String = store
        .connection()
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'event_threshold_key'",
            [],
            |row| row.get(0),
        )
        .expect("the threshold key index");
    assert!(threshold_key.contains("reference_value"));
    assert!(threshold_key.contains("WHERE event_type_code = 3"));

    drop(store);
    let reopened = Store::open(&path).expect("the migrated database reopens");
    assert_eq!(
        reopened.schema_version().expect("the version"),
        SCHEMA_VERSION
    );
}

#[test]
fn a_corrupt_file_is_rejected() {
    let path = database("corrupt");
    fs::write(&path, b"not a database").expect("the corrupt file");
    let error = Store::open(&path).expect_err("the corrupt file is rejected");
    assert!(matches!(error, StoreError::Corrupt { .. }));
}

#[test]
fn foreign_keys_are_enabled() {
    let store = Store::open_in_memory().expect("the in-memory store opens");
    let enabled: i64 = store
        .connection()
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("the pragma");
    assert_eq!(enabled, 1);
}

#[test]
fn removing_a_dataset_cascades_and_leaves_the_database() {
    let mut store = Store::open_in_memory().expect("the in-memory store opens");
    store
        .connection()
        .execute(
            "INSERT INTO dataset \
             (name, data_version, model_version, model_contract, research_contract, structure_scale_code, m1_bars, layer3_materialized) \
             VALUES ('demo', 'version', '0.1.0', '2026-10-21', '2026-10-24', 2, 0, 1)",
            [],
        )
        .expect("the dataset row");

    let report = store.remove("demo").expect("the dataset is removed");
    assert_eq!(report.name, "demo");
    assert_eq!(report.bars, 0);
    assert!(matches!(
        store.dataset_id("demo"),
        Err(StoreError::DatasetNotFound { .. })
    ));
    assert!(store.remove("demo").is_err());
}

#[test]
fn an_uninitialized_database_is_not_a_store() {
    let path = database("uninitialized");
    Connection::open(&path).expect("the empty SQLite file");
    let error = Store::open_read_only(&path).expect_err("version zero is rejected");
    assert!(matches!(error, StoreError::Uninitialized { .. }));
}

#[test]
fn the_100k_cap_is_enforced_by_the_schema() {
    let store = Store::open_in_memory().expect("the in-memory store opens");
    let insert = |bars: i64| {
        store.connection().execute(
            "INSERT INTO dataset \
             (name, data_version, model_version, model_contract, research_contract, structure_scale_code, m1_bars, layer3_materialized) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
            params!["demo", "version", "0.1.0", "2026-10-21", "2026-10-24", 2, bars],
        )
    };
    insert(100_000).expect("the cap is accepted");
    insert(100_001).expect_err("the over-cap row is rejected");
}
