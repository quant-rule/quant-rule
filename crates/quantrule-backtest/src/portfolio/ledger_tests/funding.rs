//! Funding, halt, and cash-boundary tests.
//!
//! These are separated from the ledger accounting tests so each file stays
//! within the repository's source-size bound. They use the shared fixtures and
//! helpers from the parent test module.

use super::*;

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
fn funding_marks_the_position_at_the_boundary_price() {
    let mut ledger = funded_ledger();
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
    ledger.mark(0, 0, 110.0).expect("the mark is valid");

    let charge = ledger
        .apply_funding(1, 28_800, 90.0, Some(10.0))
        .expect("the boundary charges")
        .expect("the charge is due");

    approx(charge.amount, 0.9);
    approx(ledger.state().cash, 89_899.1);
    approx(ledger.state().equity, 98_899.1);
}

#[test]
fn the_halt_latches_once_and_blocks_entries() {
    let mut config = funded_config();
    config.max_drawdown =
        MaxDrawdownConfig::new(Some(0.02), DrawdownAction::Halt).expect("the 2% drawdown is valid");
    let mut ledger = PortfolioLedger::new(config).expect("the ledger is valid");
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
    approx(event.drawdown, 0.022);
    approx(event.equity, 97_800.0);
    approx(event.peak_equity, 100_000.0);
    approx(event.limit, 0.02);
    assert_eq!(ledger.size_entry(79.0), Err(SizingRejection::Halted));
    assert_eq!(
        ledger.apply_fill(&fill(
            OrderSide::Buy,
            10.0,
            79.0,
            0.0,
            1,
            900,
            FillReason::Entry,
        )),
        Err(PortfolioLedgerError::Halted)
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
fn an_entry_cost_breach_halts_and_records_one_risk_event() {
    let mut config = config();
    config.initial_cash = 100_000.0;
    config.max_drawdown = MaxDrawdownConfig::new(Some(0.0005), DrawdownAction::Halt)
        .expect("the tight drawdown is valid");
    let mut ledger = PortfolioLedger::new(config).expect("the ledger is valid");

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

    assert!(ledger.state().halted);
    assert_eq!(ledger.risk_events().len(), 1);
    let event = ledger.risk_events()[0];
    assert_eq!(event.bar, 0);
    assert_eq!(event.time, 0);
    approx(event.drawdown, 0.001);
    approx(event.equity, 99_900.0);
    approx(ledger.metrics().max_drawdown, event.drawdown);
}

#[test]
fn a_funding_breach_halts_and_records_one_risk_event() {
    let mut config = config();
    config.initial_cash = 100_000.0;
    config.funding = FundingConfig::new(0.01, FundingInterval::H8).expect("the funding is valid");
    config.max_drawdown = MaxDrawdownConfig::new(Some(0.0015), DrawdownAction::Halt)
        .expect("the tight drawdown is valid");
    let mut ledger = PortfolioLedger::new(config).expect("the ledger is valid");
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
    assert!(!ledger.state().halted);

    let charge = ledger
        .apply_funding(1, 28_800, 100.0, Some(10.0))
        .expect("the boundary charges")
        .expect("the charge is due");

    approx(charge.amount, 100.0);
    assert!(ledger.state().halted);
    assert_eq!(ledger.risk_events().len(), 1);
    let event = ledger.risk_events()[0];
    assert_eq!(event.bar, 1);
    assert_eq!(event.time, 28_800);
    approx(event.drawdown, 0.002);
    approx(event.equity, 99_800.0);
    approx(ledger.metrics().max_drawdown, event.drawdown);
}

#[test]
fn a_closing_loss_breach_halts_and_records_one_risk_event() {
    let mut config = config();
    config.initial_cash = 100_000.0;
    config.max_drawdown = MaxDrawdownConfig::new(Some(0.002), DrawdownAction::Halt)
        .expect("the tight drawdown is valid");
    let mut ledger = PortfolioLedger::new(config).expect("the ledger is valid");
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
    assert!(!ledger.state().halted);

    ledger
        .apply_fill(&fill(
            OrderSide::Sell,
            100.0,
            99.0,
            0.0,
            2,
            1_800,
            FillReason::Exit,
        ))
        .expect("the exit is valid");

    assert!(ledger.state().halted);
    assert_eq!(ledger.risk_events().len(), 1);
    let event = ledger.risk_events()[0];
    assert_eq!(event.bar, 2);
    assert_eq!(event.time, 1_800);
    approx(event.drawdown, 1.0 - 99_700.1 / 100_000.0);
    approx(ledger.metrics().max_drawdown, event.drawdown);
}

#[test]
fn an_all_in_entry_is_refused() {
    let mut ledger = PortfolioLedger::new(config()).expect("the ledger is valid");
    let before = ledger.state();
    let error = ledger
        .apply_fill(&fill(
            OrderSide::Buy,
            100.0,
            100.0,
            0.0,
            0,
            0,
            FillReason::Entry,
        ))
        .expect_err("an all-in entry plus commission must be refused");

    assert_eq!(
        error,
        PortfolioLedgerError::InsufficientCash {
            required: 10_010.0,
            available: 10_000.0,
        }
    );
    assert_eq!(ledger.state(), before);
    assert!(ledger.trades().is_empty());
}
