//! Two-sided stop-entry execution.
//!
//! The engine's bracket-entry path lives here so the market-order engine and
//! the two-sided entry path stay separate. The child module can read the
//! engine's private state; callers still see only `ExecutionEngine`'s public
//! methods.

use quantrule_data::Candle;

use crate::intent::BracketIntent;
use crate::portfolio::floored_steps;
use crate::side::Side;

use super::{
    ExecutionBatch, ExecutionError, FillReason, Order, OrderRecord, OrderSide, OrderState,
    PendingBracket, Position, PositionState, RejectReason, Rejection, Trigger, bracket,
    filled_record, is_tradable, order_side,
};

pub(super) fn bracket_entry_reject_reason(
    engine: &super::ExecutionEngine,
    intent: &BracketIntent,
) -> Option<RejectReason> {
    if !intent.is_valid() {
        return Some(RejectReason::InvalidBracket);
    }
    if let Some(max_slippage_bps) = intent.max_slippage_bps
        && (!max_slippage_bps.is_finite()
            || max_slippage_bps < 0.0
            || engine.config.slippage_bps() > max_slippage_bps)
    {
        return Some(RejectReason::SlippageLimit);
    }
    None
}

impl super::ExecutionEngine {
    /// The execution-level rejection reason for a two-sided entry, if any.
    pub(super) fn bracket_entry_reject_reason(
        &self,
        intent: &BracketIntent,
    ) -> Option<RejectReason> {
        bracket_entry_reject_reason(self, intent)
    }

    pub(super) fn apply_bracket_entry(
        &mut self,
        mut bracket: PendingBracket,
        bar: &Candle,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        if !is_tradable(bar) {
            self.reject_bracket(batch, bracket, index, time, RejectReason::ZeroVolume);
            return Ok(());
        }

        let upper = bracket.intent.upper_price;
        let lower = bracket.intent.lower_price;
        let open = bar.open();
        let (
            side,
            reference,
            cancelled_side,
            cancelled_trigger,
            filled_order_id,
            cancelled_order_id,
        ) = if open >= upper {
            (
                Side::Long,
                open,
                OrderSide::Sell,
                Trigger::Stop { price: lower },
                bracket.upper_order_id,
                bracket.lower_order_id,
            )
        } else if open <= lower {
            (
                Side::Short,
                open,
                OrderSide::Buy,
                Trigger::Stop { price: upper },
                bracket.lower_order_id,
                bracket.upper_order_id,
            )
        } else {
            let long_hit = bar.high() >= upper;
            let short_hit = bar.low() <= lower;
            match (long_hit, short_hit) {
                (true, true) => {
                    // OHLC cannot order two intrabar triggers from an open
                    // between them. Treating one as first would invent a
                    // path, so the bar is skipped rather than guessed.
                    self.reject_bracket(
                        batch,
                        bracket,
                        index,
                        time,
                        RejectReason::AmbiguousTrigger,
                    );
                    return Ok(());
                }
                (true, false) => (
                    Side::Long,
                    upper,
                    OrderSide::Sell,
                    Trigger::Stop { price: lower },
                    bracket.upper_order_id,
                    bracket.lower_order_id,
                ),
                (false, true) => (
                    Side::Short,
                    lower,
                    OrderSide::Buy,
                    Trigger::Stop { price: upper },
                    bracket.lower_order_id,
                    bracket.upper_order_id,
                ),
                (false, false) => {
                    bracket.remaining_bars -= 1;
                    if bracket.remaining_bars == 0 {
                        self.reject_bracket(
                            batch,
                            bracket,
                            index,
                            time,
                            RejectReason::BracketExpired,
                        );
                    } else {
                        self.pending_bracket = Some(bracket);
                    }
                    return Ok(());
                }
            }
        };

        self.reject_bracket_leg(
            batch,
            cancelled_order_id,
            cancelled_side,
            bracket.quantity,
            cancelled_trigger,
            bracket.submitted_bar,
            bracket.submitted_time,
            index,
            time,
            RejectReason::OcoCancelled,
        );
        self.fill_bracket_leg(
            bracket,
            side,
            reference,
            filled_order_id,
            index,
            time,
            batch,
        )
    }

    // A fill is a flat record; the arguments are its fields and adding a
    // builder would not enforce a new invariant.
    #[allow(clippy::too_many_arguments)]
    fn fill_bracket_leg(
        &mut self,
        bracket: PendingBracket,
        side: Side,
        reference: f64,
        order_id: u64,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        let order_side = order_side(side);
        let trigger_price = match side {
            Side::Long => bracket.intent.upper_price,
            Side::Short => bracket.intent.lower_price,
        };
        let order = Order::new(
            order_side,
            bracket.quantity,
            FillReason::Entry,
            Trigger::Stop {
                price: trigger_price,
            },
            bracket.submitted_bar,
            bracket.submitted_time,
        )
        .map_err(ExecutionError::Order)?;

        let mut fill = self.fill(
            order_side,
            bracket.quantity,
            reference,
            index,
            time,
            FillReason::Entry,
        )?;
        if let Some(max_notional) = bracket.max_notional {
            if !max_notional.is_finite() || max_notional <= 0.0 {
                self.reject_bracket_leg(
                    batch,
                    order_id,
                    order_side,
                    bracket.quantity,
                    order.trigger,
                    bracket.submitted_bar,
                    bracket.submitted_time,
                    index,
                    time,
                    RejectReason::RiskLimit,
                );
                return Ok(());
            }
            let notional = fill.price * fill.quantity;
            if notional > max_notional {
                let mut capped = max_notional / fill.price;
                if let Some(step) = bracket.quantity_step {
                    if !step.is_finite() || step <= 0.0 {
                        self.reject_bracket_leg(
                            batch,
                            order_id,
                            order_side,
                            bracket.quantity,
                            order.trigger,
                            bracket.submitted_bar,
                            bracket.submitted_time,
                            index,
                            time,
                            RejectReason::RiskLimit,
                        );
                        return Ok(());
                    }
                    let steps = floored_steps(capped, step);
                    if steps < 1.0 {
                        self.reject_bracket_leg(
                            batch,
                            order_id,
                            order_side,
                            bracket.quantity,
                            order.trigger,
                            bracket.submitted_bar,
                            bracket.submitted_time,
                            index,
                            time,
                            RejectReason::RiskLimit,
                        );
                        return Ok(());
                    }
                    capped = steps * step;
                }
                if !capped.is_finite() || capped <= 0.0 {
                    self.reject_bracket_leg(
                        batch,
                        order_id,
                        order_side,
                        bracket.quantity,
                        order.trigger,
                        bracket.submitted_bar,
                        bracket.submitted_time,
                        index,
                        time,
                        RejectReason::RiskLimit,
                    );
                    return Ok(());
                }
                fill = self.fill(
                    order_side,
                    capped,
                    reference,
                    index,
                    time,
                    FillReason::Entry,
                )?;
            }
        }

        let (stop_price, target_price) = match bracket::resolve(
            side,
            fill.price,
            bracket.atr14,
            bracket.intent.stop,
            bracket.intent.target,
        ) {
            Ok(bracket) => bracket,
            Err(reason) => {
                self.reject_bracket_leg(
                    batch,
                    order_id,
                    order.side,
                    order.quantity,
                    order.trigger,
                    bracket.submitted_bar,
                    bracket.submitted_time,
                    index,
                    time,
                    reason,
                );
                return Ok(());
            }
        };
        if !bracket::is_valid(side, fill.price, stop_price, target_price) {
            self.reject_bracket_leg(
                batch,
                order_id,
                order.side,
                order.quantity,
                order.trigger,
                bracket.submitted_bar,
                bracket.submitted_time,
                index,
                time,
                RejectReason::InvalidBracket,
            );
            return Ok(());
        }

        let position = Position::new(
            side,
            fill.quantity,
            fill.price,
            index,
            time,
            stop_price,
            target_price,
        )
        .map_err(ExecutionError::Position)?;
        let fill_id = self.next_fill_id();
        self.position = PositionState::Open(position);
        batch.orders.push(filled_record(
            order_id,
            &order,
            fill.quantity,
            index,
            time,
            fill_id,
        ));
        batch.fills.push(fill);
        Ok(())
    }

    pub(super) fn reject_bracket_decision(
        &mut self,
        batch: &mut ExecutionBatch,
        intent: &BracketIntent,
        quantity: Option<f64>,
        submitted_bar: usize,
        submitted_time: i64,
        reason: RejectReason,
    ) {
        for (side, price) in [
            (OrderSide::Buy, intent.upper_price),
            (OrderSide::Sell, intent.lower_price),
        ] {
            let order_id = self.next_order_id();
            batch.orders.push(OrderRecord {
                order_id,
                submitted_bar,
                submitted_time,
                side,
                quantity,
                reason: FillReason::Entry,
                trigger: Trigger::Stop { price },
                state: OrderState::Rejected,
                filled_bar: None,
                filled_time: None,
                fill_id: None,
                rejection_reason: Some(reason),
            });
            batch.rejections.push(Rejection {
                order_id,
                bar: submitted_bar,
                time: submitted_time,
                reason,
            });
        }
    }

    pub(super) fn reject_bracket(
        &mut self,
        batch: &mut ExecutionBatch,
        bracket: PendingBracket,
        bar: usize,
        time: i64,
        reason: RejectReason,
    ) {
        self.reject_bracket_leg(
            batch,
            bracket.upper_order_id,
            OrderSide::Buy,
            bracket.quantity,
            Trigger::Stop {
                price: bracket.intent.upper_price,
            },
            bracket.submitted_bar,
            bracket.submitted_time,
            bar,
            time,
            reason,
        );
        self.reject_bracket_leg(
            batch,
            bracket.lower_order_id,
            OrderSide::Sell,
            bracket.quantity,
            Trigger::Stop {
                price: bracket.intent.lower_price,
            },
            bracket.submitted_bar,
            bracket.submitted_time,
            bar,
            time,
            reason,
        );
    }

    // A bracket leg is a flat record; the arguments are its fields and adding
    // a builder would not enforce a new invariant.
    #[allow(clippy::too_many_arguments)]
    fn reject_bracket_leg(
        &mut self,
        batch: &mut ExecutionBatch,
        order_id: u64,
        side: OrderSide,
        quantity: f64,
        trigger: Trigger,
        submitted_bar: usize,
        submitted_time: i64,
        bar: usize,
        time: i64,
        reason: RejectReason,
    ) {
        batch.orders.push(OrderRecord {
            order_id,
            submitted_bar,
            submitted_time,
            side,
            quantity: Some(quantity),
            reason: FillReason::Entry,
            trigger,
            state: OrderState::Rejected,
            filled_bar: None,
            filled_time: None,
            fill_id: None,
            rejection_reason: Some(reason),
        });
        batch.rejections.push(Rejection {
            order_id,
            bar,
            time,
            reason,
        });
    }
}
