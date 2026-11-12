//! Pipeline-stage commands over the shared application service.
//!
//! Each command validates the webview paths, calls one `AppService` use case,
//! and returns a serializable summary. No command parses a config, computes a
//! reading, or writes an artifact.

use quantrule_app::service::AppService;
use quantrule_app::stages::analytics::AnalyticsRequest;
use quantrule_app::stages::backtest::BacktestRequest;
use quantrule_app::stages::execution::ExecutionRequest;
use quantrule_app::stages::model::ModelReportRequest;
use quantrule_app::stages::portfolio::PortfolioRequest;
use quantrule_app::stages::qualification::QualificationRequest;
use quantrule_app::stages::strategy::{
    Side, StrategyExplainRequest, StrategyPosition, StrategyRunRequest,
};
use serde::Serialize;

use super::scope::{text, validate_output_path, validate_workspace_path};

#[derive(Debug, Serialize)]
pub struct CandleInspectionView {
    candles: usize,
}

#[derive(Debug, Serialize)]
pub struct TimeframeInspectionView {
    timeframes: Vec<TimeframeCountView>,
}

#[derive(Debug, Serialize)]
pub struct TimeframeCountView {
    timeframe: String,
    candles: usize,
}

#[derive(Debug, Serialize)]
pub struct EventBatchView {
    events: usize,
    timeframes: Vec<TimeframeCountView>,
}

#[derive(Debug, Serialize)]
pub struct BundleView {
    root: String,
    artifacts: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BacktestView {
    root: String,
    manifest: String,
    stages: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QualificationView {
    root: String,
    qualification: String,
    verdict: String,
    summary: String,
}

#[derive(Debug, Serialize)]
pub struct AnalyticsView {
    root: String,
    metrics: String,
    tables: String,
    summary: String,
}

#[tauri::command(rename_all = "snake_case")]
pub fn inspect_candles(path: String) -> Result<CandleInspectionView, String> {
    let path = validate_workspace_path(&path, "dataset")?;
    let window = AppService::inspect_candles(&path).map_err(text)?;
    Ok(CandleInspectionView {
        candles: window.len(),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn inspect_candles_tolerant(path: String) -> Result<CandleInspectionView, String> {
    let path = validate_workspace_path(&path, "dataset")?;
    let loaded = AppService::inspect_candles_tolerant(&path).map_err(text)?;
    Ok(CandleInspectionView {
        candles: loaded.window.len(),
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn inspect_timeframes(path: String) -> Result<TimeframeInspectionView, String> {
    let path = validate_workspace_path(&path, "dataset")?;
    let set = AppService::inspect_timeframes(&path).map_err(text)?;
    let timeframes = set
        .series_iter()
        .map(|series| TimeframeCountView {
            timeframe: series.timeframe().code().to_owned(),
            candles: series.len(),
        })
        .collect();
    Ok(TimeframeInspectionView { timeframes })
}

#[tauri::command(rename_all = "snake_case")]
pub fn compute_events(path: String, window: String) -> Result<EventBatchView, String> {
    let path = validate_workspace_path(&path, "dataset")?;
    let mode = quantrule_app::pipeline::WindowMode::from_code(&window)
        .ok_or_else(|| format!("unknown window '{window}'; use full or latest"))?;
    let run = AppService::compute_events(&path, mode).map_err(text)?;
    let timeframes = run
        .timeframes()
        .iter()
        .map(|timeframe| TimeframeCountView {
            timeframe: timeframe.timeframe_secs().to_string(),
            candles: timeframe.len(),
        })
        .collect();
    Ok(EventBatchView {
        events: run.timeframes().iter().map(|timeframe| timeframe.all_events().len()).sum(),
        timeframes,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_model_report(
    config: String,
    spec: Option<String>,
    out: Option<String>,
) -> Result<BundleView, String> {
    let config = validate_workspace_path(&config, "run config")?;
    let spec = spec
        .map(|path| validate_workspace_path(&path, "model spec"))
        .transpose()?;
    let out = out
        .map(|path| validate_output_path(&path))
        .transpose()?;
    let request = ModelReportRequest {
        config: &config,
        spec: spec.as_deref(),
        out: out.as_deref(),
    };
    let bundle = AppService::run_model_report(&request).map_err(text)?;
    Ok(BundleView {
        root: bundle.path.to_string_lossy().into_owned(),
        artifacts: bundle.names,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn explain_strategy(
    model: String,
    strategy: String,
    bar: usize,
    position: Option<String>,
) -> Result<serde_json::Value, String> {
    let model = validate_workspace_path(&model, "model bundle")?;
    let strategy = validate_workspace_path(&strategy, "strategy config")?;
    let position = parse_position(position.as_deref())?;
    let request = StrategyExplainRequest {
        model: &model,
        strategy: &strategy,
        bar,
        position,
    };
    AppService::explain_strategy(&request).map_err(text)
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_strategy(
    model: String,
    strategy: String,
    out: Option<String>,
) -> Result<BundleView, String> {
    let model = validate_workspace_path(&model, "model bundle")?;
    let strategy = validate_workspace_path(&strategy, "strategy config")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = StrategyRunRequest {
        model: &model,
        strategy: &strategy,
        out: out.as_deref(),
    };
    let outcome = AppService::run_strategy(&request).map_err(text)?;
    Ok(BundleView {
        root: outcome.root.to_string_lossy().into_owned(),
        artifacts: outcome.artifacts,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_execution(
    decisions: String,
    config: String,
    out: Option<String>,
) -> Result<BundleView, String> {
    let decisions = validate_workspace_path(&decisions, "strategy bundle")?;
    let config = validate_workspace_path(&config, "run config")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = ExecutionRequest {
        config: &config,
        decisions: &decisions,
        out: out.as_deref(),
    };
    let outcome = AppService::run_execution(&request).map_err(text)?;
    Ok(BundleView {
        root: outcome.root.to_string_lossy().into_owned(),
        artifacts: outcome.artifacts,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn replay_portfolio(
    fills: String,
    config: String,
    out: Option<String>,
) -> Result<BundleView, String> {
    let fills = validate_workspace_path(&fills, "execution bundle")?;
    let config = validate_workspace_path(&config, "run config")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = PortfolioRequest {
        config: &config,
        fills: &fills,
        out: out.as_deref(),
    };
    let outcome = AppService::replay_portfolio(&request).map_err(text)?;
    Ok(BundleView {
        root: outcome.root.to_string_lossy().into_owned(),
        artifacts: outcome.artifacts,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn run_backtest(
    config: String,
    strategy: String,
    out: Option<String>,
) -> Result<BacktestView, String> {
    let config = validate_workspace_path(&config, "run config")?;
    let strategy = validate_workspace_path(&strategy, "strategy config")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = BacktestRequest {
        config: &config,
        strategy: &strategy,
        out: out.as_deref(),
    };
    let outcome = AppService::run_backtest(&request).map_err(text)?;
    Ok(BacktestView {
        root: outcome.root.to_string_lossy().into_owned(),
        manifest: outcome.manifest.to_string_lossy().into_owned(),
        stages: outcome.stages,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn qualify(
    evidence: String,
    criteria: String,
    out: Option<String>,
) -> Result<QualificationView, String> {
    let evidence = validate_workspace_path(&evidence, "backtest bundle")?;
    let criteria = validate_workspace_path(&criteria, "criteria")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = QualificationRequest {
        evidence: &evidence,
        criteria: &criteria,
        out: out.as_deref(),
    };
    let outcome = AppService::qualify(&request).map_err(text)?;
    Ok(QualificationView {
        root: outcome.root.to_string_lossy().into_owned(),
        qualification: outcome.qualification.to_string_lossy().into_owned(),
        verdict: outcome.verdict.to_owned(),
        summary: outcome.summary,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn evaluate_analytics(
    evidence: String,
    out: Option<String>,
) -> Result<AnalyticsView, String> {
    let evidence = validate_workspace_path(&evidence, "backtest bundle")?;
    let out = out.map(|path| validate_output_path(&path)).transpose()?;
    let request = AnalyticsRequest {
        evidence: &evidence,
        out: out.as_deref(),
    };
    let outcome = AppService::evaluate_analytics(&request).map_err(text)?;
    Ok(AnalyticsView {
        root: outcome.root.to_string_lossy().into_owned(),
        metrics: outcome.metrics.to_string_lossy().into_owned(),
        tables: outcome.tables.to_string_lossy().into_owned(),
        summary: outcome.summary.to_string_lossy().into_owned(),
    })
}

fn parse_position(value: Option<&str>) -> Result<StrategyPosition, String> {
    match value.unwrap_or("flat") {
        "flat" => Ok(StrategyPosition::flat()),
        "long" => Ok(StrategyPosition::opened(Side::Long, 0)),
        "short" => Ok(StrategyPosition::opened(Side::Short, 0)),
        other => Err(format!(
            "unknown position '{other}'; use flat, long, or short"
        )),
    }
}
