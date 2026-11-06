//! Build the analytics metrics and evaluation tables from the result artifacts.
//!
//! The builder republishes the portfolio metrics, copies the portfolio tables,
//! derives the monthly-return and cost tables, and aggregates the strategy
//! decisions by timeframe. It writes no verdict and computes no signal.
//!
//! A metric that the evidence does not support is `null` and carries a stable
//! reason code. A table the evidence cannot support is listed as unsupported
//! with a reason code; it is never synthesized.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use quantrule_data::Timeframe;

use super::evidence::{
    AnalyticsEvidence, DecisionKind, DecisionReason, DecisionSide, EQUITY_HEADER, FUNDING_HEADER,
    PortfolioSnapshot, RISK_EVENTS_HEADER, TRADES_HEADER, row_count,
};

/// One generated CSV table.
#[derive(Debug, Clone)]
pub(crate) struct Table {
    /// The table name without the extension.
    pub(crate) name: &'static str,
    /// The bundle-relative path.
    pub(crate) file: String,
    /// The fixed header line.
    pub(crate) header: &'static str,
    /// The complete CSV text, including the header.
    pub(crate) text: String,
    /// The number of data rows.
    pub(crate) rows: usize,
}

/// A table the evidence cannot support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnsupportedTable {
    /// The table name without the extension.
    pub(crate) name: &'static str,
    /// The stable reason code.
    pub(crate) reason_code: &'static str,
}

/// The decision counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DecisionCounts {
    /// Hold decisions.
    pub(crate) hold: u64,
    /// Market-entry decisions.
    pub(crate) enter: u64,
    /// Bracket-entry decisions.
    pub(crate) enter_bracket: u64,
    /// Exit decisions.
    pub(crate) exit: u64,
}

impl DecisionCounts {
    /// Counts one decision.
    pub(crate) fn observe(&mut self, decision: DecisionKind) {
        match decision {
            DecisionKind::Hold => self.hold += 1,
            DecisionKind::Enter => self.enter += 1,
            DecisionKind::EnterBracket => self.enter_bracket += 1,
            DecisionKind::Exit => self.exit += 1,
        }
    }

    /// The total number of decisions.
    #[must_use]
    pub(crate) fn total(self) -> u64 {
        self.hold + self.enter + self.enter_bracket + self.exit
    }
}

/// The analytics metrics and tables.
#[derive(Debug, Clone)]
pub(crate) struct AnalyticsMetrics {
    /// The decision counts across every timeframe.
    pub(crate) decisions: DecisionCounts,
    /// Stable reasons for every null metric.
    pub(crate) null_reasons: BTreeMap<&'static str, &'static str>,
    /// The tables, in the fixed declared order.
    pub(crate) tables: Vec<Table>,
    /// The tables the evidence cannot support.
    pub(crate) unsupported: Vec<UnsupportedTable>,
}

/// Builds the analytics metrics and tables.
#[must_use]
pub(crate) fn build(evidence: &AnalyticsEvidence) -> AnalyticsMetrics {
    let mut tables = Vec::new();
    tables.push(copy_table("trades", TRADES_HEADER, &evidence.trades_csv));
    tables.push(monthly_returns_table(&evidence.portfolio.monthly_returns));
    tables.push(costs_table(&evidence.portfolio));
    let decisions = decision_tables(&evidence.decisions);
    tables.push(decisions.table);
    tables.push(decisions.by_timeframe);
    tables.push(copy_table("equity", EQUITY_HEADER, &evidence.equity_csv));
    tables.push(copy_table("funding", FUNDING_HEADER, &evidence.funding_csv));
    tables.push(copy_table(
        "risk_events",
        RISK_EVENTS_HEADER,
        &evidence.risk_events_csv,
    ));

    AnalyticsMetrics {
        decisions: decisions.counts,
        null_reasons: null_reasons(&evidence.portfolio),
        tables,
        unsupported: vec![
            UnsupportedTable {
                name: "by_regime",
                reason_code: "regime_label_not_published",
            },
            UnsupportedTable {
                name: "by_instrument",
                reason_code: "instrument_identity_not_published",
            },
        ],
    }
}

fn copy_table(name: &'static str, header: &'static str, text: &str) -> Table {
    Table {
        name,
        file: format!("tables/{name}.csv"),
        header,
        text: text.to_owned(),
        rows: row_count(text),
    }
}

fn monthly_returns_table(returns: &[super::evidence::MonthlyReturn]) -> Table {
    let header = "year,month,return_fraction";
    let mut text = format!("{header}\n");
    for month in returns {
        let value = month
            .return_fraction
            .map_or_else(|| "null".to_owned(), |value| value.to_string());
        let _ = writeln!(text, "{},{},{}", month.year, month.month, value);
    }
    Table {
        name: "monthly_returns",
        file: "tables/monthly_returns.csv".to_owned(),
        header,
        text,
        rows: returns.len(),
    }
}

fn costs_table(portfolio: &PortfolioSnapshot) -> Table {
    let header = "component,amount";
    let net_costs = portfolio.execution_fees + portfolio.commission + portfolio.funding_paid
        - portfolio.funding_received;
    let mut text = format!("{header}\n");
    for (component, amount) in [
        ("execution_fees", portfolio.execution_fees),
        ("commission", portfolio.commission),
        ("funding_paid", portfolio.funding_paid),
        ("funding_received", portfolio.funding_received),
        ("net_costs", net_costs),
    ] {
        let _ = writeln!(text, "{component},{amount}");
    }
    Table {
        name: "costs",
        file: "tables/costs.csv".to_owned(),
        header,
        text,
        rows: 5,
    }
}

struct DecisionTables {
    counts: DecisionCounts,
    table: Table,
    by_timeframe: Table,
}

fn decision_tables(records: &[super::evidence::DecisionRecord]) -> DecisionTables {
    let mut groups: BTreeMap<
        (
            u64,
            DecisionKind,
            Option<DecisionSide>,
            Option<DecisionReason>,
        ),
        u64,
    > = BTreeMap::new();
    let mut counts = DecisionCounts::default();
    for record in records {
        *groups
            .entry((
                record.timeframe_secs,
                record.decision,
                record.side,
                record.reason,
            ))
            .or_insert(0) += 1;
        counts.observe(record.decision);
    }

    let header = "timeframe,decision,side,reason,count";
    let mut text = format!("{header}\n");
    let mut rows = 0;
    for ((secs, decision, side, reason), count) in &groups {
        let side = side.map_or("none", DecisionSide::code);
        let reason = reason.map_or("none", DecisionReason::code);
        let _ = writeln!(
            text,
            "{},{},{},{},{}",
            timeframe_label(*secs),
            decision.code(),
            side,
            reason,
            count
        );
        rows += 1;
    }
    let table = Table {
        name: "decisions",
        file: "tables/decisions.csv".to_owned(),
        header,
        text,
        rows,
    };

    let mut per_timeframe: BTreeMap<u64, DecisionCounts> = BTreeMap::new();
    for record in records {
        per_timeframe
            .entry(record.timeframe_secs)
            .or_default()
            .observe(record.decision);
    }
    let header = "timeframe,bars,hold,enter,enter_bracket,exit";
    let mut text = format!("{header}\n");
    let mut rows = 0;
    for (secs, counts) in &per_timeframe {
        let _ = writeln!(
            text,
            "{},{},{},{},{},{}",
            timeframe_label(*secs),
            counts.total(),
            counts.hold,
            counts.enter,
            counts.enter_bracket,
            counts.exit
        );
        rows += 1;
    }
    let by_timeframe = Table {
        name: "by_timeframe",
        file: "tables/by_timeframe.csv".to_owned(),
        header,
        text,
        rows,
    };

    DecisionTables {
        counts,
        table,
        by_timeframe,
    }
}

fn null_reasons(portfolio: &PortfolioSnapshot) -> BTreeMap<&'static str, &'static str> {
    let mut reasons = BTreeMap::new();
    if portfolio.win_rate.is_none() {
        reasons.insert(
            "win_rate",
            if portfolio.trades == 0 {
                "no_trades"
            } else {
                "missing_measurement"
            },
        );
    }
    if portfolio.profit_factor.is_none() {
        reasons.insert(
            "profit_factor",
            if portfolio.trades == 0 {
                "no_trades"
            } else if portfolio.losses == 0 {
                "no_losses"
            } else {
                "missing_measurement"
            },
        );
    }
    if portfolio.average_trade.is_none() {
        reasons.insert(
            "average_trade",
            if portfolio.trades == 0 {
                "no_trades"
            } else {
                "missing_measurement"
            },
        );
    }
    if portfolio.average_hold_bars.is_none() {
        reasons.insert(
            "average_hold_bars",
            if portfolio.trades == 0 {
                "no_trades"
            } else {
                "missing_measurement"
            },
        );
    }
    if portfolio.exposure.is_none() {
        reasons.insert("exposure", "no_marks");
    }
    reasons
}

fn timeframe_label(seconds: u64) -> String {
    Timeframe::ALL
        .iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == seconds)
        .map_or_else(
            || format!("{seconds}s"),
            |timeframe| timeframe.code().to_owned(),
        )
}

#[cfg(test)]
mod tests {
    use super::{DecisionCounts, decision_tables, null_reasons, timeframe_label};
    use crate::analytics::evidence::{
        DecisionKind, DecisionReason, DecisionRecord, DecisionSide, MonthlyReturn,
        PortfolioSnapshot,
    };

    fn snapshot(trades: u64, losses: u64) -> PortfolioSnapshot {
        PortfolioSnapshot {
            initial_cash: 10_000.0,
            final_equity: 10_000.0,
            total_return: 0.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            gross_pnl: 0.0,
            net_pnl: 0.0,
            execution_fees: 0.0,
            commission: 0.0,
            funding_paid: 0.0,
            funding_received: 0.0,
            max_drawdown: 0.0,
            max_drawdown_duration_bars: 0,
            trades,
            wins: 0,
            losses,
            win_rate: (trades > 0).then_some(0.0),
            profit_factor: (trades > 0 && losses > 0).then_some(0.0),
            average_trade: (trades > 0).then_some(0.0),
            average_hold_bars: (trades > 0).then_some(0.0),
            exposure: None,
            halted: false,
            monthly_returns: vec![MonthlyReturn {
                year: 2025,
                month: 1,
                return_fraction: None,
            }],
        }
    }

    #[test]
    fn no_trades_names_every_null_metric() {
        let reasons = null_reasons(&snapshot(0, 0));
        assert_eq!(reasons["win_rate"], "no_trades");
        assert_eq!(reasons["profit_factor"], "no_trades");
        assert_eq!(reasons["average_trade"], "no_trades");
        assert_eq!(reasons["average_hold_bars"], "no_trades");
        assert_eq!(reasons["exposure"], "no_marks");
    }

    #[test]
    fn a_trade_with_no_losses_names_the_profit_factor_reason() {
        let reasons = null_reasons(&snapshot(1, 0));
        assert_eq!(reasons["profit_factor"], "no_losses");
        assert!(!reasons.contains_key("win_rate"));
    }

    #[test]
    fn decisions_are_grouped_and_ordered() {
        let records = vec![
            DecisionRecord {
                timeframe_secs: 300,
                decision: DecisionKind::Hold,
                side: None,
                reason: None,
            },
            DecisionRecord {
                timeframe_secs: 300,
                decision: DecisionKind::Enter,
                side: Some(DecisionSide::Long),
                reason: Some(DecisionReason::Entry),
            },
            DecisionRecord {
                timeframe_secs: 900,
                decision: DecisionKind::Exit,
                side: Some(DecisionSide::Long),
                reason: Some(DecisionReason::EventCondition),
            },
        ];
        let tables = decision_tables(&records);
        assert_eq!(
            tables.counts,
            DecisionCounts {
                hold: 1,
                enter: 1,
                enter_bracket: 0,
                exit: 1,
            }
        );
        assert_eq!(
            tables.table.text,
            "timeframe,decision,side,reason,count\n5m,hold,none,none,1\n5m,enter,long,entry,1\n15m,exit,long,event_condition,1\n"
        );
        assert_eq!(
            tables.by_timeframe.text,
            "timeframe,bars,hold,enter,enter_bracket,exit\n5m,2,1,1,0,0\n15m,1,0,0,0,1\n"
        );
    }

    #[test]
    fn timeframes_are_labelled_by_their_code() {
        assert_eq!(timeframe_label(60), "1m");
        assert_eq!(timeframe_label(300), "5m");
        assert_eq!(timeframe_label(900), "15m");
        assert_eq!(timeframe_label(3600), "1h");
        assert_eq!(timeframe_label(42), "42s");
    }
}
