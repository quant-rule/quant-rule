//! Evidence schema and row parity between the Postgres and SQLite profiles.
//!
//! The fast tier has no Postgres. The suite derives the released `runs` and
//! `research` shape from the checked-in Postgres migrations and compares it
//! with the SQLite baseline the evidence service applies: every column in
//! position and every mapped type. It then writes one representative run row in
//! the SQLite storage shape and reads it back through both the raw table and
//! `SqliteRunStore`, so a column-order or type drift fails loudly. The suite
//! also proves the sealed-row trigger still refuses update and delete and that
//! the migration ledger refuses a changed body, an unknown version, and a
//! database ahead. A live Postgres method suite is the opt-in
//! `sqlite_conformance.rs` test.

use std::fs;
use std::path::{Path, PathBuf};

use quantrule_api::ProfileId;
use quantrule_evidence_service::migrations;
use quantrule_evidence_service::research::HypothesisStore;
use quantrule_evidence_service::runs::{RunRecord, RunStore};
use quantrule_evidence_service::sqlite::{SqliteHypothesisStore, SqliteRunStore};
use quantrule_research::{CostModel, Hypothesis, Partition, SealedHypothesis, Split};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::migrations::{Migration, MigrationError, checksum};
use rusqlite::params;
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};

const RUNS: &str = "crates/quantrule-evidence-service/migrations/0002_runs.sql";
const RESEARCH: &str = "crates/quantrule-evidence-service/migrations/0004_research.sql";
const HYPOTHESIS_BODY: &str =
    "crates/quantrule-evidence-service/migrations/0005_hypothesis_body.sql";
const HYPOTHESIS_PROFILE: &str =
    "crates/quantrule-evidence-service/migrations/0008_hypothesis_profile.sql";
const SUBMISSIONS: &str = "crates/quantrule-evidence-service/migrations/0009_run_submissions.sql";
const ATTEMPTS: &str = "crates/quantrule-evidence-service/migrations/0013_run_attempts.sql";
const MODEL_ARTIFACT: &str =
    "crates/quantrule-evidence-service/migrations/0015_run_model_artifact.sql";
const STAGE: &str = "crates/quantrule-evidence-service/migrations/0014_stage.sql";
const SQLITE: &str = "crates/quantrule-evidence-service/migrations/sqlite/0001_runs_research.sql";
const SQLITE_STAGE: &str = "crates/quantrule-evidence-service/migrations/sqlite/0002_stage.sql";
const FIXTURE: &str = include_str!("fixtures/run_parity.json");

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(root().join(path)).expect("the migration reads")
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

fn create_table_columns(path: &str, table: &str) -> Vec<(String, String)> {
    let text = read(path);
    let marker = format!("CREATE TABLE IF NOT EXISTS {table} (");
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
            || line.starts_with("FOREIGN KEY")
            || line.starts_with("REFERENCES")
        {
            continue;
        }
        let mut parts = line.split_whitespace();
        let name = parts
            .next()
            .expect("a column name")
            .trim_end_matches([',', ';'])
            .to_owned();
        let sql_type = parts
            .next()
            .expect("a column type")
            .trim_end_matches([',', ';'])
            .to_owned();
        columns.push((name, sqlite_type(&sql_type).to_owned()));
    }
    assert!(!columns.is_empty(), "no columns parsed for {table}");
    columns
}

fn added_columns(path: &str, table: &str) -> Vec<(String, String)> {
    let text = read(path);
    let prefix = format!("ALTER TABLE {table} ADD COLUMN ");
    let mut columns = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let rest = rest.trim_end_matches(';');
        let mut parts = rest.split_whitespace();
        let name = parts.next().expect("a column name").to_owned();
        let sql_type = parts.next().expect("a column type").to_owned();
        columns.push((name, sqlite_type(&sql_type).to_owned()));
    }
    columns
}

fn expected_tables() -> Vec<(&'static str, Vec<(String, String)>)> {
    let mut runs = create_table_columns(RUNS, "runs.runs");
    runs.extend(added_columns(SUBMISSIONS, "runs.runs"));
    runs.extend(added_columns(ATTEMPTS, "runs.runs"));
    runs.extend(added_columns(MODEL_ARTIFACT, "runs.runs"));
    let mut hypotheses = create_table_columns(RESEARCH, "research.hypotheses");
    hypotheses.extend(added_columns(HYPOTHESIS_BODY, "research.hypotheses"));
    hypotheses.extend(added_columns(HYPOTHESIS_PROFILE, "research.hypotheses"));
    vec![
        ("runs__runs", runs),
        ("runs__stages", create_table_columns(RUNS, "runs.stages")),
        (
            "runs__artifacts",
            create_table_columns(RUNS, "runs.artifacts"),
        ),
        (
            "runs__submissions",
            create_table_columns(SUBMISSIONS, "runs.submissions"),
        ),
        ("research__hypotheses", hypotheses),
        ("stage__runs", create_table_columns(STAGE, "stage.runs")),
        (
            "stage__submissions",
            create_table_columns(STAGE, "stage.submissions"),
        ),
        (
            "stage__artifacts",
            create_table_columns(STAGE, "stage.artifacts"),
        ),
    ]
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

async fn foreign_keys(
    database: &SqliteDatabase,
    table: &str,
) -> Vec<(String, String, String, String)> {
    let table = table.to_owned();
    database
        .call(move |connection| {
            let mut statement = connection.prepare(&format!("PRAGMA foreign_key_list({table})"))?;
            let rows = statement.query_map([], |row| {
                Ok((
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(6)?,
                ))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the SQLite foreign keys")
}

async fn baseline(database: &SqliteDatabase) {
    apply_migrations_in(database, "runs", &migrations::sqlite())
        .await
        .expect("the evidence baseline");
}

#[tokio::test]
async fn the_evidence_baseline_matches_the_postgres_tables() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&database).await;
    for (table, expected) in expected_tables() {
        assert_eq!(
            sqlite_columns(&database, table).await,
            expected,
            "{table} drifted from the Postgres shape"
        );
    }

    for table in ["runs__stages", "runs__artifacts", "runs__submissions"] {
        assert_eq!(
            foreign_keys(&database, table).await,
            vec![(
                "runs__runs".to_owned(),
                "run_id".to_owned(),
                "run_id".to_owned(),
                "CASCADE".to_owned()
            )],
            "{table} does not cascade from the run row"
        );
    }
    for table in ["stage__submissions", "stage__artifacts"] {
        assert_eq!(
            foreign_keys(&database, table).await,
            vec![(
                "stage__runs".to_owned(),
                "stage_run_id".to_owned(),
                "stage_run_id".to_owned(),
                "CASCADE".to_owned()
            )],
            "{table} does not cascade from the stage row"
        );
    }

    let baseline = format!(
        "{}\n{}",
        statements(&read(SQLITE)),
        statements(&read(SQLITE_STAGE))
    );
    for table in [
        "runs__runs",
        "runs__stages",
        "runs__artifacts",
        "runs__submissions",
        "research__hypotheses",
        "stage__runs",
        "stage__submissions",
        "stage__artifacts",
    ] {
        assert!(baseline.contains(table), "{table} is missing");
    }
    for forbidden in [
        "CREATE SCHEMA",
        "jsonb",
        "timestamptz",
        "boolean",
        "plpgsql",
        "to_char",
        "now()",
    ] {
        assert!(
            !baseline
                .to_ascii_lowercase()
                .contains(&forbidden.to_ascii_lowercase()),
            "the SQLite baseline names {forbidden}"
        );
    }
    for forbidden in ["gateway__", "catalog__", "simulation__", "queue__"] {
        assert!(
            !baseline.contains(forbidden),
            "the SQLite baseline names another service prefix {forbidden}"
        );
    }

    let triggers: Vec<(String, String)> = database
        .call(|connection| {
            let mut statement = connection.prepare(
                "SELECT name, sql FROM sqlite_master
                  WHERE type = 'trigger' AND tbl_name = 'research__hypotheses'
                  ORDER BY name",
            )?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the triggers read");
    assert_eq!(triggers.len(), 2, "the sealed-row triggers are missing");
    let trigger_sql = triggers
        .iter()
        .map(|(_, sql)| sql.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(trigger_sql.contains("BEFORE UPDATE ON research__hypotheses"));
    assert!(trigger_sql.contains("BEFORE DELETE ON research__hypotheses"));
    assert_eq!(
        trigger_sql
            .matches("RAISE(ABORT, 'a sealed hypothesis is immutable')")
            .count(),
        2,
        "both sealed-row triggers must abort"
    );

    let indexes: Vec<(String, i64)> = database
        .call(|connection| {
            let mut statement = connection.prepare("PRAGMA index_list(runs__runs)")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the index list");
    assert!(
        indexes
            .iter()
            .any(|(name, unique)| name == "runs__status_submitted_idx" && *unique == 0),
        "the run status index is missing"
    );

    let stage_indexes: Vec<(String, i64)> = database
        .call(|connection| {
            let mut statement = connection.prepare("PRAGMA index_list(stage__runs)")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the stage index list");
    assert!(
        stage_indexes
            .iter()
            .any(|(name, unique)| name == "stage__status_submitted_idx" && *unique == 0),
        "the stage status index is missing"
    );
}

fn json_value(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::String(text) => rusqlite::types::Value::Text(text.clone()),
        Value::Number(number) => {
            rusqlite::types::Value::Integer(number.as_i64().expect("a fixture integer"))
        }
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(flag) => rusqlite::types::Value::Integer(i64::from(*flag)),
        other => panic!("a fixture field is a scalar, not {other:?}"),
    }
}

async fn insert_row(database: &SqliteDatabase, table: &str, row: &Map<String, Value>) {
    let columns: Vec<&str> = row.keys().map(String::as_str).collect();
    let placeholders: Vec<String> = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect();
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    let values: Vec<rusqlite::types::Value> = row.values().map(json_value).collect();
    database
        .call(move |connection| connection.execute(&sql, rusqlite::params_from_iter(values.iter())))
        .await
        .expect("the fixture row inserts");
}

async fn select_row(
    database: &SqliteDatabase,
    table: &str,
    key_column: &str,
    key: &str,
) -> Map<String, Value> {
    let sql = format!("SELECT * FROM {table} WHERE {key_column} = ?1");
    let key = key.to_owned();
    database
        .call(move |connection| {
            let mut statement = connection.prepare(&sql)?;
            let names: Vec<String> = statement.column_names().map(str::to_owned).collect();
            let mut rows = statement.query(params![key])?;
            let row = rows.next()?.expect("the fixture row exists");
            let mut selected = Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = match row.get_ref(index)? {
                    ValueRef::Null => Value::Null,
                    ValueRef::Integer(value) => json!(value),
                    ValueRef::Real(value) => json!(value),
                    ValueRef::Text(bytes) => {
                        Value::String(String::from_utf8(bytes.to_vec()).expect("UTF-8 text"))
                    }
                    ValueRef::Blob(_) => panic!("a run field is not a blob"),
                };
                selected.insert(name.clone(), value);
            }
            Ok::<_, rusqlite::Error>(selected)
        })
        .await
        .expect("the fixture row selects")
}

#[tokio::test]
async fn a_representative_run_row_reads_as_the_record_and_the_storage_shape() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&database).await;

    let fixture: Value = serde_json::from_str(FIXTURE).expect("the fixture parses");
    assert_eq!(fixture["contract"], "quantrule.evidence-parity/v1");
    let profile_id = fixture["profile_id"].as_str().expect("the profile id");
    let storage = fixture["storage"].as_object().expect("the storage row");
    let expected: RunRecord =
        serde_json::from_value(fixture["run"].clone()).expect("the expected record");
    let run_id = expected.run_id.to_string();

    insert_row(&database, "runs__runs", storage).await;
    let stages = expected
        .stages
        .iter()
        .map(|stage| {
            json!({
                "run_id": &run_id,
                "name": stage.name.as_str(),
                "state": stage.state.as_str(),
                "elapsed_ms": stage.elapsed_ms,
            })
        })
        .collect::<Vec<_>>();
    for stage in stages {
        insert_row(
            &database,
            "runs__stages",
            stage.as_object().expect("the stage row"),
        )
        .await;
    }
    for artifact in &expected.artifacts {
        let row = json!({
            "run_id": &run_id,
            "name": &artifact.name,
            "bytes": artifact.bytes,
            "sha256": artifact.sha256.as_hex(),
        });
        insert_row(
            &database,
            "runs__artifacts",
            row.as_object().expect("the artifact row"),
        )
        .await;
    }
    let submission = json!({
        "profile_id": profile_id,
        "run_id": &run_id,
        "submitted_at": &expected.submitted_at,
    });
    insert_row(
        &database,
        "runs__submissions",
        submission.as_object().expect("the submission row"),
    )
    .await;

    let selected = select_row(&database, "runs__runs", "run_id", &run_id).await;
    assert_eq!(
        selected,
        storage.clone(),
        "the SQLite storage row differs from the fixture"
    );

    let attempts_kind: String = database
        .call({
            let run_id = run_id.clone();
            move |connection| {
                connection.query_row(
                    "SELECT typeof(attempts) FROM runs__runs WHERE run_id = ?1",
                    params![run_id],
                    |row| row.get(0),
                )
            }
        })
        .await
        .expect("the attempt storage class");
    assert_eq!(attempts_kind, "integer", "attempts is not an INTEGER");

    let store = SqliteRunStore::new(database.clone());
    let profile = ProfileId::parse(profile_id).expect("the parity profile");
    assert_eq!(
        store
            .get(profile.clone(), run_id.clone())
            .await
            .expect("the store read"),
        Some(expected.clone()),
        "the store read differs from the fixture record"
    );
    assert_eq!(
        store
            .list(profile.clone(), None, 10)
            .await
            .expect("the list"),
        vec![expected.summary()],
        "the list projection differs from the record"
    );
    let other = ProfileId::parse("parity-other").expect("the other profile");
    assert_eq!(
        store.get(other, run_id).await.expect("the scoped read"),
        None,
        "the run leaked to another profile"
    );
}

fn sealed() -> SealedHypothesis {
    let split = Split::new(
        Partition::new("dev", 100, 200).expect("the development partition"),
        Partition::new("val", 200, 300).expect("the validation partition"),
    )
    .expect("the split");
    Hypothesis::new(
        "a retest holds",
        "strategy/retest.toml",
        "sha256:abc",
        CostModel::new(1.0, 0.0, 0.0).expect("the costs"),
        split,
        "expectancy above zero",
    )
    .expect("the hypothesis")
    .seal("2026-11-15T00:00:00Z")
    .expect("the seal")
}

#[tokio::test]
async fn a_sealed_hypothesis_refuses_update_and_delete() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&database).await;
    let store = SqliteHypothesisStore::new(database.clone());
    let profile = ProfileId::parse("default").expect("the profile");
    let sealed = sealed();
    let fingerprint = sealed.fingerprint().to_owned();
    store
        .insert(profile.clone(), sealed.clone())
        .await
        .expect("the insert");

    let update = database
        .call({
            let fingerprint = fingerprint.clone();
            move |connection| {
                connection.execute(
                    "UPDATE research__hypotheses SET premise = 'changed' WHERE fingerprint = ?1",
                    params![fingerprint],
                )
            }
        })
        .await
        .expect_err("an update of a sealed row succeeded");
    assert!(
        update.to_string().contains("immutable"),
        "the update refusal does not name immutability: {update}"
    );

    let delete = database
        .call({
            let fingerprint = fingerprint.clone();
            move |connection| {
                connection.execute(
                    "DELETE FROM research__hypotheses WHERE fingerprint = ?1",
                    params![fingerprint],
                )
            }
        })
        .await
        .expect_err("a delete of a sealed row succeeded");
    assert!(
        delete.to_string().contains("immutable"),
        "the delete refusal does not name immutability: {delete}"
    );

    assert_eq!(
        store.load(profile, fingerprint).await.expect("the read"),
        Some(sealed),
        "the sealed row changed or vanished"
    );
}

async fn insert_ledger_row(database: &SqliteDatabase, version: i64) {
    database
        .call(move |connection| {
            connection.execute(
                "INSERT INTO schema_migrations (ledger, version, name, checksum)
                 VALUES ('runs', ?1, 'manual', 'manual')",
                params![version],
            )
        })
        .await
        .expect("the manual ledger row");
}

async fn table_exists(database: &SqliteDatabase, table: &str) -> bool {
    let table = table.to_owned();
    database
        .call(move |connection| {
            connection.query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get::<_, i64>(0),
            )
        })
        .await
        .expect("the table count")
        > 0
}

#[tokio::test]
async fn the_baseline_is_idempotent_and_a_changed_body_is_refused() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    assert_eq!(
        apply_migrations_in(&database, "runs", &migrations::sqlite())
            .await
            .expect("the first apply"),
        4
    );
    assert_eq!(
        apply_migrations_in(&database, "runs", &migrations::sqlite())
            .await
            .expect("the second apply"),
        0
    );

    let mut changed = migrations::sqlite();
    changed[0] = Migration {
        version: 1,
        name: "runs-research",
        sql: "CREATE TABLE probe_changed (id TEXT PRIMARY KEY) STRICT;",
    };
    assert_eq!(
        apply_migrations_in(&database, "runs", &changed)
            .await
            .expect_err("the changed body"),
        MigrationError::ChecksumMismatch { version: 1 }
    );
}

#[tokio::test]
async fn a_database_ahead_or_an_unknown_applied_version_is_refused() {
    let ahead = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&ahead).await;
    insert_ledger_row(&ahead, 5).await;
    assert_eq!(
        apply_migrations_in(&ahead, "runs", &migrations::sqlite())
            .await
            .expect_err("the database is ahead"),
        MigrationError::DatabaseAhead {
            applied: 5,
            available: 4
        }
    );

    let unknown = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&unknown).await;
    insert_ledger_row(&unknown, 5).await;
    let probe = Migration {
        version: 6,
        name: "probe",
        sql: "CREATE TABLE probe_version (id TEXT PRIMARY KEY) STRICT;",
    };
    let mut available = migrations::sqlite();
    available.push(probe);
    assert_eq!(
        apply_migrations_in(&unknown, "runs", &available)
            .await
            .expect_err("the unknown applied version"),
        MigrationError::UnknownApplied(5)
    );
    assert!(
        !table_exists(&unknown, "probe_version").await,
        "the refused plan left a table behind"
    );
    assert_eq!(
        checksum(migrations::sqlite()[0].sql),
        checksum(include_str!("../migrations/sqlite/0001_runs_research.sql")),
        "the baseline checksum is not the checked-in body"
    );
    assert_eq!(
        checksum(migrations::sqlite()[1].sql),
        checksum(include_str!("../migrations/sqlite/0002_stage.sql")),
        "the stage migration checksum is not the checked-in body"
    );
    assert_eq!(
        checksum(migrations::sqlite()[2].sql),
        checksum(include_str!(
            "../migrations/sqlite/0003_run_model_artifact.sql"
        )),
        "the model-artifact migration checksum is not the checked-in body"
    );
    assert_eq!(
        checksum(migrations::sqlite()[3].sql),
        checksum(include_str!(
            "../migrations/sqlite/0004_research_ledger.sql"
        )),
        "the research-ledger migration checksum is not the checked-in body"
    );
}

#[tokio::test]
async fn the_ledger_holds_only_the_evidence_migrations() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    baseline(&database).await;
    let rows: Vec<(String, i64, String)> = database
        .call(|connection| {
            let mut statement = connection
                .prepare("SELECT ledger, version, name FROM schema_migrations ORDER BY version")?;
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
        .expect("the ledger rows");
    assert_eq!(
        rows,
        vec![
            ("runs".to_owned(), 1, "runs-research".to_owned()),
            ("runs".to_owned(), 2, "stage".to_owned()),
            ("runs".to_owned(), 3, "run-model-artifact".to_owned()),
            ("runs".to_owned(), 4, "research-ledger".to_owned()),
        ],
        "the evidence ledger holds another service's rows"
    );
}
