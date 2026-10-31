//! Tests for the portfolio ledger.
//!
//! The golden fixture is hand-derived from the contract's accounting rules:
//! commission on equity before each operation, the fill's own execution fee,
//! funding at an aligned 8-hour boundary, and a stop exit. The expected trades
//! and daily equity are checked in under `tests/fixtures/`; the test compares
//! the ledger's records against them, so a numerical or structural regression
//! fails loudly.

use crate::execution::{Fill, FillReason, OrderSide};
use crate::side::Side;

use super::config::{
    CommissionBase, CommissionConfig, CommissionPer, Currency, DrawdownAction, FundingConfig,
    FundingInterval, MarkToMarket, MaxDrawdownConfig, PortfolioConfig,
};
use super::ledger::{PortfolioLedger, PortfolioLedgerError};
use super::sizing::{SizingMode, SizingRejection};

const TRADES: &str = include_str!("../../tests/fixtures/portfolio_trades_golden.csv");
const EQUITY: &str = include_str!("../../tests/fixtures/portfolio_equity_golden.csv");

fn config() -> PortfolioConfig {
    PortfolioConfig {
        initial_cash: 10_000.0,
        currency: Currency::Usd,
        sizing: SizingMode::FixedNotional {
            notional: 10_000.0,
            quantity_step: 0.0001,
        },
        leverage: 1.0,
        max_open_positions: 1,
        allow_short: true,
        commission: CommissionConfig::new(0.001, CommissionBase::Equity, CommissionPer::Operation)
            .expect("the commission is valid"),
        funding: FundingConfig::new(0.0001, FundingInterval::H8).expect("the funding is valid"),
        max_drawdown: MaxDrawdownConfig::new(Some(0.20), DrawdownAction::Halt)
            .expect("the drawdown is valid"),
        mark_to_market: MarkToMarket::BarClose,
    }
}

fn ledger() -> PortfolioLedger {
    PortfolioLedger::new(config()).expect("the ledger is valid")
}

fn fill(
    side: OrderSide,
    quantity: f64,
    price: f64,
    fee: f64,
    bar: usize,
    time: i64,
    reason: FillReason,
) -> Fill {
    Fill::new(side, quantity, price, fee, 0.0, bar, time, reason).expect("the test fill is valid")
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

#[test]
fn a_partial_exit_is_rejected_without_changing_the_account() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    let before = ledger.state();

    let error = ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            50.0,
            110.0,
            0.0,
            1,
            900,
            FillReason::Exit,
        ))
        .expect_err("a partial exit must be rejected");

    assert_eq!(
        error,
        PortfolioLedgerError::ExitQuantityMismatch {
            expected: 100.0,
            found: 50.0,
        }
    );
    assert_eq!(ledger.state(), before);
    assert!(ledger.trades().is_empty());
}

#[test]
fn a_fill_updates_peak_equity_and_drawdown() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");

    let state = ledger.state();
    approx(state.equity, 9_990.0);
    approx(state.peak_equity, 10_000.0);
    approx(state.drawdown, 1.0 - 9_990.0 / 10_000.0);
}

#[test]
fn a_gap_exit_commission_uses_the_prior_mark() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 100.0).expect("the mark is valid");
    let prior_equity = ledger.state().equity;

    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            120.0,
            0.0,
            1,
            900,
            FillReason::Exit,
        ))
        .expect("the gap exit is valid");

    let trade = ledger.trades()[0];
    let entry_commission = config().commission.rate() * 10_000.0;
    let exit_commission = config().commission.rate() * prior_equity;
    approx(trade.commission, entry_commission + exit_commission);
}

#[test]
fn a_long_trade_charges_commission_on_both_fills() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 100.0).expect("the mark is valid");
    ledger.mark(1, 900, 110.0).expect("the mark is valid");
    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            110.0,
            0.0,
            2,
            1800,
            FillReason::TakeProfit,
        ))
        .expect("the exit is valid");
    ledger.mark(2, 1800, 110.0).expect("the mark is valid");

    assert_eq!(ledger.trades().len(), 1);
    let trade = ledger.trades()[0];
    assert_eq!(trade.trade_id, 1);
    assert_eq!(trade.side, Side::Long);
    assert_eq!(trade.quantity, 100.0);
    assert_eq!(trade.entry_bar, 0);
    assert_eq!(trade.exit_bar, 2);
    assert_eq!(trade.hold_bars, 2);
    assert_eq!(trade.exit_reason, super::records::ExitReason::Target);
    approx(trade.gross_pnl, 1_000.0);
    approx(trade.execution_fees, 0.0);
    approx(trade.commission, 20.99);
    approx(trade.funding, 0.0);
    approx(trade.net_pnl, 979.01);
    approx(
        trade.return_fraction.expect("the return is defined"),
        0.097_901,
    );

    let metrics = ledger.metrics();
    approx(metrics.final_equity, 10_979.01);
    approx(metrics.total_return, 0.097_901);
    approx(metrics.realized_pnl, 979.01);
    approx(metrics.unrealized_pnl, 0.0);
    approx(metrics.gross_pnl, 1_000.0);
    approx(metrics.net_pnl, 979.01);
    approx(metrics.commission, 20.99);
    assert_eq!(metrics.trades, 1);
    assert_eq!(metrics.wins, 1);
    assert_eq!(metrics.losses, 0);
    assert_eq!(metrics.win_rate, Some(1.0));
    assert_eq!(metrics.profit_factor, None);
    approx(metrics.average_trade.expect("one trade"), 979.01);
    assert_eq!(metrics.average_hold_bars, Some(2.0));
    approx(metrics.max_drawdown, 0.001);
    assert_eq!(metrics.max_drawdown_duration_bars, 1);
    approx(
        metrics.exposure.expect("the marks define exposure"),
        21_000.0 / 31_959.01,
    );
    assert!(!metrics.halted);
    assert!(metrics.open_position.is_none());
    assert_eq!(metrics.monthly_returns.len(), 1);
    approx(
        metrics.monthly_returns[0]
            .return_fraction
            .expect("the month has a return"),
        0.097_901,
    );

    approx(ledger.state().cash, 10_979.01);
    approx(ledger.state().equity, 10_979.01);
    assert!(ledger.state().is_flat());
}

#[test]
fn a_short_trade_receives_funding_and_stops_out() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 100.0).expect("the mark is valid");
    let charge = ledger
        .apply_funding(1, 28_800, 100.0, Some(10.0))
        .expect("the funding is valid")
        .expect("the boundary charges");
    approx(charge.amount, -1.0);
    approx(charge.rate, 0.0001);
    approx(charge.quantity, 100.0);
    ledger.mark(1, 28_800, 105.0).expect("the mark is valid");
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            105.0,
            0.0,
            2,
            29_700,
            FillReason::StopLoss,
        ))
        .expect("the exit is valid");
    ledger.mark(2, 29_700, 105.0).expect("the mark is valid");

    let metrics = ledger.metrics();
    approx(metrics.final_equity, 9_481.509);
    approx(metrics.realized_pnl, -518.491);
    approx(metrics.unrealized_pnl, 0.0);
    approx(metrics.gross_pnl, -500.0);
    approx(metrics.net_pnl, -518.491);
    approx(metrics.commission, 19.491);
    approx(metrics.funding_paid, 0.0);
    approx(metrics.funding_received, 1.0);
    approx(metrics.max_drawdown, 0.051_849_1);
    assert_eq!(metrics.max_drawdown_duration_bars, 3);
    assert_eq!(metrics.trades, 1);
    assert_eq!(metrics.wins, 0);
    assert_eq!(metrics.losses, 1);
    assert_eq!(metrics.win_rate, Some(0.0));
    assert_eq!(metrics.profit_factor, Some(0.0));
    approx(metrics.average_trade.expect("one trade"), -518.491);
    assert_eq!(metrics.average_hold_bars, Some(2.0));
    approx(
        metrics.exposure.expect("the marks define exposure"),
        -20_500.0 / 28_962.509,
    );
    assert_eq!(ledger.funding_charges().len(), 1);
    assert_eq!(ledger.equity_points().len(), 1);
    assert_eq!(ledger.equity_points()[0].date, 0);
}

#[test]
fn the_golden_fixture_matches() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 100.0).expect("the mark is valid");
    ledger
        .apply_funding(1, 28_800, 100.0, Some(10.0))
        .expect("the funding is valid");
    ledger.mark(1, 28_800, 105.0).expect("the mark is valid");
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            105.0,
            0.0,
            2,
            29_700,
            FillReason::StopLoss,
        ))
        .expect("the exit is valid");
    ledger.mark(2, 29_700, 105.0).expect("the mark is valid");

    let mut lines = TRADES.lines();
    assert_eq!(
        lines.next(),
        Some(
            "trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason"
        )
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), ledger.trades().len(), "trade row count changed");
    for (row, trade) in rows.iter().zip(ledger.trades()) {
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

    let mut lines = EQUITY.lines();
    assert_eq!(
        lines.next(),
        Some("date,cash,position_value,equity,peak_equity,drawdown")
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(
        rows.len(),
        ledger.equity_points().len(),
        "equity row count changed"
    );
    for (row, point) in rows.iter().zip(ledger.equity_points()) {
        let fields: Vec<&str> = row.split(',').collect();
        assert_eq!(fields.len(), 6, "wrong equity column count in {row:?}");
        assert_eq!(fields[0].parse::<i64>().expect("date"), point.date);
        approx(number(fields[1]), point.cash);
        approx(number(fields[2]), point.position_value);
        approx(number(fields[3]), point.equity);
        approx(number(fields[4]), point.peak_equity);
        approx(number(fields[5]), point.drawdown);
    }
}

#[test]
fn funding_is_skipped_on_a_zero_volume_or_unaligned_bar() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");

    assert_eq!(
        ledger
            .apply_funding(1, 28_800, 100.0, Some(0.0))
            .expect("the call is valid"),
        None
    );
    assert_eq!(
        ledger
            .apply_funding(1, 28_801, 100.0, Some(10.0))
            .expect("the call is valid"),
        None
    );
    assert_eq!(ledger.funding_charges().len(), 0);

    let charge = ledger
        .apply_funding(1, 28_800, 100.0, Some(10.0))
        .expect("the call is valid")
        .expect("the boundary charges");
    approx(charge.amount, -1.0);
    assert_eq!(ledger.funding_charges().len(), 1);
}

#[test]
fn the_halt_latches_once_and_blocks_entries() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 79.0).expect("the mark is valid");

    assert!(ledger.state().halted);
    assert_eq!(ledger.risk_events().len(), 1);
    let event = ledger.risk_events()[0];
    assert_eq!(event.bar, 0);
    assert_eq!(event.time, 0);
    approx(event.drawdown, 0.211);
    approx(event.equity, 7_890.0);
    approx(event.peak_equity, 10_000.0);
    approx(event.limit, 0.20);
    assert_eq!(
        ledger.size_entry(Side::Long, 79.0),
        Err(SizingRejection::Halted)
    );

    ledger.mark(1, 900, 70.0).expect("the mark is valid");
    assert_eq!(ledger.risk_events().len(), 1);

    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            70.0,
            0.0,
            2,
            1800,
            FillReason::Exit,
        ))
        .expect("the exit is valid");
    assert_eq!(ledger.trades().len(), 1);
    assert!(ledger.state().halted);
}

#[test]
fn an_empty_run_has_no_trades_and_null_ratios() {
    let ledger = ledger();
    let metrics = ledger.metrics();

    assert_eq!(metrics.final_equity, 10_000.0);
    assert_eq!(metrics.total_return, 0.0);
    assert_eq!(metrics.realized_pnl, 0.0);
    assert_eq!(metrics.unrealized_pnl, 0.0);
    assert_eq!(metrics.gross_pnl, 0.0);
    assert_eq!(metrics.net_pnl, 0.0);
    assert_eq!(metrics.max_drawdown, 0.0);
    assert_eq!(metrics.max_drawdown_duration_bars, 0);
    assert_eq!(metrics.trades, 0);
    assert_eq!(metrics.win_rate, None);
    assert_eq!(metrics.profit_factor, None);
    assert_eq!(metrics.average_trade, None);
    assert_eq!(metrics.average_hold_bars, None);
    assert_eq!(metrics.exposure, None);
    assert!(!metrics.halted);
    assert_eq!(metrics.open_position, None);
    assert!(metrics.monthly_returns.is_empty());
}

#[test]
fn an_open_position_is_marked_and_reported() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 105.0).expect("the mark is valid");

    let metrics = ledger.metrics();
    approx(metrics.final_equity, 10_490.0);
    approx(metrics.realized_pnl, 0.0);
    approx(metrics.unrealized_pnl, 490.0);
    approx(metrics.gross_pnl, 500.0);
    approx(metrics.net_pnl, 490.0);
    assert_eq!(metrics.trades, 0);
    assert_eq!(
        metrics.open_position.expect("the position is open").side,
        Side::Long
    );
    approx(
        metrics
            .open_position
            .expect("the position is open")
            .entry_price,
        100.0,
    );
    approx(ledger.state().equity, 10_490.0);
}

#[test]
fn repeated_runs_are_identical() {
    fn run() -> PortfolioLedger {
        let mut ledger = ledger();
        ledger
            .apply_fill(&fill(
                OrderSide::Sell,
                100.0,
                100.0,
                0.0,
                0,
                0,
                FillReason::Entry,
            ))
            .expect("the entry is valid");
        ledger.mark(0, 0, 100.0).expect("the mark is valid");
        ledger
            .apply_funding(1, 28_800, 100.0, Some(10.0))
            .expect("the funding is valid");
        ledger.mark(1, 28_800, 105.0).expect("the mark is valid");
        ledger
            .apply_fill(&fill(
                OrderSide::Buy,
                100.0,
                105.0,
                0.0,
                2,
                29_700,
                FillReason::StopLoss,
            ))
            .expect("the exit is valid");
        ledger.mark(2, 29_700, 105.0).expect("the mark is valid");
        ledger
    }

    let first = run();
    for _ in 0..10 {
        let next = run();
        assert_eq!(next.trades(), first.trades());
        assert_eq!(next.equity_points(), first.equity_points());
        assert_eq!(next.funding_charges(), first.funding_charges());
        assert_eq!(next.risk_events(), first.risk_events());
        assert_eq!(next.metrics(), first.metrics());
    }
}

#[test]
fn later_bars_do_not_change_earlier_records() {
    let prefix_marks = 2;
    let mut prefix = ledger();
    prefix
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    prefix.mark(0, 0, 100.0).expect("the mark is valid");
    prefix.mark(1, 86_400, 105.0).expect("the mark is valid");

    let mut full = ledger();
    full.apply_fill(&fill(
        OrderSide::Buy,
        100.0,
        100.0,
        0.0,
        0,
        0,
        FillReason::Entry,
    ))
    .expect("the entry is valid");
    full.mark(0, 0, 100.0).expect("the mark is valid");
    full.mark(1, 86_400, 105.0).expect("the mark is valid");
    full.mark(2, 172_800, 90.0).expect("the mark is valid");

    assert_eq!(prefix.trades(), full.trades());
    assert_eq!(
        prefix.equity_points(),
        &full.equity_points()[..prefix_marks]
    );

    let mut changed = ledger();
    changed
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    changed.mark(0, 0, 100.0).expect("the mark is valid");
    changed.mark(1, 86_400, 105.0).expect("the mark is valid");
    changed.mark(2, 172_800, 120.0).expect("the mark is valid");

    assert_eq!(
        prefix.equity_points(),
        &changed.equity_points()[..prefix_marks]
    );
    assert_eq!(prefix.trades(), changed.trades());
}

#[test]
fn invalid_fill_sequences_are_rejected() {
    let mut open = ledger();
    open.apply_fill(&fill(
        OrderSide::Buy,
        100.0,
        100.0,
        0.0,
        0,
        0,
        FillReason::Entry,
    ))
    .expect("the entry is valid");

    assert_eq!(
        open.apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            1,
            900,
            FillReason::Entry
        )),
        Err(PortfolioLedgerError::PositionAlreadyOpen)
    );
    assert_eq!(
        open.apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            1,
            900,
            FillReason::Exit
        )),
        Err(PortfolioLedgerError::WrongSide)
    );

    let mut invalid = fill(OrderSide::Sell, 100.0, 100.0, 0.0, 1, 900, FillReason::Exit);
    invalid.quantity = 0.0;
    assert_eq!(
        open.apply_fill(&invalid),
        Err(PortfolioLedgerError::InvalidFill)
    );
    assert_eq!(
        open.mark(1, 900, 0.0),
        Err(PortfolioLedgerError::NonFinitePrice)
    );

    let mut flat = ledger();
    flat.apply_fill(&fill(
        OrderSide::Buy,
        100.0,
        100.0,
        0.0,
        0,
        0,
        FillReason::Entry,
    ))
    .expect("the entry is valid");
    flat.apply_fill(&fill(
        OrderSide::Sell,
        100.0,
        100.0,
        0.0,
        1,
        900,
        FillReason::Exit,
    ))
    .expect("the exit is valid");
    assert_eq!(
        flat.apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            100.0,
            0.0,
            2,
            1800,
            FillReason::Exit
        )),
        Err(PortfolioLedgerError::NoPosition)
    );
}

#[test]
fn month_arithmetic_is_correct() {
    use super::ledger::{month_of, next_month};

    assert_eq!(month_of(0), (1970, 1));
    assert_eq!(month_of(1_735_689_600), (2025, 1));
    assert_eq!(month_of(1_767_225_599), (2025, 12));
    assert_eq!(month_of(1_709_164_800), (2024, 2));
    assert_eq!(next_month(2025, 1), (2025, 2));
    assert_eq!(next_month(2025, 12), (2026, 1));
}

#[test]
fn zero_equity_has_no_exposure_and_a_full_loss() {
    let mut ledger = ledger();
    ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect("the entry is valid");
    ledger.mark(0, 0, 0.1).expect("the mark is valid");

    let metrics = ledger.metrics();
    assert_eq!(metrics.final_equity, 0.0);
    assert_eq!(metrics.total_return, -1.0);
    assert_eq!(metrics.exposure, None);
    approx(metrics.unrealized_pnl, -10_000.0);
    approx(metrics.net_pnl, -10_000.0);
    approx(metrics.max_drawdown, 1.0);
    assert!(metrics.halted);
    assert_eq!(
        metrics
            .open_position
            .expect("the position is open")
            .quantity,
        100.0
    );
}
