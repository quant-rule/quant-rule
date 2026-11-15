//! Gateway schema and row parity between the Postgres and SQLite profiles.
//!
//! The fast tier has no Postgres. The suite derives the released
//! `gateway.profiles` shape from the checked-in Postgres migration and compares
//! it with the SQLite baseline the gateway applies: every column in position and
//! every mapped type. It then writes one profile row in the SQLite storage shape
//! and in the Postgres wire shape, normalizes both, and compares them with one
//! recorded row. A live Postgres method suite is the opt-in
//! `profile_conformance.rs` test.

use std::fs;
use std::path::Path;

use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_runtime::adapters::{Dialect, Schema};
use rusqlite::types::ValueRef;
use serde_json::{Map, Value, json};

const POSTGRES: &str = "crates/quantrule-gateway/migrations/0007_profiles.sql";
const SQLITE: &str = "crates/quantrule-gateway/migrations/sqlite/0001_profiles.sql";
const FIXTURE: &str = include_str!("fixtures/profile_parity.json");

fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn postgres_columns() -> Vec<(String, String)> {
    let text = fs::read_to_string(root().join(POSTGRES)).expect("the Postgres migration reads");
    let mut columns = Vec::new();
    let mut inside = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with("CREATE TABLE IF NOT EXISTS gateway.profiles (") {
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
    columns
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

async fn sqlite_columns(database: &SqliteDatabase) -> Vec<(String, String)> {
    database
        .call(|connection| {
            let mut statement = connection.prepare("PRAGMA table_info(gateway__profiles)")?;
            let rows = statement.query_map([], |row| {
                Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()
        })
        .await
        .expect("the SQLite column list")
}

#[tokio::test]
async fn the_gateway_baseline_matches_the_postgres_table() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "gateway",
        &quantrule_gateway::migrations::sqlite(),
    )
    .await
    .expect("the gateway baseline");
    assert_eq!(sqlite_columns(&database).await, postgres_columns());

    assert_eq!(
        Dialect::Sqlite
            .table(Schema::Gateway, "profiles")
            .expect("the SQLite name"),
        "gateway__profiles"
    );
    assert_eq!(
        Dialect::Postgres
            .table(Schema::Gateway, "profiles")
            .expect("the Postgres name"),
        "gateway.profiles"
    );

    let baseline =
        statements(&fs::read_to_string(root().join(SQLITE)).expect("the SQLite baseline reads"));
    assert!(baseline.contains("gateway__profiles"));
    assert!(
        !baseline.contains("gateway_profiles"),
        "the single-underscore prefix is not a SQLite name"
    );
    assert!(
        !baseline.contains("gateway.profiles"),
        "a dotted Postgres name is not a SQLite name"
    );
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
async fn a_profile_row_normalizes_the_same_on_both_shapes() {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "gateway",
        &quantrule_gateway::migrations::sqlite(),
    )
    .await
    .expect("the gateway baseline");

    let fixture: Value = serde_json::from_str(FIXTURE).expect("the fixture parses");
    assert_eq!(fixture["contract"], "quantrule.gateway-parity/v1");

    let sqlite_row = fixture["sqlite"].as_object().expect("the SQLite row");
    insert_row(&database, sqlite_row).await;
    let selected = select_row(&database, "conformance-profile").await;
    let normalized_sqlite = normalize(&selected);

    let postgres_row = fixture["postgres"].as_object().expect("the Postgres row");
    let normalized_postgres = normalize(postgres_row);

    let expected = fixture["normalized"]
        .as_object()
        .expect("the normalized row");
    assert_eq!(normalized_sqlite, expected.clone(), "SQLite row drift");
    assert_eq!(
        normalized_postgres,
        expected.clone(),
        "Postgres wire row drift"
    );
}

async fn insert_row(database: &SqliteDatabase, row: &Map<String, Value>) {
    let columns: Vec<&str> = row.keys().map(String::as_str).collect();
    let placeholders: Vec<String> = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect();
    let sql = format!(
        "INSERT INTO gateway__profiles ({}) VALUES ({})",
        columns.join(", "),
        placeholders.join(", ")
    );
    let values: Vec<rusqlite::types::Value> = row
        .iter()
        .map(|(_, value)| match value {
            Value::String(text) => rusqlite::types::Value::Text(text.clone()),
            Value::Null => rusqlite::types::Value::Null,
            other => panic!("a profile field is text or null: {other:?}"),
        })
        .collect();
    database
        .call(move |connection| connection.execute(&sql, rusqlite::params_from_iter(values.iter())))
        .await
        .expect("the profile row inserts");
}

async fn select_row(database: &SqliteDatabase, profile_id: &str) -> Map<String, Value> {
    let profile_id = profile_id.to_owned();
    database
        .call(move |connection| {
            let mut statement = connection.prepare(
                "SELECT profile_id, name, created_at
                   FROM gateway__profiles
                  WHERE profile_id = ?1",
            )?;
            let names: Vec<String> = statement.column_names().map(str::to_owned).collect();
            let mut rows = statement.query(rusqlite::params![profile_id])?;
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
                    ValueRef::Blob(_) => panic!("a profile field is not a blob"),
                };
                selected.insert(name.clone(), value);
            }
            Ok::<_, rusqlite::Error>(selected)
        })
        .await
        .expect("the profile row selects")
}

fn normalize(row: &Map<String, Value>) -> Map<String, Value> {
    row.iter()
        .map(|(name, value)| {
            let normalized = if name == "created_at" {
                let text = value.as_str().expect("a timestamp string");
                Value::String(canonical_instant(text).expect("an RFC 3339 instant"))
            } else {
                value.clone()
            };
            (name.clone(), normalized)
        })
        .collect()
}

fn canonical_instant(value: &str) -> Result<String, String> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() < 19 {
        return Err(format!("{value:?} is shorter than an instant"));
    }
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || (bytes[10] != b'T' && bytes[10] != b' ')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err(format!("{value:?} is not a date and time"));
    }
    for index in [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18] {
        if !bytes[index].is_ascii_digit() {
            return Err(format!("{value:?} has a non-numeric field"));
        }
    }
    let suffix = &value[19..];
    if !matches!(suffix, "" | "Z" | "+00" | "+00:00" | "-00" | "-00:00") {
        return Err(format!("{value:?} is not UTC"));
    }
    Ok(format!("{}T{}Z", &value[..10], &value[11..19]))
}

#[cfg(test)]
mod tests {
    use super::canonical_instant;

    #[test]
    fn an_rfc3339_and_a_postgres_instant_canonicalize_alike() {
        assert_eq!(
            canonical_instant("2026-11-15T00:00:00Z").expect("rfc3339"),
            "2026-11-15T00:00:00Z"
        );
        assert_eq!(
            canonical_instant("2026-11-15 00:00:00+00").expect("postgres"),
            "2026-11-15T00:00:00Z"
        );
    }

    #[test]
    fn a_non_utc_or_malformed_instant_is_refused() {
        for value in [
            "",
            "2026-11-15",
            "2026-11-15T00:00:00+02:00",
            "2026-11-15 00:00:00.123+00",
            "not an instant",
        ] {
            assert!(canonical_instant(value).is_err(), "{value:?} was accepted");
        }
    }
}
