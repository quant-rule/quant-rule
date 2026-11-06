//! Bracket and fill handling.

use super::records::*;
use super::*;

impl ExecutionEngine {
    pub(super) fn apply_bracket(
        &mut self,
        bar: &Candle,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        let PositionState::Open(position) = self.position else {
            return Ok(());
        };
        if !is_tradable(bar) {
            return Ok(());
        }

        // On the entry bar the position did not exist at the open, so the
        // open cannot gap through a level. Evaluate the bar's range from the
        // fill instead; a stop tighter than the slippage otherwise fills at a
        // pre-entry price and silently worsens the exit.
        let (reference, reason, trigger_price, side) = if index == position.entry_bar {
            match position.side {
                Side::Long => {
                    if bar.low() <= position.stop_price {
                        (
                            position.stop_price,
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Sell,
                        )
                    } else if bar.high() >= position.target_price {
                        (
                            position.target_price,
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Sell,
                        )
                    } else {
                        return Ok(());
                    }
                }
                Side::Short => {
                    if bar.high() >= position.stop_price {
                        (
                            position.stop_price,
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Buy,
                        )
                    } else if bar.low() <= position.target_price {
                        (
                            position.target_price,
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Buy,
                        )
                    } else {
                        return Ok(());
                    }
                }
            }
        } else {
            // A gap at the open proves which level was reached first. Resolve
            // it before the intrabar stop-first tie rule; only an open between
            // both levels leaves the intrabar order unknowable from OHLC.
            match position.side {
                Side::Long => {
                    if bar.open() <= position.stop_price {
                        (
                            bar.open(),
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Sell,
                        )
                    } else if bar.open() >= position.target_price {
                        (
                            bar.open(),
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Sell,
                        )
                    } else if long_stop_hit(bar, position.stop_price) {
                        (
                            long_stop_reference(bar, position.stop_price),
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Sell,
                        )
                    } else if long_target_hit(bar, position.target_price) {
                        (
                            long_target_reference(bar, position.target_price),
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Sell,
                        )
                    } else {
                        return Ok(());
                    }
                }
                Side::Short => {
                    if bar.open() >= position.stop_price {
                        (
                            bar.open(),
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Buy,
                        )
                    } else if bar.open() <= position.target_price {
                        (
                            bar.open(),
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Buy,
                        )
                    } else if short_stop_hit(bar, position.stop_price) {
                        (
                            short_stop_reference(bar, position.stop_price),
                            FillReason::StopLoss,
                            position.stop_price,
                            OrderSide::Buy,
                        )
                    } else if short_target_hit(bar, position.target_price) {
                        (
                            short_target_reference(bar, position.target_price),
                            FillReason::TakeProfit,
                            position.target_price,
                            OrderSide::Buy,
                        )
                    } else {
                        return Ok(());
                    }
                }
            }
        };

        let trigger = match reason {
            FillReason::StopLoss => Trigger::Stop {
                price: trigger_price,
            },
            FillReason::TakeProfit => Trigger::Target {
                price: trigger_price,
            },
            FillReason::Entry | FillReason::Exit => {
                return Err(ExecutionError::UnexpectedFillReason);
            }
        };
        // The bracket became live when the entry filled, so that is its
        // submission. Only the level that triggers becomes an order.
        let order = Order::new(
            side,
            position.quantity,
            reason,
            trigger,
            position.entry_bar,
            position.entry_time,
        )
        .map_err(ExecutionError::Order)?;
        let fill = self.fill(order.side, order.quantity, reference, index, time, reason)?;
        let order_id = self.next_order_id();
        let fill_id = self.next_fill_id();
        self.position = PositionState::Flat;
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

    pub(super) fn fill(
        &self,
        side: OrderSide,
        quantity: f64,
        reference_price: f64,
        bar: usize,
        time: i64,
        reason: FillReason,
    ) -> Result<Fill, ExecutionError> {
        let slippage = reference_price * self.config.slippage_rate();
        let executed_price = match side {
            OrderSide::Buy => reference_price + slippage,
            OrderSide::Sell => reference_price - slippage,
        };
        let fee = executed_price * quantity * self.config.fee_rate();
        Fill::new(
            side,
            quantity,
            reference_price,
            fee,
            slippage,
            bar,
            time,
            reason,
        )
        .map_err(ExecutionError::Fill)
    }
}
