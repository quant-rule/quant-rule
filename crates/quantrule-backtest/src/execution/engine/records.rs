//! Execution records and bracket arithmetic.

use super::*;

impl ExecutionEngine {
    /// Records a decision refused before an order was accepted.
    pub(super) fn reject_decision(
        &mut self,
        batch: &mut ExecutionBatch,
        decision: RejectedDecision,
    ) {
        let order_id = self.next_order_id();
        batch.orders.push(OrderRecord {
            order_id,
            submitted_bar: decision.submitted_bar,
            submitted_time: decision.submitted_time,
            side: decision.side,
            quantity: decision.quantity,
            reason: decision.reason,
            trigger: Trigger::Market,
            state: OrderState::Rejected,
            filled_bar: None,
            filled_time: None,
            fill_id: None,
            rejection_reason: Some(decision.reject_reason),
        });
        batch.rejections.push(Rejection {
            order_id,
            bar: decision.submitted_bar,
            time: decision.submitted_time,
            reason: decision.reject_reason,
        });
    }

    /// Records a pending order refused on a later bar.
    pub(super) fn reject_pending(
        &mut self,
        batch: &mut ExecutionBatch,
        pending: PendingOrder,
        bar: usize,
        time: i64,
        reason: RejectReason,
    ) {
        batch.orders.push(OrderRecord {
            order_id: pending.order_id,
            submitted_bar: pending.order.submitted_bar,
            submitted_time: pending.order.submitted_time,
            side: pending.order.side,
            quantity: Some(pending.order.quantity),
            reason: pending.order.reason,
            trigger: pending.order.trigger,
            state: OrderState::Rejected,
            filled_bar: None,
            filled_time: None,
            fill_id: None,
            rejection_reason: Some(reason),
        });
        batch.rejections.push(Rejection {
            order_id: pending.order_id,
            bar,
            time,
            reason,
        });
    }
}

pub(super) fn pending_record(order_id: u64, order: &Order) -> OrderRecord {
    OrderRecord {
        order_id,
        submitted_bar: order.submitted_bar,
        submitted_time: order.submitted_time,
        side: order.side,
        quantity: Some(order.quantity),
        reason: order.reason,
        trigger: order.trigger,
        state: OrderState::Pending,
        filled_bar: None,
        filled_time: None,
        fill_id: None,
        rejection_reason: None,
    }
}

pub(super) fn filled_record(
    order_id: u64,
    order: &Order,
    quantity: f64,
    bar: usize,
    time: i64,
    fill_id: u64,
) -> OrderRecord {
    OrderRecord {
        order_id,
        submitted_bar: order.submitted_bar,
        submitted_time: order.submitted_time,
        side: order.side,
        quantity: Some(quantity),
        reason: order.reason,
        trigger: order.trigger,
        state: OrderState::Filled,
        filled_bar: Some(bar),
        filled_time: Some(time),
        fill_id: Some(fill_id),
        rejection_reason: None,
    }
}

pub(super) fn is_tradable(bar: &Candle) -> bool {
    // An absent volume is not zero: the source did not supply it, so the bar
    // is treated as tradable rather than as an empty one.
    bar.volume() != Some(0.0)
}

/// The quantity an entry may be submitted with.
///
/// A missing quantity and an unusable one are distinct rejections: the first
/// says sizing produced nothing, the second says it produced a value the engine
/// cannot trade.
pub(super) fn usable_quantity(quantity: Option<f64>) -> Result<f64, RejectReason> {
    match quantity {
        None => Err(RejectReason::MissingQuantity),
        Some(value) if value.is_finite() && value > 0.0 => Ok(value),
        Some(_) => Err(RejectReason::NonPositiveQuantity),
    }
}

pub(super) fn long_stop_hit(bar: &Candle, stop: f64) -> bool {
    bar.open() <= stop || bar.low() <= stop
}

pub(super) fn long_stop_reference(bar: &Candle, stop: f64) -> f64 {
    if bar.open() <= stop { bar.open() } else { stop }
}

pub(super) fn long_target_hit(bar: &Candle, target: f64) -> bool {
    bar.open() >= target || bar.high() >= target
}

pub(super) fn long_target_reference(bar: &Candle, target: f64) -> f64 {
    if bar.open() >= target {
        bar.open()
    } else {
        target
    }
}

pub(super) fn short_stop_hit(bar: &Candle, stop: f64) -> bool {
    bar.open() >= stop || bar.high() >= stop
}

pub(super) fn short_stop_reference(bar: &Candle, stop: f64) -> f64 {
    if bar.open() >= stop { bar.open() } else { stop }
}

pub(super) fn short_target_hit(bar: &Candle, target: f64) -> bool {
    bar.open() <= target || bar.low() <= target
}

pub(super) fn short_target_reference(bar: &Candle, target: f64) -> f64 {
    if bar.open() <= target {
        bar.open()
    } else {
        target
    }
}

pub(super) fn order_side(side: Side) -> OrderSide {
    match side {
        Side::Long => OrderSide::Buy,
        Side::Short => OrderSide::Sell,
    }
}
