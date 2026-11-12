//! Research-memory commands.

use std::fs;

use quantrule_app::memory::{MemoryError, MetricValue, NewClaim, NewNote, ResearchMemory};
use serde_json::Value;

use super::scope::{open_memory, set_memory, text, validate_output_path};
use super::views::{
    ClaimView, MemoryStatusView, MetricView, NoteView, RunRecordView, WorkspaceRecordView,
    metric_view,
};

#[tauri::command(rename_all = "snake_case")]
pub fn memory_status(db: String) -> Result<MemoryStatusView, String> {
    let memory = ResearchMemory::open(&db).map_err(text)?;
    let canonical = fs::canonicalize(&db).map_err(text)?;
    set_memory(canonical)?;
    Ok(MemoryStatusView {
        schema: memory.schema().map_err(text)?,
        version: memory.version().map_err(text)?,
        path: db,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_workspaces(db: String) -> Result<Vec<WorkspaceRecordView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.workspaces())
        .map(|records| records.into_iter().map(WorkspaceRecordView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_runs(db: String, workspace_id: i64) -> Result<Vec<RunRecordView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.runs(workspace_id))
        .map(|records| records.into_iter().map(RunRecordView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_metrics(db: String, run_id: i64) -> Result<Vec<MetricView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.metrics(run_id))
        .map(|records| records.into_iter().map(metric_view).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_claims(db: String, run_id: i64) -> Result<Vec<ClaimView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.claims(run_id))
        .map(|records| records.into_iter().map(ClaimView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_notes(db: String, workspace_id: i64) -> Result<Vec<NoteView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.notes(workspace_id))
        .map(|records| records.into_iter().map(NoteView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_notes_for_run(db: String, run_id: i64) -> Result<Vec<NoteView>, String> {
    open_memory(&db)
        .and_then(|memory| memory.notes_for_run(run_id))
        .map(|records| records.into_iter().map(NoteView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_metric_set(
    db: String,
    run_id: i64,
    name: String,
    kind: String,
    value: Option<f64>,
    text_value: Option<String>,
) -> Result<i64, String> {
    let value = match kind.as_str() {
        "real" => {
            let number = value.ok_or_else(|| String::from("a real metric requires a value"))?;
            if !number.is_finite() {
                return Err(String::from("a real metric value must be finite"));
            }
            MetricValue::Real(number)
        }
        "integer" => {
            let text_value =
                text_value.ok_or_else(|| String::from("an integer metric requires text_value"))?;
            MetricValue::Integer(
                text_value
                    .parse()
                    .map_err(|_| String::from("the integer metric text is not an integer"))?,
            )
        }
        "text" => MetricValue::Text(
            text_value.ok_or_else(|| String::from("a text metric requires text_value"))?,
        ),
        "null" => MetricValue::Null,
        other => return Err(format!("metric kind {other} is not supported")),
    };
    let memory = open_memory(&db).map_err(text)?;
    memory
        .upsert_metric(run_id, &name, &value)
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_metric_delete(db: String, run_id: i64, name: String) -> Result<bool, String> {
    open_memory(&db)
        .and_then(|memory| memory.delete_metric(run_id, &name))
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_claim_add(
    db: String,
    run_id: i64,
    claim_type: String,
    statement: String,
    status: String,
    created_at: i64,
) -> Result<i64, String> {
    open_memory(&db)
        .and_then(|memory| {
            memory.insert_claim(NewClaim {
                run_id,
                claim_type: &claim_type,
                statement: &statement,
                status: &status,
                created_at,
            })
        })
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_claim_delete(db: String, id: i64) -> Result<bool, String> {
    open_memory(&db)
        .and_then(|memory| memory.delete_claim(id))
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_note_add(
    db: String,
    workspace_id: i64,
    run_id: Option<i64>,
    body: String,
    created_at: i64,
    updated_at: i64,
) -> Result<i64, String> {
    open_memory(&db)
        .and_then(|memory| {
            memory.insert_note(NewNote {
                workspace_id,
                run_id,
                body: &body,
                created_at,
                updated_at,
            })
        })
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_note_update(
    db: String,
    id: i64,
    body: String,
    updated_at: i64,
) -> Result<bool, String> {
    open_memory(&db)
        .and_then(|memory| memory.update_note(id, &body, updated_at))
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_note_delete(db: String, id: i64) -> Result<bool, String> {
    open_memory(&db)
        .and_then(|memory| memory.delete_note(id))
        .map_err(text)
}


#[tauri::command(rename_all = "snake_case")]
pub fn memory_export(db: String, out: String) -> Result<String, String> {
    let memory = open_memory(&db).map_err(text)?;
    let out = validate_output_path(&out)?;
    quantrule_app::memory::export_to_file(&memory, &out).map_err(text)?;
    Ok(out.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_import(db: String, from: String) -> Result<String, String> {
    let from = validate_output_path(&from)?;
    let document =
        fs::read_to_string(&from).map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    let value: Value = serde_json::from_str(&document)
        .map_err(|error| format!("cannot parse {}: {error}", from.display()))?;
    let mut memory = open_memory(&db).map_err(text)?;
    quantrule_app::memory::import_json(&mut memory, &value).map_err(text)?;
    Ok(from.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_backup(db: String, out: String) -> Result<String, String> {
    let memory = open_memory(&db).map_err(text)?;
    let out = validate_output_path(&out)?;
    quantrule_app::memory::backup(&memory, &out).map_err(text)?;
    Ok(out.to_string_lossy().into_owned())
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_restore(backup: String, target: String) -> Result<String, String> {
    let backup = validate_output_path(&backup)?;
    let target = validate_output_path(&target)?;
    quantrule_app::memory::restore(&backup, &target)
        .map(|_| target.to_string_lossy().into_owned())
        .map_err(|error: MemoryError| text(error))
}
