//! Tests for the trade-intent boundary in the execution engine.
//!
//! The engine resolves the strategy's stop and target specifications, rejects
//! unsupported or expired intents, and reports the position as the strategy
//! sees it. These tests share the engine fixtures and helpers with
//! `engine_tests`.

use crate::intent::{EntryStyle, RiskPolicy, StopSpec, TargetSpec};
use crate::side::Side;

use super::engine_tests::{
    SLIPPAGE_BPS, advance, approx, candle, enter, enter_with, entry_series, open_position, step,
    test_engine,
};
use super::{FillReason, OrderIntent, RejectReason, Rejection};

#[test]
fn an_absolute_stop_and_target_need_no_atr() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.stop = StopSpec::Price { price: 98.0 };
    intent.target = TargetSpec::Price { price: 104.0 };

    step(&mut engine, &first, None, enter_with(0, 0, intent, None));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);
    assert_eq!(batch.fills[0].reason, FillReason::Entry);

    let position = open_position(&engine);
    approx(position.stop_price, 98.0);
    approx(position.target_price, 104.0);
}

#[test]
fn an_absolute_stop_and_reward_target_need_no_atr() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.stop = StopSpec::Price { price: 98.0 };
    intent.target = TargetSpec::RewardToRisk { multiple: 2.0 };

    step(&mut engine, &first, None, enter_with(0, 0, intent, None));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);

    let position = open_position(&engine);
    approx(position.stop_price, 98.0);
    approx(position.target_price, 104.06);
}

#[test]
fn an_absolute_target_with_an_atr_stop_uses_the_decision_atr() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.stop = StopSpec::AtrMultiple { multiple: 2.0 };
    intent.target = TargetSpec::Price { price: 104.0 };

    step(
        &mut engine,
        &first,
        Some(1.0),
        enter_with(0, 0, intent, None),
    );
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);

    let position = open_position(&engine);
    approx(position.stop_price, 98.02);
    approx(position.target_price, 104.0);
}

#[test]
fn an_invalid_absolute_bracket_is_rejected() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.stop = StopSpec::Price { price: 101.0 };
    intent.target = TargetSpec::Price { price: 104.0 };

    step(&mut engine, &first, None, enter_with(0, 0, intent, None));
    let batch = step(&mut engine, &second, None, OrderIntent::Hold);
    assert!(batch.fills.is_empty());
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 1,
            time: 900,
            reason: RejectReason::InvalidBracket,
        }]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn an_unsupported_entry_style_is_rejected() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let second = candle(1, 900, 100.0, 101.0, 99.5, 100.5, Some(10.0));
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.entry = EntryStyle::Limit {
        price: 99.0,
        valid_for_bars: 3,
    };

    let batch = step(
        &mut engine,
        &first,
        Some(1.0),
        enter_with(0, 0, intent, None),
    );
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 0,
            time: 0,
            reason: RejectReason::UnsupportedEntryStyle,
        }]
    );
    assert!(advance(&mut engine, &second).fills.is_empty());
    assert!(engine.position().is_flat());
}

#[test]
fn an_expired_intent_is_rejected() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.valid_until = Some(0);

    advance(&mut engine, &first);
    let batch = step(
        &mut engine,
        &second,
        Some(1.0),
        enter_with(1, 900, intent, None),
    );
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 1,
            time: 900,
            reason: RejectReason::ExpiredIntent,
        }]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn an_intent_slippage_limit_below_execution_slippage_is_rejected() {
    let mut engine = test_engine();
    let first = candle(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0));
    let mut intent = RiskPolicy::default().intent(Side::Long);
    intent.max_slippage_bps = Some(SLIPPAGE_BPS - 1.0);

    let batch = step(
        &mut engine,
        &first,
        Some(1.0),
        enter_with(0, 0, intent, None),
    );
    assert_eq!(
        batch.rejections,
        &[Rejection {
            order_id: 0,
            bar: 0,
            time: 0,
            reason: RejectReason::SlippageLimit,
        }]
    );
    assert!(engine.position().is_flat());
}

#[test]
fn the_intent_carries_the_bracket_not_the_execution_config() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    let mut intent = RiskPolicy::default().intent(Side::Short);
    intent.stop = StopSpec::AtrMultiple { multiple: 4.0 };
    intent.target = TargetSpec::RewardToRisk { multiple: 2.0 };

    step(
        &mut engine,
        &first,
        Some(1.0),
        enter_with(0, 0, intent, None),
    );
    let batch = step(&mut engine, &second, Some(1.0), OrderIntent::Hold);
    assert_eq!(batch.fills.len(), 1);

    let position = open_position(&engine);
    assert_eq!(position.side, Side::Short);
    approx(position.stop_price, 103.98);
    approx(position.target_price, 91.98);
}

#[test]
fn position_view_reports_the_entry_bar_and_bars_held() {
    let mut engine = test_engine();
    let (first, second) = entry_series();

    step(&mut engine, &first, Some(1.0), enter(0, 0, Side::Long));
    step(&mut engine, &second, Some(1.0), OrderIntent::Hold);

    let view = engine.position_view();
    assert_eq!(view.side, Some(Side::Long));
    assert_eq!(view.entry_bar, Some(1));
    assert_eq!(view.bars_held, 0);

    let third = candle(2, 1800, 100.5, 101.0, 99.5, 100.8, Some(10.0));
    step(&mut engine, &third, Some(1.0), OrderIntent::Hold);
    let view = engine.position_view();
    assert_eq!(view.side, Some(Side::Long));
    assert_eq!(view.entry_bar, Some(1));
    assert_eq!(view.bars_held, 1);

    let fourth = candle(3, 2700, 100.8, 101.0, 100.0, 100.5, Some(10.0));
    step(&mut engine, &fourth, Some(1.0), OrderIntent::Hold);
    let view = engine.position_view();
    assert_eq!(view.bars_held, 2);
}
