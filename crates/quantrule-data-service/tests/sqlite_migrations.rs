//! The catalog SQLite baseline under the shared migration ledger.
//!
//! The runner is the runtime's; this suite proves the catalog uses it correctly:
//! the first apply records one ledger row, a second apply is a no-op, a changed
//! body and an unknown applied version are refused, and a database ahead of the
//! binary is refused rather than guessed at.

use quantrule_data_service::migrations;
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::migrations::{Migration, MigrationError};

const CHANGED_CATALOG: &str =
    "CREATE TABLE IF NOT EXISTS catalog__probe_changed (id INTEGER) STRICT";
const PROBE_ONE: &str = "CREATE TABLE IF NOT EXISTS catalog__probe_one (id INTEGER) STRICT";
const PROBE_TWO: &str = "CREATE TABLE IF NOT EXISTS catalog__probe_two (id INTEGER) STRICT";
const PROBE_THREE: &str = "CREATE TABLE IF NOT EXISTS catalog__probe_three (id INTEGER) STRICT";
const PROBE_FOUR: &str = "CREATE TABLE IF NOT EXISTS catalog__probe_four (id INTEGER) STRICT";

fn probe(version: i64, sql: &'static str) -> Migration {
    Migration {
        version,
        name: "probe",
        sql,
    }
}

async fn ledger_rows(database: &SqliteDatabase) -> Vec<(String, i64, String)> {
    database
        .call(|connection| {
            let mut statement = connection.prepare(
                "SELECT ledger, version, name
                   FROM schema_migrations
                  WHERE ledger = 'catalog'
                  ORDER BY version",
            )?;
            let rows =
                statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the ledger rows")
}

#[tokio::test]
async fn the_first_apply_records_the_catalog_ledger_and_the_second_is_a_no_op() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let applied = apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the first apply");
    assert_eq!(applied, 1);
    assert_eq!(
        ledger_rows(&database).await,
        vec![("catalog".to_owned(), 1, "catalog".to_owned())]
    );

    let applied = apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the second apply");
    assert_eq!(applied, 0, "the second apply was not a no-op");
    assert_eq!(ledger_rows(&database).await.len(), 1);
}

#[tokio::test]
async fn a_changed_migration_body_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the first apply");

    let changed = vec![Migration {
        version: 1,
        name: "catalog",
        sql: CHANGED_CATALOG,
    }];
    assert_eq!(
        apply_migrations_in(&database, "catalog", &changed).await,
        Err(MigrationError::ChecksumMismatch { version: 1 })
    );
    assert_eq!(
        ledger_rows(&database).await,
        vec![("catalog".to_owned(), 1, "catalog".to_owned())],
        "a refused plan changed the ledger"
    );
    assert_eq!(
        apply_migrations_in(&database, "catalog", &migrations::sqlite())
            .await
            .expect("the original still applies"),
        0
    );
}

#[tokio::test]
async fn an_unknown_applied_version_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "catalog",
        &[probe(1, PROBE_ONE), probe(3, PROBE_THREE)],
    )
    .await
    .expect("the probe baseline");

    let available = [
        probe(1, PROBE_ONE),
        probe(2, PROBE_TWO),
        probe(4, PROBE_FOUR),
    ];
    assert_eq!(
        apply_migrations_in(&database, "catalog", &available).await,
        Err(MigrationError::UnknownApplied(3))
    );
    assert_eq!(
        ledger_rows(&database).await,
        vec![
            ("catalog".to_owned(), 1, "probe".to_owned()),
            ("catalog".to_owned(), 3, "probe".to_owned()),
        ]
    );
}

#[tokio::test]
async fn a_database_ahead_of_the_binary_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "catalog",
        &[probe(1, PROBE_ONE), probe(3, PROBE_THREE)],
    )
    .await
    .expect("the probe baseline");

    assert_eq!(
        apply_migrations_in(
            &database,
            "catalog",
            &[probe(1, PROBE_ONE), probe(2, PROBE_TWO)],
        )
        .await,
        Err(MigrationError::DatabaseAhead {
            applied: 3,
            available: 2,
        })
    );
    assert_eq!(ledger_rows(&database).await.len(), 2);
}

#[tokio::test]
async fn the_ledgers_are_independent_in_one_file() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[probe(1, PROBE_ONE)])
        .await
        .expect("the probe ledger");
    apply_migrations_in(&database, "catalog", &migrations::sqlite())
        .await
        .expect("the catalog ledger");

    let rows: Vec<(String, i64)> = database
        .call(|connection| {
            let mut statement = connection
                .prepare("SELECT ledger, version FROM schema_migrations ORDER BY ledger")?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the ledger rows");
    assert_eq!(
        rows,
        vec![("catalog".to_owned(), 1), ("probe".to_owned(), 1)]
    );
}
