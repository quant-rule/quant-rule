//! Risk engine tests.

use super::*;

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
fn a_risk_capped_fill_respects_the_quantity_step() {
    let mut engine = test_engine();
    let (first, second) = entry_series();
    assert!(advance(&mut engine, &first).is_empty());
    engine
        .submit_with_step(
            enter_limited(0, 0, Side::Long, 50.0),
            Some(QUANTITY),
            Some(1.0),
            Some(0.1),
        )
        .expect("the entry is valid");

    let batch = advance(&mut engine, &second);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    assert!(fill.price * fill.quantity <= 50.0 + 1e-9);
    let steps = fill.quantity / 0.1;
    assert!((steps - steps.round()).abs() < 1e-9);
    assert_eq!(batch.orders[0].quantity, Some(fill.quantity));
}

#[test]
fn a_risk_cap_fits_the_exact_step_boundary() {
    let mut engine =
        ExecutionEngine::new(ExecutionConfig::new(0.0, 0.0).expect("the zero costs are valid"))
            .expect("the engine is valid");
    let first = candle(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0));
    let second = candle(1, 900, 1.0, 1.0, 1.0, 1.0, Some(10.0));
    assert!(advance(&mut engine, &first).is_empty());
    engine
        .submit_with_step(
            enter_limited(0, 0, Side::Long, 0.7),
            Some(QUANTITY),
            Some(0.1),
            Some(0.1),
        )
        .expect("the entry is valid");

    let batch = advance(&mut engine, &second);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    approx(fill.price, 1.0);
    assert!(
        (fill.quantity - 0.7).abs() < 1e-12,
        "the cap must fit seven steps, got {}",
        fill.quantity
    );
}

#[test]
fn a_risk_cap_fits_one_step_when_the_price_is_seven_times_the_cap() {
    let mut engine =
        ExecutionEngine::new(ExecutionConfig::new(0.0, 0.0).expect("the zero costs are valid"))
            .expect("the engine is valid");
    let first = candle(0, 0, 7.0, 7.0, 7.0, 7.0, Some(10.0));
    let second = candle(1, 900, 7.0, 7.0, 7.0, 7.0, Some(10.0));
    assert!(advance(&mut engine, &first).is_empty());
    engine
        .submit_with_step(
            enter_limited(0, 0, Side::Long, 0.7),
            Some(QUANTITY),
            Some(0.1),
            Some(0.1),
        )
        .expect("the entry is valid");

    let batch = advance(&mut engine, &second);
    assert_eq!(batch.fills.len(), 1);
    let fill = batch.fills[0];
    approx(fill.price, 7.0);
    assert!(
        (fill.quantity - 0.1).abs() < 1e-12,
        "the cap must fit one step, got {}",
        fill.quantity
    );
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
