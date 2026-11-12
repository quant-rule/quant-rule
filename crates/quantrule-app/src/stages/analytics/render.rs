//! Render the analytics bundle.
//!
//! The renderer copies the descriptive metrics and table identities into
//! `metrics.json`, `summary.txt`, and `provenance.json`. It writes no verdict,
//! no acceptance, and no strategy decision.

use std::fmt::Write as _;

use serde_json::{Value, json};

use crate::artifact::backtest::MANIFEST_SCHEMA;

use super::evidence::AnalyticsEvidence;
use super::metrics::AnalyticsMetrics;

/// Renders `metrics.json`.
///
/// # Errors
///
/// Returns the serializer's error when the value cannot be encoded.
pub fn metrics_json_bytes(
    evidence: &AnalyticsEvidence,
    metrics: &AnalyticsMetrics,
) -> Result<Vec<u8>, serde_json::Error> {
    let portfolio = &evidence.portfolio;
    let net_costs = portfolio.execution_fees + portfolio.commission + portfolio.funding_paid
        - portfolio.funding_received;
    let value = json!({
        "schema": "quantrule.analytics-metrics/v1",
        "stage": "analytics",
        "evidence": {
            "backtest_manifest": {
                "path": "manifest.json",
                "schema": MANIFEST_SCHEMA,
                "hash": &evidence.bundle.manifest_hash,
            },
            "data_version": &evidence.bundle.data_version,
            "config_id": &evidence.bundle.config_id,
            "code_commit": &evidence.bundle.code_commit,
            "period": {
                "from": &evidence.bundle.period_from,
                "to": &evidence.bundle.period_to,
            },
            "seed": &evidence.bundle.seed,
        },
        "metrics": {
            "trade_count": portfolio.trades,
            "win_count": portfolio.wins,
            "loss_count": portfolio.losses,
            "win_rate": portfolio.win_rate,
            "profit_factor": portfolio.profit_factor,
            "average_trade": portfolio.average_trade,
            "average_hold_bars": portfolio.average_hold_bars,
            "exposure": portfolio.exposure,
            "max_drawdown": portfolio.max_drawdown,
            "max_drawdown_duration_bars": portfolio.max_drawdown_duration_bars,
            "total_return": portfolio.total_return,
            "initial_cash": portfolio.initial_cash,
            "final_equity": portfolio.final_equity,
            "gross_pnl": portfolio.gross_pnl,
            "net_pnl": portfolio.net_pnl,
            "realized_pnl": portfolio.realized_pnl,
            "unrealized_pnl": portfolio.unrealized_pnl,
            "execution_fees": portfolio.execution_fees,
            "commission": portfolio.commission,
            "funding_paid": portfolio.funding_paid,
            "funding_received": portfolio.funding_received,
            "net_costs": net_costs,
            "halted": portfolio.halted,
            "monthly_returns": portfolio.monthly_returns.iter().map(|month| json!({
                "year": month.year,
                "month": month.month,
                "return_fraction": month.return_fraction,
            })).collect::<Vec<Value>>(),
            "decisions": {
                "hold": metrics.decisions.hold,
                "enter": metrics.decisions.enter,
                "enter_bracket": metrics.decisions.enter_bracket,
                "exit": metrics.decisions.exit,
            },
            "benchmarks": metrics.benchmarks.json(),
        },
        "null_reasons": &metrics.null_reasons,
        "tables": metrics.tables.iter().map(|table| json!({
            "name": table.name,
            "path": &table.file,
            "columns": table.header.split(',').collect::<Vec<_>>(),
            "rows": table.rows,
        })).collect::<Vec<Value>>(),
        "unsupported": metrics.unsupported.iter().map(|table| json!({
            "name": table.name,
            "reason_code": table.reason_code,
        })).collect::<Vec<Value>>(),
    });
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Renders `summary.txt`.
#[must_use]
pub fn summary_bytes(evidence: &AnalyticsEvidence, metrics: &AnalyticsMetrics) -> String {
    let portfolio = &evidence.portfolio;
    let net_costs = portfolio.execution_fees + portfolio.commission + portfolio.funding_paid
        - portfolio.funding_received;
    let tables = metrics
        .tables
        .iter()
        .map(|table| table.file.trim_start_matches("tables/").to_owned())
        .collect::<Vec<_>>()
        .join(" ");
    let unsupported = if metrics.unsupported.is_empty() {
        "none".to_owned()
    } else {
        metrics
            .unsupported
            .iter()
            .map(|table| table.name)
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule analytics evaluate");
    let _ = writeln!(out, "data_version: {}", evidence.bundle.data_version);
    let _ = writeln!(out, "config_id: {}", evidence.bundle.config_id);
    let _ = writeln!(
        out,
        "period: {}",
        period_label(
            evidence.bundle.period_from.as_deref(),
            evidence.bundle.period_to.as_deref()
        )
    );
    let _ = writeln!(out, "trades: {}", portfolio.trades);
    let _ = writeln!(out, "wins: {}", portfolio.wins);
    let _ = writeln!(out, "losses: {}", portfolio.losses);
    let _ = writeln!(out, "win_rate: {}", optional_number(portfolio.win_rate));
    let _ = writeln!(
        out,
        "profit_factor: {}",
        optional_number(portfolio.profit_factor)
    );
    let _ = writeln!(
        out,
        "average_trade: {}",
        optional_number(portfolio.average_trade)
    );
    let _ = writeln!(
        out,
        "average_hold_bars: {}",
        optional_number(portfolio.average_hold_bars)
    );
    let _ = writeln!(out, "exposure: {}", optional_number(portfolio.exposure));
    let _ = writeln!(out, "total_return: {}", number(portfolio.total_return));
    let _ = writeln!(out, "max_drawdown: {}", number(portfolio.max_drawdown));
    let _ = writeln!(
        out,
        "max_drawdown_duration_bars: {}",
        portfolio.max_drawdown_duration_bars
    );
    let _ = writeln!(out, "initial_cash: {}", number(portfolio.initial_cash));
    let _ = writeln!(out, "final_equity: {}", number(portfolio.final_equity));
    let _ = writeln!(out, "gross_pnl: {}", number(portfolio.gross_pnl));
    let _ = writeln!(out, "net_pnl: {}", number(portfolio.net_pnl));
    let _ = writeln!(out, "realized_pnl: {}", number(portfolio.realized_pnl));
    let _ = writeln!(out, "unrealized_pnl: {}", number(portfolio.unrealized_pnl));
    let _ = writeln!(
        out,
        "costs: execution_fees={} commission={} funding_paid={} funding_received={} net_costs={}",
        number(portfolio.execution_fees),
        number(portfolio.commission),
        number(portfolio.funding_paid),
        number(portfolio.funding_received),
        number(net_costs),
    );
    let _ = writeln!(
        out,
        "decisions: hold={} enter={} enter_bracket={} exit={}",
        metrics.decisions.hold,
        metrics.decisions.enter,
        metrics.decisions.enter_bracket,
        metrics.decisions.exit,
    );
    let _ = writeln!(out, "monthly_returns: {}", portfolio.monthly_returns.len());
    for line in metrics.benchmarks.summary_lines() {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, "tables: {tables}");
    let _ = writeln!(out, "unsupported: {unsupported}");
    out
}

/// Renders `provenance.json`.
///
/// # Errors
///
/// Returns the serializer's error when the value cannot be encoded.
pub fn provenance_json_bytes(
    evidence: &AnalyticsEvidence,
    metrics: &AnalyticsMetrics,
) -> Result<Vec<u8>, serde_json::Error> {
    let value = json!({
        "schema": crate::artifact::backtest::STAGE_SCHEMA,
        "stage": "analytics",
        "artifact": "analytics",
        "data_version": &evidence.bundle.data_version,
        "config_id": &evidence.bundle.config_id,
        "code_commit": &evidence.bundle.code_commit,
        "source_artifacts": [{
            "stage": "backtest",
            "artifact": "backtest",
            "schema": MANIFEST_SCHEMA,
            "data_version": &evidence.bundle.data_version,
            "config_id": &evidence.bundle.config_id,
            "manifest_hash": &evidence.bundle.manifest_hash,
        }],
        "seed": &evidence.bundle.seed,
        "period": {
            "from": &evidence.bundle.period_from,
            "to": &evidence.bundle.period_to,
        },
        "artifacts": ["metrics.json", "tables", "summary.txt", "provenance.json"],
        "analytics": {
            "table_count": metrics.tables.len(),
            "decision_count": metrics.decisions.total(),
            "unsupported": metrics.unsupported.iter().map(|table| table.name).collect::<Vec<_>>(),
        },
    });
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn period_label(from: Option<&str>, to: Option<&str>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => "open..open".to_owned(),
    }
}

fn number(value: f64) -> String {
    value.to_string()
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".to_owned(), number)
}

#[cfg(test)]
mod tests {
    use super::{optional_number, period_label};

    #[test]
    fn optional_numbers_render_null_and_zero_distinctly() {
        assert_eq!(optional_number(None), "null");
        assert_eq!(optional_number(Some(0.0)), "0");
        assert_eq!(optional_number(Some(0.25)), "0.25");
    }

    #[test]
    fn periods_render_in_a_fixed_order() {
        assert_eq!(
            period_label(Some("2025-01-01"), Some("2025-12-31")),
            "2025-01-01..2025-12-31"
        );
        assert_eq!(period_label(None, None), "open..open");
    }
}
