//! The `execution run` command.
//!
//! The command reads a strategy decisions bundle and the run configuration,
//! drives the causal fill engine, and writes the orders, fills, and rejections
//! bundle. The execution stage does not decide direction, and it does not
//! account for cash, equity, or P&L. A standalone run sizes entries from the
//! run's initial cash; the full backtest sizes from the ledger's current equity.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use quantrule_backtest::execution::{
    ExecutionBatch, ExecutionEngine, ExecutionError, Fill, FillReason, OrderIntent, OrderRecord,
    OrderSide, OrderState, RejectReason, Rejection, Trigger,
};
use quantrule_data::Candle;
use serde_json::{Value, json};

use super::decisions::{DecisionKind, DecisionsBundle, DecisionsError};
use crate::config::{ConfigError, RunConfig};
use crate::csv::SourceError;
use crate::pipeline;
use crate::render_report::{self, BundleError};
use crate::request::ExecutionRunArgs;

/// The execution command failed.
#[derive(Debug)]
pub(crate) enum ExecutionRunError {
    /// The run configuration is invalid.
    Config(ConfigError),
    /// The strategy decisions bundle is invalid.
    Decisions(DecisionsError),
    /// The candle source or the data engine failed.
    Source(SourceError),
    /// The execution engine rejected a caller invariant.
    Execution(ExecutionError),
    /// The execution timeframe has no candles.
    EmptySeries {
        /// The timeframe code.
        timeframe: &'static str,
    },
    /// The decisions and the run config name different data versions.
    DataVersionMismatch {
        /// The decision artifact's version.
        decisions: String,
        /// The run configuration's version.
        run: String,
    },
    /// A decision names a timeframe other than the run's execution timeframe.
    TimeframeMismatch {
        /// The decision timeframe in seconds.
        decision: u64,
        /// The execution timeframe in seconds.
        execution: u64,
    },
    /// The decisions and the candle series have different lengths.
    DecisionCountMismatch {
        /// The decision count.
        decisions: usize,
        /// The candle count.
        candles: usize,
    },
    /// A decision does not describe the matching candle.
    DecisionBarMismatch {
        /// The candle index.
        index: usize,
        /// The decision's bar index.
        decision_bar: usize,
        /// The decision's open time.
        decision_time: i64,
        /// The candle's open time.
        candle_time: i64,
    },
    /// A bracket decision needs the runner's side-aware sizing.
    UnsupportedBracket {
        /// The decision bar.
        bar: usize,
        /// The upper boundary.
        upper_price: f64,
        /// The lower boundary.
        lower_price: f64,
    },
    /// The output bundle could not be written.
    Bundle(BundleError),
    /// The execution artifact could not be encoded.
    Json(serde_json::Error),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for ExecutionRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Decisions(error) => error.fmt(formatter),
            Self::Source(error) => error.fmt(formatter),
            Self::Execution(error) => error.fmt(formatter),
            Self::EmptySeries { timeframe } => {
                write!(
                    formatter,
                    "the execution timeframe {timeframe} has no candles"
                )
            }
            Self::DataVersionMismatch { decisions, run } => write!(
                formatter,
                "the decisions data version '{decisions}' does not match the run's '{run}'"
            ),
            Self::TimeframeMismatch {
                decision,
                execution,
            } => write!(
                formatter,
                "a decision names timeframe {decision}s, but the run's execution timeframe is {execution}s"
            ),
            Self::DecisionCountMismatch { decisions, candles } => write!(
                formatter,
                "the bundle has {decisions} decisions, but the execution series has {candles} candles"
            ),
            Self::DecisionBarMismatch {
                index,
                decision_bar,
                decision_time,
                candle_time,
            } => write!(
                formatter,
                "candle {index} is at {candle_time}s, but the decision is bar {decision_bar} at {decision_time}s"
            ),
            Self::UnsupportedBracket {
                bar,
                upper_price,
                lower_price,
            } => write!(
                formatter,
                "the bracket decision at bar {bar} ({lower_price}..{upper_price}) needs the runner's side-aware sizing"
            ),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "cannot encode the execution artifact: {error}"),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for ExecutionRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Decisions(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Execution(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::EmptySeries { .. }
            | Self::DataVersionMismatch { .. }
            | Self::TimeframeMismatch { .. }
            | Self::DecisionCountMismatch { .. }
            | Self::DecisionBarMismatch { .. }
            | Self::UnsupportedBracket { .. } => None,
        }
    }
}

/// Runs the execution stage.
///
/// # Errors
///
/// Returns [`ExecutionRunError`] when a config, decision artifact, candle
/// series, execution invariant, output target, or writer fails.
pub(crate) fn run(
    args: &ExecutionRunArgs,
    writer: &mut impl Write,
) -> Result<(), ExecutionRunError> {
    let config = RunConfig::load(&args.config).map_err(ExecutionRunError::Config)?;
    let decisions = DecisionsBundle::load(&args.decisions).map_err(ExecutionRunError::Decisions)?;
    if decisions.data_version != config.data.data_version {
        return Err(ExecutionRunError::DataVersionMismatch {
            decisions: decisions.data_version,
            run: config.data.data_version.clone(),
        });
    }

    let fill_timeframe = config.execution.fill_timeframe;
    let fill_secs = u64::from(fill_timeframe.seconds());
    for decision in &decisions.decisions {
        if decision.timeframe_secs != fill_secs {
            return Err(ExecutionRunError::TimeframeMismatch {
                decision: decision.timeframe_secs,
                execution: fill_secs,
            });
        }
    }

    let set = pipeline::load_set_with_limit(
        &config.data.source.path,
        config.data.bounds,
        config.data.max_candles,
    )
    .map_err(ExecutionRunError::Source)?;
    let candles: Vec<Candle> = set.series(fill_timeframe).iter().copied().collect();
    if candles.is_empty() {
        return Err(ExecutionRunError::EmptySeries {
            timeframe: fill_timeframe.code(),
        });
    }
    if decisions.decisions.len() != candles.len() {
        return Err(ExecutionRunError::DecisionCountMismatch {
            decisions: decisions.decisions.len(),
            candles: candles.len(),
        });
    }
    for (index, (decision, candle)) in decisions.decisions.iter().zip(candles.iter()).enumerate() {
        if decision.bar_index != index || decision.open_time != candle.timestamp() {
            return Err(ExecutionRunError::DecisionBarMismatch {
                index,
                decision_bar: decision.bar_index,
                decision_time: decision.open_time,
                candle_time: candle.timestamp(),
            });
        }
    }

    let mut engine =
        ExecutionEngine::new(config.execution.engine).map_err(ExecutionRunError::Execution)?;
    let initial_cash = config.portfolio.initial_cash;
    let sizing = config.portfolio.sizing;
    let quantity_step = sizing.quantity_step();
    let max_notional = sizing.max_notional(initial_cash, config.portfolio.leverage);
    let mut orders = Vec::new();
    let mut fills = Vec::new();
    let mut rejections = Vec::new();

    for (index, (decision, candle)) in decisions.decisions.iter().zip(candles.iter()).enumerate() {
        let contiguous =
            index > 0 && candle.timestamp() - candles[index - 1].timestamp() == fill_secs as i64;
        let batch = engine
            .advance(candle, contiguous)
            .map_err(ExecutionRunError::Execution)?;
        collect(batch, &mut orders, &mut fills, &mut rejections);

        let (intent, quantity, step) = match decision.kind {
            DecisionKind::Hold => (OrderIntent::Hold, None, None),
            DecisionKind::Enter(intent) => {
                let quantity = sizing.size(initial_cash, candle.close()).ok();
                (
                    OrderIntent::Enter {
                        intent,
                        bar_index: decision.bar_index,
                        open_time: decision.open_time,
                        max_notional: Some(max_notional),
                    },
                    quantity,
                    Some(quantity_step),
                )
            }
            DecisionKind::Exit(side) => (
                OrderIntent::Exit {
                    side,
                    bar_index: decision.bar_index,
                    open_time: decision.open_time,
                },
                None,
                None,
            ),
            DecisionKind::EnterBracket(intent) => {
                return Err(ExecutionRunError::UnsupportedBracket {
                    bar: decision.bar_index,
                    upper_price: intent.upper_price,
                    lower_price: intent.lower_price,
                });
            }
        };
        let batch = engine
            .submit_with_step(intent, quantity, decision.atr14, step)
            .map_err(ExecutionRunError::Execution)?;
        collect(batch, &mut orders, &mut fills, &mut rejections);
    }
    let batch = engine.finish();
    collect(batch, &mut orders, &mut fills, &mut rejections);

    let mut final_orders: BTreeMap<u64, OrderRecord> = BTreeMap::new();
    for order in orders {
        final_orders.insert(order.order_id, order);
    }
    let final_orders: Vec<OrderRecord> = final_orders.into_values().collect();

    let root = args
        .out
        .clone()
        .unwrap_or_else(|| {
            args.decisions
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."))
        })
        .join("execution");
    let meta = ExecutionArtifactMeta {
        data_version: &decisions.data_version,
        config_id: &config.config_hash,
        strategy_config_id: &decisions.config_id,
        period_from: decisions.period_from.as_deref(),
        period_to: decisions.period_to.as_deref(),
        fill_timeframe: fill_timeframe.code(),
        decision_bars: decisions.decisions.len(),
        bars: candles.len(),
        sizing,
        sizing_basis: "initial_cash",
        sizing_equity: Some(initial_cash),
        max_notional: Some(max_notional),
        fee_rate: config.execution.engine.fee_rate(),
        slippage_bps: config.execution.engine.slippage_bps(),
    };
    write_bundle(&root, &final_orders, &fills, &rejections, &meta)?;

    if args.json {
        let value = json!({
            "bundle": root,
            "artifacts": ["orders.csv", "fills.csv", "rejections.csv", "summary.txt", "provenance.json"],
        });
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&value).map_err(ExecutionRunError::Json)?
        )
        .map_err(ExecutionRunError::Output)?;
    } else {
        for name in [
            "orders.csv",
            "fills.csv",
            "rejections.csv",
            "summary.txt",
            "provenance.json",
        ] {
            writeln!(writer, "{}", root.join(name).display()).map_err(ExecutionRunError::Output)?;
        }
    }
    writer.flush().map_err(ExecutionRunError::Output)
}

fn collect(
    batch: ExecutionBatch,
    orders: &mut Vec<OrderRecord>,
    fills: &mut Vec<Fill>,
    rejections: &mut Vec<Rejection>,
) {
    orders.extend(batch.orders);
    fills.extend(batch.fills);
    rejections.extend(batch.rejections);
}

fn orders_csv(orders: &[OrderRecord]) -> String {
    let mut csv = String::from(
        "order_id,submitted_bar,submitted_time,side,quantity,reason,trigger_kind,trigger_price,state,filled_bar,filled_time,fill_id,rejection_reason\n",
    );
    for order in orders {
        let (trigger_kind, trigger_price) = match order.trigger {
            Trigger::Market => ("market", String::new()),
            Trigger::Stop { price } => ("stop", number(price)),
            Trigger::Target { price } => ("target", number(price)),
        };
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            order.order_id,
            order.submitted_bar,
            order.submitted_time,
            order_side(order.side),
            optional_number(order.quantity),
            fill_reason(order.reason),
            trigger_kind,
            trigger_price,
            order_state(order.state),
            optional_usize(order.filled_bar),
            optional_i64(order.filled_time),
            optional_u64(order.fill_id),
            order.rejection_reason.map_or("", reject_reason),
        );
    }
    csv
}

fn fills_csv(fills: &[Fill]) -> String {
    let mut csv =
        String::from("fill_id,side,quantity,reference_price,price,slippage,fee,bar,time,reason\n");
    for (id, fill) in fills.iter().enumerate() {
        let _ = writeln!(
            csv,
            "{id},{},{},{},{},{},{},{},{},{}",
            order_side(fill.side),
            number(fill.quantity),
            number(fill.reference_price),
            number(fill.price),
            number(fill.slippage),
            number(fill.fee),
            fill.bar,
            fill.time,
            fill_reason(fill.reason),
        );
    }
    csv
}

fn rejections_csv(rejections: &[Rejection]) -> String {
    let mut csv = String::from("order_id,bar,time,reason\n");
    for rejection in rejections {
        let _ = writeln!(
            csv,
            "{},{},{},{}",
            rejection.order_id,
            rejection.bar,
            rejection.time,
            reject_reason(rejection.reason),
        );
    }
    csv
}

/// The identity and configuration an execution bundle records.
pub(crate) struct ExecutionArtifactMeta<'a> {
    /// The input candle identity.
    pub(crate) data_version: &'a str,
    /// The run configuration identity.
    pub(crate) config_id: &'a str,
    /// The strategy configuration identity.
    pub(crate) strategy_config_id: &'a str,
    /// The requested period start, as written.
    pub(crate) period_from: Option<&'a str>,
    /// The requested period end, as written.
    pub(crate) period_to: Option<&'a str>,
    /// The execution timeframe code.
    pub(crate) fill_timeframe: &'a str,
    /// The number of decision bars.
    pub(crate) decision_bars: usize,
    /// The number of candle bars.
    pub(crate) bars: usize,
    /// The sizing mode.
    pub(crate) sizing: quantrule_backtest::portfolio::SizingMode,
    /// How the sizing equity was selected.
    pub(crate) sizing_basis: &'a str,
    /// The fixed sizing equity, or `None` for the ledger's current equity.
    pub(crate) sizing_equity: Option<f64>,
    /// The fixed notional cap, or `None` for the ledger's dynamic cap.
    pub(crate) max_notional: Option<f64>,
    /// The execution fee rate.
    pub(crate) fee_rate: f64,
    /// The execution slippage in basis points.
    pub(crate) slippage_bps: f64,
}

/// Writes the execution bundle from the collected records.
///
/// # Errors
///
/// Returns [`ExecutionRunError::Bundle`] when the target is unavailable, and
/// [`ExecutionRunError::Json`] when an artifact cannot be encoded.
pub(crate) fn write_bundle(
    root: &Path,
    orders: &[OrderRecord],
    fills: &[Fill],
    rejections: &[Rejection],
    meta: &ExecutionArtifactMeta<'_>,
) -> Result<(), ExecutionRunError> {
    render_report::ensure_available(root).map_err(ExecutionRunError::Bundle)?;
    let summary = summary_bytes(meta, orders, fills, rejections);
    let provenance = provenance_bytes(meta)?;
    let artifacts = [
        ("orders.csv", orders_csv(orders).into_bytes()),
        ("fills.csv", fills_csv(fills).into_bytes()),
        ("rejections.csv", rejections_csv(rejections).into_bytes()),
        ("summary.txt", summary.into_bytes()),
    ];
    render_report::write_stage_bundle(root, &artifacts, &provenance, "fills.csv")
        .map_err(ExecutionRunError::Bundle)
}

fn summary_bytes(
    meta: &ExecutionArtifactMeta<'_>,
    orders: &[OrderRecord],
    fills: &[Fill],
    rejections: &[Rejection],
) -> String {
    let mut pending = 0;
    let mut filled = 0;
    let mut rejected = 0;
    for order in orders {
        match order.state {
            OrderState::Pending => pending += 1,
            OrderState::Filled => filled += 1,
            OrderState::Rejected => rejected += 1,
        }
    }
    let mut buys = 0;
    let mut sells = 0;
    let mut entry = 0;
    let mut exit = 0;
    let mut stop_loss = 0;
    let mut take_profit = 0;
    let mut fee = 0.0;
    let mut slippage = 0.0;
    for fill in fills {
        match fill.side {
            OrderSide::Buy => buys += 1,
            OrderSide::Sell => sells += 1,
        }
        match fill.reason {
            FillReason::Entry => entry += 1,
            FillReason::Exit => exit += 1,
            FillReason::StopLoss => stop_loss += 1,
            FillReason::TakeProfit => take_profit += 1,
        }
        fee += fill.fee;
        slippage += fill.slippage * fill.quantity;
    }
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule execution run");
    let _ = writeln!(out, "data_version: {}", meta.data_version);
    let _ = writeln!(out, "execution_config_id: {}", meta.config_id);
    let _ = writeln!(out, "execution_timeframe: {}", meta.fill_timeframe);
    let _ = writeln!(out, "decision_bars: {}", meta.decision_bars);
    let _ = writeln!(out, "bars: {}", meta.bars);
    let _ = writeln!(out, "sizing: {}", sizing_label(meta.sizing));
    let _ = writeln!(out, "sizing_basis: {}", meta.sizing_basis);
    let _ = writeln!(out, "sizing_equity: {}", summary_number(meta.sizing_equity));
    let _ = writeln!(out, "max_notional: {}", summary_number(meta.max_notional));
    let _ = writeln!(
        out,
        "orders: pending={pending} filled={filled} rejected={rejected}"
    );
    let _ = writeln!(
        out,
        "fills: count={} buy={buys} sell={sells} entry={entry} exit={exit} stop_loss={stop_loss} take_profit={take_profit}",
        fills.len()
    );
    let _ = writeln!(out, "rejections: count={}", rejections.len());
    let _ = writeln!(
        out,
        "costs: fee={} slippage={}",
        number(fee),
        number(slippage)
    );
    out
}

fn sizing_label(sizing: quantrule_backtest::portfolio::SizingMode) -> &'static str {
    match sizing {
        quantrule_backtest::portfolio::SizingMode::FixedNotional { .. } => "fixed_notional",
        quantrule_backtest::portfolio::SizingMode::EquityFraction { .. } => "equity_fraction",
    }
}

fn provenance_bytes(meta: &ExecutionArtifactMeta<'_>) -> Result<Vec<u8>, ExecutionRunError> {
    let value = json!({
        "schema": "quantrule.stage-provenance/v1",
        "stage": "execution",
        "artifact": "fills",
        "data_version": meta.data_version,
        "config_id": meta.config_id,
        "code_commit": "unknown",
        "source_artifacts": [{
            "stage": "strategy",
            "artifact": "decisions",
            "schema": "quantrule.stage-provenance/v1",
            "data_version": meta.data_version,
            "config_id": meta.strategy_config_id,
        }],
        "seed": Value::Null,
        "period": {
            "from": meta.period_from,
            "to": meta.period_to,
        },
        "artifacts": ["orders.csv", "fills.csv", "rejections.csv", "summary.txt", "provenance.json"],
        "execution": {
            "fill_timeframe": meta.fill_timeframe,
            "fee_rate": meta.fee_rate,
            "slippage_bps": meta.slippage_bps,
            "sizing": sizing_label(meta.sizing),
            "sizing_basis": meta.sizing_basis,
            "sizing_equity": meta.sizing_equity,
            "max_notional": meta.max_notional,
        },
    });
    let mut bytes = serde_json::to_vec(&value).map_err(ExecutionRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn summary_number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), number)
}

fn order_side(side: OrderSide) -> &'static str {
    match side {
        OrderSide::Buy => "buy",
        OrderSide::Sell => "sell",
    }
}

fn fill_reason(reason: FillReason) -> &'static str {
    match reason {
        FillReason::Entry => "entry",
        FillReason::Exit => "exit",
        FillReason::StopLoss => "stop_loss",
        FillReason::TakeProfit => "take_profit",
    }
}

fn order_state(state: OrderState) -> &'static str {
    match state {
        OrderState::Pending => "pending",
        OrderState::Filled => "filled",
        OrderState::Rejected => "rejected",
    }
}

fn reject_reason(reason: RejectReason) -> &'static str {
    match reason {
        RejectReason::NoNextBar => "no_next_bar",
        RejectReason::NonContiguousBar => "non_contiguous_bar",
        RejectReason::NoPosition => "no_position",
        RejectReason::WrongExitSide => "wrong_exit_side",
        RejectReason::PositionAlreadyOpen => "position_already_open",
        RejectReason::ZeroVolume => "zero_volume",
        RejectReason::MissingAtr => "missing_atr",
        RejectReason::InvalidBracket => "invalid_bracket",
        RejectReason::UnsupportedEntryStyle => "unsupported_entry_style",
        RejectReason::ExpiredIntent => "expired_intent",
        RejectReason::SlippageLimit => "slippage_limit",
        RejectReason::MissingQuantity => "missing_quantity",
        RejectReason::NonPositiveQuantity => "non_positive_quantity",
        RejectReason::RiskLimit => "risk_limit",
        RejectReason::OcoCancelled => "oco_cancelled",
        RejectReason::AmbiguousTrigger => "ambiguous_trigger",
        RejectReason::BracketExpired => "bracket_expired",
    }
}

fn number(value: f64) -> String {
    value.to_string()
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(String::new, number)
}

fn optional_usize(value: Option<usize>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn optional_i64(value: Option<i64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::{fill_reason, order_side, order_state, reject_reason};
    use quantrule_backtest::execution::{FillReason, OrderSide, OrderState, RejectReason};

    #[test]
    fn execution_codes_are_stable() {
        assert_eq!(order_side(OrderSide::Buy), "buy");
        assert_eq!(order_side(OrderSide::Sell), "sell");
        assert_eq!(fill_reason(FillReason::Entry), "entry");
        assert_eq!(fill_reason(FillReason::StopLoss), "stop_loss");
        assert_eq!(order_state(OrderState::Pending), "pending");
        assert_eq!(order_state(OrderState::Rejected), "rejected");
        assert_eq!(reject_reason(RejectReason::MissingAtr), "missing_atr");
        assert_eq!(
            reject_reason(RejectReason::PositionAlreadyOpen),
            "position_already_open"
        );
    }
}
