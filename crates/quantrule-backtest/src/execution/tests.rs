//! Unit tests for the execution vocabulary.

use super::{
    ExecutionConfig, ExecutionConfigError, Fill, FillError, FillReason, Order, OrderError,
    OrderSide, OrderState, Position, PositionError, PositionState, Trigger,
};
use crate::side::Side;

fn market_order(side: OrderSide, reason: FillReason) -> Order {
    Order::new(side, 1.0, reason, Trigger::Market, 10, 1_700_000_000)
        .expect("the market order is valid")
}

fn stop_order() -> Order {
    Order::new(
        OrderSide::Sell,
        1.0,
        FillReason::StopLoss,
        Trigger::Stop { price: 95.0 },
        10,
        1_700_000_000,
    )
    .expect("the stop order is valid")
}

fn target_order() -> Order {
    Order::new(
        OrderSide::Sell,
        1.0,
        FillReason::TakeProfit,
        Trigger::Target { price: 110.0 },
        10,
        1_700_000_000,
    )
    .expect("the target order is valid")
}

fn fill() -> Fill {
    Fill::new(
        OrderSide::Buy,
        2.0,
        100.0,
        0.10,
        0.05,
        11,
        1_700_000_900,
        FillReason::Entry,
    )
    .expect("the fill is valid")
}

fn fill_with(
    quantity: f64,
    reference_price: f64,
    fee: f64,
    slippage: f64,
) -> Result<Fill, FillError> {
    Fill::new(
        OrderSide::Buy,
        quantity,
        reference_price,
        fee,
        slippage,
        0,
        0,
        FillReason::Entry,
    )
}

fn long_position() -> Position {
    Position::new(Side::Long, 2.0, 100.0, 10, 1_700_000_000, 95.0, 110.0)
        .expect("the long position is valid")
}

fn short_position() -> Position {
    Position::new(Side::Short, 2.0, 100.0, 10, 1_700_000_000, 105.0, 90.0)
        .expect("the short position is valid")
}

#[test]
fn order_side_codes_and_opposites() {
    assert_eq!(OrderSide::Buy.code(), 1);
    assert_eq!(OrderSide::Sell.code(), -1);
    assert_eq!(OrderSide::Buy.opposite(), OrderSide::Sell);
    assert_eq!(OrderSide::Sell.opposite(), OrderSide::Buy);
    assert!(OrderSide::Buy.is_buy());
    assert!(!OrderSide::Buy.is_sell());
    assert!(OrderSide::Sell.is_sell());
    assert!(!OrderSide::Sell.is_buy());
}

#[test]
fn trigger_prices_are_explicit() {
    assert_eq!(Trigger::Market.price(), None);
    assert_eq!(Trigger::Stop { price: 95.0 }.price(), Some(95.0));
    assert_eq!(Trigger::Target { price: 110.0 }.price(), Some(110.0));
}

#[test]
fn market_orders_carry_entry_or_exit() {
    let entry = market_order(OrderSide::Buy, FillReason::Entry);
    assert_eq!(entry.side, OrderSide::Buy);
    assert_eq!(entry.quantity, 1.0);
    assert_eq!(entry.reason, FillReason::Entry);
    assert_eq!(entry.trigger, Trigger::Market);
    assert_eq!(entry.submitted_bar, 10);
    assert_eq!(entry.submitted_time, 1_700_000_000);

    let exit = market_order(OrderSide::Sell, FillReason::Exit);
    assert_eq!(exit.reason, FillReason::Exit);
}

#[test]
fn stop_and_target_orders_are_valid() {
    let stop = stop_order();
    assert_eq!(stop.trigger, Trigger::Stop { price: 95.0 });
    assert_eq!(stop.reason, FillReason::StopLoss);

    let target = target_order();
    assert_eq!(target.trigger, Trigger::Target { price: 110.0 });
    assert_eq!(target.reason, FillReason::TakeProfit);
}

#[test]
fn order_rejects_invalid_quantity() {
    for quantity in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let result = Order::new(
            OrderSide::Buy,
            quantity,
            FillReason::Entry,
            Trigger::Market,
            0,
            0,
        );
        assert!(
            matches!(
                result,
                Err(OrderError::NonFiniteQuantity | OrderError::NonPositiveQuantity)
            ),
            "quantity {quantity} was accepted"
        );
    }
}

#[test]
fn order_rejects_invalid_trigger_price() {
    for price in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let stop = Order::new(
            OrderSide::Sell,
            1.0,
            FillReason::StopLoss,
            Trigger::Stop { price },
            0,
            0,
        );
        assert!(
            matches!(
                stop,
                Err(OrderError::NonFiniteTriggerPrice | OrderError::NonPositiveTriggerPrice)
            ),
            "stop price {price} was accepted"
        );

        let target = Order::new(
            OrderSide::Sell,
            1.0,
            FillReason::TakeProfit,
            Trigger::Target { price },
            0,
            0,
        );
        assert!(
            matches!(
                target,
                Err(OrderError::NonFiniteTriggerPrice | OrderError::NonPositiveTriggerPrice)
            ),
            "target price {price} was accepted"
        );
    }
}

#[test]
fn order_rejects_reason_trigger_mismatch() {
    let entry_with_stop = Order::new(
        OrderSide::Buy,
        1.0,
        FillReason::Entry,
        Trigger::Stop { price: 95.0 },
        0,
        0,
    );
    assert_eq!(entry_with_stop, Err(OrderError::ReasonTriggerMismatch));

    let stop_with_market = Order::new(
        OrderSide::Sell,
        1.0,
        FillReason::StopLoss,
        Trigger::Market,
        0,
        0,
    );
    assert_eq!(stop_with_market, Err(OrderError::ReasonTriggerMismatch));

    let target_with_stop = Order::new(
        OrderSide::Sell,
        1.0,
        FillReason::TakeProfit,
        Trigger::Stop { price: 95.0 },
        0,
        0,
    );
    assert_eq!(target_with_stop, Err(OrderError::ReasonTriggerMismatch));

    let exit_with_target = Order::new(
        OrderSide::Sell,
        1.0,
        FillReason::Exit,
        Trigger::Target { price: 110.0 },
        0,
        0,
    );
    assert_eq!(exit_with_target, Err(OrderError::ReasonTriggerMismatch));
}

#[test]
fn order_state_is_terminal_only_after_pending() {
    let states = [
        OrderState::Pending,
        OrderState::Filled,
        OrderState::Rejected,
    ];

    assert!(!states[0].is_terminal());
    for state in &states[1..] {
        assert!(state.is_terminal(), "{state:?} was not terminal");
    }
}

#[test]
fn fill_accepts_valid_values_and_reports_notional() {
    let fill = fill();
    assert_eq!(fill.side, OrderSide::Buy);
    assert_eq!(fill.quantity, 2.0);
    assert_eq!(fill.price, 100.05);
    assert_eq!(fill.reference_price, 100.0);
    assert_eq!(fill.fee, 0.10);
    assert_eq!(fill.slippage, 0.05);
    assert_eq!(fill.bar, 11);
    assert_eq!(fill.time, 1_700_000_900);
    assert_eq!(fill.reason, FillReason::Entry);
    assert!((fill.notional() - 200.1).abs() < 1e-12);
}

#[test]
fn fill_applies_slippage_against_the_position() {
    let buy = Fill::new(
        OrderSide::Buy,
        1.0,
        100.0,
        0.0,
        0.05,
        0,
        0,
        FillReason::Entry,
    )
    .expect("the buy fill is valid");
    assert_eq!(buy.price, 100.05);

    let sell = Fill::new(
        OrderSide::Sell,
        1.0,
        100.0,
        0.0,
        0.05,
        0,
        0,
        FillReason::Exit,
    )
    .expect("the sell fill is valid");
    assert_eq!(sell.price, 99.95);
}

#[test]
fn fill_rejects_invalid_values() {
    assert!(matches!(
        fill_with(0.0, 100.0, 0.0, 0.0),
        Err(FillError::NonPositiveQuantity)
    ));
    assert!(matches!(
        fill_with(f64::NAN, 100.0, 0.0, 0.0),
        Err(FillError::NonFiniteQuantity)
    ));
    assert!(matches!(
        fill_with(1.0, 0.0, 0.0, 0.0),
        Err(FillError::NonPositiveReferencePrice)
    ));
    assert!(matches!(
        fill_with(1.0, f64::NAN, 0.0, 0.0),
        Err(FillError::NonFiniteReferencePrice)
    ));
    assert!(matches!(
        fill_with(1.0, 100.0, -0.1, 0.0),
        Err(FillError::NegativeFee)
    ));
    assert!(matches!(
        fill_with(1.0, 100.0, f64::NAN, 0.0),
        Err(FillError::NonFiniteFee)
    ));
    assert!(matches!(
        fill_with(1.0, 100.0, 0.0, -0.1),
        Err(FillError::NegativeSlippage)
    ));
    assert!(matches!(
        fill_with(1.0, 100.0, 0.0, f64::NAN),
        Err(FillError::NonFiniteSlippage)
    ));
    assert!(matches!(
        Fill::new(
            OrderSide::Sell,
            1.0,
            100.0,
            0.0,
            100.0,
            0,
            0,
            FillReason::Exit
        ),
        Err(FillError::NonPositivePrice)
    ));
    assert!(matches!(
        fill_with(1.0, f64::MAX, 0.0, f64::MAX),
        Err(FillError::NonFinitePrice)
    ));
    assert!(fill_with(1.0, 100.0, 0.0, 0.0).is_ok());
}

#[test]
fn fill_reason_exit_flags_and_trigger_match() {
    assert!(!FillReason::Entry.is_exit());
    assert!(FillReason::Exit.is_exit());
    assert!(FillReason::StopLoss.is_exit());
    assert!(FillReason::TakeProfit.is_exit());

    assert!(FillReason::Entry.matches_trigger(Trigger::Market));
    assert!(FillReason::Exit.matches_trigger(Trigger::Market));
    assert!(FillReason::StopLoss.matches_trigger(Trigger::Stop { price: 95.0 }));
    assert!(FillReason::TakeProfit.matches_trigger(Trigger::Target { price: 110.0 }));
    assert!(!FillReason::Entry.matches_trigger(Trigger::Stop { price: 95.0 }));
    assert!(!FillReason::StopLoss.matches_trigger(Trigger::Market));
}

#[test]
fn position_accepts_long_and_short() {
    let long = long_position();
    assert!(long.is_long());
    assert!(!long.is_short());
    assert_eq!(long.stop_price, 95.0);
    assert_eq!(long.target_price, 110.0);

    let short = short_position();
    assert!(short.is_short());
    assert!(!short.is_long());
    assert_eq!(short.stop_price, 105.0);
    assert_eq!(short.target_price, 90.0);
}

#[test]
fn position_rejects_invalid_values() {
    assert!(matches!(
        Position::new(Side::Long, 0.0, 100.0, 0, 0, 95.0, 110.0),
        Err(PositionError::NonPositiveQuantity)
    ));
    assert!(matches!(
        Position::new(Side::Long, f64::NAN, 100.0, 0, 0, 95.0, 110.0),
        Err(PositionError::NonFiniteQuantity)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 0.0, 0, 0, 95.0, 110.0),
        Err(PositionError::NonPositiveEntryPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, f64::NAN, 0, 0, 95.0, 110.0),
        Err(PositionError::NonFiniteEntryPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, 0.0, 110.0),
        Err(PositionError::NonPositiveStopPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, f64::NAN, 110.0),
        Err(PositionError::NonFiniteStopPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, 95.0, 0.0),
        Err(PositionError::NonPositiveTargetPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, 95.0, f64::NAN),
        Err(PositionError::NonFiniteTargetPrice)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, 100.0, 110.0),
        Err(PositionError::StopNotBelowEntry)
    ));
    assert!(matches!(
        Position::new(Side::Long, 1.0, 100.0, 0, 0, 95.0, 100.0),
        Err(PositionError::TargetNotAboveEntry)
    ));
    assert!(matches!(
        Position::new(Side::Short, 1.0, 100.0, 0, 0, 100.0, 90.0),
        Err(PositionError::StopNotAboveEntry)
    ));
    assert!(matches!(
        Position::new(Side::Short, 1.0, 100.0, 0, 0, 105.0, 100.0),
        Err(PositionError::TargetNotBelowEntry)
    ));
}

#[test]
fn position_state_covers_flat_open_and_closed() {
    let flat = PositionState::Flat;
    assert!(flat.is_flat());
    assert!(!flat.is_open());
    assert!(!flat.is_closed());

    let open = PositionState::Open(long_position());
    assert!(!open.is_flat());
    assert!(open.is_open());
    assert!(!open.is_closed());

    let closed = PositionState::Closed;
    assert!(!closed.is_flat());
    assert!(!closed.is_open());
    assert!(closed.is_closed());
}

#[test]
fn execution_config_validates_costs() {
    let config = ExecutionConfig::new(0.001, 2.0).expect("the costs are valid");
    assert_eq!(config.fee_rate(), 0.001);
    assert_eq!(config.slippage_bps(), 2.0);
    assert!((config.slippage_rate() - 0.0002).abs() < 1e-12);

    assert!(matches!(
        ExecutionConfig::new(-0.001, 0.0),
        Err(ExecutionConfigError::NegativeFeeRate)
    ));
    assert!(matches!(
        ExecutionConfig::new(f64::NAN, 0.0),
        Err(ExecutionConfigError::NonFiniteFeeRate)
    ));
    assert!(matches!(
        ExecutionConfig::new(0.0, -1.0),
        Err(ExecutionConfigError::NegativeSlippage)
    ));
    assert!(matches!(
        ExecutionConfig::new(0.0, f64::INFINITY),
        Err(ExecutionConfigError::NonFiniteSlippage)
    ));
}

#[test]
fn repeated_construction_is_identical() {
    assert_eq!(stop_order(), stop_order());
    assert_eq!(fill(), fill());
    assert_eq!(long_position(), long_position());
    assert_eq!(
        ExecutionConfig::new(0.001, 2.0),
        ExecutionConfig::new(0.001, 2.0)
    );
}
