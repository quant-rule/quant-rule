//! Tests for the evaluation benchmarks.

use quantrule_data::Candle;

use super::{
    DEFAULT_PATHS, DEFAULT_SEED, RandomBenchmarkConfig, SimTrade, buy_and_hold, daily_path,
    equity_metrics, random_trade_paths,
};
use crate::portfolio::{ExitReason, TradeRecord};
use crate::side::Side;

fn candle(index: usize, close: f64) -> Candle {
    Candle::new(
        index as u64,
        index as i64 * 86_400,
        close,
        close + 1.0,
        close - 1.0,
        close,
        Some(1.0),
    )
    .expect("the test candle is valid")
}

fn trade(side: Side, hold_bars: usize) -> TradeRecord {
    TradeRecord {
        trade_id: 1,
        side,
        quantity: 1.0,
        entry_bar: 0,
        entry_time: 0,
        entry_price: 100.0,
        exit_bar: hold_bars,
        exit_time: hold_bars as i64 * 86_400,
        exit_price: 101.0,
        gross_pnl: 1.0,
        execution_fees: 0.0,
        commission: 0.0,
        funding: 0.0,
        net_pnl: 1.0,
        return_fraction: Some(0.01),
        hold_bars,
        exit_reason: ExitReason::StrategyExit,
    }
}

#[test]
fn equity_metrics_reports_drawdown_target_and_recovery() {
    let points = vec![
        (0, 100.0),
        (86_400, 120.0),
        (172_800, 90.0),
        (259_200, 130.0),
        (345_600, 130.0),
    ];
    let metrics = equity_metrics(&points, 100.0, 0.10, 365).expect("the series is valid");
    assert!((metrics.max_drawdown - 0.25).abs() < 1e-12);
    assert_eq!(metrics.max_drawdown_peak_to_trough_secs, 86_400);
    assert_eq!(metrics.max_drawdown_recovery_secs, Some(86_400));
    assert_eq!(metrics.longest_underwater_secs, 172_800);
    assert_eq!(metrics.time_to_target_secs, Some(86_400));
    assert!(metrics.target_reached);
}

#[test]
fn equity_metrics_handles_a_short_run() {
    let points = vec![(0, 100.0), (86_400, 105.0)];
    let metrics = equity_metrics(&points, 100.0, 0.10, 365).expect("the series is valid");
    assert_eq!(metrics.max_drawdown, 0.0);
    assert_eq!(metrics.max_drawdown_recovery_secs, None);
    assert_eq!(metrics.time_to_target_secs, Some(86_400));
    assert!(metrics.target_reached);

    let flat = vec![(0, 100.0), (86_400, 100.0)];
    let metrics = equity_metrics(&flat, 100.0, 0.10, 365).expect("the series is valid");
    assert_eq!(metrics.time_to_target_secs, None);
    assert!(!metrics.target_reached);
}

#[test]
fn buy_and_hold_invests_after_the_entry_commission() {
    let candles = vec![candle(0, 100.0), candle(1, 110.0)];
    let benchmark = buy_and_hold(&candles, 1_000.0, 0.001, 0.10, 365).expect("the path exists");
    assert!((benchmark.quantity - 9.99).abs() < 1e-12);
    let last = benchmark.points.last().expect("the path has points");
    assert!((last.1 - 1_098.9).abs() < 1e-9);
    assert!(benchmark.metrics.total_return > 0.0);
}

#[test]
fn daily_path_marks_an_open_trade() {
    let mut days = std::collections::BTreeMap::new();
    days.insert(0, (0, 100.0));
    days.insert(86_400, (86_400, 110.0));
    days.insert(172_800, (172_800, 120.0));
    let trades = vec![SimTrade {
        entry_time: 0,
        exit_time: 172_800,
        entry_price: 100.0,
        quantity: 1.0,
        entry_commission: 1.0,
        net_pnl: 19.0,
        side: Side::Long,
    }];
    let points = daily_path(&days, 1_000.0, &trades, 0, 172_800);
    assert_eq!(points[0], (0, 999.0));
    assert!((points[1].1 - 1_009.0).abs() < 1e-12);
    assert!((points[2].1 - 1_019.0).abs() < 1e-12);
}

#[test]
fn random_paths_are_deterministic_and_seeded() {
    let candles: Vec<Candle> = (0..200)
        .map(|index| candle(index, 100.0 + index as f64))
        .collect();
    let trades = vec![trade(Side::Long, 5), trade(Side::Short, 3)];
    let config = RandomBenchmarkConfig {
        initial_cash: 1_000.0,
        commission_rate: 0.001,
        max_drawdown_limit: Some(0.10),
        seed: DEFAULT_SEED,
        paths: DEFAULT_PATHS,
        target: 0.10,
        evaluation_days: 365,
        strategy_return: 0.05,
    };
    let first = random_trade_paths(&trades, &candles, &config).expect("the paths exist");
    let second = random_trade_paths(&trades, &candles, &config).expect("the paths exist");
    assert_eq!(first, second);
    assert_eq!(first.paths, DEFAULT_PATHS);
    assert_eq!(first.raw_paths.len(), DEFAULT_PATHS);
    assert!(first.return_distribution.p05 <= first.return_distribution.p95);
    assert!((0.0..=100.0).contains(&first.strategy_percentile));
}

#[test]
fn a_different_seed_changes_the_paths() {
    let candles: Vec<Candle> = (0..200)
        .map(|index| candle(index, 100.0 + index as f64))
        .collect();
    let trades = vec![trade(Side::Long, 5)];
    let mut config = RandomBenchmarkConfig {
        initial_cash: 1_000.0,
        commission_rate: 0.0,
        max_drawdown_limit: Some(0.10),
        seed: 1,
        paths: 10,
        target: 0.10,
        evaluation_days: 365,
        strategy_return: 0.0,
    };
    let first = random_trade_paths(&trades, &candles, &config).expect("the paths exist");
    config.seed = 2;
    let second = random_trade_paths(&trades, &candles, &config).expect("the paths exist");
    assert_ne!(first.raw_paths, second.raw_paths);
}
