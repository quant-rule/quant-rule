//! Bracket engine tests.

use super::*;

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
fn an_entry_bar_stop_uses_the_fill_not_the_pre_entry_open() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    // The stop is 100.01, one cent below the fill, so it is tighter than the
    // slippage. The pre-entry open is below the stop; only the fill and the
    // bar's low may decide the exit.
    let second = candle(1, 900, 100.0, 100.05, 99.99, 100.0, Some(10.0));
    let intent = TradeIntent {
        side: Side::Long,
        entry: crate::intent::EntryStyle::MarketNextOpen,
        stop: StopSpec::Price { price: 100.01 },
        target: TargetSpec::Price { price: 100.10 },
        valid_until: None,
        max_slippage_bps: None,
    };

    step(&mut engine, &first, None, enter_with(0, 0, intent, None));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 2);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    approx(batch.fills[0].reference_price, 100.0);
    approx(batch.fills[0].price, 100.02);
    let exit = batch.fills[1];
    assert_eq!(exit.reason, FillReason::StopLoss);
    approx(exit.reference_price, 100.01);
    assert!(
        (exit.price - 99.99).abs() < 1e-4,
        "the stop must fill near 99.99, got {}",
        exit.price
    );
    assert!(engine.position().is_flat());
}

#[test]
fn a_short_entry_bar_stop_uses_the_fill_not_the_pre_entry_open() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let second = candle(1, 900, 100.0, 100.01, 99.95, 100.0, Some(10.0));
    let intent = TradeIntent {
        side: Side::Short,
        entry: crate::intent::EntryStyle::MarketNextOpen,
        stop: StopSpec::Price { price: 99.99 },
        target: TargetSpec::Price { price: 99.90 },
        valid_until: None,
        max_slippage_bps: None,
    };

    step(&mut engine, &first, None, enter_with(0, 0, intent, None));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 2);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    approx(batch.fills[0].reference_price, 100.0);
    approx(batch.fills[0].price, 99.98);
    let exit = batch.fills[1];
    assert_eq!(exit.reason, FillReason::StopLoss);
    approx(exit.reference_price, 99.99);
    assert!(
        (exit.price - 100.01).abs() < 1e-4,
        "the stop must fill near 100.01, got {}",
        exit.price
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
