//! The `backtest run` command: the coupled stage runner.
//!
//! The command computes the model, then interleaves the strategy, execution,
//! and portfolio engines so the strategy sees the position execution actually
//! holds. It writes the same stage bundles as the standalone commands and a
//! manifest that names every artifact. It adds no domain logic of its own.

use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_backtest::execution::{
    ExecutionEngine, ExecutionError, Fill, OrderIntent, OrderRecord, Rejection,
};
use quantrule_backtest::portfolio::{PortfolioLedger, PortfolioLedgerError};
use quantrule_backtest::side::Side;
use quantrule_backtest::strategy::{Decision, EventStrategyInput};
use serde_json::{Value, json};

use crate::config::{ConfigError, RunConfig};
use crate::execution::{ExecutionArtifactMeta, ExecutionRunError, write_bundle as write_execution};
use crate::model_bundle::model_bundle_from;
use crate::pipeline::{self, PipelineError};
use crate::portfolio::{self, PortfolioRunError};
use crate::render_report::{self, BundleError};
use crate::request::{BacktestRunArgs, PortfolioReplayArgs};
use crate::strategy::{DecisionCounts, StrategyArtifactMeta, artifacts, decision_record};

/// The backtest command failed.
#[derive(Debug)]
pub(crate) enum BacktestRunError {
    /// The run configuration is invalid.
    Config(ConfigError),
    /// The strategy configuration is invalid.
    StrategyConfig(ConfigError),
    /// The model stage failed.
    Pipeline(PipelineError),
    /// The model bundle could not be written.
    Model(String),
    /// A strategy artifact could not be encoded.
    StrategyArtifact(crate::strategy::StrategyRunError),
    /// The execution engine rejected a caller invariant.
    Execution(ExecutionError),
    /// The portfolio ledger rejected a fill or a mark.
    Ledger(PortfolioLedgerError),
    /// The execution bundle could not be written.
    ExecutionBundle(ExecutionRunError),
    /// The portfolio replay failed.
    Portfolio(PortfolioRunError),
    /// A stage bundle could not be written.
    Bundle(BundleError),
    /// A manifest file could not be written.
    Export(crate::export::ExportError),
    /// The manifest could not be encoded.
    Json(serde_json::Error),
    /// A stage provenance could not be read.
    Manifest {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong.
        message: String,
    },
    /// The execution timeframe is absent from the model.
    MissingTimeframe {
        /// The timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for BacktestRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::StrategyConfig(error) => error.fmt(formatter),
            Self::Pipeline(PipelineError::Source(error)) => error.fmt(formatter),
            Self::Pipeline(PipelineError::EmptyPeriod) => {
                formatter.write_str("the requested period selected no candle")
            }
            Self::Model(message) => formatter.write_str(message),
            Self::StrategyArtifact(error) => error.fmt(formatter),
            Self::Execution(error) => error.fmt(formatter),
            Self::Ledger(error) => error.fmt(formatter),
            Self::ExecutionBundle(error) => error.fmt(formatter),
            Self::Portfolio(error) => error.fmt(formatter),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Export(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "cannot encode the backtest artifact: {error}"),
            Self::Manifest { path, message } => {
                write!(formatter, "cannot read {}: {message}", path.display())
            }
            Self::MissingTimeframe { timeframe_secs } => write!(
                formatter,
                "the model does not publish the execution timeframe {timeframe_secs}s"
            ),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for BacktestRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) | Self::StrategyConfig(error) => Some(error),
            Self::Pipeline(PipelineError::Source(error)) => Some(error),
            Self::Pipeline(PipelineError::EmptyPeriod) => None,
            Self::Model(_) => None,
            Self::StrategyArtifact(error) => Some(error),
            Self::Execution(error) => Some(error),
            Self::Ledger(error) => Some(error),
            Self::ExecutionBundle(error) => Some(error),
            Self::Portfolio(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Export(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Manifest { .. } | Self::MissingTimeframe { .. } => None,
            Self::Output(error) => Some(error),
        }
    }
}

/// Runs the coupled backtest and writes the stage bundles.
///
/// # Errors
///
/// Returns [`BacktestRunError`] when a config, model, execution, ledger, stage
/// bundle, manifest, or writer fails.
pub(crate) fn run(args: &BacktestRunArgs, writer: &mut impl Write) -> Result<(), BacktestRunError> {
    let config = RunConfig::load(&args.config).map_err(BacktestRunError::Config)?;
    let (strategy_config, strategy_config_id) =
        crate::config::load_strategy(&args.strategy).map_err(BacktestRunError::StrategyConfig)?;
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| config.output_root.clone());
    let root = out.join("backtest");

    let model_run = pipeline::run_model(&config).map_err(BacktestRunError::Pipeline)?;
    model_bundle_from(&config, &root, &model_run)
        .map_err(|error| BacktestRunError::Model(error.to_string()))?;

    let fill_timeframe = config.execution.fill_timeframe;
    let execution = u64::from(fill_timeframe.seconds());
    let timeframe = model_run
        .timeframe(execution)
        .ok_or(BacktestRunError::MissingTimeframe {
            timeframe_secs: execution,
        })?;
    let candles = timeframe.candles();
    let window = strategy_config.event_window_bars();
    let context: Vec<u64> = strategy_config
        .context_timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    for timeframe_secs in &context {
        if model_run.timeframe(*timeframe_secs).is_none() {
            return Err(BacktestRunError::MissingTimeframe {
                timeframe_secs: *timeframe_secs,
            });
        }
    }
    let policy_fraction = config.policy.risk.max_position_fraction;
    let leverage = config.portfolio.leverage;

    let mut engine =
        ExecutionEngine::new(config.execution.engine).map_err(BacktestRunError::Execution)?;
    let mut ledger = PortfolioLedger::new(config.portfolio).map_err(BacktestRunError::Ledger)?;
    let mut records = Vec::with_capacity(candles.len());
    let mut counts = DecisionCounts::default();
    let mut orders = Vec::new();
    let mut fills = Vec::new();
    let mut rejections = Vec::new();

    for (bar, candle) in candles.iter().enumerate() {
        let contiguous =
            bar > 0 && candle.timestamp() - candles[bar - 1].timestamp() == execution as i64;
        ledger
            .apply_funding(bar, candle.timestamp(), candle.open(), candle.volume())
            .map_err(BacktestRunError::Ledger)?;
        let batch = engine
            .advance(candle, contiguous)
            .map_err(BacktestRunError::Execution)?;
        for fill in &batch.fills {
            ledger.apply_fill(fill).map_err(BacktestRunError::Ledger)?;
        }
        collect(batch, &mut orders, &mut fills, &mut rejections);
        ledger
            .mark(bar, candle.timestamp(), candle.close())
            .map_err(BacktestRunError::Ledger)?;

        let mut input = EventStrategyInput::from_model_run(&model_run, execution, bar, window)
            .with_contiguous(contiguous);
        for timeframe_secs in &context {
            let Some(context_timeframe) = model_run.timeframe(*timeframe_secs) else {
                continue;
            };
            let execution_close = candle.timestamp() + execution as i64;
            let open_times = context_timeframe.open_times();
            let position = open_times
                .partition_point(|open| *open + *timeframe_secs as i64 <= execution_close);
            if position == 0 {
                continue;
            }
            input = input.with_view(&model_run, *timeframe_secs, position - 1, window);
        }

        let position = engine.position_view();
        let decision = strategy_config.decide(&input, position);
        let atr14 = timeframe.bar(bar).and_then(|bar| bar.readings.atr14);
        let record = decision_record(&strategy_config, &input, position, decision, atr14);
        counts.observe(decision);
        records.push(record);

        let max_notional = ledger.state().equity * policy_fraction * leverage;
        let step = ledger.quantity_step();
        let (intent, quantity, quantity_step) = match decision {
            Decision::Hold => (OrderIntent::Hold, None, None),
            Decision::Enter(entry) => {
                let quantity = ledger.size_entry(entry.side(), candle.close()).ok();
                (
                    OrderIntent::Enter {
                        intent: entry.intent,
                        bar_index: bar,
                        open_time: candle.timestamp(),
                        max_notional: Some(max_notional),
                    },
                    quantity,
                    Some(step),
                )
            }
            Decision::EnterBracket(entry) => {
                // The bracket side is not known until the fill. Size the
                // notional as a long entry; execution applies the same
                // quantity to either boundary.
                let quantity = ledger.size_entry(Side::Long, candle.close()).ok();
                (
                    OrderIntent::EnterBracket {
                        intent: entry.intent,
                        bar_index: bar,
                        open_time: candle.timestamp(),
                        max_notional: Some(max_notional),
                    },
                    quantity,
                    Some(step),
                )
            }
            Decision::Exit(exit) => (
                OrderIntent::Exit {
                    side: exit.side,
                    bar_index: bar,
                    open_time: candle.timestamp(),
                },
                None,
                None,
            ),
        };
        let batch = engine
            .submit_with_step(intent, quantity, atr14, quantity_step)
            .map_err(BacktestRunError::Execution)?;
        collect(batch, &mut orders, &mut fills, &mut rejections);
    }
    let batch = engine.finish();
    collect(batch, &mut orders, &mut fills, &mut rejections);

    let mut final_orders: BTreeMap<u64, OrderRecord> = BTreeMap::new();
    for order in orders {
        final_orders.insert(order.order_id, order);
    }
    let final_orders: Vec<OrderRecord> = final_orders.into_values().collect();

    let strategy_meta = StrategyArtifactMeta {
        data_version: &config.data.data_version,
        model_config_id: &config.config_hash,
        strategy_config_id: &strategy_config_id,
        period_from: config.data.from.as_deref(),
        period_to: config.data.to.as_deref(),
        strategy_id: strategy_config.id.as_deref(),
        engine: strategy_config.engine_name(),
        execution_timeframe: strategy_config.execution_timeframe.code(),
        context_timeframes: strategy_config
            .context_timeframes
            .iter()
            .map(|timeframe| timeframe.code())
            .collect(),
    };
    let strategy_summary = artifacts::summary_bytes(
        &strategy_meta,
        candles.len(),
        candles.first().map(quantrule_data::Candle::timestamp),
        candles.last().map(quantrule_data::Candle::timestamp),
        &counts,
    );
    let strategy_decisions =
        artifacts::decisions_ndjson(&records).map_err(BacktestRunError::StrategyArtifact)?;
    let strategy_provenance =
        artifacts::provenance_bytes(&strategy_meta).map_err(BacktestRunError::StrategyArtifact)?;
    let strategy_artifacts = [
        ("summary.txt", strategy_summary.into_bytes()),
        ("decisions.ndjson", strategy_decisions.into_bytes()),
    ];
    render_report::write_stage_bundle(
        &root.join("strategy"),
        &strategy_artifacts,
        &strategy_provenance,
        "decisions.ndjson",
    )
    .map_err(BacktestRunError::Bundle)?;

    let execution_meta = ExecutionArtifactMeta {
        data_version: &config.data.data_version,
        config_id: &config.config_hash,
        strategy_config_id: &strategy_config_id,
        period_from: config.data.from.as_deref(),
        period_to: config.data.to.as_deref(),
        fill_timeframe: fill_timeframe.code(),
        decision_bars: records.len(),
        bars: candles.len(),
        sizing: config.portfolio.sizing,
        sizing_basis: "ledger_current",
        sizing_equity: None,
        max_notional: None,
        fee_rate: config.execution.engine.fee_rate(),
        slippage_bps: config.execution.engine.slippage_bps(),
    };
    write_execution(
        &root.join("execution"),
        &final_orders,
        &fills,
        &rejections,
        &execution_meta,
    )
    .map_err(BacktestRunError::ExecutionBundle)?;

    let portfolio_args = PortfolioReplayArgs {
        fills: root.join("execution"),
        config: args.config.clone(),
        out: Some(root.clone()),
        json: false,
    };
    portfolio::replay(&portfolio_args, &mut io::sink()).map_err(BacktestRunError::Portfolio)?;

    let stages = [
        stage_info(&root, "model", "model")?,
        stage_info(&root, "strategy", "decisions")?,
        stage_info(&root, "execution", "fills")?,
        stage_info(&root, "portfolio", "portfolio")?,
    ];
    let manifest = manifest_bytes(&config, &stages)?;
    let provenance = backtest_provenance_bytes(&config, &stages)?;
    crate::export::write_atomic(&root.join("provenance.json"), &provenance)
        .map_err(BacktestRunError::Export)?;
    crate::export::write_atomic(&root.join("manifest.json"), &manifest)
        .map_err(BacktestRunError::Export)?;

    if args.json {
        let value = json!({
            "bundle": root,
            "manifest": root.join("manifest.json"),
            "artifacts": ["manifest.json", "model", "strategy", "execution", "portfolio", "provenance.json"],
        });
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&value).map_err(BacktestRunError::Json)?
        )
        .map_err(BacktestRunError::Output)?;
    } else {
        writeln!(writer, "{}", root.join("manifest.json").display())
            .map_err(BacktestRunError::Output)?;
        for stage in ["model", "strategy", "execution", "portfolio"] {
            writeln!(writer, "{}", root.join(stage).display()).map_err(BacktestRunError::Output)?;
        }
    }
    writer.flush().map_err(BacktestRunError::Output)
}

fn collect(
    batch: quantrule_backtest::execution::ExecutionBatch,
    orders: &mut Vec<OrderRecord>,
    fills: &mut Vec<Fill>,
    rejections: &mut Vec<Rejection>,
) {
    orders.extend(batch.orders);
    fills.extend(batch.fills);
    rejections.extend(batch.rejections);
}

struct StageInfo {
    stage: &'static str,
    artifact: &'static str,
    schema: String,
    data_version: String,
    config_id: String,
    hash: String,
}

fn stage_info(
    root: &Path,
    stage: &'static str,
    artifact: &'static str,
) -> Result<StageInfo, BacktestRunError> {
    let path = root.join(stage).join("provenance.json");
    let text = std::fs::read_to_string(&path).map_err(|error| BacktestRunError::Manifest {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let value: Value = serde_json::from_str(&text).map_err(|error| BacktestRunError::Manifest {
        path: path.clone(),
        message: format!("not JSON: {error}"),
    })?;
    let object = value
        .as_object()
        .ok_or_else(|| BacktestRunError::Manifest {
            path: path.clone(),
            message: "the provenance root is not an object".to_owned(),
        })?;
    let schema = object
        .get("schema")
        .and_then(Value::as_str)
        .ok_or_else(|| BacktestRunError::Manifest {
            path: path.clone(),
            message: "schema is missing".to_owned(),
        })?
        .to_owned();
    let data_version = object
        .get("data_version")
        .and_then(Value::as_str)
        .ok_or_else(|| BacktestRunError::Manifest {
            path: path.clone(),
            message: "data_version is missing".to_owned(),
        })?
        .to_owned();
    let config_id = object
        .get("config_id")
        .or_else(|| object.get("config_hash"))
        .and_then(Value::as_str)
        .ok_or_else(|| BacktestRunError::Manifest {
            path: path.clone(),
            message: "config identity is missing".to_owned(),
        })?
        .to_owned();
    let hash = crate::fingerprint::fnv1a(text.as_bytes());
    Ok(StageInfo {
        stage,
        artifact,
        schema,
        data_version,
        config_id,
        hash,
    })
}

fn manifest_bytes(config: &RunConfig, stages: &[StageInfo]) -> Result<Vec<u8>, BacktestRunError> {
    let value = json!({
        "schema": "quantrule.backtest-manifest/v1",
        "run": config.name,
        "data_version": config.data.data_version,
        "config_id": config.config_hash,
        "code_commit": "unknown",
        "seed": Value::Null,
        "period": {
            "from": config.data.from,
            "to": config.data.to,
        },
        "stage_order": ["model", "strategy", "execution", "portfolio"],
        "stages": stages.iter().map(|stage| json!({
            "stage": stage.stage,
            "path": stage.stage,
            "provenance": format!("{}/provenance.json", stage.stage),
            "provenance_hash": stage.hash,
        })).collect::<Vec<Value>>(),
    });
    let mut bytes = serde_json::to_vec(&value).map_err(BacktestRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn backtest_provenance_bytes(
    config: &RunConfig,
    stages: &[StageInfo],
) -> Result<Vec<u8>, BacktestRunError> {
    let value = json!({
        "schema": "quantrule.stage-provenance/v1",
        "stage": "backtest",
        "artifact": "backtest",
        "data_version": config.data.data_version,
        "config_id": config.config_hash,
        "code_commit": "unknown",
        "source_artifacts": stages.iter().map(|stage| json!({
            "stage": stage.stage,
            "artifact": stage.artifact,
            "schema": stage.schema,
            "data_version": stage.data_version,
            "config_id": stage.config_id,
        })).collect::<Vec<Value>>(),
        "seed": Value::Null,
        "period": {
            "from": config.data.from,
            "to": config.data.to,
        },
        "artifacts": ["manifest.json", "model", "strategy", "execution", "portfolio", "provenance.json"],
    });
    let mut bytes = serde_json::to_vec(&value).map_err(BacktestRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use crate::fingerprint::fnv1a;

    #[test]
    fn the_provenance_fingerprint_is_stable() {
        assert_eq!(fnv1a(b"abc"), fnv1a(b"abc"));
        assert_ne!(fnv1a(b"abc"), fnv1a(b"abd"));
    }
}
