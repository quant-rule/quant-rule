//! Logical export/import and file backup/restore.
//!
//! Export is a versioned JSON document ordered by stable IDs. Import accepts
//! that document only into an empty memory, so a merge can never silently
//! overwrite a record. Backup uses SQLite's `VACUUM INTO`, which writes one
//! consistent database file; restore copies that file to a new path and opens
//! it through the same migration and integrity checks.

use std::fs;
use std::path::Path;

use rusqlite::params;
use serde_json::{Map, Value, json};

use super::store::ResearchMemory;
use super::types::{MemoryError, MetricValue};

/// The logical export schema.
pub const EXPORT_SCHEMA: &str = "quantrule.research-memory-export/v1";

/// Builds the ordered export value for one memory.
///
/// # Errors
///
/// Returns [`MemoryError`] when a query fails.
pub fn export_json(memory: &ResearchMemory) -> Result<Value, MemoryError> {
    let mut workspaces = Vec::new();
    let mut datasets = Vec::new();
    let mut configs = Vec::new();
    let mut runs = Vec::new();
    let mut metrics = Vec::new();
    let mut claims = Vec::new();
    let mut notes = Vec::new();

    for workspace in memory.workspaces()? {
        workspaces.push(json!({
            "id": workspace.id,
            "root": workspace.root,
            "name": workspace.name,
            "created_at": workspace.created_at,
        }));
        for dataset in memory.datasets(workspace.id)? {
            datasets.push(json!({
                "id": dataset.id,
                "workspace_id": dataset.workspace_id,
                "name": dataset.name,
                "path": dataset.path,
                "data_version": dataset.data_version,
                "instrument": dataset.instrument,
                "candles": dataset.candles,
                "first_open_time": dataset.first_open_time,
                "last_open_time": dataset.last_open_time,
                "created_at": dataset.created_at,
            }));
        }
        for config in memory.configs(workspace.id)? {
            configs.push(json!({
                "id": config.id,
                "workspace_id": config.workspace_id,
                "name": config.name,
                "path": config.path,
                "config_hash": config.config_hash,
                "schema_version": config.schema_version,
                "created_at": config.created_at,
            }));
        }
        for run in memory.runs(workspace.id)? {
            runs.push(json!({
                "id": run.id,
                "workspace_id": run.workspace_id,
                "config_id": run.config_id,
                "run_key": run.run_key,
                "data_version": run.data_version,
                "config_hash": run.config_hash,
                "status": run.status,
                "started_at": run.started_at,
                "finished_at": run.finished_at,
                "summary_json": run.summary_json,
            }));
            for metric in memory.metrics(run.id)? {
                let (kind, value, text) = match metric.value {
                    MetricValue::Real(number) => ("real", Some(number), None),
                    MetricValue::Integer(number) => ("integer", None, Some(number.to_string())),
                    MetricValue::Text(text) => ("text", None, Some(text)),
                    MetricValue::Null => ("null", None, None),
                };
                metrics.push(json!({
                    "id": metric.id,
                    "run_id": metric.run_id,
                    "name": metric.name,
                    "kind": kind,
                    "value": value,
                    "text_value": text,
                }));
            }
            for claim in memory.claims(run.id)? {
                claims.push(json!({
                    "id": claim.id,
                    "run_id": claim.run_id,
                    "claim_type": claim.claim_type,
                    "statement": claim.statement,
                    "status": claim.status,
                    "created_at": claim.created_at,
                }));
            }
        }
        for note in memory.notes(workspace.id)? {
            notes.push(json!({
                "id": note.id,
                "workspace_id": note.workspace_id,
                "run_id": note.run_id,
                "body": note.body,
                "created_at": note.created_at,
                "updated_at": note.updated_at,
            }));
        }
    }

    Ok(json!({
        "schema": EXPORT_SCHEMA,
        "workspaces": workspaces,
        "datasets": datasets,
        "configs": configs,
        "runs": runs,
        "metrics": metrics,
        "claims": claims,
        "notes": notes,
    }))
}

/// Serialises the export as pretty JSON with one trailing newline.
///
/// # Errors
///
/// Returns [`MemoryError::Json`] when the value cannot be serialised.
pub fn export_bytes(memory: &ResearchMemory) -> Result<Vec<u8>, MemoryError> {
    let value = export_json(memory)?;
    let mut bytes = serde_json::to_vec_pretty(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Writes the export to a file, replacing an existing file.
///
/// # Errors
///
/// Returns [`MemoryError::Io`] when the file cannot be written, and
/// [`MemoryError::Json`] when the document cannot be built.
pub fn export_to_file(memory: &ResearchMemory, path: &Path) -> Result<(), MemoryError> {
    let bytes = export_bytes(memory)?;
    fs::write(path, bytes).map_err(|source| MemoryError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Imports a logical export into an empty memory.
///
/// The whole import runs in one transaction. A rejected record leaves the
/// target empty.
///
/// # Errors
///
/// Returns [`MemoryError::InvalidExport`] for a bad shape,
/// [`MemoryError::NonEmptyTarget`] when the target already holds data,
/// [`MemoryError::InvalidRecord`] for an invalid record, and
/// [`MemoryError::Database`] when SQLite rejects a write.
pub fn import_json(memory: &mut ResearchMemory, value: &Value) -> Result<(), MemoryError> {
    let schema = value
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_export("missing schema"))?;
    if schema != EXPORT_SCHEMA {
        return Err(invalid_export(format!(
            "schema {schema} is not {EXPORT_SCHEMA}"
        )));
    }
    if !memory.is_empty()? {
        return Err(MemoryError::NonEmptyTarget);
    }

    let workspaces = array(value, "workspaces")?;
    let datasets = array(value, "datasets")?;
    let configs = array(value, "configs")?;
    let runs = array(value, "runs")?;
    let metrics = array(value, "metrics")?;
    let claims = array(value, "claims")?;
    let notes = array(value, "notes")?;

    let transaction = memory.connection_mut().transaction()?;

    for record in workspaces {
        let record = object(record, "workspace")?;
        transaction.execute(
            "INSERT INTO workspaces (id, root, name, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                positive_id(record, "id")?,
                non_empty(string(record, "root")?, "workspace.root")?,
                non_empty(string(record, "name")?, "workspace.name")?,
                integer(record, "created_at")?
            ],
        )?;
    }

    for record in datasets {
        let record = object(record, "dataset")?;
        transaction.execute(
            "INSERT INTO datasets
                 (id, workspace_id, name, path, data_version, instrument, candles,
                  first_open_time, last_open_time, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                positive_id(record, "id")?,
                positive_id(record, "workspace_id")?,
                non_empty(string(record, "name")?, "dataset.name")?,
                non_empty(string(record, "path")?, "dataset.path")?,
                non_empty(string(record, "data_version")?, "dataset.data_version")?,
                non_empty(string(record, "instrument")?, "dataset.instrument")?,
                non_negative(integer(record, "candles")?, "dataset.candles")?,
                optional_integer(record, "first_open_time")?,
                optional_integer(record, "last_open_time")?,
                integer(record, "created_at")?
            ],
        )?;
    }

    for record in configs {
        let record = object(record, "config")?;
        transaction.execute(
            "INSERT INTO configs
                 (id, workspace_id, name, path, config_hash, schema_version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                positive_id(record, "id")?,
                positive_id(record, "workspace_id")?,
                non_empty(string(record, "name")?, "config.name")?,
                non_empty(string(record, "path")?, "config.path")?,
                non_empty(string(record, "config_hash")?, "config.config_hash")?,
                non_empty(string(record, "schema_version")?, "config.schema_version")?,
                integer(record, "created_at")?
            ],
        )?;
    }

    for record in runs {
        let record = object(record, "run")?;
        transaction.execute(
            "INSERT INTO runs
                 (id, workspace_id, config_id, run_key, data_version, config_hash,
                  status, started_at, finished_at, summary_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                positive_id(record, "id")?,
                positive_id(record, "workspace_id")?,
                optional_positive_id(record, "config_id")?,
                non_empty(string(record, "run_key")?, "run.run_key")?,
                non_empty(string(record, "data_version")?, "run.data_version")?,
                non_empty(string(record, "config_hash")?, "run.config_hash")?,
                non_empty(string(record, "status")?, "run.status")?,
                integer(record, "started_at")?,
                optional_integer(record, "finished_at")?,
                json_text(record, "summary_json")?
            ],
        )?;
    }

    for record in metrics {
        let record = object(record, "metric")?;
        let kind = string(record, "kind")?;
        let value = optional_real(record, "value")?;
        let text = optional_string(record, "text_value")?;
        match kind.as_str() {
            "real" if value.is_none() => {
                return Err(invalid_export("real metric has no value"));
            }
            "integer" if text.is_none() => {
                return Err(invalid_export("integer metric has no text_value"));
            }
            "text" if text.is_none() => {
                return Err(invalid_export("text metric has no text_value"));
            }
            "real" | "integer" | "text" | "null" => {}
            other => {
                return Err(invalid_export(format!(
                    "metric kind {other} is not supported"
                )));
            }
        }
        transaction.execute(
            "INSERT INTO metrics (id, run_id, name, kind, value, text_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                integer(record, "id")?,
                integer(record, "run_id")?,
                string(record, "name")?,
                kind,
                value,
                text
            ],
        )?;
    }

    for record in claims {
        let record = object(record, "claim")?;
        transaction.execute(
            "INSERT INTO claims (id, run_id, claim_type, statement, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                positive_id(record, "id")?,
                positive_id(record, "run_id")?,
                non_empty(string(record, "claim_type")?, "claim.claim_type")?,
                non_empty(string(record, "statement")?, "claim.statement")?,
                non_empty(string(record, "status")?, "claim.status")?,
                integer(record, "created_at")?
            ],
        )?;
    }

    for record in notes {
        let record = object(record, "note")?;
        transaction.execute(
            "INSERT INTO notes (id, workspace_id, run_id, body, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                positive_id(record, "id")?,
                positive_id(record, "workspace_id")?,
                optional_positive_id(record, "run_id")?,
                non_empty(string(record, "body")?, "note.body")?,
                integer(record, "created_at")?,
                integer(record, "updated_at")?
            ],
        )?;
    }

    transaction.commit()?;
    memory.verify_integrity()
}

/// Writes a consistent SQLite backup. The target must not exist.
///
/// # Errors
///
/// Returns [`MemoryError::InvalidRecord`] when the target exists, and
/// [`MemoryError::Database`] or [`MemoryError::Io`] when the backup fails.
pub fn backup(memory: &ResearchMemory, path: &Path) -> Result<(), MemoryError> {
    if path.exists() {
        return Err(MemoryError::TargetExists {
            path: path.to_path_buf(),
        });
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| MemoryError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let target = path.to_str().ok_or_else(|| {
        MemoryError::InvalidRecord(String::from("backup path is not valid UTF-8"))
    })?;
    memory.connection().execute("VACUUM INTO ?1", [target])?;
    Ok(())
}

/// Restores a backup file to a new database path and opens it.
///
/// The target must not exist. The restored memory runs the schema check and the
/// integrity check before it is returned.
///
/// # Errors
///
/// Returns [`MemoryError::InvalidRecord`] when the target exists,
/// [`MemoryError::Io`] when the copy fails, and the open errors of
/// [`ResearchMemory::open`].
pub fn restore(backup: &Path, target: &Path) -> Result<ResearchMemory, MemoryError> {
    if target.exists() {
        return Err(MemoryError::TargetExists {
            path: target.to_path_buf(),
        });
    }
    if !backup.is_file() {
        return Err(MemoryError::Io {
            path: backup.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "backup file does not exist"),
        });
    }
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|source| MemoryError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    if let Err(source) = fs::copy(backup, target) {
        let _ = fs::remove_file(target);
        return Err(MemoryError::Io {
            path: target.to_path_buf(),
            source,
        });
    }
    let memory = match ResearchMemory::open(target) {
        Ok(memory) => memory,
        Err(error) => {
            let _ = fs::remove_file(target);
            return Err(error);
        }
    };
    if let Err(error) = memory.verify_integrity() {
        drop(memory);
        let _ = fs::remove_file(target);
        return Err(error);
    }
    Ok(memory)
}

fn positive_id(record: &Map<String, Value>, key: &str) -> Result<i64, MemoryError> {
    let value = integer(record, key)?;
    if value <= 0 {
        return Err(invalid_export(format!("{key} must be positive")));
    }
    Ok(value)
}

fn optional_positive_id(
    record: &Map<String, Value>,
    key: &str,
) -> Result<Option<i64>, MemoryError> {
    match optional_integer(record, key)? {
        Some(value) if value > 0 => Ok(Some(value)),
        Some(_) => Err(invalid_export(format!("{key} must be positive"))),
        None => Ok(None),
    }
}

fn non_empty(value: String, field: &str) -> Result<String, MemoryError> {
    if value.trim().is_empty() {
        return Err(invalid_export(format!("{field} must not be empty")));
    }
    Ok(value)
}

fn non_negative(value: i64, field: &str) -> Result<i64, MemoryError> {
    if value < 0 {
        return Err(invalid_export(format!("{field} must not be negative")));
    }
    Ok(value)
}

fn json_text(record: &Map<String, Value>, key: &str) -> Result<String, MemoryError> {
    let text = string(record, key)?;
    serde_json::from_str::<Value>(&text)
        .map_err(|error| invalid_export(format!("{key} is not JSON: {error}")))?;
    Ok(text)
}

fn invalid_export(reason: impl Into<String>) -> MemoryError {
    MemoryError::InvalidExport(reason.into())
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, MemoryError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_export(format!("{key} must be an array")))
}

fn object<'a>(value: &'a Value, label: &str) -> Result<&'a Map<String, Value>, MemoryError> {
    value
        .as_object()
        .ok_or_else(|| invalid_export(format!("{label} must be an object")))
}

fn string(record: &Map<String, Value>, key: &str) -> Result<String, MemoryError> {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid_export(format!("{key} must be a string")))
}

fn optional_string(record: &Map<String, Value>, key: &str) -> Result<Option<String>, MemoryError> {
    match record.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.clone())),
        Some(_) => Err(invalid_export(format!("{key} must be a string or null"))),
    }
}

fn integer(record: &Map<String, Value>, key: &str) -> Result<i64, MemoryError> {
    record
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| invalid_export(format!("{key} must be an integer")))
}

fn optional_integer(record: &Map<String, Value>, key: &str) -> Result<Option<i64>, MemoryError> {
    match record.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| invalid_export(format!("{key} must be an integer or null"))),
        Some(_) => Err(invalid_export(format!("{key} must be an integer or null"))),
    }
}

fn optional_real(record: &Map<String, Value>, key: &str) -> Result<Option<f64>, MemoryError> {
    match record.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_f64()
            .map(Some)
            .ok_or_else(|| invalid_export(format!("{key} must be a number or null"))),
        Some(_) => Err(invalid_export(format!("{key} must be a number or null"))),
    }
}
