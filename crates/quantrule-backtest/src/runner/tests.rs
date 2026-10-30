//! Tests for the full-pipeline runner.
//!
//! The synthetic M15 series is a deterministic triangle wave on a small trend,
//! so the model's readings and the strategy's decisions are reproducible without
//! a clock or a random source. The checked-in fixtures fix the non-hold
//! decisions, the closed trades, and the summary metrics; orders, fills, and
//! equity are asserted from the same run.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::PivotScale;

use crate::execution::{ExecutionConfig, RejectReason};
use crate::portfolio::ExitReason;
use crate::portfolio::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig, SizingMode,
};
use crate::strategy::{Decision, Side, StrategyConfig};

use super::{BacktestConfig, BacktestRun, run_backtest};

const DECISIONS: &str = include_str!("../../tests/fixtures/runner_decisions_golden.csv");
const TRADES: &str = include_str!("../../tests/fixtures/runner_trades_golden.csv");
const METRICS: &str = include_str!("../../tests/fixtures/runner_metrics_golden.csv");

fn candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    for index in 0..count {
        let phase = index % 40;
        let wave = if phase < 20 {
            phase as f64
        } else {
            (40 - phase) as f64
        };
        let close: f64 = 100.0 + wave + 0.0625 * index as f64;
        let high = close + 0.20;
        let low = close - 0.20;
        candles.push(
            Candle::new(
                index as u64,
                index as i64 * 900,
                close,
                high,
                low,
                close,
                Some(100.0),
            )
            .expect("the test candle is valid"),
        );
    }
    candles
}

fn falling_candles(count: usize) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    for index in 0..count {
        let phase = index % 40;
        let wave = if phase < 20 {
            phase as f64
        } else {
            (40 - phase) as f64
        };
        let close: f64 = 200.0 - wave - 0.0625 * index as f64;
        candles.push(
            Candle::new(
                index as u64,
                index as i64 * 900,
                close,
                close + 0.20,
                close - 0.20,
                close,
                Some(100.0),
            )
            .expect("the test candle is valid"),
        );
    }
    candles
}

fn config() -> BacktestConfig {
    BacktestConfig {
        model: MarketStateConfig::new(PivotScale::Major),
        strategy: StrategyConfig::new(900, 0.1, -0.1).expect("the strategy config is valid"),
        execution: ExecutionConfig::with_bracket(0.0, 2.0, 2.0, 3.0)
            .expect("the execution config is valid"),
        portfolio: PortfolioConfig {
            initial_cash: 10_000.0,
            currency: Currency::Usd,
            sizing: SizingMode::FixedNotional {
                notional: 10_000.0,
                quantity_step: 0.0001,
            },
            leverage: 1.0,
            max_open_positions: 1,
            allow_short: true,
            commission: CommissionConfig::new(
                0.001,
                CommissionBase::Equity,
                CommissionPer::Operation,
            )
            .expect("the commission is valid"),
            funding: FundingConfig::new(0.0, FundingInterval::H8).expect("the funding is valid"),
            max_drawdown: MaxDrawdownConfig::new(Some(0.5), DrawdownAction::Halt)
                .expect("the drawdown is valid"),
            mark_to_market: MarkToMarket::BarClose,
        },
    }
}

fn run(candles: &[Candle], config: &BacktestConfig) -> BacktestRun {
    run_backtest(
        &[TimeframeInput {
            timeframe_secs: 900,
            candles,
        }],
        config,
    )
    .expect("the run completes")
}

fn approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

fn number(field: &str) -> f64 {
    field
        .parse::<f64>()
        .unwrap_or_else(|error| panic!("invalid fixture number {field:?}: {error}"))
}

fn metric(name: &str) -> Option<f64> {
    let mut lines = METRICS.lines();
    assert_eq!(
        lines.next(),
        Some("name,value"),
        "metrics fixture header changed"
    );
    for line in lines {
        let (field, value) = line
            .split_once(',')
            .unwrap_or_else(|| panic!("invalid metrics row {line:?}"));
        if field == name {
            return if value.is_empty() {
                None
            } else {
                Some(number(value))
            };
        }
    }
    panic!("metric {name} is missing");
}

fn metric_value(name: &str) -> f64 {
    metric(name).unwrap_or_else(|| panic!("metric {name} must have a value"))
}

/// Keeps the last record per order, as a report does.
fn last_orders(run: &BacktestRun) -> Vec<crate::execution::OrderRecord> {
    let mut orders = Vec::new();
    for batch in run.execution_batches() {
        for order in &batch.orders {
            if let Some(index) = orders
                .iter()
                .position(|seen: &crate::execution::OrderRecord| seen.order_id == order.order_id)
            {
                orders[index] = *order;
            } else {
                orders.push(*order);
            }
        }
    }
    orders
}

#[test]
fn the_runner_matches_the_golden_run() {
    let candles = candles(320);
    let run = run(&candles, &config());

    let expected: Vec<(&str, &str)> = DECISIONS
        .lines()
        .skip(1)
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 3, "wrong decision column count in {line:?}");
            (fields[1], fields[2])
        })
        .collect();
    let actual: Vec<(&str, &str)> = run
        .decisions()
        .iter()
        .filter_map(|decision| match decision {
            Decision::Hold => None,
            Decision::Enter(entry) => Some(("enter", side_label(entry.side))),
            Decision::Exit(exit) => Some(("exit", side_label(exit.side))),
        })
        .collect();
    assert_eq!(actual, expected, "decision sequence changed");

    let orders = last_orders(&run);
    assert_eq!(orders.len(), 24, "order count changed");
    for (id, order) in orders.iter().enumerate() {
        assert_eq!(order.order_id, id as u64);
    }
    assert!(
        orders
            .iter()
            .all(|order| order.state == crate::execution::OrderState::Filled),
        "every order must fill in the executed walk"
    );
    assert!(
        orders.iter().all(|order| order.rejection_reason.is_none()),
        "the executed walk must not carry virtual rejections"
    );
    assert_eq!(
        orders
            .iter()
            .filter_map(|order| order.fill_id)
            .collect::<Vec<_>>(),
        (0..24).collect::<Vec<_>>()
    );

    let fills: Vec<_> = run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.fills.iter().copied())
        .collect();
    assert_eq!(fills.len(), 24, "fill count changed");
    assert_eq!(
        fills.iter().map(|fill| fill.bar).collect::<Vec<_>>(),
        [
            209, 213, 214, 218, 219, 224, 225, 228, 249, 253, 254, 258, 259, 264, 265, 268, 289,
            293, 294, 298, 299, 304, 305, 308
        ]
    );
    assert!(
        fills
            .iter()
            .enumerate()
            .all(|(index, fill)| (index % 2 == 0)
                == (fill.reason == crate::execution::FillReason::Entry))
    );

    let mut lines = TRADES.lines();
    assert_eq!(
        lines.next(),
        Some(
            "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason"
        )
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(
        rows.len(),
        run.ledger().trades().len(),
        "trade count changed"
    );
    for (row, trade) in rows.iter().zip(run.ledger().trades()) {
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields.len(), 17, "wrong trade column count in {row:?}");
        assert_eq!(fields[0].parse::<u64>().expect("trade id"), trade.trade_id);
        assert_eq!(fields[1], trade.side.code().to_string());
        approx(number(fields[2]), trade.quantity);
        assert_eq!(
            fields[3].parse::<usize>().expect("entry bar"),
            trade.entry_bar
        );
        assert_eq!(
            fields[4].parse::<i64>().expect("entry time"),
            trade.entry_time
        );
        approx(number(fields[5]), trade.entry_price);
        assert_eq!(
            fields[6].parse::<usize>().expect("exit bar"),
            trade.exit_bar
        );
        assert_eq!(
            fields[7].parse::<i64>().expect("exit time"),
            trade.exit_time
        );
        approx(number(fields[8]), trade.exit_price);
        approx(number(fields[9]), trade.gross_pnl);
        approx(number(fields[10]), trade.execution_fees);
        approx(number(fields[11]), trade.commission);
        approx(number(fields[12]), trade.funding);
        approx(number(fields[13]), trade.net_pnl);
        approx(
            number(fields[14]),
            trade
                .return_fraction
                .expect("the fixture return is defined"),
        );
        assert_eq!(
            fields[15].parse::<usize>().expect("hold bars"),
            trade.hold_bars
        );
        assert_eq!(fields[16], trade.exit_reason.code());
    }

    let points = run.ledger().equity_points();
    assert_eq!(points.len(), 4, "equity point count changed");
    assert_eq!(
        points.iter().map(|point| point.date).collect::<Vec<_>>(),
        [0, 86_400, 172_800, 259_200]
    );
    approx(points[3].equity, metric_value("final_equity"));

    let metrics = run.metrics();
    approx(metrics.final_equity, metric_value("final_equity"));
    approx(metrics.total_return, metric_value("total_return"));
    approx(metrics.realized_pnl, metric_value("realized_pnl"));
    approx(metrics.unrealized_pnl, metric_value("unrealized_pnl"));
    approx(metrics.gross_pnl, metric_value("gross_pnl"));
    approx(metrics.net_pnl, metric_value("net_pnl"));
    approx(metrics.commission, metric_value("commission"));
    approx(metrics.funding_paid, 0.0);
    approx(metrics.funding_received, 0.0);
    approx(metrics.max_drawdown, metric_value("max_drawdown"));
    assert_eq!(
        metrics.max_drawdown_duration_bars,
        metric_value("max_drawdown_duration_bars") as usize
    );
    assert_eq!(metrics.trades, metric_value("trades") as usize);
    assert_eq!(metrics.wins, metric_value("wins") as usize);
    assert_eq!(metrics.losses, metric_value("losses") as usize);
    approx(
        metrics.win_rate.expect("there are trades"),
        metric_value("win_rate"),
    );
    assert_eq!(metrics.profit_factor, metric("profit_factor"));
    approx(
        metrics.average_trade.expect("there are trades"),
        metric_value("average_trade"),
    );
    approx(
        metrics.average_hold_bars.expect("there are trades"),
        metric_value("average_hold_bars"),
    );
    approx(
        metrics.exposure.expect("there are marks"),
        metric_value("exposure"),
    );
    assert!(!metrics.halted);
    assert!(metrics.open_position.is_none());
    approx(metrics.final_equity, 10_000.0 + metrics.net_pnl);
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

#[test]
fn repeated_runs_are_identical() {
    let candles = candles(320);
    let first = run(&candles, &config());
    for _ in 0..5 {
        assert_eq!(run(&candles, &config()), first);
    }
}

#[test]
fn later_bars_do_not_change_earlier_results() {
    let candles = candles(320);
    let prefix = run(&candles[..250], &config());
    let full = run(&candles, &config());

    assert_eq!(prefix.decisions(), &full.decisions()[..250]);
    assert_eq!(
        prefix.ledger().trades(),
        &full.ledger().trades()[..prefix.ledger().trades().len()]
    );
    assert_eq!(
        &prefix.ledger().equity_points()[..2],
        &full.ledger().equity_points()[..2]
    );

    let mut changed = candles.clone();
    changed[300] = Candle::new(300, 270_000, 100.0, 200.0, 50.0, 150.0, Some(100.0))
        .expect("the changed candle is valid");
    let changed = run(&changed, &config());
    assert_eq!(changed.decisions()[..250], full.decisions()[..250]);
    assert_eq!(
        &changed.ledger().trades()[..prefix.ledger().trades().len()],
        &full.ledger().trades()[..prefix.ledger().trades().len()]
    );
    assert_eq!(
        &changed.ledger().equity_points()[..2],
        &full.ledger().equity_points()[..2]
    );
}

#[test]
fn empty_inputs_produce_an_empty_run() {
    let run = run_backtest(&[], &config()).expect("the empty run completes");
    assert!(run.is_empty());
    assert!(run.decisions().is_empty());
    assert!(run.execution_batches().is_empty());
    assert_eq!(run.metrics().final_equity, 10_000.0);
    assert_eq!(run.metrics().trades, 0);
    assert_eq!(run.metrics().exposure, None);
}

#[test]
fn a_missing_strategy_timeframe_produces_an_empty_run() {
    let candles = candles(320);
    let run = run_backtest(
        &[TimeframeInput {
            timeframe_secs: 300,
            candles: &candles,
        }],
        &config(),
    )
    .expect("the run completes");
    assert!(run.is_empty());
    assert_eq!(run.metrics().final_equity, 10_000.0);
}

#[test]
fn a_run_without_decisions_has_no_trades() {
    let candles = candles(150);
    let run = run(&candles, &config());
    let metrics = run.metrics();

    assert_eq!(metrics.trades, 0);
    assert_eq!(metrics.win_rate, None);
    assert_eq!(metrics.average_trade, None);
    assert!(metrics.open_position.is_none());
    assert_eq!(metrics.exposure, Some(0.0));
}

#[test]
fn an_open_position_at_the_end_is_marked() {
    let candles = candles(292);
    let run = run(&candles, &config());
    let metrics = run.metrics();

    assert_eq!(metrics.trades, 8);
    let position = metrics.open_position.expect("the last entry is still open");
    assert_eq!(position.side, Side::Long);
    approx(
        metrics.final_equity,
        run.ledger().equity_points().last().expect("a point").equity,
    );
    approx(metrics.final_equity, 10_000.0 + metrics.net_pnl);
}

#[test]
fn the_halt_blocks_a_later_entry() {
    let candles = candles(320);
    let mut config = config();
    config.portfolio.max_drawdown =
        MaxDrawdownConfig::new(Some(0.001), DrawdownAction::Halt).expect("the drawdown is valid");
    let run = run(&candles, &config);
    let metrics = run.metrics();

    assert!(metrics.halted);
    assert_eq!(run.ledger().risk_events().len(), 1);
    assert_eq!(metrics.trades, 1, "the second entry must be blocked");
    let rejection = run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.rejections.iter())
        .find(|rejection| rejection.bar == 248)
        .expect("the blocked entry is recorded");
    assert_eq!(rejection.reason, RejectReason::MissingQuantity);
}

#[test]
fn the_configured_bracket_multiples_are_used() {
    let candles = candles(320);
    let mut wide = config();
    wide.execution =
        ExecutionConfig::with_bracket(0.0, 2.0, 2.0, 10.0).expect("the execution config is valid");
    let run = run(&candles, &wide);
    let metrics = run.metrics();

    assert_eq!(metrics.trades, 3);
    for trade in run.ledger().trades() {
        assert_eq!(trade.exit_reason, ExitReason::AgreementLost);
    }
    assert!(metrics.open_position.is_none());
}

#[test]
fn a_downtrend_produces_a_short_trade() {
    let falling = falling_candles(320);
    let run = run(&falling, &config());
    assert!(
        run.ledger()
            .trades()
            .iter()
            .any(|trade| trade.side == Side::Short),
        "the downtrend must produce a short trade"
    );
}

#[test]
fn a_rejected_short_is_never_an_open_strategy_position() {
    let mut config = config();
    config.portfolio.allow_short = false;
    let run = run(&falling_candles(320), &config);
    let walk = run.strategy_walk();

    assert!(
        walk.decisions().iter().any(|decision| matches!(
            decision,
            Decision::Enter(entry) if entry.side == Side::Short
        )),
        "the fixture must exercise a short entry"
    );
    assert!(
        walk.positions()
            .iter()
            .all(|position| *position != Some(Side::Short)),
        "a rejected short must not become a strategy position"
    );
    for (position, decision) in walk.positions().iter().zip(walk.decisions()) {
        match decision {
            Decision::Enter(_) => assert_eq!(*position, None),
            Decision::Exit(exit) => assert_eq!(*position, Some(exit.side)),
            Decision::Hold => {}
        }
    }
    assert!(
        !walk
            .decisions()
            .iter()
            .any(|decision| matches!(decision, Decision::Exit(exit) if exit.side == Side::Short)),
        "the strategy must not exit a short position it never held"
    );
    assert!(
        run.execution_batches()
            .iter()
            .flat_map(|batch| batch.rejections.iter())
            .any(|rejection| rejection.reason == RejectReason::MissingQuantity),
        "the fixture must exercise the short sizing rejection"
    );
}
