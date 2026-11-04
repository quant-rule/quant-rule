//! Unit tests for two-sided stop-entry brackets and OCO.
//!
//! The engine places one stop-entry order at each boundary, fills the first
//! leg reached, and cancels the other. The tests cover the trigger, gap,
//! expiry, ambiguity, zero-volume, contiguity, risk-cap, and end-of-data paths.

use crate::intent::{BracketIntent, StopSpec, TargetSpec};
use crate::side::Side;

use super::engine_tests::{advance_with, approx, candle, open_position, step, test_engine};
use super::{
    FillReason, OrderIntent, OrderRecord, OrderState, PositionState, RejectReason, Trigger,
};

const UPPER: f64 = 105.0;
const LOWER: f64 = 95.0;

fn bracket(valid_for_bars: u32) -> OrderIntent {
    OrderIntent::EnterBracket {
        intent: BracketIntent {
            upper_price: UPPER,
            lower_price: LOWER,
            stop: StopSpec::AtrMultiple { multiple: 1.0 },
            target: TargetSpec::RewardToRisk { multiple: 4.0 },
            valid_for_bars,
            max_slippage_bps: None,
        },
        bar_index: 0,
        open_time: 0,
        max_notional: None,
    }
}

fn first_bar() -> quantrule_data::Candle {
    candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0))
}

fn rejections(batch: &super::ExecutionBatch) -> Vec<(u64, RejectReason)> {
    batch
        .rejections
        .iter()
        .map(|rejection| (rejection.order_id, rejection.reason))
        .collect()
}

fn last_records(batch: &super::ExecutionBatch) -> Vec<(u64, OrderState, Option<RejectReason>)> {
    batch
        .orders
        .iter()
        .map(|record| (record.order_id, record.state, record.rejection_reason))
        .collect()
}

#[test]
fn a_long_stop_entry_fills_at_the_upper_boundary_and_cancels_the_short() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 104.8, 106.0, 104.5, 105.5, Some(10.0));
    let submitted = step(&mut engine, &first, Some(1.0), bracket(1));
    assert_eq!(submitted.orders.len(), 2, "two pending legs");
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    approx(batch.fills[0].reference_price, UPPER);
    let position = open_position(&engine);
    assert_eq!(position.side, Side::Long);
    approx(position.stop_price, batch.fills[0].price - 1.0);
    approx(position.target_price, batch.fills[0].price + 4.0);
    assert_eq!(
        rejections(&batch),
        vec![(1, RejectReason::OcoCancelled)],
        "the lower leg is cancelled"
    );
    assert_eq!(
        last_records(&batch),
        vec![
            (1, OrderState::Rejected, Some(RejectReason::OcoCancelled)),
            (0, OrderState::Filled, None),
        ]
    );
}

#[test]
fn a_short_stop_entry_fills_at_the_lower_boundary_and_cancels_the_long() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 95.2, 95.5, 94.0, 95.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 1);
    approx(batch.fills[0].reference_price, LOWER);
    let position = open_position(&engine);
    assert_eq!(position.side, Side::Short);
    approx(position.stop_price, batch.fills[0].price + 1.0);
    approx(position.target_price, batch.fills[0].price - 4.0);
    assert_eq!(rejections(&batch), vec![(0, RejectReason::OcoCancelled)]);
}

#[test]
fn a_gap_through_the_upper_boundary_fills_at_the_open() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 106.0, 107.0, 105.5, 106.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 1);
    approx(batch.fills[0].reference_price, 106.0);
    assert_eq!(open_position(&engine).side, Side::Long);
}

#[test]
fn a_gap_through_the_lower_boundary_fills_at_the_open() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 94.0, 94.5, 93.0, 94.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 1);
    approx(batch.fills[0].reference_price, 94.0);
    assert_eq!(open_position(&engine).side, Side::Short);
}

#[test]
fn both_boundaries_touched_without_a_gap_reject_both_legs() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 100.0, 106.0, 94.0, 100.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert!(batch.fills.is_empty());
    assert_eq!(
        rejections(&batch),
        vec![
            (0, RejectReason::AmbiguousTrigger),
            (1, RejectReason::AmbiguousTrigger),
        ]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn the_bracket_expires_after_its_window() {
    let mut engine = test_engine();
    let first = first_bar();
    let quiet = candle(1, 900, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let breakout = candle(2, 1_800, 100.0, 106.0, 99.0, 105.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let expired = step(&mut engine, &quiet, Some(1.0), OrderIntent::Hold);
    assert_eq!(
        rejections(&expired),
        vec![
            (0, RejectReason::BracketExpired),
            (1, RejectReason::BracketExpired),
        ]
    );
    let after = step(&mut engine, &breakout, Some(1.0), OrderIntent::Hold);
    assert!(after.fills.is_empty());
    assert_eq!(engine.position(), PositionState::Flat);
}

#[test]
fn a_multi_bar_bracket_waits_for_its_window() {
    let mut engine = test_engine();
    let first = first_bar();
    let quiet = candle(1, 900, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let breakout = candle(2, 1_800, 104.8, 106.0, 104.5, 105.5, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(2));
    let waiting = step(&mut engine, &quiet, Some(1.0), OrderIntent::Hold);
    assert!(waiting.rejections.is_empty());
    let batch = step(&mut engine, &breakout, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(open_position(&engine).side, Side::Long);
}

#[test]
fn a_zero_volume_bar_rejects_both_legs() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 100.0, 106.0, 94.0, 100.0, Some(0.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert!(batch.fills.is_empty());
    assert_eq!(
        rejections(&batch),
        vec![(0, RejectReason::ZeroVolume), (1, RejectReason::ZeroVolume),]
    );
}

#[test]
fn a_non_contiguous_bar_rejects_both_legs() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 100.0, 106.0, 94.0, 100.0, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = advance_with(&mut engine, &second, false);

    assert!(batch.fills.is_empty());
    assert_eq!(
        rejections(&batch),
        vec![
            (0, RejectReason::NonContiguousBar),
            (1, RejectReason::NonContiguousBar),
        ]
    );
}

#[test]
fn finish_rejects_a_pending_bracket() {
    let mut engine = test_engine();
    let first = first_bar();
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = engine.finish();

    assert_eq!(
        rejections(&batch),
        vec![(0, RejectReason::NoNextBar), (1, RejectReason::NoNextBar)]
    );
}

#[test]
fn an_invalid_bracket_is_rejected_before_an_order_exists() {
    let mut engine = test_engine();
    let first = first_bar();
    let mut intent = bracket(1);
    let OrderIntent::EnterBracket {
        intent: ref mut bracket_intent,
        ..
    } = intent
    else {
        panic!("a bracket")
    };
    bracket_intent.upper_price = 95.0;
    bracket_intent.lower_price = 105.0;
    let batch = step(&mut engine, &first, Some(1.0), intent);

    assert_eq!(
        rejections(&batch),
        vec![
            (0, RejectReason::InvalidBracket),
            (1, RejectReason::InvalidBracket),
        ]
    );
    assert_eq!(batch.orders.len(), 2);
    assert!(
        batch
            .orders
            .iter()
            .all(|record| record.state == OrderState::Rejected)
    );
}

#[test]
fn a_missing_atr_rejects_the_filled_leg() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 100.0, 106.0, 99.0, 105.0, Some(10.0));
    step(&mut engine, &first, None, bracket(1));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);

    assert!(batch.fills.is_empty());
    assert_eq!(
        rejections(&batch),
        vec![
            (1, RejectReason::OcoCancelled),
            (0, RejectReason::MissingAtr),
        ]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn a_max_notional_caps_the_filled_quantity() {
    let mut engine = test_engine();
    let first = first_bar();
    let second = candle(1, 900, 104.8, 106.0, 104.5, 105.5, Some(10.0));
    let mut intent = bracket(1);
    let OrderIntent::EnterBracket {
        ref mut max_notional,
        ..
    } = intent
    else {
        panic!("a bracket")
    };
    *max_notional = Some(50.0);
    // The engine submits one quantity for both legs; the cap is applied to the
    // leg that fills.
    step(&mut engine, &first, Some(10.0), intent);
    let batch = step(&mut engine, &second, Some(10.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert!(fill.notional() <= 50.0 + 1e-9);
    approx(open_position(&engine).quantity, fill.quantity);
}

#[test]
fn the_position_bracket_is_live_on_the_entry_bar() {
    let mut engine = test_engine();
    let first = first_bar();
    // The long entry triggers at 105; the same bar then trades through the
    // 1 ATR stop at 104.
    let second = candle(1, 900, 104.5, 106.0, 103.9, 104.5, Some(10.0));
    step(&mut engine, &first, Some(1.0), bracket(1));
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    assert_eq!(batch.fills.len(), 2);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);
    assert_eq!(batch.fills[1].reason, FillReason::StopLoss);
    assert_eq!(engine.position(), PositionState::Flat);
}

#[test]
fn the_bracket_path_is_deterministic() {
    fn run() -> (Vec<OrderRecord>, Vec<super::Fill>, Vec<super::Rejection>) {
        let mut engine = test_engine();
        let first = first_bar();
        let second = candle(1, 900, 100.0, 106.0, 99.0, 105.0, Some(10.0));
        let mut orders = Vec::new();
        let mut fills = Vec::new();
        let mut rejections = Vec::new();
        for (bar, intent) in [(&first, bracket(1)), (&second, OrderIntent::Hold)] {
            let batch = step(&mut engine, bar, Some(1.0), intent);
            orders.extend(batch.orders);
            fills.extend(batch.fills);
            rejections.extend(batch.rejections);
        }
        (orders, fills, rejections)
    }

    assert_eq!(run(), run());
}

#[test]
fn a_stop_entry_trigger_is_a_stop_trigger_on_the_pending_record() {
    let mut engine = test_engine();
    let first = first_bar();
    let batch = step(&mut engine, &first, Some(1.0), bracket(1));
    assert_eq!(batch.orders.len(), 2);
    assert_eq!(batch.orders[0].trigger, Trigger::Stop { price: UPPER });
    assert_eq!(batch.orders[1].trigger, Trigger::Stop { price: LOWER });
    assert_eq!(batch.orders[0].reason, FillReason::Entry);
    assert_eq!(batch.orders[1].reason, FillReason::Entry);
}
