//! The Tauri command boundary.
//!
//! Every command delegates to `quantrule-app`. The views are serializable
//! snapshots of application-service values; the desktop layer never parses a
//! config, computes a reading, or writes SQL.

use std::fs;
use std::path::Path;

use quantrule_app::dataset::Dataset;
use quantrule_app::memory::{
    ClaimRecord, MemoryError, MetricRecord, MetricValue, NewClaim, NewNote, NoteRecord,
    ResearchMemory, RunRecord, WorkspaceRecord,
};
use quantrule_app::service::{
    AppService, ModelRunOutcome, ModelRunSummary, RecordRunInput, RunSummary,
    TimeframeModelSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct DatasetRefView {
    name: String,
    path: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceView {
    root: String,
    name: String,
    datasets: Vec<DatasetRefView>,
    runs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DatasetSummaryView {
    name: String,
    path: String,
    candles: usize,
    first_open_time: Option<i64>,
    last_open_time: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct RunSummaryView {
    name: String,
    output_root: String,
    config_hash: String,
    model_id: Option<String>,
    execution_id: Option<String>,
    portfolio_id: Option<String>,
    instrument: String,
    data_version: String,
    data_source: String,
    data_timeframes: Vec<String>,
    model_timeframes: Vec<String>,
    execution_fill_timeframe: String,
    initial_cash: f64,
    leverage: f64,
    max_open_positions: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeframeView {
    timeframe_secs: u64,
    bars: usize,
    events: usize,
    levels: usize,
}

#[derive(Debug, Deserialize)]
pub struct ModelSummaryView {
    timeframes: Vec<TimeframeView>,
    events: usize,
}

impl From<ModelSummaryView> for ModelRunSummary {
    fn from(view: ModelSummaryView) -> Self {
        Self {
            timeframes: view
                .timeframes
                .into_iter()
                .map(|timeframe| TimeframeModelSummary {
                    timeframe_secs: timeframe.timeframe_secs,
                    bars: timeframe.bars,
                    events: timeframe.events,
                    levels: timeframe.levels,
                })
                .collect(),
            events: view.events,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ModelRunView {
    run: RunSummaryView,
    timeframes: Vec<TimeframeView>,
    events: usize,
}

#[derive(Debug, Serialize)]
pub struct MemoryStatusView {
    schema: String,
    version: i64,
    path: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceRecordView {
    id: i64,
    root: String,
    name: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct RunRecordView {
    id: i64,
    workspace_id: i64,
    config_id: Option<i64>,
    run_key: String,
    data_version: String,
    config_hash: String,
    status: String,
    started_at: i64,
    finished_at: Option<i64>,
    summary_json: String,
}

#[derive(Debug, Serialize)]
pub struct MetricView {
    id: i64,
    run_id: i64,
    name: String,
    kind: String,
    value: Value,
}

#[derive(Debug, Serialize)]
pub struct ClaimView {
    id: i64,
    run_id: i64,
    claim_type: String,
    statement: String,
    status: String,
    created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct NoteView {
    id: i64,
    workspace_id: i64,
    run_id: Option<i64>,
    body: String,
    created_at: i64,
    updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct RecordedRunView {
    workspace_id: i64,
    dataset_id: i64,
    config_id: i64,
    run_id: i64,
}

impl From<RunSummary> for RunSummaryView {
    fn from(summary: RunSummary) -> Self {
        Self {
            name: summary.name,
            output_root: summary.output_root.to_string_lossy().into_owned(),
            config_hash: summary.config_hash,
            model_id: summary.model_id,
            execution_id: summary.execution_id,
            portfolio_id: summary.portfolio_id,
            instrument: summary.instrument,
            data_version: summary.data_version,
            data_source: summary.data_source.to_string_lossy().into_owned(),
            data_timeframes: summary.data_timeframes,
            model_timeframes: summary.model_timeframes,
            execution_fill_timeframe: summary.execution_fill_timeframe,
            initial_cash: summary.initial_cash,
            leverage: summary.leverage,
            max_open_positions: summary.max_open_positions,
        }
    }
}

impl From<WorkspaceRecord> for WorkspaceRecordView {
    fn from(record: WorkspaceRecord) -> Self {
        Self {
            id: record.id,
            root: record.root,
            name: record.name,
            created_at: record.created_at,
        }
    }
}

impl From<RunRecord> for RunRecordView {
    fn from(record: RunRecord) -> Self {
        Self {
            id: record.id,
            workspace_id: record.workspace_id,
            config_id: record.config_id,
            run_key: record.run_key,
            data_version: record.data_version,
            config_hash: record.config_hash,
            status: record.status,
            started_at: record.started_at,
            finished_at: record.finished_at,
            summary_json: record.summary_json,
        }
    }
}

impl From<ClaimRecord> for ClaimView {
    fn from(record: ClaimRecord) -> Self {
        Self {
            id: record.id,
            run_id: record.run_id,
            claim_type: record.claim_type,
            statement: record.statement,
            status: record.status,
            created_at: record.created_at,
        }
    }
}

impl From<NoteRecord> for NoteView {
    fn from(record: NoteRecord) -> Self {
        Self {
            id: record.id,
            workspace_id: record.workspace_id,
            run_id: record.run_id,
            body: record.body,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

fn metric_view(record: MetricRecord) -> MetricView {
    let (kind, value) = match record.value {
        MetricValue::Real(number) => ("real".to_owned(), Value::from(number)),
        MetricValue::Integer(number) => ("integer".to_owned(), Value::from(number)),
        MetricValue::Text(text) => ("text".to_owned(), Value::String(text)),
        MetricValue::Null => ("null".to_owned(), Value::Null),
    };
    MetricView {
        id: record.id,
        run_id: record.run_id,
        name: record.name,
        kind,
        value,
    }
}

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[tauri::command(rename_all = "snake_case")]
pub fn open_workspace(root: String) -> Result<WorkspaceView, String> {
    let service = AppService::open(&root).map_err(text)?;
    let workspace = service.workspace();
    let datasets = workspace
        .datasets()
        .map_err(text)?
        .into_iter()
        .map(|dataset| DatasetRefView {
            name: dataset.name(),
            path: dataset.path().to_string_lossy().into_owned(),
        })
        .collect();
    let runs = workspace
        .run_configs()
        .map_err(text)?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    let name = workspace
        .root()
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| workspace.root().to_string_lossy().into_owned());
    Ok(WorkspaceView {
        root: workspace.root().to_string_lossy().into_owned(),
        name,
        datasets,
        runs,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn dataset_summary(path: String) -> Result<DatasetSummaryView, String> {
    let summary = Dataset::open(&path)
        .and_then(|dataset| dataset.summary())
        .map_err(text)?;
    Ok(DatasetSummaryView {
        name: summary.name,
        path: summary.path.to_string_lossy().into_owned(),
        candles: summary.candles,
        first_open_time: summary.first_open_time,
        last_open_time: summary.last_open_time,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn check_run(config: String) -> Result<RunSummaryView, String> {
    AppService::open_for_config(&config)
        .and_then(|service| service.check_run(&config))
        .map(RunSummaryView::from)
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_model(config: String) -> Result<ModelRunView, String> {
    let outcome: ModelRunOutcome = AppService::open_for_config(&config)
        .and_then(|service| service.run_model(&config))
        .map_err(text)?;
    Ok(ModelRunView {
        run: RunSummaryView::from(outcome.run),
        timeframes: outcome
            .summary
            .timeframes
            .into_iter()
            .map(|timeframe| TimeframeView {
                timeframe_secs: timeframe.timeframe_secs,
                bars: timeframe.bars,
                events: timeframe.events,
                levels: timeframe.levels,
            })
            .collect(),
        events: outcome.summary.events,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn export_run_summary(config: String, out: String) -> Result<RunSummaryView, String> {
    AppService::open_for_config(&config)
        .and_then(|service| service.export_run_summary(&config, &out))
        .map(RunSummaryView::from)
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_status(db: String) -> Result<MemoryStatusView, String> {
    let memory = ResearchMemory::open(&db).map_err(text)?;
    Ok(MemoryStatusView {
        schema: memory.schema().map_err(text)?,
        version: memory.version().map_err(text)?,
        path: db,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_workspaces(db: String) -> Result<Vec<WorkspaceRecordView>, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.workspaces())
        .map(|records| records.into_iter().map(WorkspaceRecordView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_runs(db: String, workspace_id: i64) -> Result<Vec<RunRecordView>, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.runs(workspace_id))
        .map(|records| records.into_iter().map(RunRecordView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_metrics(db: String, run_id: i64) -> Result<Vec<MetricView>, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.metrics(run_id))
        .map(|records| records.into_iter().map(metric_view).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_claims(db: String, run_id: i64) -> Result<Vec<ClaimView>, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.claims(run_id))
        .map(|records| records.into_iter().map(ClaimView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_notes(db: String, workspace_id: i64) -> Result<Vec<NoteView>, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.notes(workspace_id))
        .map(|records| records.into_iter().map(NoteView::from).collect())
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_notes_for_run(db: String, run_id: i64) -> Result<Vec<NoteView>, String> {
    ResearchMemory::open(&db)
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
    let memory = ResearchMemory::open(&db).map_err(text)?;
    memory
        .upsert_metric(run_id, &name, &value)
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_metric_delete(db: String, run_id: i64, name: String) -> Result<bool, String> {
    ResearchMemory::open(&db)
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
    ResearchMemory::open(&db)
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
    ResearchMemory::open(&db)
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
    ResearchMemory::open(&db)
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
    ResearchMemory::open(&db)
        .and_then(|memory| memory.update_note(id, &body, updated_at))
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_note_delete(db: String, id: i64) -> Result<bool, String> {
    ResearchMemory::open(&db)
        .and_then(|memory| memory.delete_note(id))
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn record_run(
    db: String,
    config: String,
    run_key: String,
    status: String,
    started_at: i64,
    finished_at: Option<i64>,
    model: Option<ModelSummaryView>,
) -> Result<RecordedRunView, String> {
    let memory = ResearchMemory::open(&db).map_err(text)?;
    let model = model.map(ModelRunSummary::from);
    let config_path = Path::new(&config);
    let recorded = AppService::open_for_config(&config)
        .and_then(|service| {
            service.record_run_with_model(
                &memory,
                RecordRunInput {
                    config: config_path,
                    run_key: &run_key,
                    status: &status,
                    started_at,
                    finished_at,
                    model: model.as_ref(),
                },
            )
        })
        .map_err(text)?;
    Ok(RecordedRunView {
        workspace_id: recorded.workspace_id,
        dataset_id: recorded.dataset_id,
        config_id: recorded.config_id,
        run_id: recorded.run_id,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_export(db: String, out: String) -> Result<String, String> {
    let memory = ResearchMemory::open(&db).map_err(text)?;
    quantrule_app::memory::export_to_file(&memory, Path::new(&out)).map_err(text)?;
    Ok(out)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_import(db: String, from: String) -> Result<String, String> {
    let document =
        fs::read_to_string(&from).map_err(|error| format!("cannot read {from}: {error}"))?;
    let value: Value =
        serde_json::from_str(&document).map_err(|error| format!("cannot parse {from}: {error}"))?;
    let mut memory = ResearchMemory::open(&db).map_err(text)?;
    quantrule_app::memory::import_json(&mut memory, &value).map_err(text)?;
    Ok(from)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_backup(db: String, out: String) -> Result<String, String> {
    let memory = ResearchMemory::open(&db).map_err(text)?;
    quantrule_app::memory::backup(&memory, Path::new(&out)).map_err(text)?;
    Ok(out)
}

#[tauri::command(rename_all = "snake_case")]
pub fn memory_restore(backup: String, target: String) -> Result<String, String> {
    quantrule_app::memory::restore(Path::new(&backup), Path::new(&target))
        .map(|_| target)
        .map_err(|error: MemoryError| text(error))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use quantrule_app::memory::{MetricRecord, MetricValue};

    use super::{
        check_run, memory_claim_add, memory_claim_delete, memory_claims, memory_export,
        memory_metric_delete, memory_metric_set, memory_metrics, memory_note_add,
        memory_note_delete, memory_note_update, memory_notes_for_run, memory_runs,
        memory_status, memory_workspaces, metric_view, open_workspace, record_run, run_model,
    };

    fn repository_root() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn the_repository_workspace_lists_its_runs() {
        let view = open_workspace(repository_root()).expect("the repository opens");
        assert!(!view.name.is_empty());
        assert!(view
            .runs
            .iter()
            .any(|path| path.ends_with("config/examples/btcusd-2024/run.json")));
    }

    fn fixture_run() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/quantrule-shell/tests/fixtures/config-run/run.json")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn the_run_commands_match_the_service() {
        let summary = check_run(fixture_run()).expect("the fixture validates");
        assert_eq!(summary.name, "fixture");
        assert_eq!(summary.instrument, "EURUSD");
        assert_eq!(summary.data_version, "test:v1");
        assert_eq!(summary.data_timeframes, ["1m", "5m", "15m", "1h"]);
        assert_eq!(summary.model_timeframes, ["1m", "5m", "15m", "1h"]);
        assert_eq!(summary.execution_fill_timeframe, "5m");
        let model = run_model(fixture_run()).expect("the fixture runs");
        assert!(!model.timeframes.is_empty());
        assert!(model.events > 0);
    }

    #[test]
    fn the_memory_commands_record_and_export() {
        let directory =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/desktop-tests/memory");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the test directory is created");
        let db = directory.join("research.db");
        let db = db.to_string_lossy().into_owned();
        let recorded = record_run(
            db.clone(),
            fixture_run(),
            "run-1".to_owned(),
            "complete".to_owned(),
            1_763_000_000,
            Some(1_763_000_060),
            None,
        )
        .expect("the run is recorded");
        assert!(recorded.run_id > 0);
        let status = memory_status(db.clone()).expect("the memory opens");
        assert_eq!(status.schema, "quantrule.research-memory/v1");
        let workspaces = memory_workspaces(db.clone()).expect("the workspaces are read");
        assert_eq!(workspaces.len(), 1);
        let runs = memory_runs(db.clone(), workspaces[0].id).expect("the runs are read");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_key, "run-1");
        assert!(
            memory_notes_for_run(db.clone(), runs[0].id)
                .expect("the run notes are read")
                .is_empty()
        );
        let run_id = runs[0].id;
        assert!(memory_metric_set(
            db.clone(),
            run_id,
            "sharpe".to_owned(),
            "real".to_owned(),
            Some(1.5),
            None
        )
        .expect("the metric is written")
            > 0);
        assert_eq!(
            memory_metrics(db.clone(), run_id)
                .expect("the metrics are read")
                .len(),
            1
        );
        let claim = memory_claim_add(
            db.clone(),
            run_id,
            "Behaviour Claim".to_owned(),
            "the run is deterministic".to_owned(),
            "accepted".to_owned(),
            1,
        )
        .expect("the claim is written");
        assert_eq!(
            memory_claims(db.clone(), run_id)
                .expect("the claims are read")
                .len(),
            1
        );
        let note = memory_note_add(
            db.clone(),
            workspaces[0].id,
            Some(run_id),
            "first review".to_owned(),
            1,
            1,
        )
        .expect("the note is written");
        assert_eq!(
            memory_notes_for_run(db.clone(), run_id)
                .expect("the run notes are read")
                .len(),
            1
        );
        assert!(memory_note_update(db.clone(), note, "updated".to_owned(), 2)
            .expect("the note is updated"));
        assert!(memory_note_delete(db.clone(), note).expect("the note is deleted"));
        assert!(memory_claim_delete(db.clone(), claim).expect("the claim is deleted"));
        assert!(memory_metric_delete(db.clone(), run_id, "sharpe".to_owned())
            .expect("the metric is deleted"));
        let out = directory.join("export.json");
        memory_export(db, out.to_string_lossy().into_owned()).expect("the export is written");
        assert!(out.is_file());
    }

    #[test]
    fn metric_views_preserve_null_and_integer() {
        let null = metric_view(MetricRecord {
            id: 1,
            run_id: 1,
            name: "sharpe".to_owned(),
            value: MetricValue::Null,
        });
        assert_eq!(null.kind, "null");
        assert!(null.value.is_null());

        let integer = metric_view(MetricRecord {
            id: 2,
            run_id: 1,
            name: "trades".to_owned(),
            value: MetricValue::Integer(42),
        });
        assert_eq!(integer.kind, "integer");
        assert_eq!(integer.value, serde_json::json!(42));
    }
}
