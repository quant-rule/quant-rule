//! Entry submission and pending-entry handling.

use super::records::*;
use super::*;

impl ExecutionEngine {
    /// The execution-level rejection reason for an entry request, if any.
    ///
    /// These checks are independent of the fill price, so they are applied when
    /// the order is submitted. A bracket that is invalid only around the actual
    /// fill is checked when the order fills.
    pub(super) fn entry_reject_reason(
        &self,
        intent: &TradeIntent,
        bar_index: usize,
    ) -> Option<RejectReason> {
        if !intent.entry.is_market_next_open() {
            return Some(RejectReason::UnsupportedEntryStyle);
        }
        if let Some(valid_until) = intent.valid_until
            && bar_index > valid_until
        {
            return Some(RejectReason::ExpiredIntent);
        }
        if let Some(max_slippage_bps) = intent.max_slippage_bps
            && (!max_slippage_bps.is_finite()
                || max_slippage_bps < 0.0
                || self.config.slippage_bps() > max_slippage_bps)
        {
            return Some(RejectReason::SlippageLimit);
        }
        if !intent.stop.is_valid() || !intent.target.is_valid() {
            return Some(RejectReason::InvalidBracket);
        }
        None
    }

    pub(super) fn validate_intent(
        &self,
        intent: OrderIntent,
        index: usize,
        time: i64,
    ) -> Result<(), ExecutionError> {
        let (bar_index, open_time) = match intent {
            OrderIntent::Hold => return Ok(()),
            OrderIntent::Enter {
                bar_index,
                open_time,
                ..
            }
            | OrderIntent::EnterBracket {
                bar_index,
                open_time,
                ..
            }
            | OrderIntent::Exit {
                bar_index,
                open_time,
                ..
            } => (bar_index, open_time),
        };
        if bar_index != index {
            return Err(ExecutionError::DecisionBarMismatch {
                expected: index,
                found: bar_index,
            });
        }
        if open_time != time {
            return Err(ExecutionError::DecisionTimeMismatch {
                expected: time,
                found: open_time,
            });
        }
        Ok(())
    }

    pub(super) fn apply_pending(
        &mut self,
        pending: PendingOrder,
        bar: &Candle,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        if !is_tradable(bar) {
            self.reject_pending(batch, pending, index, time, RejectReason::ZeroVolume);
            return Ok(());
        }

        match pending.order.reason {
            FillReason::Entry => {
                if self.position.is_open() {
                    self.reject_pending(
                        batch,
                        pending,
                        index,
                        time,
                        RejectReason::PositionAlreadyOpen,
                    );
                } else {
                    self.enter(pending, bar, index, time, batch)?;
                }
            }
            FillReason::Exit => {
                let PositionState::Open(position) = self.position else {
                    self.reject_pending(batch, pending, index, time, RejectReason::NoPosition);
                    return Ok(());
                };
                let fill = self.fill(
                    pending.order.side,
                    position.quantity,
                    bar.open(),
                    index,
                    time,
                    FillReason::Exit,
                )?;
                let fill_id = self.next_fill_id();
                self.position = PositionState::Flat;
                batch.orders.push(filled_record(
                    pending.order_id,
                    &pending.order,
                    fill.quantity,
                    index,
                    time,
                    fill_id,
                ));
                batch.fills.push(fill);
            }
            // A pending order is always a market entry or exit. The bracket is
            // carried by the position, not by the pending slot.
            FillReason::StopLoss | FillReason::TakeProfit => {}
        }
        Ok(())
    }

    pub(super) fn enter(
        &mut self,
        pending: PendingOrder,
        bar: &Candle,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        let Some(intent) = pending.intent else {
            // A pending entry always carries its intent. If that invariant is
            // ever broken, reject the order rather than inventing a bracket.
            self.reject_pending(batch, pending, index, time, RejectReason::InvalidBracket);
            return Ok(());
        };

        let mut fill = self.fill(
            pending.order.side,
            pending.order.quantity,
            bar.open(),
            index,
            time,
            FillReason::Entry,
        )?;
        if let Some(max_notional) = pending.max_notional {
            if !max_notional.is_finite() || max_notional <= 0.0 {
                self.reject_pending(batch, pending, index, time, RejectReason::RiskLimit);
                return Ok(());
            }
            let notional = fill.price * fill.quantity;
            if notional > max_notional {
                let mut capped = max_notional / fill.price;
                if let Some(step) = pending.quantity_step {
                    if !step.is_finite() || step <= 0.0 {
                        self.reject_pending(batch, pending, index, time, RejectReason::RiskLimit);
                        return Ok(());
                    }
                    let steps = floored_steps(capped, step);
                    if steps < 1.0 {
                        self.reject_pending(batch, pending, index, time, RejectReason::RiskLimit);
                        return Ok(());
                    }
                    capped = steps * step;
                }
                if !capped.is_finite() || capped <= 0.0 {
                    self.reject_pending(batch, pending, index, time, RejectReason::RiskLimit);
                    return Ok(());
                }
                fill = self.fill(
                    pending.order.side,
                    capped,
                    bar.open(),
                    index,
                    time,
                    FillReason::Entry,
                )?;
            }
        }

        let side = intent.side;
        let (stop_price, target_price) =
            match bracket::resolve(side, fill.price, pending.atr14, intent.stop, intent.target) {
                Ok(bracket) => bracket,
                Err(reason) => {
                    self.reject_pending(batch, pending, index, time, reason);
                    return Ok(());
                }
            };
        if !bracket::is_valid(side, fill.price, stop_price, target_price) {
            self.reject_pending(batch, pending, index, time, RejectReason::InvalidBracket);
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
        .map_err(ExecutionError::Position)?
        .with_entry_atr(pending.atr14);
        let fill_id = self.next_fill_id();
        self.position = PositionState::Open(position);
        batch.orders.push(filled_record(
            pending.order_id,
            &pending.order,
            fill.quantity,
            index,
            time,
            fill_id,
        ));
        batch.fills.push(fill);
        Ok(())
    }
}
