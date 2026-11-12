//! Workspace, dataset, run, and export commands.

use quantrule_app::dataset::Dataset;
use quantrule_app::service::{
    AppService, ModelRunOutcome, ModelRunSummary, RecordRunInput,
};

use super::scope::{
    open_memory, set_workspace, text, validate_output_path, validate_workspace_path,
};
use super::views::{
    DatasetRefView, DatasetSummaryView, ModelRunView, ModelSummaryView, RecordedRunView,
    RunSummaryView, TimeframeView, WorkspaceView,
};

#[tauri::command(rename_all = "snake_case")]
pub fn open_workspace(root: String) -> Result<WorkspaceView, String> {
    let service = AppService::open(&root).map_err(text)?;
    set_workspace(service.workspace().root().to_path_buf())?;
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
    let path = validate_workspace_path(&path, "dataset")?;
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
    let config = validate_workspace_path(&config, "run config")?;
    AppService::open_for_config(&config)
        .and_then(|service| service.check_run(&config))
        .map(RunSummaryView::from)
        .map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_model(config: String) -> Result<ModelRunView, String> {
    let config = validate_workspace_path(&config, "run config")?;
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
    let config = validate_workspace_path(&config, "run config")?;
    let out = validate_output_path(&out)?;
    AppService::open_for_config(&config)
        .and_then(|service| service.export_run_summary(&config, &out))
        .map(RunSummaryView::from)
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
    let config = validate_workspace_path(&config, "run config")?;
    let memory = open_memory(&db).map_err(text)?;
    let model = model.map(ModelRunSummary::from);
    let recorded = AppService::open_for_config(&config)
        .and_then(|service| {
            service.record_run_with_model(
                &memory,
                RecordRunInput {
                    config: config.as_path(),
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

