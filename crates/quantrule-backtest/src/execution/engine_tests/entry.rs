//! Entry engine tests.

use super::*;

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
fn a_wide_slippage_is_rejected_at_construction() {
    let wide = ExecutionConfig::new(0.0, 10_000.0).expect("the costs are valid");
    assert_eq!(
        ExecutionEngine::new(wide).expect_err("slippage at 100% must be rejected"),
        ExecutionError::SlippageAtOrAboveOne
    );
}
