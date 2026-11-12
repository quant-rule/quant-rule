//! The `portfolio replay` command.
//!
//! The command reads an execution fills bundle and the run configuration,
//! drives the portfolio ledger, and writes the trades, equity, funding, risk
//! events, metrics, summary, and provenance. The portfolio stage does not
//! decide a signal and does not alter a fill.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::portfolio::{
    EquityPoint, ExitReason, FundingCharge, PortfolioLedger, PortfolioLedgerError,
    PortfolioMetrics, RiskEvent, SizingMode, TradeRecord,
};
use quantrule_data::Candle;
use serde_json::{Value, json};

use super::fills::{FillsBundle, FillsError};
use crate::artifact::render_report::{self, BundleWriteError};
use crate::config::{ConfigError, RunConfig};
use crate::csv::SourceError;
use crate::pipeline;

/// The portfolio command failed.
#[derive(Debug)]
pub enum PortfolioRunError {
    /// The run configuration is invalid.
    Config(ConfigError),
    /// The execution fills bundle is invalid.
    Fills(FillsError),
    /// The candle source or the data engine failed.
    Source(SourceError),
    /// The portfolio ledger rejected a fill or a mark.
    Ledger(PortfolioLedgerError),
    /// The execution timeframe has no candles.
    EmptySeries {
        /// The timeframe code.
        timeframe: &'static str,
    },
    /// The fills and the run config name different data versions.
    DataVersionMismatch {
        /// The fills artifact's version.
        fills: String,
        /// The run configuration's version.
        run: String,
    },
    /// A fill names a bar outside the execution series.
    FillBarOutOfRange {
        /// The fill index.
        fill: usize,
        /// The fill's bar.
        bar: usize,
        /// The available bars.
        bars: usize,
    },
    /// A fill does not describe the matching candle.
    FillBarMismatch {
        /// The fill index.
        fill: usize,
        /// The fill's bar.
        bar: usize,
        /// The fill's time.
        time: i64,
        /// The candle's open time.
        candle_time: i64,
    },
    /// The output bundle could not be written.
    Bundle(BundleWriteError),
    /// The portfolio artifact could not be encoded.
    Json(serde_json::Error),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for PortfolioRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Fills(error) => error.fmt(formatter),
            Self::Source(error) => error.fmt(formatter),
            Self::Ledger(error) => error.fmt(formatter),
            Self::EmptySeries { timeframe } => {
                write!(
                    formatter,
                    "the execution timeframe {timeframe} has no candles"
                )
            }
            Self::DataVersionMismatch { fills, run } => write!(
                formatter,
                "the fills data version '{fills}' does not match the run's '{run}'"
            ),
            Self::FillBarOutOfRange { fill, bar, bars } => write!(
                formatter,
                "fill {fill} is bar {bar}, but the execution series has {bars} bars"
            ),
            Self::FillBarMismatch {
                fill,
                bar,
                time,
                candle_time,
            } => write!(
                formatter,
                "fill {fill} is bar {bar} at {time}s, but the candle is at {candle_time}s"
            ),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "cannot encode the portfolio artifact: {error}"),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for PortfolioRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Fills(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Ledger(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::EmptySeries { .. }
            | Self::DataVersionMismatch { .. }
            | Self::FillBarOutOfRange { .. }
            | Self::FillBarMismatch { .. } => None,
        }
    }
}

/// Replays one execution fills bundle through the portfolio ledger.
///
/// One portfolio replay request.
pub struct PortfolioRequest<'a> {
    /// The run configuration path.
    pub config: &'a Path,
    /// The execution fills bundle directory.
    pub fills: &'a Path,
    /// The output root; the fills bundle's parent when `None`.
    pub out: Option<&'a Path>,
}

/// The written portfolio bundle.
pub struct PortfolioOutcome {
    /// The bundle root.
    pub root: PathBuf,
    /// The artifact names under the root.
    pub artifacts: Vec<String>,
    /// The `--json` envelope.
    pub json: Vec<u8>,
}

/// # Errors
///
/// Returns [`PortfolioRunError`] when a config, fill artifact, candle series,
/// ledger rule, or output target fails.
pub fn replay(request: &PortfolioRequest<'_>) -> Result<PortfolioOutcome, PortfolioRunError> {
    let config = RunConfig::load(request.config).map_err(PortfolioRunError::Config)?;
    let fills = FillsBundle::load(request.fills).map_err(PortfolioRunError::Fills)?;
    if fills.data_version != config.data.data_version {
        return Err(PortfolioRunError::DataVersionMismatch {
            fills: fills.data_version,
            run: config.data.data_version.clone(),
        });
    }

    let fill_timeframe = config.execution.fill_timeframe;
    let set = pipeline::load_set_with_limit(
        &config.data.source.path,
        config.data.bounds,
        config.data.max_candles,
    )
    .map_err(PortfolioRunError::Source)?;
    let candles: Vec<Candle> = set.series(fill_timeframe).iter().copied().collect();
    if candles.is_empty() {
        return Err(PortfolioRunError::EmptySeries {
            timeframe: fill_timeframe.code(),
        });
    }
    for (index, fill) in fills.fills.iter().enumerate() {
        let Some(candle) = candles.get(fill.bar) else {
            return Err(PortfolioRunError::FillBarOutOfRange {
                fill: index,
                bar: fill.bar,
                bars: candles.len(),
            });
        };
        if fill.time != candle.timestamp() {
            return Err(PortfolioRunError::FillBarMismatch {
                fill: index,
                bar: fill.bar,
                time: fill.time,
                candle_time: candle.timestamp(),
            });
        }
    }

    let mut fills_by_bar: BTreeMap<usize, Vec<quantrule_backtest::execution::Fill>> =
        BTreeMap::new();
    for fill in &fills.fills {
        fills_by_bar.entry(fill.bar).or_default().push(*fill);
    }

    let mut ledger = PortfolioLedger::new(config.portfolio).map_err(PortfolioRunError::Ledger)?;
    for (bar, candle) in candles.iter().enumerate() {
        ledger
            .apply_funding(bar, candle.timestamp(), candle.open(), candle.volume())
            .map_err(PortfolioRunError::Ledger)?;
        if let Some(bar_fills) = fills_by_bar.get(&bar) {
            for fill in bar_fills {
                ledger.apply_fill(fill).map_err(PortfolioRunError::Ledger)?;
            }
        }
        ledger
            .mark(bar, candle.timestamp(), candle.close())
            .map_err(PortfolioRunError::Ledger)?;
    }

    let metrics = ledger.metrics();
    let root = request
        .out
        .map_or_else(
            || {
                request
                    .fills
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| PathBuf::from("."))
            },
            Path::to_path_buf,
        )
        .join("portfolio");
    render_report::ensure_available(&root).map_err(PortfolioRunError::Bundle)?;

    let summary = summary_bytes(
        &config,
        &fills,
        fill_timeframe.code(),
        candles.len(),
        &metrics,
    );
    let provenance = provenance_bytes(&config, &fills, fill_timeframe.code())?;
    let artifacts = [
        ("trades.csv", trades_csv(ledger.trades()).into_bytes()),
        (
            "equity.csv",
            equity_csv(ledger.equity_points()).into_bytes(),
        ),
        (
            "funding.csv",
            funding_csv(ledger.funding_charges()).into_bytes(),
        ),
        (
            "risk_events.csv",
            risk_events_csv(ledger.risk_events()).into_bytes(),
        ),
        ("summary.txt", summary.into_bytes()),
        ("metrics.json", metrics_json(&metrics)?),
    ];
    render_report::write_stage_bundle(&root, &artifacts, &provenance, "metrics.json")
        .map_err(PortfolioRunError::Bundle)?;

    let artifact_names = vec![
        "trades.csv".to_owned(),
        "equity.csv".to_owned(),
        "funding.csv".to_owned(),
        "risk_events.csv".to_owned(),
        "metrics.json".to_owned(),
        "summary.txt".to_owned(),
        "provenance.json".to_owned(),
    ];
    let value = json!({
        "bundle": root,
        "artifacts": artifact_names,
    });
    let mut json = serde_json::to_vec(&value).map_err(PortfolioRunError::Json)?;
    json.push(b'\n');
    Ok(PortfolioOutcome {
        root,
        artifacts: artifact_names,
        json,
    })
}

fn trades_csv(trades: &[TradeRecord]) -> String {
    let mut csv = String::from(
        "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return_fraction,hold_bars,exit_reason\n",
    );
    for trade in trades {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            trade.trade_id,
            side(trade.side),
            number(trade.quantity),
            trade.entry_bar,
            trade.entry_time,
            number(trade.entry_price),
            trade.exit_bar,
            trade.exit_time,
            number(trade.exit_price),
            number(trade.gross_pnl),
            number(trade.execution_fees),
            number(trade.commission),
            number(trade.funding),
            number(trade.net_pnl),
            optional_number(trade.return_fraction),
            trade.hold_bars,
            exit_reason(trade.exit_reason),
        );
    }
    csv
}

fn equity_csv(points: &[EquityPoint]) -> String {
    let mut csv = String::from("date,cash,position_value,equity,peak_equity,drawdown\n");
    for point in points {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{}",
            point.date,
            number(point.cash),
            number(point.position_value),
            number(point.equity),
            number(point.peak_equity),
            number(point.drawdown),
        );
    }
    csv
}

fn funding_csv(charges: &[FundingCharge]) -> String {
    let mut csv = String::from("bar,time,rate,price,quantity,amount\n");
    for charge in charges {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{}",
            charge.bar,
            charge.time,
            number(charge.rate),
            number(charge.price),
            number(charge.quantity),
            number(charge.amount),
        );
    }
    csv
}

fn risk_events_csv(events: &[RiskEvent]) -> String {
    let mut csv = String::from("bar,time,drawdown,equity,peak_equity,limit\n");
    for event in events {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{}",
            event.bar,
            event.time,
            number(event.drawdown),
            number(event.equity),
            number(event.peak_equity),
            number(event.limit),
        );
    }
    csv
}

fn metrics_json(metrics: &PortfolioMetrics) -> Result<Vec<u8>, PortfolioRunError> {
    let value = json!({
        "schema": "quantrule.portfolio-metrics/v1",
        "initial_cash": metrics.initial_cash,
        "final_equity": metrics.final_equity,
        "total_return": metrics.total_return,
        "realized_pnl": metrics.realized_pnl,
        "unrealized_pnl": metrics.unrealized_pnl,
        "gross_pnl": metrics.gross_pnl,
        "net_pnl": metrics.net_pnl,
        "execution_fees": metrics.execution_fees,
        "commission": metrics.commission,
        "funding_paid": metrics.funding_paid,
        "funding_received": metrics.funding_received,
        "max_drawdown": metrics.max_drawdown,
        "max_drawdown_duration_bars": metrics.max_drawdown_duration_bars,
        "trades": metrics.trades,
        "wins": metrics.wins,
        "losses": metrics.losses,
        "win_rate": metrics.win_rate,
        "profit_factor": metrics.profit_factor,
        "average_trade": metrics.average_trade,
        "average_hold_bars": metrics.average_hold_bars,
        "exposure": metrics.exposure,
        "halted": metrics.halted,
        "open_position": metrics.open_position.as_ref().map(|position| json!({
            "side": side(position.side),
            "quantity": position.quantity,
            "entry_price": position.entry_price,
            "entry_bar": position.entry_bar,
            "entry_time": position.entry_time,
            "entry_equity": position.entry_equity,
            "entry_fees": position.entry_fees,
            "entry_commission": position.entry_commission,
            "funding_paid": position.funding_paid,
            "funding_received": position.funding_received,
        })),
        "monthly_returns": metrics.monthly_returns.iter().map(|month| json!({
            "year": month.year,
            "month": month.month,
            "return_fraction": month.return_fraction,
        })).collect::<Vec<Value>>(),
    });
    let mut bytes = serde_json::to_vec(&value).map_err(PortfolioRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn summary_bytes(
    config: &RunConfig,
    fills: &FillsBundle,
    timeframe: &str,
    bars: usize,
    metrics: &PortfolioMetrics,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule portfolio replay");
    let _ = writeln!(out, "data_version: {}", fills.data_version);
    let _ = writeln!(out, "portfolio_config_id: {}", config.config_hash);
    let _ = writeln!(out, "execution_timeframe: {timeframe}");
    let _ = writeln!(out, "bars: {bars}");
    let _ = writeln!(out, "fills: {}", fills.fills.len());
    let _ = writeln!(out, "trades: {}", metrics.trades);
    let _ = writeln!(out, "wins: {}", metrics.wins);
    let _ = writeln!(out, "losses: {}", metrics.losses);
    let _ = writeln!(out, "net_pnl: {}", number(metrics.net_pnl));
    let _ = writeln!(out, "total_return: {}", number(metrics.total_return));
    let _ = writeln!(out, "max_drawdown: {}", number(metrics.max_drawdown));
    let _ = writeln!(out, "execution_fees: {}", number(metrics.execution_fees));
    let _ = writeln!(out, "commission: {}", number(metrics.commission));
    let _ = writeln!(out, "funding_paid: {}", number(metrics.funding_paid));
    let _ = writeln!(
        out,
        "funding_received: {}",
        number(metrics.funding_received)
    );
    let _ = writeln!(out, "win_rate: {}", summary_number(metrics.win_rate));
    let _ = writeln!(
        out,
        "profit_factor: {}",
        summary_number(metrics.profit_factor)
    );
    let _ = writeln!(out, "exposure: {}", summary_number(metrics.exposure));
    let _ = writeln!(out, "halted: {}", metrics.halted);
    out
}

fn provenance_bytes(
    config: &RunConfig,
    fills: &FillsBundle,
    timeframe: &str,
) -> Result<Vec<u8>, PortfolioRunError> {
    let value = json!({
        "schema": "quantrule.stage-provenance/v1",
        "stage": "portfolio",
        "artifact": "portfolio",
        "data_version": fills.data_version.as_str(),
        "config_id": config.config_hash.as_str(),
        "code_commit": "unknown",
        "source_artifacts": [{
            "stage": "execution",
            "artifact": "fills",
            "schema": "quantrule.stage-provenance/v1",
            "data_version": fills.data_version.as_str(),
            "config_id": fills.config_id.as_str(),
        }],
        "seed": Value::Null,
        "period": {
            "from": fills.period_from.as_deref(),
            "to": fills.period_to.as_deref(),
        },
        "artifacts": [
            "trades.csv",
            "equity.csv",
            "funding.csv",
            "risk_events.csv",
            "metrics.json",
            "summary.txt",
            "provenance.json"
        ],
        "portfolio": {
            "initial_cash": config.portfolio.initial_cash,
            "sizing": sizing_label(config.portfolio.sizing),
            "leverage": config.portfolio.leverage,
            "commission_rate": config.portfolio.commission.rate(),
            "funding_rate": config.portfolio.funding.rate(),
            "execution_timeframe": timeframe,
        },
    });
    let mut bytes = serde_json::to_vec(&value).map_err(PortfolioRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn side(side: quantrule_backtest::side::Side) -> &'static str {
    match side {
        quantrule_backtest::side::Side::Long => "long",
        quantrule_backtest::side::Side::Short => "short",
    }
}

fn exit_reason(reason: ExitReason) -> &'static str {
    reason.code()
}

fn sizing_label(sizing: SizingMode) -> &'static str {
    match sizing {
        SizingMode::FixedNotional { .. } => "fixed_notional",
        SizingMode::EquityFraction { .. } => "equity_fraction",
    }
}

fn number(value: f64) -> String {
    value.to_string()
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(String::new, number)
}

fn summary_number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), number)
}

#[cfg(test)]
mod tests {
    use super::{exit_reason, side, sizing_label};
    use quantrule_backtest::portfolio::{ExitReason, SizingMode};
    use quantrule_backtest::side::Side;

    #[test]
    fn portfolio_codes_are_stable() {
        assert_eq!(side(Side::Long), "long");
        assert_eq!(side(Side::Short), "short");
        assert_eq!(exit_reason(ExitReason::StrategyExit), "strategy");
        assert_eq!(exit_reason(ExitReason::Stop), "stop");
        assert_eq!(exit_reason(ExitReason::Target), "target");
        assert_eq!(
            sizing_label(SizingMode::FixedNotional {
                notional: 1.0,
                quantity_step: 0.1,
            }),
            "fixed_notional"
        );
    }
}
