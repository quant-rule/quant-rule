//! The execution and portfolio report bundles.
//!
//! Both bundles run the full pipeline once, so the execution quantities and the
//! portfolio equity come from the same fills. The execution bundle publishes
//! orders, fills, rejections, slippage, and fees and no capital; the portfolio
//! bundle publishes cash, equity, trades, costs, funding, and drawdown and no
//! model or strategy label.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use quantrule_analytics::layers::summarize_model_run;
use quantrule_analytics::summary::{AxisSummary, DatasetSummary, RelationSummary};
use quantrule_backtest::execution::{
    Fill, FillReason, OrderRecord, OrderState, RejectReason, Rejection, Trigger,
};
use quantrule_backtest::portfolio::{
    EquityPoint, ExitReason, FundingCharge, OpenPosition, PortfolioMetrics, RiskEvent, SizingMode,
    SizingRejection, TradeRecord,
};
use quantrule_backtest::runner::BacktestRun;
use quantrule_data::Timeframe;
use quantrule_model::run::ModelRun;
use serde_json::{Value, json};

use super::{
    BundleOutput, ReportError, RunError, artifact_names, chart_range, config_files, day_label,
    json_bytes, period_label,
};
use crate::chart;
use crate::config::RunConfig;
use crate::render_report::{self, Provenance};

pub(crate) fn execution_bundle_from(
    config: &RunConfig,
    out: &Path,
    run: &BacktestRun,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("execution");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;

    let orders = last_orders(run);
    let fills = paired_fills(run);
    let rejections = all_rejections(run);
    let summary = execution_summary(&orders, &fills, &rejections, run.sizing_rejections());

    let orders_csv = orders_csv(&orders);
    let fills_csv = fills_csv(&fills);
    let rejections_csv = rejections_csv(&rejections, &orders);
    let summary_text = execution_text(config, &summary);
    let report_json = execution_json(config, run, &orders, &fills, &rejections, &summary);
    let chart = execution_chart(config, run, &fills)?;

    let names = artifact_names(&[
        "summary.txt",
        "report.json",
        "orders.csv",
        "fills.csv",
        "rejections.csv",
        "price-fills.png",
        "provenance.json",
    ]);
    let files = config_files(config);
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "execution",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time: first_open_time(run, config),
        last_open_time: last_open_time(run, config),
        config_files: &files,
        config: json!({
            "fill_timeframe": config.execution.fill_timeframe.code(),
            "fee_rate": config.execution.engine.fee_rate(),
            "slippage_bps": config.execution.engine.slippage_bps(),
        }),
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

    let json = json_bytes(&report_json)?;
    let artifacts = vec![
        ("summary.txt", summary_text.into_bytes()),
        ("report.json", json.clone()),
        ("orders.csv", orders_csv.into_bytes()),
        ("fills.csv", fills_csv.into_bytes()),
        ("rejections.csv", rejections_csv.into_bytes()),
        ("price-fills.png", chart),
    ];
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

pub(crate) fn portfolio_bundle_from(
    config: &RunConfig,
    out: &Path,
    run: &BacktestRun,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("portfolio");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;
    let ledger = run.ledger();
    let metrics = run.metrics();

    let trades_csv = trades_csv(ledger.trades());
    let equity_csv = equity_csv(ledger.equity_points());
    let summary_text = portfolio_text(config, &metrics);
    let report_json = portfolio_json(config, run, &metrics);
    let equity = chart::render_line_png(
        &ledger
            .equity_points()
            .iter()
            .map(|point| (point.date, point.equity))
            .collect::<Vec<_>>(),
        "equity",
        chart::UP,
    )
    .map_err(RunError::Chart)?;
    let drawdown = chart::render_line_png(
        &ledger
            .equity_points()
            .iter()
            .map(|point| (point.date, point.drawdown))
            .collect::<Vec<_>>(),
        "drawdown",
        chart::DOWN,
    )
    .map_err(RunError::Chart)?;

    let names = artifact_names(&[
        "summary.txt",
        "report.json",
        "trades.csv",
        "equity.csv",
        "equity.png",
        "drawdown.png",
        "provenance.json",
    ]);
    let files = config_files(config);
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "portfolio",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time: first_open_time(run, config),
        last_open_time: last_open_time(run, config),
        config_files: &files,
        config: json!({
            "initial_cash": config.portfolio.initial_cash,
            "currency": config.portfolio.currency.code(),
            "sizing": sizing_value(config.portfolio.sizing),
            "leverage": config.portfolio.leverage,
            "max_open_positions": config.portfolio.max_open_positions,
            "allow_short": config.portfolio.allow_short,
            "commission": {
                "rate": config.portfolio.commission.rate(),
                "base": config.portfolio.commission.base().code(),
                "per": config.portfolio.commission.per().code(),
            },
            "funding": {
                "rate": config.portfolio.funding.rate(),
                "interval_hours": config.portfolio.funding.interval().hours(),
            },
            "max_drawdown": {
                "limit": config.portfolio.max_drawdown.limit(),
                "action": config.portfolio.max_drawdown.action().code(),
            },
            "mark_to_market": config.portfolio.mark_to_market.code(),
        }),
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

    let json = json_bytes(&report_json)?;
    let artifacts = vec![
        ("summary.txt", summary_text.into_bytes()),
        ("report.json", json.clone()),
        ("trades.csv", trades_csv.into_bytes()),
        ("equity.csv", equity_csv.into_bytes()),
        ("equity.png", equity),
        ("drawdown.png", drawdown),
    ];
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

fn first_open_time(run: &BacktestRun, config: &RunConfig) -> Option<i64> {
    run.model_run()
        .timeframe(config.strategy.engine.timeframe_secs())
        .and_then(|timeframe| timeframe.candle(0))
        .map(|candle| candle.timestamp())
}

fn last_open_time(run: &BacktestRun, config: &RunConfig) -> Option<i64> {
    run.model_run()
        .timeframe(config.strategy.engine.timeframe_secs())
        .and_then(|timeframe| timeframe.candle(timeframe.len().saturating_sub(1)))
        .map(|candle| candle.timestamp())
}

fn last_orders(run: &BacktestRun) -> Vec<OrderRecord> {
    let mut orders = Vec::new();
    for batch in run.execution_batches() {
        for order in &batch.orders {
            if let Some(index) = orders
                .iter()
                .position(|seen: &OrderRecord| seen.order_id == order.order_id)
            {
                orders[index] = *order;
            } else {
                orders.push(*order);
            }
        }
    }
    orders.sort_by_key(|order| order.order_id);
    orders
}

fn paired_fills(run: &BacktestRun) -> Vec<(OrderRecord, Fill)> {
    let mut filled = Vec::new();
    for batch in run.execution_batches() {
        let orders: Vec<OrderRecord> = batch
            .orders
            .iter()
            .filter(|order| order.fill_id.is_some())
            .copied()
            .collect();
        for (order, fill) in orders.into_iter().zip(batch.fills.iter().copied()) {
            filled.push((order, fill));
        }
    }
    filled.sort_by_key(|(order, _)| order.fill_id);
    filled
}

fn all_rejections(run: &BacktestRun) -> Vec<Rejection> {
    run.execution_batches()
        .iter()
        .flat_map(|batch| batch.rejections.iter().copied())
        .collect()
}

fn execution_chart(
    config: &RunConfig,
    run: &BacktestRun,
    fills: &[(OrderRecord, Fill)],
) -> Result<Vec<u8>, RunError> {
    let Some(timeframe) = run
        .model_run()
        .timeframe(config.strategy.engine.timeframe_secs())
    else {
        return Err(RunError::Report(ReportError::NoStrategyBars {
            timeframe_secs: config.strategy.engine.timeframe_secs(),
        }));
    };
    let candles: Vec<_> = (0..timeframe.len())
        .filter_map(|bar| timeframe.candle(bar))
        .collect();
    let range = chart_range(config, &candles)?;
    let markers: Vec<(usize, bool)> = fills
        .iter()
        .map(|(_, fill)| (fill.bar, fill.reason == FillReason::Entry))
        .collect();
    let timeframe = Timeframe::ALL
        .iter()
        .copied()
        .find(|timeframe| u64::from(timeframe.seconds()) == config.strategy.engine.timeframe_secs())
        .unwrap_or(Timeframe::M15);
    chart::render_decisions_png(&candles, &markers, &config.name, &range, timeframe)
        .map_err(RunError::Chart)
}

struct ExecutionSummary {
    orders: usize,
    filled: usize,
    rejected: usize,
    pending: usize,
    fills: usize,
    entries: usize,
    exits: usize,
    fill_reasons: [usize; 4],
    rejections: Vec<(String, usize)>,
    sizing_rejections: Vec<(String, usize)>,
    slippage_total: f64,
    slippage_average: Option<f64>,
    fee_total: f64,
}

fn execution_summary(
    orders: &[OrderRecord],
    fills: &[(OrderRecord, Fill)],
    rejections: &[Rejection],
    sizing_rejections: &[(usize, SizingRejection)],
) -> ExecutionSummary {
    let mut fill_reasons = [0_usize; 4];
    let mut entries = 0;
    let mut exits = 0;
    let mut slippage_total = 0.0;
    let mut fee_total = 0.0;
    for (_, fill) in fills {
        let index = match fill.reason {
            FillReason::Entry => {
                entries += 1;
                0
            }
            FillReason::Exit => {
                exits += 1;
                1
            }
            FillReason::StopLoss => {
                exits += 1;
                2
            }
            FillReason::TakeProfit => {
                exits += 1;
                3
            }
        };
        fill_reasons[index] += 1;
        slippage_total += fill.slippage * fill.quantity;
        fee_total += fill.fee;
    }
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for rejection in rejections {
        *counts
            .entry(reject_reason_code(rejection.reason).to_owned())
            .or_default() += 1;
    }
    let mut sizing_counts: BTreeMap<String, usize> = BTreeMap::new();
    for (_, rejection) in sizing_rejections {
        *sizing_counts
            .entry(rejection.code().to_owned())
            .or_default() += 1;
    }
    let fills_count = fills.len();
    ExecutionSummary {
        orders: orders.len(),
        filled: orders
            .iter()
            .filter(|order| order.state == OrderState::Filled)
            .count(),
        rejected: orders
            .iter()
            .filter(|order| order.state == OrderState::Rejected)
            .count(),
        pending: orders
            .iter()
            .filter(|order| order.state == OrderState::Pending)
            .count(),
        fills: fills_count,
        entries,
        exits,
        fill_reasons,
        rejections: counts.into_iter().collect(),
        sizing_rejections: sizing_counts.into_iter().collect(),
        slippage_total,
        slippage_average: (fills_count > 0).then(|| slippage_total / fills_count as f64),
        fee_total,
    }
}

fn orders_csv(orders: &[OrderRecord]) -> String {
    let mut csv = String::from(
        "order_id,submitted_bar,submitted_time,side,quantity,trigger,state,filled_bar,filled_time,fill_id,rejection_reason\n",
    );
    for order in orders {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{}",
            order.order_id,
            order.submitted_bar,
            order.submitted_time,
            order.side.code(),
            number(order.quantity),
            trigger_label(order.trigger),
            state_label(order.state),
            order
                .filled_bar
                .map_or_else(String::new, |value| value.to_string()),
            order
                .filled_time
                .map_or_else(String::new, |value| value.to_string()),
            order
                .fill_id
                .map_or_else(String::new, |value| value.to_string()),
            order
                .rejection_reason
                .map_or_else(String::new, |reason| reject_reason_code(reason).to_owned()),
        );
    }
    csv
}

fn fills_csv(fills: &[(OrderRecord, Fill)]) -> String {
    let mut csv = String::from(
        "fill_id,order_id,side,quantity,reference_price,fill_price,slippage,fee,reason,bar,time,position_side,entry\n",
    );
    for (order, fill) in fills {
        let position_side = if fill.reason == FillReason::Entry {
            fill.side.code()
        } else {
            -fill.side.code()
        };
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            order.fill_id.unwrap_or_default(),
            order.order_id,
            fill.side.code(),
            fill.quantity,
            fill.reference_price,
            fill.price,
            fill.slippage,
            fill.fee,
            fill_reason_label(fill.reason),
            fill.bar,
            fill.time,
            position_side,
            fill.reason == FillReason::Entry,
        );
    }
    csv
}

fn rejections_csv(rejections: &[Rejection], orders: &[OrderRecord]) -> String {
    let mut rows: Vec<(usize, String, i64, &'static str)> = rejections
        .iter()
        .map(|rejection| {
            let decision = orders
                .iter()
                .find(|order| order.order_id == rejection.order_id)
                .map_or("entry", |order| match order.reason {
                    FillReason::Entry => "entry",
                    FillReason::Exit | FillReason::StopLoss | FillReason::TakeProfit => "exit",
                });
            (
                rejection.bar,
                reject_reason_code(rejection.reason).to_owned(),
                rejection.time,
                decision,
            )
        })
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let mut csv = String::from("bar,time,reason,decision\n");
    for (bar, reason, time, decision) in rows {
        let _ = writeln!(csv, "{bar},{time},{reason},{decision}");
    }
    csv
}

fn execution_text(config: &RunConfig, summary: &ExecutionSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule execution report");
    let _ = writeln!(out, "data_version: {}", config.data.data_version);
    let _ = writeln!(out, "requested_period: {}", period_label(config));
    let _ = writeln!(out, "orders: {}", summary.orders);
    let _ = writeln!(out, "filled: {}", summary.filled);
    let _ = writeln!(out, "rejected: {}", summary.rejected);
    let _ = writeln!(out, "pending_at_end: {}", summary.pending);
    let _ = writeln!(out, "fills: {}", summary.fills);
    let _ = writeln!(out, "entries: {}", summary.entries);
    let _ = writeln!(out, "exits: {}", summary.exits);
    let _ = writeln!(out, "fill_reasons:");
    let _ = writeln!(out, "  entry: {}", summary.fill_reasons[0]);
    let _ = writeln!(out, "  exit: {}", summary.fill_reasons[1]);
    let _ = writeln!(out, "  stop: {}", summary.fill_reasons[2]);
    let _ = writeln!(out, "  target: {}", summary.fill_reasons[3]);
    let _ = writeln!(out, "rejections:");
    if summary.rejections.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for (reason, count) in &summary.rejections {
            let _ = writeln!(out, "  {reason}: {count}");
        }
    }
    let _ = writeln!(out, "sizing_rejections:");
    if summary.sizing_rejections.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for (reason, count) in &summary.sizing_rejections {
            let _ = writeln!(out, "  {reason}: {count}");
        }
    }
    let _ = writeln!(out, "slippage:");
    let _ = writeln!(out, "  total: {}", summary.slippage_total);
    let _ = writeln!(
        out,
        "  average: {}",
        summary
            .slippage_average
            .map_or_else(|| String::from("null"), |value| value.to_string())
    );
    let _ = writeln!(out, "fees:");
    let _ = writeln!(out, "  total: {}", summary.fee_total);
    out
}

fn execution_json(
    config: &RunConfig,
    run: &BacktestRun,
    orders: &[OrderRecord],
    fills: &[(OrderRecord, Fill)],
    rejections: &[Rejection],
    summary: &ExecutionSummary,
) -> Value {
    let order_values: Vec<Value> = orders
        .iter()
        .map(|order| {
            json!({
                "order_id": order.order_id,
                "submitted_bar": order.submitted_bar,
                "submitted_time": order.submitted_time,
                "side": order.side.code(),
                "quantity": order.quantity,
                "trigger": trigger_label(order.trigger),
                "state": state_label(order.state),
                "filled_bar": order.filled_bar,
                "filled_time": order.filled_time,
                "fill_id": order.fill_id,
                "rejection_reason": order.rejection_reason.map(reject_reason_code),
            })
        })
        .collect();
    let fill_values: Vec<Value> = fills
        .iter()
        .map(|(order, fill)| {
            let position_side = if fill.reason == FillReason::Entry {
                fill.side.code()
            } else {
                -fill.side.code()
            };
            json!({
                "fill_id": order.fill_id,
                "order_id": order.order_id,
                "side": fill.side.code(),
                "quantity": fill.quantity,
                "reference_price": fill.reference_price,
                "fill_price": fill.price,
                "slippage": fill.slippage,
                "fee": fill.fee,
                "reason": fill_reason_label(fill.reason),
                "bar": fill.bar,
                "time": fill.time,
                "position_side": position_side,
                "entry": fill.reason == FillReason::Entry,
            })
        })
        .collect();
    let rejection_values: Vec<Value> = rejections
        .iter()
        .map(|rejection| {
            let decision = orders
                .iter()
                .find(|order| order.order_id == rejection.order_id)
                .map_or("entry", |order| match order.reason {
                    FillReason::Entry => "entry",
                    FillReason::Exit | FillReason::StopLoss | FillReason::TakeProfit => "exit",
                });
            json!({
                "bar": rejection.bar,
                "time": rejection.time,
                "reason": reject_reason_code(rejection.reason),
                "decision": decision,
            })
        })
        .collect();
    let rejection_counts: serde_json::Map<String, Value> = summary
        .rejections
        .iter()
        .map(|(reason, count)| (reason.clone(), json!(count)))
        .collect();
    let sizing_rejection_counts: serde_json::Map<String, Value> = summary
        .sizing_rejections
        .iter()
        .map(|(reason, count)| (reason.clone(), json!(count)))
        .collect();
    json!({
        "report": "execution",
        "report_version": render_report::REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": config.data.data_version,
        "requested_period": {"from": config.data.from, "to": config.data.to},
        "actual_period": {
            "first_open_time": first_open_time(run, config),
            "last_open_time": last_open_time(run, config),
        },
        "config": {
            "fill_timeframe": config.execution.fill_timeframe.code(),
            "fee_rate": config.execution.engine.fee_rate(),
            "slippage_bps": config.execution.engine.slippage_bps(),
        },
        "summary": {
            "orders": summary.orders,
            "filled": summary.filled,
            "rejected": summary.rejected,
            "pending_at_end": summary.pending,
            "fills": summary.fills,
            "entries": summary.entries,
            "exits": summary.exits,
            "fill_reasons": {
                "entry": summary.fill_reasons[0],
                "exit": summary.fill_reasons[1],
                "stop": summary.fill_reasons[2],
                "target": summary.fill_reasons[3],
            },
            "rejections": rejection_counts,
            "sizing_rejections": sizing_rejection_counts,
            "slippage": {
                "total": summary.slippage_total,
                "average": summary.slippage_average,
            },
            "fees": {"total": summary.fee_total},
        },
        "records": {
            "orders": order_values,
            "fills": fill_values,
            "rejections": rejection_values,
        },
    })
}

fn trades_csv(trades: &[TradeRecord]) -> String {
    let mut csv = String::from(
        "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason\n",
    );
    for trade in trades {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            trade.trade_id,
            trade.side.code(),
            trade.quantity,
            trade.entry_bar,
            trade.entry_time,
            trade.entry_price,
            trade.exit_bar,
            trade.exit_time,
            trade.exit_price,
            trade.gross_pnl,
            trade.execution_fees,
            trade.commission,
            trade.funding,
            trade.net_pnl,
            number(trade.return_fraction),
            trade.hold_bars,
            exit_reason_label(trade.exit_reason),
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
            day_label(point.date),
            point.cash,
            point.position_value,
            point.equity,
            point.peak_equity,
            point.drawdown,
        );
    }
    csv
}

fn portfolio_text(config: &RunConfig, metrics: &PortfolioMetrics) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule portfolio report");
    let _ = writeln!(out, "data_version: {}", config.data.data_version);
    let _ = writeln!(out, "requested_period: {}", period_label(config));
    let _ = writeln!(out, "initial_cash: {}", metrics.initial_cash);
    let _ = writeln!(out, "final_equity: {}", metrics.final_equity);
    let _ = writeln!(out, "total_return: {}", metrics.total_return);
    let _ = writeln!(out, "realized_pnl: {}", metrics.realized_pnl);
    let _ = writeln!(out, "unrealized_pnl: {}", metrics.unrealized_pnl);
    let _ = writeln!(out, "gross_pnl: {}", metrics.gross_pnl);
    let _ = writeln!(out, "net_pnl: {}", metrics.net_pnl);
    let _ = writeln!(out, "execution_fees: {}", metrics.execution_fees);
    let _ = writeln!(out, "commission: {}", metrics.commission);
    let _ = writeln!(out, "funding_paid: {}", metrics.funding_paid);
    let _ = writeln!(out, "funding_received: {}", metrics.funding_received);
    let _ = writeln!(out, "max_drawdown: {}", metrics.max_drawdown);
    let _ = writeln!(
        out,
        "max_drawdown_duration_bars: {}",
        metrics.max_drawdown_duration_bars
    );
    let _ = writeln!(out, "trades: {}", metrics.trades);
    let _ = writeln!(out, "wins: {}", metrics.wins);
    let _ = writeln!(out, "losses: {}", metrics.losses);
    let _ = writeln!(out, "win_rate: {}", optional(metrics.win_rate));
    let _ = writeln!(out, "profit_factor: {}", optional(metrics.profit_factor));
    let _ = writeln!(out, "average_trade: {}", optional(metrics.average_trade));
    let _ = writeln!(
        out,
        "average_hold_bars: {}",
        optional(metrics.average_hold_bars)
    );
    let _ = writeln!(out, "exposure: {}", optional(metrics.exposure));
    let _ = writeln!(out, "halted: {}", metrics.halted);
    let _ = writeln!(
        out,
        "open_position: {}",
        open_position_label(metrics.open_position)
    );
    let _ = writeln!(out, "monthly_returns:");
    for month in monthly_returns(metrics) {
        let _ = writeln!(out, "  {}: {}", month.0, optional(month.1));
    }
    out
}

fn portfolio_json(config: &RunConfig, run: &BacktestRun, metrics: &PortfolioMetrics) -> Value {
    let ledger = run.ledger();
    let trades: Vec<Value> = ledger
        .trades()
        .iter()
        .map(|trade| {
            json!({
                "trade_id": trade.trade_id,
                "side": trade.side.code(),
                "quantity": trade.quantity,
                "entry_bar": trade.entry_bar,
                "entry_time": trade.entry_time,
                "entry_price": trade.entry_price,
                "exit_bar": trade.exit_bar,
                "exit_time": trade.exit_time,
                "exit_price": trade.exit_price,
                "gross_pnl": trade.gross_pnl,
                "execution_fees": trade.execution_fees,
                "commission": trade.commission,
                "funding": trade.funding,
                "net_pnl": trade.net_pnl,
                "return": trade.return_fraction,
                "hold_bars": trade.hold_bars,
                "exit_reason": exit_reason_label(trade.exit_reason),
            })
        })
        .collect();
    let equity_curve: Vec<Value> = ledger
        .equity_points()
        .iter()
        .map(|point| {
            json!({
                "date": day_label(point.date),
                "cash": point.cash,
                "position_value": point.position_value,
                "equity": point.equity,
                "peak_equity": point.peak_equity,
                "drawdown": point.drawdown,
            })
        })
        .collect();
    let funding: Vec<Value> = ledger.funding_charges().iter().map(funding_value).collect();
    let risk_events: Vec<Value> = ledger.risk_events().iter().map(risk_value).collect();
    json!({
        "report": "portfolio",
        "report_version": render_report::REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": config.data.data_version,
        "requested_period": {"from": config.data.from, "to": config.data.to},
        "actual_period": {
            "first_open_time": first_open_time(run, config),
            "last_open_time": last_open_time(run, config),
        },
        "config": {
            "initial_cash": config.portfolio.initial_cash,
            "currency": config.portfolio.currency.code(),
            "sizing": sizing_value(config.portfolio.sizing),
            "leverage": config.portfolio.leverage,
            "max_open_positions": config.portfolio.max_open_positions,
            "allow_short": config.portfolio.allow_short,
            "commission": {
                "rate": config.portfolio.commission.rate(),
                "base": config.portfolio.commission.base().code(),
                "per": config.portfolio.commission.per().code(),
            },
            "funding": {
                "rate": config.portfolio.funding.rate(),
                "interval_hours": config.portfolio.funding.interval().hours(),
            },
            "max_drawdown": {
                "limit": config.portfolio.max_drawdown.limit(),
                "action": config.portfolio.max_drawdown.action().code(),
            },
            "mark_to_market": config.portfolio.mark_to_market.code(),
        },
        "summary": {
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
            "open_position": metrics.open_position.map(open_position_value),
            "monthly_returns": monthly_returns(metrics)
                .into_iter()
                .map(|(month, value)| json!({"month": month, "return": value}))
                .collect::<Vec<_>>(),
        },
        "records": {
            "trades": trades,
            "equity_curve": equity_curve,
            "funding": funding,
            "risk_events": risk_events,
        },
    })
}

fn monthly_returns(metrics: &PortfolioMetrics) -> Vec<(String, Option<f64>)> {
    metrics
        .monthly_returns
        .iter()
        .map(|month| {
            (
                format!("{:04}-{:02}", month.year, month.month),
                month.return_fraction,
            )
        })
        .collect()
}

fn funding_value(charge: &FundingCharge) -> Value {
    json!({
        "bar": charge.bar,
        "time": charge.time,
        "rate": charge.rate,
        "price": charge.price,
        "quantity": charge.quantity,
        "amount": charge.amount,
    })
}

fn risk_value(event: &RiskEvent) -> Value {
    json!({
        "bar": event.bar,
        "time": event.time,
        "drawdown": event.drawdown,
        "equity": event.equity,
        "peak_equity": event.peak_equity,
        "limit": event.limit,
    })
}

fn open_position_value(position: OpenPosition) -> Value {
    json!({
        "side": position.side.code(),
        "quantity": position.quantity,
        "entry_price": position.entry_price,
        "entry_bar": position.entry_bar,
        "entry_time": position.entry_time,
        "entry_equity": position.entry_equity,
        "entry_fees": position.entry_fees,
        "entry_commission": position.entry_commission,
        "funding_paid": position.funding_paid,
        "funding_received": position.funding_received,
    })
}

fn open_position_label(position: Option<OpenPosition>) -> String {
    match position {
        None => String::from("null"),
        Some(position) => format!(
            "side={} quantity={} entry_price={}",
            position.side.code(),
            position.quantity,
            position.entry_price
        ),
    }
}

fn sizing_value(sizing: SizingMode) -> Value {
    match sizing {
        SizingMode::FixedNotional {
            notional,
            quantity_step,
        } => json!({
            "mode": "fixed_notional",
            "notional": notional,
            "quantity_step": quantity_step,
        }),
        SizingMode::EquityFraction {
            fraction,
            quantity_step,
        } => json!({
            "mode": "equity_fraction",
            "fraction": fraction,
            "quantity_step": quantity_step,
        }),
    }
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(|| String::from("null"), |value| value.to_string())
}

fn state_label(state: OrderState) -> &'static str {
    match state {
        OrderState::Pending => "pending",
        OrderState::Filled => "filled",
        OrderState::Rejected => "rejected",
    }
}

fn trigger_label(trigger: Trigger) -> &'static str {
    match trigger {
        Trigger::Market => "market",
        Trigger::Stop { .. } => "stop",
        Trigger::Target { .. } => "target",
    }
}

fn reject_reason_code(reason: RejectReason) -> &'static str {
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
    }
}

fn fill_reason_label(reason: FillReason) -> &'static str {
    match reason {
        FillReason::Entry => "entry",
        FillReason::Exit => "exit",
        FillReason::StopLoss => "stop",
        FillReason::TakeProfit => "target",
    }
}

fn exit_reason_label(reason: ExitReason) -> &'static str {
    match reason {
        ExitReason::StrategyExit => "strategy",
        ExitReason::Stop => "stop",
        ExitReason::Target => "target",
    }
}

pub(crate) fn analytical_bundle_from(
    config: &RunConfig,
    out: &Path,
    model_run: &ModelRun,
) -> Result<BundleOutput, RunError> {
    let bundle = out.join("analytical");
    render_report::ensure_available(&bundle).map_err(RunError::Bundle)?;
    let selected = selected_timeframes(config);
    let summary = summarize_model_run(model_run, &selected);

    let axes_csv = axes_csv(&summary.axes);
    let relations_csv = if config.analytics.relations {
        relations_csv(&summary.relations)
    } else {
        String::from("timeframe_secs,timeframe,left,right,count,correlation\n")
    };
    let summary_text = analytical_text(config, &summary);
    let report_json = analytical_json(config, &summary);

    let names = artifact_names(&[
        "summary.txt",
        "report.json",
        "axes.csv",
        "relations.csv",
        "provenance.json",
    ]);
    let files = config_files(config);
    let first_open_time = selected_models(model_run, &selected)
        .filter_map(|timeframe| timeframe.candle(0))
        .map(|candle| candle.timestamp())
        .min();
    let last_open_time = selected_models(model_run, &selected)
        .filter_map(|timeframe| timeframe.candle(timeframe.len().saturating_sub(1)))
        .map(|candle| candle.timestamp())
        .max();
    let provenance = render_report::provenance_bytes(&Provenance {
        report: "analytical",
        data_version: &config.data.data_version,
        config_hash: &config.config_hash,
        from: config.data.from.as_deref(),
        to: config.data.to.as_deref(),
        first_open_time,
        last_open_time,
        config_files: &files,
        config: json!({
            "structure_scale": crate::command::report_run::scale_label(config.analytics.structure_scale),
            "timeframes": config.analytics.timeframes.iter().map(|timeframe| timeframe.code()).collect::<Vec<_>>(),
            "relations": config.analytics.relations,
        }),
        artifacts: &names,
    })
    .map_err(RunError::Output)?;

    let json = json_bytes(&report_json)?;
    let artifacts = vec![
        ("summary.txt", summary_text.into_bytes()),
        ("report.json", json.clone()),
        ("axes.csv", axes_csv.into_bytes()),
        ("relations.csv", relations_csv.into_bytes()),
    ];
    render_report::write_bundle(&bundle, &artifacts, &provenance).map_err(RunError::Bundle)?;
    Ok(BundleOutput {
        path: bundle,
        names,
        json,
    })
}

/// The configured analytical timeframes, in seconds.
fn selected_timeframes(config: &RunConfig) -> Vec<u64> {
    config
        .analytics
        .timeframes
        .iter()
        .map(|timeframe| u64::from(timeframe.seconds()))
        .collect()
}

/// The run's models for the selected timeframes.
fn selected_models<'a>(
    run: &'a ModelRun,
    selected: &'a [u64],
) -> impl Iterator<Item = &'a quantrule_model::timeframe::TimeframeModel> {
    run.timeframes()
        .iter()
        .filter(move |timeframe| selected.contains(&timeframe.timeframe_secs()))
}

fn axes_csv(axes: &[AxisSummary]) -> String {
    let mut csv = String::from(
        "timeframe_secs,timeframe,axis,count,nulls,min,max,mean,ordinal_0,ordinal_1,ordinal_2,ordinal_3,ordinal_4,ordinal_5,ordinal_6,ordinal_7,ordinal_8,ordinal_9,ordinal_nulls\n",
    );
    for axis in axes {
        let _ = write!(
            csv,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            axis.timeframe_secs,
            timeframe_code(axis.timeframe_secs),
            axis.axis,
            axis.count,
            axis.nulls,
            empty(axis.min),
            empty(axis.max),
            empty(axis.mean),
            axis.ordinal_counts[0],
            axis.ordinal_counts[1],
            axis.ordinal_counts[2],
            axis.ordinal_counts[3],
            axis.ordinal_counts[4],
            axis.ordinal_counts[5],
            axis.ordinal_counts[6],
            axis.ordinal_counts[7],
            axis.ordinal_counts[8],
            axis.ordinal_counts[9],
            axis.ordinal_nulls,
        );
        csv.push('\n');
    }
    csv
}

fn relations_csv(relations: &[RelationSummary]) -> String {
    let mut csv = String::from("timeframe_secs,timeframe,left,right,count,correlation\n");
    for relation in relations {
        let _ = writeln!(
            csv,
            "{},{},{},{},{},{}",
            relation.timeframe_secs,
            timeframe_code(relation.timeframe_secs),
            relation.left,
            relation.right,
            relation.count,
            empty(relation.correlation),
        );
    }
    csv
}

fn analytical_text(config: &RunConfig, summary: &DatasetSummary) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule analytical report");
    let _ = writeln!(out, "data_version: {}", config.data.data_version);
    let _ = writeln!(out, "requested_period: {}", period_label(config));
    let _ = writeln!(out, "layer1_rows: {}", summary.layer1_rows);
    let _ = writeln!(out, "layer2_rows: {}", summary.layer2_rows);
    let _ = writeln!(out, "layer3_rows: {}", summary.layer3_rows);
    let _ = writeln!(out, "timeframes:");
    for timeframe in &config.analytics.timeframes {
        let _ = writeln!(out, "  {}:", timeframe.code());
        for axis in summary
            .axes
            .iter()
            .filter(|axis| axis.timeframe_secs == u64::from(timeframe.seconds()))
        {
            let _ = writeln!(
                out,
                "    {}: count={} nulls={} min={} max={} mean={}",
                axis.axis,
                axis.count,
                axis.nulls,
                empty(axis.min),
                empty(axis.max),
                empty(axis.mean),
            );
        }
    }
    let _ = writeln!(out, "relations:");
    if config.analytics.relations {
        for relation in &summary.relations {
            let _ = writeln!(
                out,
                "  {} {}/{}: count={} correlation={}",
                timeframe_code(relation.timeframe_secs),
                relation.left,
                relation.right,
                relation.count,
                empty(relation.correlation),
            );
        }
    } else {
        let _ = writeln!(out, "  disabled");
    }
    out
}

fn analytical_json(config: &RunConfig, summary: &DatasetSummary) -> Value {
    let mut timeframes = Vec::new();
    for timeframe in &config.analytics.timeframes {
        let axes: Vec<Value> = summary
            .axes
            .iter()
            .filter(|axis| axis.timeframe_secs == u64::from(timeframe.seconds()))
            .map(|axis| {
                json!({
                    "axis": axis.axis,
                    "count": axis.count,
                    "nulls": axis.nulls,
                    "min": axis.min,
                    "max": axis.max,
                    "mean": axis.mean,
                    "ordinal_counts": axis.ordinal_counts,
                    "ordinal_nulls": axis.ordinal_nulls,
                })
            })
            .collect();
        timeframes.push(json!({
            "timeframe_secs": u64::from(timeframe.seconds()),
            "axes": axes,
        }));
    }
    let relations: Vec<Value> = if config.analytics.relations {
        summary
            .relations
            .iter()
            .map(|relation| {
                json!({
                    "timeframe_secs": relation.timeframe_secs,
                    "left": relation.left,
                    "right": relation.right,
                    "count": relation.count,
                    "correlation": relation.correlation,
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    json!({
        "report": "analytical",
        "report_version": render_report::REPORT_VERSION,
        "model_version": env!("CARGO_PKG_VERSION"),
        "data_version": config.data.data_version,
        "requested_period": {"from": config.data.from, "to": config.data.to},
        "config": {
            "structure_scale": crate::command::report_run::scale_label(config.analytics.structure_scale),
            "timeframes": config.analytics.timeframes.iter().map(|timeframe| timeframe.code()).collect::<Vec<_>>(),
            "relations": config.analytics.relations,
        },
        "summary": {
            "layer1_rows": summary.layer1_rows,
            "layer2_rows": summary.layer2_rows,
            "layer3_rows": summary.layer3_rows,
            "timeframes": timeframes,
            "relations": relations,
        },
    })
}

fn timeframe_code(timeframe_secs: u64) -> &'static str {
    Timeframe::ALL
        .iter()
        .find(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
        .map_or("custom", |timeframe| timeframe.code())
}

fn empty(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

#[cfg(test)]
mod tests {
    use quantrule_backtest::execution::OrderSide;

    use super::*;

    #[test]
    fn the_execution_summary_slippage_is_a_quote_currency_total() {
        let order = OrderRecord {
            order_id: 0,
            submitted_bar: 0,
            submitted_time: 0,
            side: OrderSide::Buy,
            quantity: Some(100.0),
            reason: FillReason::Entry,
            trigger: Trigger::Market,
            state: OrderState::Filled,
            filled_bar: Some(1),
            filled_time: Some(900),
            fill_id: Some(0),
            rejection_reason: None,
        };
        let fill = Fill::new(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0.02,
            1,
            900,
            FillReason::Entry,
        )
        .expect("the fill is valid");
        let summary = execution_summary(&[], &[(order, fill)], &[], &[]);
        assert!((summary.slippage_total - 2.0).abs() < 1e-12);
    }

    #[test]
    fn the_execution_summary_carries_sizing_rejections() {
        let summary = execution_summary(
            &[],
            &[],
            &[],
            &[(
                0,
                quantrule_backtest::portfolio::SizingRejection::ShortNotAllowed,
            )],
        );
        assert_eq!(
            summary.sizing_rejections,
            vec![("short_not_allowed".to_owned(), 1)]
        );
    }

    #[test]
    fn the_execution_summary_counts_reasons() {
        let rejection = Rejection {
            order_id: 2,
            bar: 4,
            time: 400,
            reason: RejectReason::NoPosition,
        };
        let summary = execution_summary(&[], &[], &[rejection], &[]);
        assert_eq!(summary.rejections, vec![("no_position".to_owned(), 1)]);
        assert_eq!(summary.slippage_average, None);
        assert_eq!(summary.fills, 0);
    }

    #[test]
    fn the_trade_and_equity_csvs_are_the_fixed_shape() {
        let trade = TradeRecord {
            trade_id: 1,
            side: quantrule_backtest::strategy::Side::Long,
            quantity: 1.0,
            entry_bar: 0,
            entry_time: 0,
            entry_price: 100.0,
            exit_bar: 1,
            exit_time: 900,
            exit_price: 101.0,
            gross_pnl: 1.0,
            execution_fees: 0.0,
            commission: 0.1,
            funding: 0.0,
            net_pnl: 0.9,
            return_fraction: Some(0.00009),
            hold_bars: 1,
            exit_reason: ExitReason::StrategyExit,
        };
        let csv = trades_csv(&[trade]);
        assert!(csv.starts_with("trade_id,side,"));
        assert!(csv.contains(",strategy\n"));

        let point = EquityPoint {
            date: 0,
            cash: 100.0,
            position_value: 0.0,
            equity: 100.0,
            peak_equity: 100.0,
            drawdown: 0.0,
        };
        let csv = equity_csv(&[point]);
        assert_eq!(
            csv,
            "date,cash,position_value,equity,peak_equity,drawdown\n1970-01-01,100,0,100,100,0\n"
        );
    }
}
