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
use crate::intent::RiskPolicy;
use crate::portfolio::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig, SizingMode,
};
use crate::strategy::{
    ContextEntryConfig, Decision, EntryConfig, ExecutionEntryConfig, ExitConfig, LadderConfig,
    LadderWeights, Side, StrategyConfig,
};

use super::{BacktestConfig, BacktestError, BacktestRun, run_backtest};

const DECISIONS: &str = include_str!("../../tests/fixtures/runner_decisions_golden.csv");
const TRADES: &str = include_str!("../../tests/fixtures/runner_trades_golden.csv");
const METRICS: &str = include_str!("../../tests/fixtures/runner_metrics_golden.csv");

/// A deterministic V-shaped M15 series with a short oscillation.
///
/// The first half falls and the second half rises, so the confirmed Structure
/// state moves from negative to positive. `falling_candles` mirrors it.
fn series(count: usize, falling: bool) -> Vec<Candle> {
    let mut candles = Vec::with_capacity(count);
    for index in 0..count {
        let position = index as f64;
        let primary = 8.0 * (std::f64::consts::TAU * position / 120.0).sin();
        let wave = 0.8 * (std::f64::consts::TAU * position / 17.0).sin();
        let close = 100.0
            + if falling {
                -(primary + wave)
            } else {
                primary + wave
            };
        let high = close + 0.30;
        let low = close - 0.30;
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

fn candles(count: usize) -> Vec<Candle> {
    series(count, false)
}

fn falling_candles(count: usize) -> Vec<Candle> {
    series(count, true)
}

fn context_candles(candles: &[Candle]) -> Vec<Candle> {
    candles
        .chunks(4)
        .enumerate()
        .filter_map(|(index, chunk)| {
            if chunk.len() < 4 {
                return None;
            }
            let open = chunk[0].open();
            let close = chunk[3].close();
            let high = chunk
                .iter()
                .map(|candle| candle.high())
                .fold(f64::NEG_INFINITY, f64::max);
            let low = chunk
                .iter()
                .map(|candle| candle.low())
                .fold(f64::INFINITY, f64::min);
            Candle::new(
                index as u64,
                chunk[0].timestamp(),
                open,
                high,
                low,
                close,
                Some(400.0),
            )
            .ok()
        })
        .collect()
}

fn strategy_config(risk: RiskPolicy) -> StrategyConfig {
    StrategyConfig::new(
        LadderConfig::new(900, vec![3_600]).expect("the ladder is valid"),
        LadderWeights::equal(1),
        EntryConfig::new(
            ExecutionEntryConfig::default(),
            ContextEntryConfig::new(0.5, 1.0, 1, 1).expect("the context config is valid"),
        ),
        ExitConfig::default(),
        risk,
        true,
    )
    .expect("the strategy config is valid")
}

fn config() -> BacktestConfig {
    BacktestConfig {
        model: MarketStateConfig::new(PivotScale::Major),
        strategy: strategy_config(RiskPolicy::default()),
        execution: ExecutionConfig::new(0.0, 2.0).expect("the execution config is valid"),
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
        max_position_fraction: 1.0,
    }
}

fn run(candles: &[Candle], config: &BacktestConfig) -> BacktestRun {
    let context = context_candles(candles);
    run_backtest(
        &[
            TimeframeInput {
                timeframe_secs: 900,
                candles,
            },
            TimeframeInput {
                timeframe_secs: 3_600,
                candles: &context,
            },
        ],
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
    let candles = candles(1400);
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
            Decision::Enter(entry) => Some(("enter", side_label(entry.intent.side))),
            Decision::Exit(exit) => Some(("exit", side_label(exit.side))),
        })
        .collect();
    assert_eq!(actual, expected, "decision sequence changed");

    let orders = last_orders(&run);
    assert_eq!(orders.len(), 12, "order count changed");
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
        (0..12).collect::<Vec<_>>()
    );

    let fills: Vec<_> = run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.fills.iter().copied())
        .collect();
    assert_eq!(fills.len(), 12, "fill count changed");
    assert_eq!(
        fills
            .iter()
            .map(|fill| (fill.bar, fill.reason))
            .collect::<Vec<_>>(),
        [
            (807, crate::execution::FillReason::Entry),
            (819, crate::execution::FillReason::StopLoss),
            (867, crate::execution::FillReason::Entry),
            (878, crate::execution::FillReason::StopLoss),
            (1175, crate::execution::FillReason::Entry),
            (1180, crate::execution::FillReason::Exit),
            (1235, crate::execution::FillReason::Entry),
            (1240, crate::execution::FillReason::Exit),
            (1294, crate::execution::FillReason::Entry),
            (1300, crate::execution::FillReason::Exit),
            (1354, crate::execution::FillReason::Entry),
            (1360, crate::execution::FillReason::Exit),
        ]
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
    assert_eq!(points.len(), 15, "equity point count changed");
    assert_eq!(
        points.iter().map(|point| point.date).collect::<Vec<_>>(),
        (0..15).map(|day| day * 86_400).collect::<Vec<_>>()
    );
    approx(
        points.last().expect("a point").equity,
        metric_value("final_equity"),
    );

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
    let candles = candles(1400);
    let first = run(&candles, &config());
    for _ in 0..5 {
        assert_eq!(run(&candles, &config()), first);
    }
}

#[test]
fn later_bars_do_not_change_earlier_results() {
    let candles = candles(1400);
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
fn an_invalid_risk_envelope_is_rejected() {
    let mut config = config();
    config.max_position_fraction = 1.1;
    let result = run_backtest(&[], &config);

    assert_eq!(result, Err(BacktestError::RiskEnvelope));
}

#[test]
fn a_missing_strategy_timeframe_produces_an_empty_run() {
    let candles = candles(1400);
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
    let candles = candles(1236);
    let run = run(&candles, &config());
    let metrics = run.metrics();

    assert!(metrics.trades >= 1);
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
    let candles = candles(1400);
    let mut config = config();
    config.portfolio.max_drawdown =
        MaxDrawdownConfig::new(Some(0.001), DrawdownAction::Halt).expect("the drawdown is valid");
    let run = run(&candles, &config);
    let metrics = run.metrics();

    assert!(metrics.halted);
    assert_eq!(run.ledger().risk_events().len(), 1);
    assert!(metrics.trades >= 1);
    let rejection = run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.rejections.iter())
        .find(|rejection| rejection.reason == RejectReason::MissingQuantity)
        .expect("the blocked entry is recorded");
    assert_eq!(rejection.reason, RejectReason::MissingQuantity);
}

#[test]
fn the_strategy_risk_policy_reaches_the_intent() {
    let candles = candles(1400);
    let mut wide = config();
    wide.strategy = strategy_config(RiskPolicy::new(2.0, 5.0).expect("the risk policy is valid"));
    let run = run(&candles, &wide);
    let entry = run
        .decisions()
        .iter()
        .find_map(|decision| match decision {
            Decision::Enter(entry) => Some(entry),
            Decision::Exit(_) | Decision::Hold => None,
        })
        .expect("the fixture enters");
    assert_eq!(
        entry.intent.stop,
        crate::strategy::StopSpec::AtrMultiple { multiple: 2.0 }
    );
    assert_eq!(
        entry.intent.target,
        crate::strategy::TargetSpec::RewardToRisk { multiple: 5.0 }
    );
}

#[test]
fn the_risk_envelope_caps_the_first_entry_at_half_equity() {
    let mut config = config();
    config.max_position_fraction = 0.5;
    let run = run(&candles(1400), &config);
    let fill = run
        .execution_batches()
        .iter()
        .flat_map(|batch| batch.fills.iter())
        .find(|fill| fill.reason == crate::execution::FillReason::Entry)
        .expect("the fixture enters");

    assert!(fill.price * fill.quantity <= 5_000.0 + 1e-9);
}

#[test]
fn a_downtrend_produces_a_short_trade() {
    let falling = falling_candles(1400);
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
fn every_decision_uses_the_execution_position() {
    let run = run(&candles(1400), &config());

    for (position, decision) in run.strategy_walk().positions().iter().zip(run.decisions()) {
        match decision {
            Decision::Enter(_) => assert!(position.is_flat()),
            Decision::Exit(exit) => assert_eq!(position.side, Some(exit.side)),
            Decision::Hold => {}
        }
    }
}

#[test]
fn the_final_strategy_position_matches_the_portfolio_position() {
    let run = run(&candles(1400), &config());
    let strategy = run.strategy_walk().final_position();
    let portfolio = run.metrics().open_position;

    assert_eq!(strategy.side, portfolio.map(|position| position.side));
    assert_eq!(
        strategy.entry_bar,
        portfolio.map(|position| position.entry_bar)
    );
}

#[test]
fn a_rejected_short_is_never_an_open_strategy_position() {
    let mut config = config();
    config.portfolio.allow_short = false;
    let run = run(&falling_candles(1400), &config);
    let walk = run.strategy_walk();

    assert!(
        walk.decisions().iter().any(|decision| matches!(
            decision,
            Decision::Enter(entry) if entry.intent.side == Side::Short
        )),
        "the fixture must exercise a short entry"
    );
    assert!(
        walk.positions()
            .iter()
            .all(|position| position.side != Some(Side::Short)),
        "a rejected short must not become a strategy position"
    );
    for (position, decision) in walk.positions().iter().zip(walk.decisions()) {
        match decision {
            Decision::Enter(_) => assert!(position.is_flat()),
            Decision::Exit(exit) => assert_eq!(position.side, Some(exit.side)),
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
