//! The `strategy explain` and `strategy run` commands.
//!
//! The commands read a completed model bundle and a strategy configuration,
//! call the generic engine in `quantrule-backtest`, and render the decision. The
//! strategy stage does not read a candle, a fill, a position, cash, equity, or
//! P&L. The separate position argument is the strategy's own state supplied by
//! the runner, not a market reading.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::intent::{
    BracketIntent, EntryStyle, StopSpec, StrategyPosition, TargetSpec, TradeIntent,
};
use quantrule_backtest::side::Side;
use quantrule_backtest::strategy::{Decision, EventStrategyInput, ExitReason};
use serde_json::{Value, json};

use super::artifacts::{self, DecisionCounts, StrategyArtifactMeta};
use super::bundle::{ModelBundle, StrategyBundleError};
use super::context::StrategyContextError;
use crate::artifact::render_report::{self, BundleWriteError};
use crate::config::{ConfigError, StrategyConfig};

/// One `strategy explain` request.
pub struct StrategyExplainRequest<'a> {
    /// The model bundle directory.
    pub model: &'a Path,
    /// The strategy configuration path.
    pub strategy: &'a Path,
    /// The zero-based execution bar.
    pub bar: usize,
    /// The strategy's own position state.
    pub position: StrategyPosition,
}

/// One `strategy run` request.
pub struct StrategyRunRequest<'a> {
    /// The model bundle directory.
    pub model: &'a Path,
    /// The strategy configuration path.
    pub strategy: &'a Path,
    /// The output root; the model bundle's parent when `None`.
    pub out: Option<&'a Path>,
}

/// The written decisions bundle.
pub struct StrategyOutcome {
    /// The bundle root.
    pub root: PathBuf,
    /// The artifact names under the root.
    pub artifacts: Vec<String>,
    /// The `--json` envelope.
    pub json: Vec<u8>,
}

/// The strategy command failed.
#[derive(Debug)]
pub enum StrategyRunError {
    /// The strategy file is invalid.
    Config(ConfigError),
    /// The model bundle is missing or invalid.
    Model(StrategyBundleError),
    /// The execution timeframe is not in the model bundle.
    MissingTimeframe {
        /// The requested timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The requested bar is not in the model bundle.
    BarOutOfRange {
        /// The requested bar.
        bar: usize,
        /// The available bars.
        bars: usize,
    },
    /// The strategy's timeframes do not fit the published model.
    Context(StrategyContextError),
    /// The engine cannot produce a standalone decision sequence.
    UnsupportedEngine {
        /// The engine name.
        engine: &'static str,
        /// Why the engine needs execution feedback.
        reason: &'static str,
    },
    /// The bundle could not be written.
    Bundle(BundleWriteError),
    /// The decision record could not be encoded.
    Json(serde_json::Error),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for StrategyRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Model(error) => error.fmt(formatter),
            Self::MissingTimeframe { timeframe_secs } => write!(
                formatter,
                "the model does not publish the timeframe {timeframe_secs}s"
            ),
            Self::BarOutOfRange { bar, bars } => write!(
                formatter,
                "bar {bar} is outside the execution timeframe's {bars} published bars"
            ),
            Self::Context(error) => error.fmt(formatter),
            Self::UnsupportedEngine { engine, reason } => {
                write!(
                    formatter,
                    "strategy engine '{engine}' cannot run standalone: {reason}"
                )
            }
            Self::Bundle(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "cannot encode the decision record: {error}"),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for StrategyRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Model(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::Context(error) => Some(error),
            Self::MissingTimeframe { .. }
            | Self::BarOutOfRange { .. }
            | Self::UnsupportedEngine { .. } => None,
        }
    }
}

impl From<StrategyContextError> for StrategyRunError {
    fn from(error: StrategyContextError) -> Self {
        Self::Context(error)
    }
}

/// Prints one decision and its as-of evidence.
///
/// # Errors
///
/// Returns [`StrategyRunError`] when the strategy or model bundle is invalid,
/// the bar is absent, or the writer fails.
pub fn explain(request: &StrategyExplainRequest<'_>) -> Result<Value, StrategyRunError> {
    let (config, _) =
        crate::config::load_strategy(request.strategy).map_err(StrategyRunError::Config)?;
    let bundle = ModelBundle::load(request.model).map_err(StrategyRunError::Model)?;
    super::context::validate_context(&config, None, |timeframe| {
        bundle.contains_timeframe(timeframe)
    })?;
    let execution = u64::from(config.execution_timeframe.seconds());
    let bars = bundle
        .bars(execution)
        .ok_or(StrategyRunError::MissingTimeframe {
            timeframe_secs: execution,
        })?;
    let row =
        bundle
            .execution_bar(execution, request.bar)
            .ok_or(StrategyRunError::BarOutOfRange {
                bar: request.bar,
                bars: bars.len(),
            })?;
    let context: Vec<u64> = config
        .context_timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    let input = bundle
        .input(execution, request.bar, config.event_window_bars(), &context)
        .ok_or(StrategyRunError::BarOutOfRange {
            bar: request.bar,
            bars: bars.len(),
        })?;
    let decision = config.decide(&input, request.position);
    Ok(decision_record(
        &config,
        &input,
        request.position,
        decision,
        row.atr,
    ))
}

/// Writes the decisions bundle for one model bundle and strategy configuration.
///
/// # Errors
///
/// Returns [`StrategyRunError`] when the strategy or model bundle is invalid,
/// the engine needs execution feedback, the output target is unavailable, or
/// the writer fails.
pub fn run(request: &StrategyRunRequest<'_>) -> Result<StrategyOutcome, StrategyRunError> {
    let (config, config_id) =
        crate::config::load_strategy(request.strategy).map_err(StrategyRunError::Config)?;
    if config.engine_name() == "compression_bracket" {
        return Err(StrategyRunError::UnsupportedEngine {
            engine: "compression_bracket",
            reason: "the bracket side is known only after a fill; use strategy explain for one decision",
        });
    }
    let bundle = ModelBundle::load(request.model).map_err(StrategyRunError::Model)?;
    super::context::validate_context(&config, None, |timeframe| {
        bundle.contains_timeframe(timeframe)
    })?;
    let execution = u64::from(config.execution_timeframe.seconds());
    let bars = bundle
        .bars(execution)
        .ok_or(StrategyRunError::MissingTimeframe {
            timeframe_secs: execution,
        })?;
    if bars.is_empty() {
        return Err(StrategyRunError::MissingTimeframe {
            timeframe_secs: execution,
        });
    }
    let context: Vec<u64> = config
        .context_timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect();
    let root = request
        .out
        .map_or_else(
            || {
                request
                    .model
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            },
            Path::to_path_buf,
        )
        .join("strategy");
    render_report::ensure_available(&root).map_err(StrategyRunError::Bundle)?;

    let mut position = StrategyPosition::flat();
    let mut records = Vec::with_capacity(bars.len());
    let mut counts = DecisionCounts::default();
    let mut first_open_time = None;
    let mut last_open_time = None;
    for (index, row) in bars.iter().enumerate() {
        if index > 0 && position.is_open() {
            position.bars_held += 1;
        }
        let input = bundle
            .input(execution, row.bar, config.event_window_bars(), &context)
            .ok_or(StrategyRunError::BarOutOfRange {
                bar: row.bar,
                bars: bars.len(),
            })?;
        let decision = config.decide(&input, position);
        let record = decision_record(&config, &input, position, decision, row.atr);
        counts.observe(decision);
        records.push(record);
        first_open_time.get_or_insert(row.timestamp);
        last_open_time = Some(row.timestamp);
        match decision {
            Decision::Enter(entry) => {
                position = StrategyPosition::opened(entry.side(), row.bar);
            }
            Decision::Exit(_) => position = StrategyPosition::flat(),
            Decision::EnterBracket(_) => {}
            Decision::Hold => {}
        }
    }

    let meta = StrategyArtifactMeta {
        data_version: &bundle.data_version,
        model_config_id: &bundle.config_id,
        strategy_config_id: &config_id,
        period_from: bundle.period_from.as_deref(),
        period_to: bundle.period_to.as_deref(),
        strategy_id: config.id.as_deref(),
        engine: config.engine_name(),
        execution_timeframe: config.execution_timeframe.code(),
        context_timeframes: config
            .context_timeframes
            .iter()
            .map(|timeframe| timeframe.code())
            .collect(),
    };
    let summary =
        artifacts::summary_bytes(&meta, bars.len(), first_open_time, last_open_time, &counts);
    let decisions = artifacts::decisions_ndjson(&records)?;
    let provenance = artifacts::provenance_bytes(&meta)?;
    let artifacts = [
        ("summary.txt", summary.into_bytes()),
        ("decisions.ndjson", decisions.into_bytes()),
    ];
    render_report::write_stage_bundle(&root, &artifacts, &provenance, "decisions.ndjson")
        .map_err(StrategyRunError::Bundle)?;

    let artifacts = vec![
        "decisions.ndjson".to_owned(),
        "summary.txt".to_owned(),
        "provenance.json".to_owned(),
    ];
    let value = json!({
        "bundle": root,
        "artifacts": artifacts,
    });
    let mut json = serde_json::to_vec(&value).map_err(StrategyRunError::Json)?;
    json.push(b'\n');
    Ok(StrategyOutcome {
        root,
        artifacts,
        json,
    })
}

/// Builds one strategy decision record for the bundle or the explain command.
#[must_use]
pub fn decision_record(
    config: &StrategyConfig,
    input: &EventStrategyInput,
    position: StrategyPosition,
    decision: Decision,
    atr14: Option<f64>,
) -> Value {
    let side = StrategyConfig::decision_side(decision);
    json!({
        "schema": "quantrule.strategy-decision/v1",
        "timeframe_secs": input.execution_timeframe_secs,
        "bar_index": input.execution_bar_index,
        "open_time": input.execution_open_time,
        "available_bar_index": input.execution_bar_index,
        "position_before": position_label(position.side),
        "position_after": position_after(decision, side, position),
        "decision": decision_label(decision),
        "side": side.map(side_label),
        "reason": reason_label(decision),
        "intent": intent_value(decision),
        "atr14": atr14,
        "evidence": super::evidence::decision_evidence(config, input, decision),
    })
}

fn position_after(
    decision: Decision,
    side: Option<Side>,
    position: StrategyPosition,
) -> &'static str {
    match decision {
        Decision::Enter(_) => side.map_or("flat", side_label),
        Decision::Exit(_) => "flat",
        Decision::EnterBracket(_) | Decision::Hold => position_label(position.side),
    }
}

fn decision_label(decision: Decision) -> &'static str {
    match decision {
        Decision::Hold => "hold",
        Decision::Enter(_) => "enter",
        Decision::EnterBracket(_) => "enter_bracket",
        Decision::Exit(_) => "exit",
    }
}

fn reason_label(decision: Decision) -> Option<&'static str> {
    match decision {
        Decision::Enter(_) => Some("entry"),
        Decision::EnterBracket(_) => Some("bracket_entry"),
        Decision::Exit(exit) => Some(exit_reason_label(exit.reason)),
        Decision::Hold => None,
    }
}

fn exit_reason_label(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::EventCondition => "event_condition",
        ExitReason::TimeStop => "time_stop",
        ExitReason::Giveback => "giveback",
    }
}

fn position_label(side: Option<Side>) -> &'static str {
    match side {
        None => "flat",
        Some(Side::Long) => "long",
        Some(Side::Short) => "short",
    }
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

fn intent_value(decision: Decision) -> Value {
    match decision {
        Decision::Enter(entry) => trade_intent(entry.intent),
        Decision::EnterBracket(entry) => bracket_intent(entry.intent),
        Decision::Exit(exit) => json!({
            "kind": "exit",
            "reason": exit_reason_label(exit.reason),
        }),
        Decision::Hold => Value::Null,
    }
}

fn trade_intent(intent: TradeIntent) -> Value {
    json!({
        "kind": match intent.entry {
            EntryStyle::MarketNextOpen => "market_next_open",
            EntryStyle::Limit { .. } => "limit",
        },
        "entry": entry_style(intent.entry),
        "stop": stop_spec(intent.stop),
        "target": target_spec(intent.target),
        "valid_until": intent.valid_until,
        "max_slippage_bps": intent.max_slippage_bps,
    })
}

fn bracket_intent(intent: BracketIntent) -> Value {
    json!({
        "kind": "bracket",
        "upper_price": intent.upper_price,
        "lower_price": intent.lower_price,
        "stop": stop_spec(intent.stop),
        "target": target_spec(intent.target),
        "valid_for_bars": intent.valid_for_bars,
        "max_slippage_bps": intent.max_slippage_bps,
    })
}

fn entry_style(style: EntryStyle) -> Value {
    match style {
        EntryStyle::MarketNextOpen => json!({ "kind": "market_next_open" }),
        EntryStyle::Limit {
            price,
            valid_for_bars,
        } => json!({
            "kind": "limit",
            "price": price,
            "valid_for_bars": valid_for_bars,
        }),
    }
}

fn stop_spec(spec: StopSpec) -> Value {
    match spec {
        StopSpec::AtrMultiple { multiple } => json!({
            "kind": "atr_multiple",
            "multiple": multiple,
        }),
        StopSpec::Price { price } => json!({
            "kind": "price",
            "price": price,
        }),
    }
}

fn target_spec(spec: TargetSpec) -> Value {
    match spec {
        TargetSpec::RewardToRisk { multiple } => json!({
            "kind": "reward_to_risk",
            "multiple": multiple,
        }),
        TargetSpec::Price { price } => json!({
            "kind": "price",
            "price": price,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::position_label;
    use quantrule_backtest::side::Side;

    #[test]
    fn position_labels_are_stable() {
        assert_eq!(position_label(None), "flat");
        assert_eq!(position_label(Some(Side::Long)), "long");
        assert_eq!(position_label(Some(Side::Short)), "short");
    }
}
