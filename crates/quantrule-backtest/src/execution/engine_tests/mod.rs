//! Unit tests for the causal fill engine.

use quantrule_data::Candle;

use crate::intent::{RiskPolicy, StopSpec, TargetSpec, TradeIntent};
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

#[cfg(test)]
mod bracket;
#[cfg(test)]
mod determinism;
#[cfg(test)]
mod entry;
#[cfg(test)]
mod risk;
