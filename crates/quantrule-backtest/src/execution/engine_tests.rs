//! Unit tests for the causal fill engine.

use quantrule_data::Candle;

use crate::intent::{RiskPolicy, TradeIntent};
use crate::side::Side;

use super::*;

const FEE_RATE: f64 = 0.001;
pub(super) const SLIPPAGE_BPS: f64 = 2.0;
const QUANTITY: f64 = 1.0;

pub(super) fn candle(
    index: usize,
    time: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
) -> Candle {
    Candle::new(index as u64, time, open, high, low, close, volume)
        .expect("the test candle is valid")
}

pub(super) fn enter(index: usize, time: i64, side: Side) -> OrderIntent {
    enter_with(index, time, RiskPolicy::default().intent(side), None)
}

pub(super) fn enter_with(
    index: usize,
    time: i64,
    intent: TradeIntent,
    max_notional: Option<f64>,
) -> OrderIntent {
    OrderIntent::Enter {
        intent,
        bar_index: index,
        open_time: time,
        max_notional,
    }
}

fn enter_limited(index: usize, time: i64, side: Side, max_notional: f64) -> OrderIntent {
    enter_with(
        index,
        time,
        RiskPolicy::default().intent(side),
        Some(max_notional),
    )
}

fn exit(index: usize, time: i64, side: Side) -> OrderIntent {
    OrderIntent::Exit {
        side,
        bar_index: index,
        open_time: time,
    }
}

pub(super) fn test_engine() -> ExecutionEngine {
    ExecutionEngine::new(ExecutionConfig::new(FEE_RATE, SLIPPAGE_BPS).expect("the costs are valid"))
        .expect("the engine is valid")
}

pub(super) fn approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}

pub(super) fn open_position(engine: &ExecutionEngine) -> Position {
    let PositionState::Open(position) = engine.position() else {
        panic!("expected an open position, got {:?}", engine.position());
    };
    position
}

pub(super) fn advance(engine: &mut ExecutionEngine, bar: &Candle) -> ExecutionBatch {
    engine.advance(bar, true).expect("the bar is valid")
}

pub(super) fn advance_with(
    engine: &mut ExecutionEngine,
    bar: &Candle,
    contiguous: bool,
) -> ExecutionBatch {
    engine.advance(bar, contiguous).expect("the bar is valid")
}

/// Advances `bar` and submits `intent` with an explicit quantity and ATR.
fn step_quantity(
    engine: &mut ExecutionEngine,
    bar: &Candle,
    quantity: Option<f64>,
    atr: Option<f64>,
    intent: OrderIntent,
) -> ExecutionBatch {
    let mut batch = advance(engine, bar);
    let submitted = engine
        .submit(intent, quantity, atr)
        .expect("the intent is valid");
    batch.orders.extend(submitted.orders);
    batch.rejections.extend(submitted.rejections);
    batch
}

/// Advances `bar` and submits `intent` with the test quantity and ATR.
pub(super) fn step(
    engine: &mut ExecutionEngine,
    bar: &Candle,
    atr: Option<f64>,
    intent: OrderIntent,
) -> ExecutionBatch {
    step_quantity(engine, bar, Some(QUANTITY), atr, intent)
}

/// Bars 0 and 1: a long entry decided on bar 0 fills on bar 1 at 100.
pub(super) fn entry_series() -> (Candle, Candle) {
    (
        candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
        candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0)),
    )
}

fn run(
    bars: &[Candle],
    atrs: &[Option<f64>],
    intents: &[OrderIntent],
) -> (Vec<Fill>, Vec<Rejection>, PositionState) {
    let mut engine = test_engine();
    let mut fills = Vec::new();
    let mut rejections = Vec::new();
    for ((bar, atr), intent) in bars.iter().zip(atrs).zip(intents) {
        let batch = step(&mut engine, bar, *atr, *intent);
        fills.extend(batch.fills);
        rejections.extend(batch.rejections);
    }
    let batch = engine.finish();
    rejections.extend(batch.rejections);
    (fills, rejections, engine.position())
}

#[test]
fn a_market_entry_fills_at_the_next_open_with_costs() {
    let mut engine = test_engine();
    let (first, second) = entry_series();

    let batch = step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    assert!(batch.fills.is_empty());
    assert_eq!(batch.orders.len(), 1);
    assert_eq!(batch.orders[0].order_id, 0);
    assert_eq!(batch.orders[0].state, OrderState::Pending);
    assert!(engine.position().is_flat());

    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert_eq!(fill.side, OrderSide::Buy);
    assert_eq!(fill.reason, FillReason::Entry);
    assert_eq!(fill.quantity, QUANTITY);
    assert_eq!(fill.bar, 1);
    assert_eq!(fill.time, 900);
    approx(fill.reference_price, 100.0);
    approx(fill.slippage, 0.02);
    approx(fill.price, 100.02);
    approx(fill.fee, 0.100_02);

    assert_eq!(batch.orders.len(), 1);
    let order = batch.orders[0];
    assert_eq!(order.order_id, 0);
    assert_eq!(order.state, OrderState::Filled);
    assert_eq!(order.filled_bar, Some(1));
    assert_eq!(order.filled_time, Some(900));
    assert_eq!(order.fill_id, Some(0));
    assert_eq!(order.rejection_reason, None);

    let position = open_position(&engine);
    assert_eq!(position.side, Side::Long);
    approx(position.entry_price, 100.02);
    approx(position.stop_price, 98.02);
    approx(position.target_price, 103.02);
}

#[test]
fn a_intent_does_not_fill_on_its_own_bar() {
    let mut engine = test_engine();
    let (first, _) = entry_series();

    let batch = step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    assert!(batch.fills.is_empty());
    assert!(engine.position().is_flat());
}

#[test]
fn a_second_pending_order_is_rejected() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));

    let error = engine
        .submit(enter(0, 0, Side::Long), Some(QUANTITY), Some(1.0))
        .expect_err("a second pending order must be rejected");
    assert_eq!(error, ExecutionError::PendingOrderExists);

    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.orders.len(), 1);
    assert_eq!(batch.orders[0].order_id, 0);
}

#[test]
fn a_non_contiguous_bar_rejects_a_pending_entry() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    assert!(advance(&mut engine, &first).is_empty());
    engine
        .submit(enter(0, 0, Side::Long), Some(QUANTITY), Some(1.0))
        .expect("the entry is valid");

    let gapped = candle(1, 1800, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let batch = advance_with(&mut engine, &gapped, false);
    assert!(batch.fills.is_empty());
    assert_eq!(batch.rejections.len(), 1);
    assert_eq!(batch.rejections[0].reason, RejectReason::NonContiguousBar);
    assert_eq!(batch.rejections[0].bar, 1);
    let order = batch
        .orders
        .iter()
        .find(|order| order.state == OrderState::Rejected)
        .expect("the pending order is rejected");
    assert_eq!(order.submitted_bar, 0);
    assert!(engine.position().is_flat());
}

#[test]
fn a_non_contiguous_bar_rejects_a_pending_exit_and_keeps_the_position() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert!(engine.position().is_open());

    let submitted = engine
        .submit(exit(1, 900, Side::Long), Some(QUANTITY), Some(1.0))
        .expect("the exit is valid");
    assert_eq!(submitted.orders[0].state, OrderState::Pending);

    let gapped = candle(2, 2700, 100.0, 100.5, 99.5, 100.0, Some(10.0));
    let batch = advance_with(&mut engine, &gapped, false);
    assert!(batch.fills.is_empty());
    assert_eq!(batch.rejections.len(), 1);
    assert_eq!(batch.rejections[0].reason, RejectReason::NonContiguousBar);
    assert!(engine.position().is_open());
}

#[test]
fn a_non_contiguous_bar_still_evaluates_the_bracket() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert!(engine.position().is_open());

    let gapped = candle(2, 2700, 97.0, 98.0, 96.0, 97.0, Some(10.0));
    let batch = advance_with(&mut engine, &gapped, false);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    assert!(engine.position().is_flat());
}

#[test]
fn a_long_stop_fills_at_the_stop() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.0, 100.5, 97.5, 98.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert_eq!(fill.reason, FillReason::StopLoss);
    assert_eq!(fill.side, OrderSide::Sell);
    assert_eq!(fill.bar, 2);
    approx(fill.reference_price, 98.02);
    approx(fill.slippage, 98.02 * 0.0002);
    approx(fill.price, 98.02 - 98.02 * 0.0002);
    assert!(engine.position().is_flat());

    assert_eq!(batch.orders.len(), 1);
    let order = batch.orders[0];
    assert_eq!(order.order_id, 1);
    assert_eq!(order.reason, FillReason::StopLoss);
    assert_eq!(order.trigger, Trigger::Stop { price: 98.02 });
    assert_eq!(order.submitted_bar, 1);
    assert_eq!(order.filled_bar, Some(2));
    assert_eq!(order.fill_id, Some(1));
}

#[test]
fn a_long_stop_gap_fills_at_the_open() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 97.0, 98.0, 96.0, 97.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    approx(batch.fills[0].reference_price, 97.0);
}

#[test]
fn a_long_target_fills_at_the_target() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.5, 104.0, 100.0, 103.5, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert_eq!(fill.reason, FillReason::TakeProfit);
    assert_eq!(fill.side, OrderSide::Sell);
    approx(fill.reference_price, 103.02);
    assert!(engine.position().is_flat());
}

#[test]
fn a_long_target_gap_fills_at_the_open() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 104.0, 104.5, 103.5, 104.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::TakeProfit);
    approx(batch.fills[0].reference_price, 104.0);
}

#[test]
fn a_long_gap_through_the_target_wins_over_a_later_stop() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    // The open is already beyond the target, so the target was reached before
    // the intraday dip to the stop.
    let third = candle(2, 1800, 104.0, 104.5, 97.0, 104.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::TakeProfit);
    approx(batch.fills[0].reference_price, 104.0);
}

#[test]
fn the_stop_wins_when_a_long_bar_touches_both() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.0, 104.0, 97.0, 100.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    approx(batch.fills[0].reference_price, 98.02);
}

#[test]
fn a_bracket_triggers_on_an_exact_touch_and_the_stop_wins() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    // The bar's low equals the stop and its high equals the target. The stop
    // must win when both levels are touched, equality included.
    let third = candle(2, 1800, 100.0, 103.02, 98.02, 100.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    approx(batch.fills[0].reference_price, 98.02);
}

#[test]
fn a_short_stop_gap_fills_at_the_open() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 102.0, 102.5, 101.5, 102.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    approx(batch.fills[0].reference_price, 102.0);
}

#[test]
fn a_short_entry_and_stop_are_mirrored() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].side, OrderSide::Sell);
    approx(batch.fills[0].price, 99.98);

    let position = open_position(&engine);
    assert_eq!(position.side, Side::Short);
    approx(position.stop_price, 101.98);
    approx(position.target_price, 96.98);

    let third = candle(2, 1800, 100.0, 102.0, 99.0, 100.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    assert_eq!(batch.fills[0].side, OrderSide::Buy);
    approx(batch.fills[0].reference_price, 101.98);
    approx(batch.fills[0].price, 101.98 + 101.98 * 0.0002);
    assert!(engine.position().is_flat());
}

#[test]
fn a_short_target_fills_at_the_target() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.0, 100.5, 96.5, 97.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert_eq!(fill.reason, FillReason::TakeProfit);
    assert_eq!(fill.side, OrderSide::Buy);
    approx(fill.reference_price, 96.98);
    assert!(engine.position().is_flat());
}

#[test]
fn a_short_target_gap_fills_at_the_open() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 96.0, 97.0, 95.5, 96.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::TakeProfit);
    approx(batch.fills[0].reference_price, 96.0);
}

#[test]
fn a_short_gap_through_the_target_wins_over_a_later_stop() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    // The open is already below the target, so the target was reached before
    // the intraday rise to the stop.
    let third = candle(2, 1800, 96.0, 102.0, 95.5, 96.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::TakeProfit);
    approx(batch.fills[0].reference_price, 96.0);
}

#[test]
fn the_stop_wins_when_a_short_bar_touches_both() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Short));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.0, 102.0, 96.0, 100.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    approx(batch.fills[0].reference_price, 101.98);
}

#[test]
fn a_market_exit_fills_at_the_next_open_and_cancels_the_bracket() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), exit(1, 900, Side::Long));
    assert!(engine.position().is_open());

    let third = candle(2, 1800, 101.0, 101.5, 100.5, 101.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::Exit);
    assert_eq!(batch.fills[0].side, OrderSide::Sell);
    approx(batch.fills[0].reference_price, 101.0);
    assert!(engine.position().is_flat());

    let fourth = candle(3, 2700, 100.0, 100.5, 90.0, 95.0, Some(10.0));
    assert!(
        step(&mut engine, &fourth, Some(1.0), OrderIntent::Hold)
            .fills
            .is_empty()
    );
}

#[test]
fn an_exit_at_the_open_wins_over_a_same_bar_stop() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), exit(1, 900, Side::Long));

    let third = candle(2, 1800, 101.0, 101.0, 90.0, 95.0, Some(10.0));
    let batch = step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::Exit);
    approx(batch.fills[0].reference_price, 101.0);
    assert!(engine.position().is_flat());
}

#[test]
fn an_entry_bar_can_trigger_its_own_stop() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let second = candle(1, 900, 100.0, 100.5, 97.5, 98.0, Some(10.0));

    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 2);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    assert_eq!(batch.fills[1].reason, FillReason::StopLoss);
    assert_eq!(batch.fills[1].bar, 1);
    assert_eq!(batch.orders.len(), 2);
    assert_eq!(batch.orders[0].state, OrderState::Filled);
    assert_eq!(batch.orders[1].state, OrderState::Filled);
    assert!(engine.position().is_flat());
}

#[test]
fn a_missing_or_invalid_atr_rejects_the_entry() {
    for atr in [
        None,
        Some(0.0),
        Some(-1.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
    ] {
        let mut engine = test_engine();
        let (first, second) = entry_series();
        step(&mut engine, &first, atr, enter(0, 0, Side::Long));
        let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
        assert!(batch.fills.is_empty(), "atr {atr:?} produced a fill");
        assert_eq!(
            batch.rejections,
            &[Rejection {
                order_id: 0,
                bar: 1,
                time: 900,
                reason: RejectReason::MissingAtr,
            }]
        );
        assert_eq!(batch.orders[0].state, OrderState::Rejected);
        assert_eq!(
            batch.orders[0].rejection_reason,
            Some(RejectReason::MissingAtr)
        );
        assert!(engine.position().is_flat());
    }
}

#[test]
fn an_entry_above_the_risk_limit_is_capped_at_the_fill() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(
        &mut engine,
        &first,
        Some(1.0),
        enter_limited(0, 0, Side::Long, 50.0),
    );
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert!(fill.price * fill.quantity <= 50.0 + 1e-9);
    assert!(fill.quantity < QUANTITY);
    assert_eq!(batch.orders[0].quantity, Some(fill.quantity));
    assert!(engine.position().is_open());
}

#[test]
fn a_zero_volume_next_bar_rejects_the_entry() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let second = candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(0.0));

    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert!(batch.fills.is_empty());
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 1,
            time: 900,
            reason: RejectReason::ZeroVolume,
        }]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn a_zero_volume_bar_does_not_trigger_the_bracket() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let third = candle(2, 1800, 100.0, 100.0, 97.0, 98.0, Some(0.0));
    assert!(
        step(&mut engine, &third, Some(1.0), OrderIntent::Hold)
            .fills
            .is_empty()
    );
    assert!(engine.position().is_open());

    let fourth = candle(3, 2700, 98.0, 98.5, 97.0, 97.5, Some(10.0));
    let batch = step(&mut engine, &fourth, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::StopLoss);
    assert_eq!(batch.fills[0].bar, 3);
}

#[test]
fn an_entry_while_open_is_rejected() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    let batch = step(&mut engine, &second, Some(1.0), enter(1, 900, Side::Long));
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 1,
            bar: 1,
            time: 900,
            reason: RejectReason::PositionAlreadyOpen,
        }]
    );
    assert_eq!(batch.orders[0].state, OrderState::Filled);
    assert_eq!(batch.orders[1].state, OrderState::Rejected);
    assert_eq!(batch.orders[1].quantity, Some(QUANTITY));
    assert!(engine.position().is_open());
}

#[test]
fn an_exit_while_flat_is_rejected() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));

    let batch = step(&mut engine, &first, Some(1.0), exit(0, 0, Side::Long));
    assert!(batch.fills.is_empty());
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 0,
            time: 0,
            reason: RejectReason::NoPosition,
        }]
    );
    assert_eq!(batch.orders[0].side, OrderSide::Sell);
    assert_eq!(batch.orders[0].quantity, None);
}

#[test]
fn an_exit_with_the_wrong_side_is_rejected() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let batch = engine
        .submit(exit(1, 900, Side::Short), None, None)
        .expect("the intent is valid");
    assert!(batch.fills.is_empty());
    assert_eq!(batch.rejections.len(), 1);
    assert_eq!(batch.rejections[0].reason, RejectReason::WrongExitSide);
    assert_eq!(batch.orders[0].side, OrderSide::Sell);
    assert_eq!(batch.orders[0].quantity, Some(QUANTITY));
    assert!(engine.position().is_open());
}

#[test]
fn a_intent_for_another_bar_is_an_error() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    advance(&mut engine, &first);

    assert_eq!(
        engine.submit(enter(1, 0, Side::Long), Some(QUANTITY), Some(1.0)),
        Err(ExecutionError::DecisionBarMismatch {
            expected: 0,
            found: 1,
        })
    );
    assert_eq!(
        engine.submit(enter(0, 1, Side::Long), Some(QUANTITY), Some(1.0)),
        Err(ExecutionError::DecisionTimeMismatch {
            expected: 0,
            found: 1,
        })
    );
    assert!(
        engine
            .submit(enter(0, 0, Side::Long), Some(QUANTITY), Some(1.0))
            .is_ok()
    );
}

#[test]
fn a_intent_before_any_bar_is_an_error() {
    let mut engine = test_engine();

    assert_eq!(
        engine.submit(enter(0, 0, Side::Long), Some(QUANTITY), Some(1.0)),
        Err(ExecutionError::SubmitBeforeAdvance)
    );
}

#[test]
fn finish_rejects_a_pending_order_and_leaves_an_open_position() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    let batch = engine.finish();
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 0,
            time: 0,
            reason: RejectReason::NoNextBar,
        }]
    );
    assert_eq!(batch.orders[0].state, OrderState::Rejected);
    assert!(engine.position().is_flat());
    assert!(engine.finish().is_empty());

    let mut engine = test_engine();
    let (first, second) = entry_series();
    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert!(engine.finish().is_empty());
    assert!(engine.position().is_open());
}

#[test]
fn the_fee_uses_the_executed_notional_and_the_quantity() {
    let config = ExecutionConfig::new(0.001, 2.0).expect("the costs are valid");
    let mut engine = ExecutionEngine::new(config).expect("the engine is valid");
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let second = candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0));

    advance(&mut engine, &first);
    engine
        .submit(enter(0, 0, Side::Long), Some(2.0), Some(1.0))
        .expect("the intent is valid");
    let batch = advance(&mut engine, &second);
    assert_eq!(batch.fills.len(), 1);
    approx(batch.fills[0].slippage, 0.02);
    approx(batch.fills[0].price, 100.02);
    approx(batch.fills[0].fee, 100.02 * 2.0 * 0.001);
}

#[test]
fn an_unusable_quantity_rejects_the_entry() {
    for (quantity, reason) in [
        (None, RejectReason::MissingQuantity),
        (Some(0.0), RejectReason::NonPositiveQuantity),
        (Some(-1.0), RejectReason::NonPositiveQuantity),
        (Some(f64::NAN), RejectReason::NonPositiveQuantity),
        (Some(f64::INFINITY), RejectReason::NonPositiveQuantity),
    ] {
        let mut engine = test_engine();
        let (first, second) = entry_series();
        let batch = step_quantity(
            &mut engine,
            &first,
            quantity,
            Some(1.0),
            enter(0, 0, Side::Long),
        );
        assert_eq!(
            batch.rejections,
            &[Rejection {
                order_id: 0,
                bar: 0,
                time: 0,
                reason,
            }]
        );
        assert_eq!(batch.orders[0].state, OrderState::Rejected);
        match (batch.orders[0].quantity, quantity) {
            (None, None) => {}
            (Some(actual), Some(expected)) => assert_eq!(actual.to_bits(), expected.to_bits()),
            other => panic!("the rejected quantity was not carried through: {other:?}"),
        }
        assert!(
            advance(&mut engine, &second).fills.is_empty(),
            "quantity {quantity:?} produced a fill"
        );
        assert!(engine.position().is_flat());
    }
}

#[test]
fn a_wide_slippage_is_rejected_at_construction() {
    let wide = ExecutionConfig::new(0.0, 10_000.0).expect("the costs are valid");
    assert_eq!(
        ExecutionEngine::new(wide).expect_err("slippage at 100% must be rejected"),
        ExecutionError::SlippageAtOrAboveOne
    );
}

#[test]
fn repeated_runs_are_identical() {
    let bars = [
        candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
        candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0)),
        candle(2, 1800, 100.0, 104.0, 97.0, 100.0, Some(10.0)),
        candle(3, 2700, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
    ];
    let atrs = [Some(1.0); 4];
    let intents = [
        enter(0, 0, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
        OrderIntent::Hold,
    ];

    let first = run(&bars, &atrs, &intents);
    for _ in 0..10 {
        assert_eq!(run(&bars, &atrs, &intents), first);
    }
}

#[test]
fn later_bars_do_not_change_earlier_fills() {
    let bars = [
        candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0)),
        candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0)),
        candle(2, 1800, 100.0, 100.5, 99.0, 100.0, Some(10.0)),
        candle(3, 2700, 100.0, 100.5, 99.0, 100.0, Some(10.0)),
        candle(4, 3600, 101.0, 101.5, 100.5, 101.0, Some(10.0)),
        candle(5, 4500, 101.0, 101.5, 100.5, 101.0, Some(10.0)),
    ];
    let atrs = [Some(1.0); 6];
    let intents = [
        enter(0, 0, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
        exit(3, 2700, Side::Long),
        OrderIntent::Hold,
        OrderIntent::Hold,
    ];

    let prefix = 4;
    let (prefix_fills, _, _) = run(&bars[..prefix], &atrs[..prefix], &intents[..prefix]);
    let (full_fills, _, _) = run(&bars, &atrs, &intents);

    let prefix_early: Vec<Fill> = prefix_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    let full_early: Vec<Fill> = full_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    assert_eq!(prefix_early, full_early);

    let mut changed = bars;
    changed[4] = candle(4, 3600, 200.0, 200.5, 199.0, 200.0, Some(10.0));
    let (changed_fills, _, _) = run(&changed, &atrs, &intents);
    let changed_early: Vec<Fill> = changed_fills
        .iter()
        .copied()
        .filter(|fill| fill.bar < prefix)
        .collect();
    assert_eq!(changed_early, full_early);
}
