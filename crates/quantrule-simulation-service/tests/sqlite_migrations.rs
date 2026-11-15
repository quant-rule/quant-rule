//! The simulation SQLite baseline: apply, refusal, parity, and the mark rules.
//!
//! The fast tier has no Postgres. The schema-parity test derives the released
//! `simulation.runs` and `simulation.marks` shapes from the checked-in Postgres
//! migration and compares them with the SQLite baseline the service applies.
//! The refusal tests cover the shared ledger's forward-only contract and the
//! triggers that close marking after a decision.

use std::fs;
use std::path::Path;

use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::migrations::{Migration, MigrationError};
use quantrule_simulation_service::migrations::sqlite;

const POSTGRES: &str = "crates/quantrule-simulation-service/migrations/0014_simulation.sql";

const RUN: &str = "
INSERT INTO simulation__runs (
    sim_id, profile_id, premise, bundle, inputs, inputs_digest, seed, criteria,
    window_spec, oracle_spec, check_config, status, created_at
) VALUES (
    'sim_sql', 'default', 'a premise',
    'sha256:0000000000000000000000000000000000000000000000000000000000000000',
    'inputs',
    'sha256:1111111111111111111111111111111111111111111111111111111111111111',
    0, 'min_events=30', '{}', '{}', '{}', 'marking', '2026-11-15T00:00:00Z'
);
";

const MARK: &str = "
INSERT INTO simulation__marks (
    mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action, quality,
    side, origin, note, created_at
) VALUES (
    'mark_sql', 'sim_sql', 'default', 0, 30, '2026-11-15T00:00:00Z', 'entry',
    'good', 'long', 'human', '', '2026-11-15T00:00:00Z'
);
";

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

async fn migrated() -> SqliteDatabase {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let applied = apply_migrations_in(&database, "simulation", &sqlite())
        .await
        .expect("the simulation baseline");
    assert_eq!(applied, 1);
    database
}

async fn execute(database: &SqliteDatabase, sql: impl Into<String>) -> Result<(), String> {
    let sql = sql.into();
    database
        .call(move |connection| connection.execute_batch(&sql))
        .await
        .map_err(|error| error.to_string())
}

async fn table_exists(database: &SqliteDatabase, table: &str) -> bool {
    let table = table.to_owned();
    database
        .call(move |connection| {
            connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                rusqlite::params![table],
                |row| row.get::<_, i64>(0),
            )
        })
        .await
        .expect("the catalog probe")
        == 1
}

async fn ledger(database: &SqliteDatabase) -> Vec<(String, i64, String)> {
    if !table_exists(database, "schema_migrations").await {
        return Vec::new();
    }
    database
        .call(|connection| {
            let mut statement = connection.prepare(
                "SELECT ledger, version, name FROM schema_migrations ORDER BY ledger, version",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the ledger")
}

/// The executable statements, without the explanatory comments. A comment may
/// name the Postgres spelling it is explaining.
fn statements(sql: &str) -> String {
    sql.lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn sqlite_type(postgres: &str) -> &'static str {
    match postgres {
        "text" | "jsonb" | "timestamptz" | "uuid" => "TEXT",
        "bigint" | "integer" | "smallint" | "boolean" => "INTEGER",
        "real" | "double" => "REAL",
        "bytea" => "BLOB",
        other => panic!("the parity suite has no SQLite mapping for Postgres type {other:?}"),
    }
}

fn is_column_type(value: &str) -> bool {
    matches!(
        value,
        "text"
            | "jsonb"
            | "timestamptz"
            | "uuid"
            | "bigint"
            | "integer"
            | "smallint"
            | "boolean"
            | "real"
            | "double"
            | "bytea"
    )
}

/// Read the columns of one table from the Postgres migration.
///
/// A multi-line `CHECK` is not a column definition, so a line whose second word
/// is not a Postgres type is skipped.
fn postgres_columns(table: &str) -> Vec<(String, String)> {
    let text = fs::read_to_string(root().join(POSTGRES)).expect("the Postgres migration reads");
    let marker = format!("CREATE TABLE IF NOT EXISTS simulation.{table} (");
    let mut inside = false;
    let mut columns = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with(&marker) {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with(");") {
            break;
        }
        if line.starts_with("PRIMARY KEY")
            || line.starts_with("UNIQUE")
            || line.starts_with("CHECK")
            || line.starts_with("CONSTRAINT")
        {
            continue;
        }
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(sql_type) = parts.next() else {
            continue;
        };
        let sql_type = sql_type.trim_end_matches([',', ';']);
        if !is_column_type(sql_type) {
            continue;
        }
        columns.push((
            name.trim_end_matches([',', ';']).to_owned(),
            sqlite_type(sql_type).to_owned(),
        ));
    }
    columns
}

async fn sqlite_columns(database: &SqliteDatabase, table: &str) -> Vec<(String, String)> {
    let table = table.to_owned();
    database
        .call(move |connection| {
            let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the SQLite column list")
}

#[tokio::test]
async fn the_baseline_applies_once_and_a_second_apply_is_a_no_op() {
    let database = migrated().await;
    let second = apply_migrations_in(&database, "simulation", &sqlite())
        .await
        .expect("the repeated baseline");
    assert_eq!(second, 0);
    assert_eq!(
        ledger(&database).await,
        vec![("simulation".to_owned(), 1, "simulation".to_owned())]
    );
}

#[tokio::test]
async fn the_baseline_matches_the_postgres_columns() {
    let database = migrated().await;
    for table in ["runs", "marks"] {
        assert_eq!(
            sqlite_columns(&database, &format!("simulation__{table}")).await,
            postgres_columns(table),
            "the {table} shape drifted"
        );
    }
    let baseline = statements(
        &fs::read_to_string(
            root()
                .join("crates/quantrule-simulation-service/migrations/sqlite/0001_simulation.sql"),
        )
        .expect("the SQLite baseline reads"),
    );
    assert!(baseline.contains("simulation__runs"));
    assert!(baseline.contains("simulation__marks"));
    assert!(!baseline.contains("simulation.runs"));
    for forbidden in ["CREATE SCHEMA", "jsonb", "timestamptz", "boolean"] {
        assert!(
            !baseline
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()),
            "the SQLite baseline names {forbidden}"
        );
    }
}

#[tokio::test]
async fn a_changed_body_is_refused_and_the_schema_stays() {
    let database = migrated().await;
    let changed = [Migration {
        version: 1,
        name: "simulation",
        sql: "CREATE TABLE simulation__probe (id INTEGER) STRICT;",
    }];
    let error = apply_migrations_in(&database, "simulation", &changed)
        .await
        .expect_err("a changed body");
    assert!(
        matches!(error, MigrationError::ChecksumMismatch { version: 1 }),
        "{error:?}"
    );
    assert!(!table_exists(&database, "simulation__probe").await);
    assert_eq!(ledger(&database).await.len(), 1);
}

#[tokio::test]
async fn a_database_ahead_of_the_binary_is_refused() {
    let database = migrated().await;
    execute(
        &database,
        "INSERT INTO schema_migrations (ledger, version, name, checksum)
         VALUES ('simulation', 9, 'future', 'x')",
    )
    .await
    .expect("the future row");
    let error = apply_migrations_in(&database, "simulation", &sqlite())
        .await
        .expect_err("a database ahead");
    assert!(
        matches!(
            error,
            MigrationError::DatabaseAhead {
                applied: 9,
                available: 1
            }
        ),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_applied_version_the_binary_does_not_know_is_refused() {
    let database = migrated().await;
    execute(
        &database,
        "INSERT INTO schema_migrations (ledger, version, name, checksum)
         VALUES ('simulation', 2, 'unknown', 'x')",
    )
    .await
    .expect("the unknown row");
    let available = vec![
        sqlite()[0],
        Migration {
            version: 3,
            name: "future",
            sql: "SELECT 1;",
        },
    ];
    let error = apply_migrations_in(&database, "simulation", &available)
        .await
        .expect_err("an unknown applied version");
    assert!(
        matches!(error, MigrationError::UnknownApplied(2)),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_failed_baseline_rolls_the_whole_apply_back() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    let broken = [Migration {
        version: 1,
        name: "simulation",
        sql: "CREATE TABLE simulation__probe (id INTEGER) STRICT;
              INSERT INTO simulation__missing (id) VALUES (1);",
    }];
    let error = apply_migrations_in(&database, "simulation", &broken)
        .await
        .expect_err("the failed baseline");
    assert!(matches!(error, MigrationError::Sql(_)), "{error:?}");
    assert!(!table_exists(&database, "simulation__probe").await);
    assert!(ledger(&database).await.is_empty());
}

#[tokio::test]
async fn the_baseline_refuses_a_mark_after_the_decision() {
    let database = migrated().await;
    execute(&database, RUN).await.expect("the run");
    execute(&database, MARK).await.expect("the mark");
    execute(
        &database,
        "UPDATE simulation__runs
            SET status = 'decided', decision = 'test', decision_note = 'go',
                decided_at = '2026-11-15T00:00:09Z'
          WHERE sim_id = 'sim_sql'",
    )
    .await
    .expect("the decision");

    let error = execute(
        &database,
        "INSERT INTO simulation__marks (
             mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action,
             quality, side, origin, note, created_at
         ) VALUES (
             'mark_sql_2', 'sim_sql', 'default', 0, 40, '2026-11-15T00:00:00Z',
             'entry', 'bad', 'short', 'human', '', '2026-11-15T00:00:00Z'
         )",
    )
    .await
    .expect_err("a mark after the decision");
    assert!(error.contains("marking is closed"), "{error}");

    for sql in [
        "UPDATE simulation__marks SET note = 'late' WHERE mark_id = 'mark_sql'",
        "DELETE FROM simulation__marks WHERE mark_id = 'mark_sql'",
    ] {
        let error = execute(&database, sql)
            .await
            .expect_err("a mark mutation after the decision");
        assert!(error.contains("marking is closed"), "{sql}: {error}");
    }

    // A decided run is final, but its check result may still be refreshed.
    let error = execute(
        &database,
        "UPDATE simulation__runs SET decision = 'stop' WHERE sim_id = 'sim_sql'",
    )
    .await
    .expect_err("a rewritten decision");
    assert!(error.contains("a decided run is final"), "{error}");
    execute(
        &database,
        "UPDATE simulation__runs SET check_result = '{}' WHERE sim_id = 'sim_sql'",
    )
    .await
    .expect("a check refresh on a decided run");
}

#[tokio::test]
async fn the_baseline_keeps_one_mark_per_identity() {
    let database = migrated().await;
    execute(&database, RUN).await.expect("the run");
    execute(&database, MARK).await.expect("the mark");
    let duplicate = MARK.replace("'mark_sql'", "'mark_sql_duplicate'");
    let error = execute(&database, duplicate)
        .await
        .expect_err("a duplicate mark identity");
    assert!(error.contains("UNIQUE constraint failed"), "{error}");
}

#[tokio::test]
async fn the_baseline_keeps_a_decision_and_its_time_together() {
    let database = migrated().await;
    let error = execute(
        &database,
        "INSERT INTO simulation__runs (
             sim_id, profile_id, premise, bundle, inputs, inputs_digest, seed,
             criteria, window_spec, oracle_spec, check_config, status, decision,
             created_at
         ) VALUES (
             'sim_sql_half', 'default', 'a premise',
             'sha256:0000000000000000000000000000000000000000000000000000000000000000',
             'inputs',
             'sha256:1111111111111111111111111111111111111111111111111111111111111111',
             0, 'criteria', '{}', '{}', '{}', 'marking', 'test', '2026-11-15T00:00:00Z'
         )",
    )
    .await
    .expect_err("a marking run with a decision");
    assert!(error.contains("CHECK constraint failed"), "{error}");
}
