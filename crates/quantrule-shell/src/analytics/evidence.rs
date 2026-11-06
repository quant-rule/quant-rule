//! Read the result artifacts the Analytics Engine evaluates.
//!
//! The shared [`BundleManifest`] reader validates the manifest and every stage
//! provenance hash. This module adds the portfolio metrics, the portfolio
//! tables, and the strategy decision records. It does not rerun a stage, read a
//! candle, or write a verdict.

use std::fmt;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::backtest_bundle::{BundleError, BundleManifest};

/// The portfolio metrics schema.
pub(crate) const METRICS_SCHEMA: &str = "quantrule.portfolio-metrics/v1";
/// The strategy decision schema.
pub(crate) const DECISION_SCHEMA: &str = "quantrule.strategy-decision/v1";

/// The portfolio trades header.
pub(crate) const TRADES_HEADER: &str = "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return_fraction,hold_bars,exit_reason";
/// The portfolio equity header.
pub(crate) const EQUITY_HEADER: &str = "date,cash,position_value,equity,peak_equity,drawdown";
/// The portfolio funding header.
pub(crate) const FUNDING_HEADER: &str = "bar,time,rate,price,quantity,amount";
/// The portfolio risk-event header.
pub(crate) const RISK_EVENTS_HEADER: &str = "bar,time,drawdown,equity,peak_equity,limit";

/// A result artifact could not be read.
#[derive(Debug)]
pub(crate) enum AnalyticsEvidenceError {
    /// The shared manifest or stage provenance is invalid.
    Bundle(BundleError),
    /// A required artifact is not shaped like the documented bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the artifact's own field names.
        message: String,
    },
    /// The portfolio metrics use an unsupported schema.
    UnsupportedSchema {
        /// The artifact path.
        path: PathBuf,
        /// The expected schema.
        expected: &'static str,
        /// The observed schema.
        observed: String,
    },
}

impl fmt::Display for AnalyticsEvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bundle(error) => error.fmt(formatter),
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid evidence bundle {}: {message}",
                    path.display()
                )
            }
            Self::UnsupportedSchema {
                path,
                expected,
                observed,
            } => write!(
                formatter,
                "{} uses schema '{observed}', expected '{expected}'",
                path.display()
            ),
        }
    }
}

impl std::error::Error for AnalyticsEvidenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bundle(error) => Some(error),
            Self::Invalid { .. } | Self::UnsupportedSchema { .. } => None,
        }
    }
}

/// One monthly return.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MonthlyReturn {
    /// The UTC year.
    pub(crate) year: u64,
    /// The UTC month, `1..=12`.
    pub(crate) month: u64,
    /// The month's return, or `None` when it is undefined.
    pub(crate) return_fraction: Option<f64>,
}

/// The portfolio metrics the analytics stage republishes.
#[derive(Debug, Clone)]
pub(crate) struct PortfolioSnapshot {
    /// The starting cash.
    pub(crate) initial_cash: f64,
    /// The equity at the last mark.
    pub(crate) final_equity: f64,
    /// `final_equity / initial_cash - 1`.
    pub(crate) total_return: f64,
    /// Closed-trade net P&L.
    pub(crate) realized_pnl: f64,
    /// Open-position net P&L.
    pub(crate) unrealized_pnl: f64,
    /// Closed gross P&L plus the open position's gross P&L.
    pub(crate) gross_pnl: f64,
    /// `realized_pnl + unrealized_pnl`.
    pub(crate) net_pnl: f64,
    /// Execution fees paid on every fill.
    pub(crate) execution_fees: f64,
    /// Commission paid on every fill.
    pub(crate) commission: f64,
    /// Funding paid at aligned boundaries.
    pub(crate) funding_paid: f64,
    /// Funding received at aligned boundaries.
    pub(crate) funding_received: f64,
    /// The maximum drawdown observed at any refresh.
    pub(crate) max_drawdown: f64,
    /// The longest run of marked bars with a positive drawdown.
    pub(crate) max_drawdown_duration_bars: u64,
    /// The number of closed trades.
    pub(crate) trades: u64,
    /// Closed trades with a positive net P&L.
    pub(crate) wins: u64,
    /// Closed trades with a negative net P&L.
    pub(crate) losses: u64,
    /// `wins / trades`, or `None` with no trades.
    pub(crate) win_rate: Option<f64>,
    /// Winning net P&L divided by the absolute losing net P&L.
    pub(crate) profit_factor: Option<f64>,
    /// Mean closed-trade net P&L.
    pub(crate) average_trade: Option<f64>,
    /// Mean closed-trade hold in strategy bars.
    pub(crate) average_hold_bars: Option<f64>,
    /// `sum(position_value) / sum(equity)` over marks.
    pub(crate) exposure: Option<f64>,
    /// Whether the drawdown halt has latched.
    pub(crate) halted: bool,
    /// One entry per month in the actual period.
    pub(crate) monthly_returns: Vec<MonthlyReturn>,
}

/// A strategy decision's kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DecisionKind {
    /// Hold.
    Hold,
    /// Market entry.
    Enter,
    /// Bracket entry.
    EnterBracket,
    /// Exit.
    Exit,
}

impl DecisionKind {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Enter => "enter",
            Self::EnterBracket => "enter_bracket",
            Self::Exit => "exit",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "hold" => Some(Self::Hold),
            "enter" => Some(Self::Enter),
            "enter_bracket" => Some(Self::EnterBracket),
            "exit" => Some(Self::Exit),
            _ => None,
        }
    }
}

/// A strategy decision's side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DecisionSide {
    /// Long.
    Long,
    /// Short.
    Short,
}

impl DecisionSide {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "long" => Some(Self::Long),
            "short" => Some(Self::Short),
            _ => None,
        }
    }
}

/// A strategy decision's reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DecisionReason {
    /// Entry conditions met.
    Entry,
    /// Bracket entry conditions met.
    BracketEntry,
    /// An event condition exited the position.
    EventCondition,
    /// The time stop exited the position.
    TimeStop,
}

impl DecisionReason {
    /// The stable wire code.
    #[must_use]
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::BracketEntry => "bracket_entry",
            Self::EventCondition => "event_condition",
            Self::TimeStop => "time_stop",
        }
    }

    fn from_code(code: &str) -> Option<Self> {
        match code {
            "entry" => Some(Self::Entry),
            "bracket_entry" => Some(Self::BracketEntry),
            "event_condition" => Some(Self::EventCondition),
            "time_stop" => Some(Self::TimeStop),
            _ => None,
        }
    }
}

/// One strategy decision record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DecisionRecord {
    /// The execution timeframe in seconds.
    pub(crate) timeframe_secs: u64,
    /// The decision kind.
    pub(crate) decision: DecisionKind,
    /// The side, when the decision names one.
    pub(crate) side: Option<DecisionSide>,
    /// The reason, when the decision names one.
    pub(crate) reason: Option<DecisionReason>,
}

/// The complete analytics evidence.
#[derive(Debug, Clone)]
pub(crate) struct AnalyticsEvidence {
    /// The verified bundle identity.
    pub(crate) bundle: BundleManifest,
    /// The parsed portfolio metrics.
    pub(crate) portfolio: PortfolioSnapshot,
    /// The portfolio trades table, including its header.
    pub(crate) trades_csv: String,
    /// The portfolio equity table, including its header.
    pub(crate) equity_csv: String,
    /// The portfolio funding table, including its header.
    pub(crate) funding_csv: String,
    /// The portfolio risk-event table, including its header.
    pub(crate) risk_events_csv: String,
    /// The strategy decisions, in file order.
    pub(crate) decisions: Vec<DecisionRecord>,
}

impl AnalyticsEvidence {
    /// Loads and validates the result artifacts.
    ///
    /// # Errors
    ///
    /// Returns [`AnalyticsEvidenceError`] when the shared bundle reader refuses,
    /// a required table is absent or has an unsupported header, the portfolio
    /// metrics use an unsupported schema, or a decision record is malformed.
    pub(crate) fn load(root: &std::path::Path) -> Result<Self, AnalyticsEvidenceError> {
        let bundle = BundleManifest::load(root).map_err(AnalyticsEvidenceError::Bundle)?;
        let metrics_path = "portfolio/metrics.json";
        let metrics = bundle
            .read_json(metrics_path)
            .map_err(AnalyticsEvidenceError::Bundle)?;
        let portfolio = parse_portfolio_metrics(&metrics, metrics_path)?;

        let trades_csv = bundle
            .read_string("portfolio/trades.csv")
            .map_err(AnalyticsEvidenceError::Bundle)?;
        validate_header("portfolio/trades.csv", &trades_csv, TRADES_HEADER)?;
        let equity_csv = bundle
            .read_string("portfolio/equity.csv")
            .map_err(AnalyticsEvidenceError::Bundle)?;
        validate_header("portfolio/equity.csv", &equity_csv, EQUITY_HEADER)?;
        let funding_csv = bundle
            .read_string("portfolio/funding.csv")
            .map_err(AnalyticsEvidenceError::Bundle)?;
        validate_header("portfolio/funding.csv", &funding_csv, FUNDING_HEADER)?;
        let risk_events_csv = bundle
            .read_string("portfolio/risk_events.csv")
            .map_err(AnalyticsEvidenceError::Bundle)?;
        validate_header(
            "portfolio/risk_events.csv",
            &risk_events_csv,
            RISK_EVENTS_HEADER,
        )?;

        let decisions_text = bundle
            .read_string("strategy/decisions.ndjson")
            .map_err(AnalyticsEvidenceError::Bundle)?;
        let decisions = parse_decisions(&decisions_text)?;

        Ok(Self {
            bundle,
            portfolio,
            trades_csv,
            equity_csv,
            funding_csv,
            risk_events_csv,
            decisions,
        })
    }
}

fn parse_portfolio_metrics(
    value: &Value,
    path: &str,
) -> Result<PortfolioSnapshot, AnalyticsEvidenceError> {
    let object = as_object(path, value)?;
    let schema = required_str(path, object, "schema")?;
    if schema != METRICS_SCHEMA {
        return Err(AnalyticsEvidenceError::UnsupportedSchema {
            path: PathBuf::from(path),
            expected: METRICS_SCHEMA,
            observed: schema.to_owned(),
        });
    }
    let monthly_returns = parse_monthly_returns(path, object)?;
    Ok(PortfolioSnapshot {
        initial_cash: required_number(path, object, "initial_cash")?,
        final_equity: required_number(path, object, "final_equity")?,
        total_return: required_number(path, object, "total_return")?,
        realized_pnl: required_number(path, object, "realized_pnl")?,
        unrealized_pnl: required_number(path, object, "unrealized_pnl")?,
        gross_pnl: required_number(path, object, "gross_pnl")?,
        net_pnl: required_number(path, object, "net_pnl")?,
        execution_fees: required_number(path, object, "execution_fees")?,
        commission: required_number(path, object, "commission")?,
        funding_paid: required_number(path, object, "funding_paid")?,
        funding_received: required_number(path, object, "funding_received")?,
        max_drawdown: required_number(path, object, "max_drawdown")?,
        max_drawdown_duration_bars: required_u64(path, object, "max_drawdown_duration_bars")?,
        trades: required_u64(path, object, "trades")?,
        wins: required_u64(path, object, "wins")?,
        losses: required_u64(path, object, "losses")?,
        win_rate: optional_number(path, object, "win_rate")?,
        profit_factor: optional_number(path, object, "profit_factor")?,
        average_trade: optional_number(path, object, "average_trade")?,
        average_hold_bars: optional_number(path, object, "average_hold_bars")?,
        exposure: optional_number(path, object, "exposure")?,
        halted: required_bool(path, object, "halted")?,
        monthly_returns,
    })
}

fn parse_monthly_returns(
    path: &str,
    object: &Map<String, Value>,
) -> Result<Vec<MonthlyReturn>, AnalyticsEvidenceError> {
    let values = object
        .get("monthly_returns")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(path, "monthly_returns must be an array"))?;
    let mut returns = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let label = format!("monthly_returns[{index}]");
        let entry = value
            .as_object()
            .ok_or_else(|| invalid(path, &format!("{label} must be an object")))?;
        returns.push(MonthlyReturn {
            year: entry
                .get("year")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid(path, &format!("{label}.year must be an integer")))?,
            month: entry
                .get("month")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid(path, &format!("{label}.month must be an integer")))?,
            return_fraction: optional_number(path, entry, "return_fraction")?,
        });
    }
    Ok(returns)
}

fn parse_decisions(text: &str) -> Result<Vec<DecisionRecord>, AnalyticsEvidenceError> {
    let path = "strategy/decisions.ndjson";
    let mut decisions = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| invalid(path, &format!("line {} is not JSON: {error}", index + 1)))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid(path, &format!("line {} must be an object", index + 1)))?;
        let schema = object
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(path, &format!("line {} has no schema", index + 1)))?;
        if schema != DECISION_SCHEMA {
            return Err(AnalyticsEvidenceError::UnsupportedSchema {
                path: PathBuf::from(path),
                expected: DECISION_SCHEMA,
                observed: schema.to_owned(),
            });
        }
        let timeframe_secs = object
            .get("timeframe_secs")
            .and_then(Value::as_u64)
            .filter(|secs| *secs > 0)
            .ok_or_else(|| {
                invalid(
                    path,
                    &format!(
                        "line {} timeframe_secs must be a positive integer",
                        index + 1
                    ),
                )
            })?;
        let decision_code = object
            .get("decision")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(path, &format!("line {} has no decision", index + 1)))?;
        let decision = DecisionKind::from_code(decision_code).ok_or_else(|| {
            invalid(
                path,
                &format!("line {} has unknown decision '{decision_code}'", index + 1),
            )
        })?;
        let side = match object.get("side") {
            None | Some(Value::Null) => None,
            Some(Value::String(code)) => Some(DecisionSide::from_code(code).ok_or_else(|| {
                invalid(
                    path,
                    &format!("line {} has unknown side '{code}'", index + 1),
                )
            })?),
            Some(_) => {
                return Err(invalid(
                    path,
                    &format!("line {} side must be a string or null", index + 1),
                ));
            }
        };
        let reason = match object.get("reason") {
            None | Some(Value::Null) => None,
            Some(Value::String(code)) => {
                Some(DecisionReason::from_code(code).ok_or_else(|| {
                    invalid(
                        path,
                        &format!("line {} has unknown reason '{code}'", index + 1),
                    )
                })?)
            }
            Some(_) => {
                return Err(invalid(
                    path,
                    &format!("line {} reason must be a string or null", index + 1),
                ));
            }
        };
        decisions.push(DecisionRecord {
            timeframe_secs,
            decision,
            side,
            reason,
        });
    }
    Ok(decisions)
}

fn validate_header(path: &str, text: &str, expected: &str) -> Result<(), AnalyticsEvidenceError> {
    let Some(first) = text.lines().next() else {
        return Err(invalid(path, "the table is empty"));
    };
    if first != expected {
        return Err(invalid(
            path,
            &format!("expected header '{expected}', observed '{first}'"),
        ));
    }
    Ok(())
}

fn as_object<'a>(
    path: &str,
    value: &'a Value,
) -> Result<&'a Map<String, Value>, AnalyticsEvidenceError> {
    value
        .as_object()
        .ok_or_else(|| invalid(path, "the document root must be an object"))
}

fn required_str<'a>(
    path: &str,
    object: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a str, AnalyticsEvidenceError> {
    object
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(path, &format!("{name} must be a string")))
}

fn required_number(
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<f64, AnalyticsEvidenceError> {
    let value = object
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| invalid(path, &format!("{name} must be a finite number")))?;
    if !value.is_finite() {
        return Err(invalid(path, &format!("{name} must be a finite number")));
    }
    Ok(value)
}

fn optional_number(
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<Option<f64>, AnalyticsEvidenceError> {
    match object.get(name) {
        Some(Value::Null) | None => Ok(None),
        Some(value) => {
            let number = value
                .as_f64()
                .ok_or_else(|| invalid(path, &format!("{name} must be a number or null")))?;
            if !number.is_finite() {
                return Err(invalid(
                    path,
                    &format!("{name} must be a finite number or null"),
                ));
            }
            Ok(Some(number))
        }
    }
}

fn required_u64(
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<u64, AnalyticsEvidenceError> {
    object
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid(path, &format!("{name} must be a non-negative integer")))
}

fn required_bool(
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<bool, AnalyticsEvidenceError> {
    object
        .get(name)
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid(path, &format!("{name} must be a boolean")))
}

fn invalid(path: &str, message: &str) -> AnalyticsEvidenceError {
    AnalyticsEvidenceError::Invalid {
        path: PathBuf::from(path),
        message: message.to_owned(),
    }
}

/// The number of data rows in a table, excluding its header.
#[must_use]
pub(crate) fn row_count(text: &str) -> usize {
    text.lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .count()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DecisionKind, DecisionReason, DecisionSide, EQUITY_HEADER, parse_decisions,
        parse_portfolio_metrics, row_count, validate_header,
    };

    #[test]
    fn a_metrics_document_parses_with_nullable_metrics() {
        let value = json!({
            "schema": "quantrule.portfolio-metrics/v1",
            "initial_cash": 10000.0,
            "final_equity": 9000.0,
            "total_return": -0.1,
            "realized_pnl": -1000.0,
            "unrealized_pnl": 0.0,
            "gross_pnl": -900.0,
            "net_pnl": -1000.0,
            "execution_fees": 100.0,
            "commission": 0.0,
            "funding_paid": 0.0,
            "funding_received": 0.0,
            "max_drawdown": 0.1,
            "max_drawdown_duration_bars": 4,
            "trades": 0,
            "wins": 0,
            "losses": 0,
            "win_rate": null,
            "profit_factor": null,
            "average_trade": null,
            "average_hold_bars": null,
            "exposure": null,
            "halted": false,
            "monthly_returns": [{"year": 2025, "month": 1, "return_fraction": null}]
        });
        let metrics = parse_portfolio_metrics(&value, "metrics.json").expect("the metrics parse");
        assert_eq!(metrics.trades, 0);
        assert_eq!(metrics.win_rate, None);
        assert_eq!(metrics.monthly_returns[0].return_fraction, None);
    }

    #[test]
    fn a_decision_record_parses_into_counts() {
        let text = concat!(
            "{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":300,\"decision\":\"hold\",\"side\":null,\"reason\":null}\n",
            "{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":300,\"decision\":\"enter\",\"side\":\"long\",\"reason\":\"entry\"}\n",
        );
        let decisions = parse_decisions(text).expect("the decisions parse");
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].decision, DecisionKind::Hold);
        assert_eq!(decisions[0].side, None);
        assert_eq!(decisions[1].decision, DecisionKind::Enter);
        assert_eq!(decisions[1].side, Some(DecisionSide::Long));
        assert_eq!(decisions[1].reason, Some(DecisionReason::Entry));
    }

    #[test]
    fn an_unknown_decision_is_rejected() {
        let text = "{\"schema\":\"quantrule.strategy-decision/v1\",\"timeframe_secs\":300,\"decision\":\"buy\",\"side\":null,\"reason\":null}\n";
        let error = parse_decisions(text).expect_err("the decision is unknown");
        assert!(error.to_string().contains("unknown decision"));
    }

    #[test]
    fn a_table_header_must_match() {
        validate_header("equity.csv", &format!("{EQUITY_HEADER}\n"), EQUITY_HEADER)
            .expect("the header matches");
        let error = validate_header("equity.csv", "wrong\n", EQUITY_HEADER)
            .expect_err("the header differs");
        assert!(error.to_string().contains("expected header"));
    }

    #[test]
    fn a_table_row_count_excludes_the_header_and_blank_lines() {
        assert_eq!(row_count("a,b\n1,2\n3,4\n"), 2);
        assert_eq!(row_count("a,b\n"), 0);
    }
}
