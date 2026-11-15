//! The SQLite adapter's behaviour.
//!
//! The fast tier has no Postgres and no network. These tests exercise the P1a
//! spike: one shared migration ledger with a ledger column, a forward-only plan
//! that refuses a changed checksum, an unknown applied version, and a database
//! ahead; a failed plan rolls the whole transaction back; every repository call
//! runs through the blocking pool; and a JSON document and an RFC 3339 instant
//! keep their SQLite storage shape. The gateway's own profile table and
//! repository are tested in `quantrule-gateway`.

use std::path::Path;
use std::sync::Arc;

use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::adapters::{Backend, Database, DatabaseError};
use quantrule_runtime::migrations::{Migration, MigrationError};
use rusqlite::params;
use serde_json::{Value, json};

const FIRST: Migration = Migration {
    version: 1,
    name: "first",
    sql: "CREATE TABLE probe_one (id TEXT PRIMARY KEY) STRICT;",
};

const SECOND: Migration = Migration {
    version: 2,
    name: "second",
    sql: "CREATE TABLE probe_two (id TEXT PRIMARY KEY) STRICT;",
};

const THIRD: Migration = Migration {
    version: 3,
    name: "third",
    sql: "CREATE TABLE probe_three (id TEXT PRIMARY KEY) STRICT;",
};

const FIRST_CHANGED: Migration = Migration {
    version: 1,
    name: "first",
    sql: "CREATE TABLE probe_one (id TEXT PRIMARY KEY, note TEXT) STRICT;",
};

const INVALID: Migration = Migration {
    version: 2,
    name: "invalid",
    sql: "CREATE TABLE probe_two (id TEXT PRIMARY KEY) STRICT; THIS IS NOT SQL;",
};

const DOCUMENTED: Migration = Migration {
    version: 1,
    name: "documented",
    sql: "CREATE TABLE probe_document (
              id  TEXT PRIMARY KEY,
              doc TEXT NOT NULL CHECK (json_valid(doc)),
              at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
          ) STRICT;",
};

async fn table_exists(database: &SqliteDatabase, table: &str) -> bool {
    let table = table.to_owned();
    database
        .call(move |connection| {
            connection.query_row(
                "SELECT count(*) FROM sqlite_master
                  WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get::<_, i64>(0),
            )
        })
        .await
        .expect("the table count")
        > 0
}

async fn ledger_rows(database: &SqliteDatabase, ledger: &str) -> i64 {
    let ledger = ledger.to_owned();
    database
        .call(move |connection| {
            connection.query_row(
                "SELECT count(*) FROM schema_migrations WHERE ledger = ?1",
                params![ledger],
                |row| row.get(0),
            )
        })
        .await
        .expect("the ledger count")
}

async fn insert_ledger_row(database: &SqliteDatabase, ledger: &str, version: i64) {
    let ledger = ledger.to_owned();
    database
        .call(move |connection| {
            connection.execute(
                "INSERT INTO schema_migrations (ledger, version, name, checksum)
                 VALUES (?1, ?2, 'manual', 'manual')",
                params![ledger, version],
            )
        })
        .await
        .expect("the manual ledger row");
}

#[tokio::test]
async fn a_second_apply_is_empty_and_the_ledger_is_per_service() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    assert_eq!(
        apply_migrations_in(&database, "probe", &[FIRST, SECOND])
            .await
            .expect("the first apply"),
        2
    );
    assert_eq!(
        apply_migrations_in(&database, "probe", &[FIRST, SECOND])
            .await
            .expect("the second apply"),
        0
    );
    assert_eq!(ledger_rows(&database, "probe").await, 2);

    assert_eq!(
        apply_migrations_in(&database, "other", &[THIRD])
            .await
            .expect("the other ledger"),
        1
    );
    assert_eq!(ledger_rows(&database, "probe").await, 2);
    assert_eq!(ledger_rows(&database, "other").await, 1);
    assert_eq!(ledger_rows(&database, "gateway").await, 0);
}

#[tokio::test]
async fn the_shared_ledger_has_one_row_per_ledger_and_version() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[FIRST])
        .await
        .expect("the first apply");
    let columns: Vec<String> = database
        .call(|connection| {
            let mut statement = connection.prepare("PRAGMA table_info(schema_migrations)")?;
            let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the ledger columns");
    assert_eq!(
        columns,
        ["ledger", "version", "name", "checksum", "applied_at"]
    );
}

#[tokio::test]
async fn a_changed_checksum_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[FIRST])
        .await
        .expect("the first apply");
    let error = apply_migrations_in(&database, "probe", &[FIRST_CHANGED])
        .await
        .expect_err("the changed body");
    assert_eq!(error, MigrationError::ChecksumMismatch { version: 1 });
}

#[tokio::test]
async fn an_unknown_applied_version_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[FIRST])
        .await
        .expect("the first apply");
    insert_ledger_row(&database, "probe", 2).await;
    let error = apply_migrations_in(&database, "probe", &[FIRST, THIRD])
        .await
        .expect_err("the unknown version");
    assert_eq!(error, MigrationError::UnknownApplied(2));
}

#[tokio::test]
async fn a_database_ahead_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[FIRST])
        .await
        .expect("the first apply");
    insert_ledger_row(&database, "probe", 99).await;
    let error = apply_migrations_in(&database, "probe", &[FIRST])
        .await
        .expect_err("the database is ahead");
    assert_eq!(
        error,
        MigrationError::DatabaseAhead {
            applied: 99,
            available: 1
        }
    );
}

#[tokio::test]
async fn a_failed_plan_rolls_back_the_whole_transaction() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let error = apply_migrations_in(&database, "probe", &[FIRST, INVALID])
        .await
        .expect_err("the invalid plan");
    assert!(matches!(error, MigrationError::Sql(_)));
    assert!(
        !table_exists(&database, "probe_one").await,
        "the first migration survived a failed plan"
    );
    assert!(
        !table_exists(&database, "probe_two").await,
        "the partial second migration survived a failed plan"
    );
    assert!(
        !table_exists(&database, "schema_migrations").await,
        "the ledger table survived a failed plan"
    );
}

#[tokio::test]
async fn a_ledger_name_cannot_escape_its_column() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    for ledger in ["", "Gateway", "gateway; drop table x", "gate-way"] {
        assert!(
            apply_migrations_in(&database, ledger, &[FIRST])
                .await
                .is_err(),
            "{ledger:?} was accepted"
        );
    }
}

#[tokio::test]
async fn a_json_document_and_an_rfc3339_instant_round_trip() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(&database, "probe", &[DOCUMENTED])
        .await
        .expect("the documented migration");
    database
        .call(|connection| {
            connection.execute(
                "INSERT INTO probe_document (id, doc) VALUES (?1, ?2)",
                params!["one", r#"{"skipped":2,"reason":"gap"}"#],
            )
        })
        .await
        .expect("the document inserts");

    let (doc, at): (String, String) = database
        .call(|connection| {
            connection.query_row(
                "SELECT doc, at FROM probe_document WHERE id = ?1",
                params!["one"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
        })
        .await
        .expect("the document reads");
    assert_eq!(
        serde_json::from_str::<Value>(&doc).expect("the document parses"),
        json!({"skipped": 2, "reason": "gap"})
    );
    assert_eq!(at.len(), 20, "the default is second-precision RFC 3339");
    assert_eq!(at.as_bytes()[10], b'T');
    assert_eq!(at.as_bytes()[19], b'Z');

    let kind: String = database
        .call(|connection| {
            connection.query_row(
                "SELECT typeof(doc) FROM probe_document WHERE id = ?1",
                params!["one"],
                |row| row.get(0),
            )
        })
        .await
        .expect("the storage class");
    assert_eq!(kind, "text", "a JSON document is TEXT, not JSONB");

    let invalid = database
        .call(|connection| {
            connection.execute(
                "INSERT INTO probe_document (id, doc) VALUES (?1, ?2)",
                params!["two", "not json"],
            )
        })
        .await;
    assert!(invalid.is_err(), "json_valid accepted a non-document");
}

#[tokio::test]
async fn a_file_database_uses_wal_and_reopens_without_reapplying() {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sqlite-reopen");
    let _ = std::fs::remove_dir_all(&directory);
    let path = directory.join("embedded.sqlite");
    {
        let database = SqliteDatabase::open(&path).expect("the file opens");
        assert_eq!(database.path(), Some(path.as_path()));
        apply_migrations_in(&database, "gateway", &[FIRST])
            .await
            .expect("the first apply");
        let mode: String = database
            .call(|connection| connection.query_row("PRAGMA journal_mode", [], |row| row.get(0)))
            .await
            .expect("the journal mode");
        assert_eq!(mode.to_ascii_lowercase(), "wal");
    }
    {
        let database = SqliteDatabase::open(&path).expect("the file reopens");
        assert_eq!(ledger_rows(&database, "gateway").await, 1);
        assert_eq!(
            apply_migrations_in(&database, "gateway", &[FIRST])
                .await
                .expect("the second apply"),
            0
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn the_memory_path_is_an_in_memory_database() {
    let database = SqliteDatabase::open(":memory:").expect("the database opens");
    assert_eq!(database.path(), None);
}

#[tokio::test]
async fn the_ping_reports_the_sqlite_backend() {
    let database: Arc<dyn Database> =
        Arc::new(SqliteDatabase::open_in_memory().expect("the database opens"));
    database.ping().await.expect("the ping");
    assert_eq!(database.backend(), Backend::Sqlite);
    assert_eq!(database.backend().as_str(), "sqlite");
}

#[tokio::test]
async fn two_calls_share_one_connection() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let (first, second) = tokio::join!(
        database.call(|connection| connection.execute_batch(
            "CREATE TABLE probe_a (id INTEGER);
             INSERT INTO probe_a VALUES (1);"
        )),
        database.call(|connection| connection.execute_batch(
            "CREATE TABLE probe_b (id INTEGER);
             INSERT INTO probe_b VALUES (2);"
        )),
    );
    first.expect("the first call");
    second.expect("the second call");
    let count: i64 = database
        .call(|connection| {
            connection.query_row(
                "SELECT (SELECT count(*) FROM probe_a) + (SELECT count(*) FROM probe_b)",
                [],
                |row| row.get(0),
            )
        })
        .await
        .expect("the count");
    assert_eq!(count, 2);
}

#[tokio::test]
async fn a_call_error_is_a_database_error() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let error: DatabaseError = database
        .call(|connection| connection.execute("INSERT INTO missing (id) VALUES (1)", []))
        .await
        .expect_err("the table is missing");
    assert!(
        error.to_string().contains("missing"),
        "the driver message is preserved: {error}"
    );
}
