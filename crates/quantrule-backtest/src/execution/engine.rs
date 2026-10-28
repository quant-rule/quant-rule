//! Causal fill simulation over one candle series.
//!
//! The engine consumes one closed bar at a time. A decision made on bar `N`
//! schedules a market order that fills on bar `N + 1` at the open; the fill
//! opens or closes one position. While a position is open, its ATR bracket is
//! evaluated on every bar, including the entry bar, against the bar's high and
//! low. No later bar is read to decide an earlier fill.
//!
//! The engine owns no portfolio accounting: it records the fill's price, fee,
//! and slippage, but cash, equity, trade records, and P&L belong to the
//! portfolio module. Position sizing is outside this module too: the caller
//! supplies the quantity with each decision.
//!
//! # Bar protocol
//!
//! A bar is processed in two calls, in this order:
//!
//! 1. [`ExecutionEngine::advance`] fills the order scheduled by the previous
//!    bar at this bar's open and evaluates the open position's bracket.
//! 2. [`ExecutionEngine::submit`] schedules this bar's decision for the next
//!    bar.
//!
//! The split exists because the caller must apply the bar's fills and mark the
//! position to market before it can size a new entry from the same close. A
//! single call cannot do both: the bar's own fills are not known until it has
//! advanced.
//!
//! # Order lifecycle
//!
//! Every call returns an [`ExecutionBatch`] of order records, fills, and
//! rejections. Ids are assigned in creation order from zero and never repeat.
//! A decision that becomes an order emits a pending record at submission; the
//! same order id is emitted again when the order fills or is rejected, and a
//! report keeps the last record per id. A decision refused before an order
//! exists emits a rejected record with the same id as its rejection.
//!
//! A bracket order is recorded when its stop or target is reached, with the
//! entry fill's bar and time as its submission, because that is when the
//! bracket became live. The level that never triggers is not an order.
//!
//! # Input
//!
//! The engine consumes the strategy's candle series in order. It does not
//! inspect a timeframe: selecting the series and ignoring bars from another
//! timeframe belong to the caller, and the strategy already ignores other
//! timeframes. The decision must carry the same bar index and open time as the
//! bar it was submitted with.
//!
//! # Bracket
//!
//! An entry is bracketed with the decision bar's ATR(14), anchored at the entry
//! fill price:
//!
//! | Side | Stop | Target |
//! |---|---|---|
//! | Long | `fill - 2 * ATR` | `fill + 3 * ATR` |
//! | Short | `fill + 2 * ATR` | `fill - 3 * ATR` |
//!
//! A missing, zero, or non-finite ATR rejects the entry with
//! [`RejectReason::MissingAtr`]. A bracket that is not a valid positive
//! bracket around the fill price rejects it with
//! [`RejectReason::InvalidBracket`]. The bracket is live from the entry bar
//! onward, so one bar can fill the entry at its open and trigger the stop or
//! target later in the same bar. If both levels are touched, the stop wins.
//!
//! A bar with zero volume cannot fill a market order or trigger a bracket.
//! A zero-volume next bar rejects the pending market order with
//! [`RejectReason::ZeroVolume`]; a zero-volume bar while a position is open
//! leaves the bracket active for the next bar. An absent volume is not zero:
//! it is treated as tradable.
//!
//! # End of data
//!
//! [`ExecutionEngine::finish`] rejects an order that has no next bar with
//! [`RejectReason::NoNextBar`]. It does not close an open position: the first
//! backtest has no end-of-data exit.

use quantrule_data::Candle;

use crate::strategy::{Decision, Side};

use super::{
    ExecutionConfig, Fill, FillError, FillReason, Order, OrderError, OrderRecord, OrderSide,
    OrderState, Position, PositionError, PositionState, RejectReason, Trigger,
};

/// The default stop distance in ATR units.
pub use super::config::DEFAULT_STOP_ATR_MULTIPLE as STOP_ATR_MULTIPLE;

/// The default target distance in ATR units.
pub use super::config::DEFAULT_TARGET_ATR_MULTIPLE as TARGET_ATR_MULTIPLE;

/// A pending market order and the ATR of the bar that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingOrder {
    order_id: u64,
    order: Order,
    atr14: Option<f64>,
}

/// A decision refused before an order was accepted.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RejectedDecision {
    side: OrderSide,
    quantity: Option<f64>,
    reason: FillReason,
    submitted_bar: usize,
    submitted_time: i64,
    reject_reason: RejectReason,
}

/// One decision the engine could not execute.
///
/// A rejection is not a fill. The engine records it so a caller can see that a
/// strategy decision had no effect; the strategy itself is not rewritten. The
/// rejection shares its `order_id` with the order record it resolved.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{Rejection, RejectReason};
///
/// let rejection = Rejection {
///     order_id: 3,
///     bar: 3,
///     time: 1_700_002_700,
///     reason: RejectReason::ZeroVolume,
/// };
/// assert_eq!(rejection.reason, RejectReason::ZeroVolume);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rejection {
    /// The id of the order the decision became, or would have become.
    pub order_id: u64,
    /// The bar on which the engine refused the order. For
    /// [`RejectReason::NoNextBar`], this is the order's submission bar.
    pub bar: usize,
    /// That bar's open time as UTC Unix seconds.
    pub time: i64,
    /// Why the order was refused.
    pub reason: RejectReason,
}

/// The orders, fills, and rejections produced by one engine call.
///
/// [`ExecutionEngine::advance`], [`ExecutionEngine::submit`], and
/// [`ExecutionEngine::finish`] each return one batch. Concatenating the
/// batches in call order yields the run's records; a report keeps the last
/// order record per id.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::ExecutionBatch;
///
/// let batch = ExecutionBatch::default();
/// assert!(batch.is_empty());
/// assert!(batch.orders.is_empty());
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExecutionBatch {
    /// Order records created or resolved by this call.
    pub orders: Vec<OrderRecord>,
    /// Fills produced by this call, in fill-id order.
    pub fills: Vec<Fill>,
    /// Decisions refused by this call, in detection order.
    pub rejections: Vec<Rejection>,
}

impl ExecutionBatch {
    /// Whether the call produced no orders, fills, or rejections.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.orders.is_empty() && self.fills.is_empty() && self.rejections.is_empty()
    }
}

/// The causal fill simulator for one instrument and one position.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{ExecutionConfig, ExecutionEngine};
/// use quantrule_backtest::strategy::{Decision, Entry, Side, StrategyInput};
/// use quantrule_data::Candle;
///
/// let config = ExecutionConfig::new(0.001, 2.0)?;
/// let mut engine = ExecutionEngine::new(config)?;
/// let first = Candle::new(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0))?;
/// let second = Candle::new(1, 900, 100.0, 102.0, 99.5, 101.0, Some(10.0))?;
/// let bar = StrategyInput::new(900, 0, 0, Some(0.5), Some(0.5), Some(1.0));
///
/// assert!(engine.advance(&first)?.is_empty());
/// engine.submit(
///     Decision::Enter(Entry { side: Side::Long, bar }),
///     Some(1.0),
///     Some(1.0),
/// )?;
/// let batch = engine.advance(&second)?;
/// assert_eq!(batch.fills.len(), 1);
/// assert_eq!(batch.fills[0].reference_price, 100.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct ExecutionEngine {
    config: ExecutionConfig,
    next_bar: usize,
    last_bar: Option<(usize, i64)>,
    position: PositionState,
    pending: Option<PendingOrder>,
    next_order_id: u64,
    next_fill_id: u64,
}

impl ExecutionEngine {
    /// Builds an engine with no run-wide quantity.
    ///
    /// The quantity is supplied with each entry decision by
    /// [`Self::submit`].
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::SlippageAtOrAboveOne`] when the configured
    /// slippage is at least 100% of the reference price and would make a sell
    /// fill non-positive. The execution costs are already validated by
    /// [`ExecutionConfig`].
    pub fn new(config: ExecutionConfig) -> Result<Self, ExecutionError> {
        if config.slippage_rate() >= 1.0 {
            return Err(ExecutionError::SlippageAtOrAboveOne);
        }

        Ok(Self {
            config,
            next_bar: 0,
            last_bar: None,
            position: PositionState::Flat,
            pending: None,
            next_order_id: 0,
            next_fill_id: 0,
        })
    }

    /// Advances one closed bar: fills the pending order and evaluates the
    /// bracket.
    ///
    /// The engine first fills a market order scheduled by the previous bar at
    /// this bar's open, then evaluates an open position's bracket against this
    /// bar's range. It returns the bar's order records, fills, and rejections.
    /// The decision for this bar is scheduled afterwards by [`Self::submit`].
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] when a validated value cannot be
    /// constructed.
    pub fn advance(&mut self, bar: &Candle) -> Result<ExecutionBatch, ExecutionError> {
        let index = self.next_bar;
        let time = bar.timestamp();
        self.next_bar += 1;
        self.last_bar = Some((index, time));

        let mut batch = ExecutionBatch::default();

        if let Some(pending) = self.pending.take() {
            self.apply_pending(pending, bar, index, time, &mut batch)?;
        }

        if self.position.is_open() {
            self.apply_bracket(bar, index, time, &mut batch)?;
        }

        Ok(batch)
    }

    /// Schedules one decision for the next bar.
    ///
    /// `quantity` is the entry quantity. It is `None` when the caller could not
    /// size the entry; the decision is then rejected with
    /// [`RejectReason::MissingQuantity`]. An exit always closes the open
    /// position with the position's quantity, so its quantity is ignored.
    ///
    /// `atr14` is the decision bar's ATR(14), used to bracket the entry when it
    /// fills. The decision must carry the last advanced bar's index and open
    /// time; a mismatch is a caller error, not a rejection.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::SubmitBeforeAdvance`] when no bar has been
    /// advanced, [`ExecutionError`] when the decision does not describe the
    /// last advanced bar, or when a validated value cannot be constructed.
    pub fn submit(
        &mut self,
        decision: Decision,
        quantity: Option<f64>,
        atr14: Option<f64>,
    ) -> Result<ExecutionBatch, ExecutionError> {
        let Some((index, time)) = self.last_bar else {
            return Err(ExecutionError::SubmitBeforeAdvance);
        };
        self.validate_decision(decision, index, time)?;

        let mut batch = ExecutionBatch::default();
        match decision {
            Decision::Hold => {}
            Decision::Enter(entry) => {
                let side = order_side(entry.side);
                if self.position.is_open() {
                    self.reject_decision(
                        &mut batch,
                        RejectedDecision {
                            side,
                            quantity,
                            reason: FillReason::Entry,
                            submitted_bar: index,
                            submitted_time: time,
                            reject_reason: RejectReason::PositionAlreadyOpen,
                        },
                    );
                } else {
                    match usable_quantity(quantity) {
                        Err(reject_reason) => self.reject_decision(
                            &mut batch,
                            RejectedDecision {
                                side,
                                quantity,
                                reason: FillReason::Entry,
                                submitted_bar: index,
                                submitted_time: time,
                                reject_reason,
                            },
                        ),
                        Ok(quantity) => {
                            let order = Order::new(
                                side,
                                quantity,
                                FillReason::Entry,
                                Trigger::Market,
                                index,
                                time,
                            )
                            .map_err(ExecutionError::Order)?;
                            let order_id = self.next_order_id();
                            batch.orders.push(pending_record(order_id, &order));
                            self.pending = Some(PendingOrder {
                                order_id,
                                order,
                                atr14,
                            });
                        }
                    }
                }
            }
            Decision::Exit(exit) => match self.position {
                PositionState::Open(position) => {
                    // The engine closes the position it holds; the decision's
                    // side is the strategy's view of that position, not the
                    // order side.
                    let order = Order::new(
                        order_side(position.side).opposite(),
                        position.quantity,
                        FillReason::Exit,
                        Trigger::Market,
                        index,
                        time,
                    )
                    .map_err(ExecutionError::Order)?;
                    let order_id = self.next_order_id();
                    batch.orders.push(pending_record(order_id, &order));
                    self.pending = Some(PendingOrder {
                        order_id,
                        order,
                        atr14,
                    });
                }
                PositionState::Flat | PositionState::Closed => self.reject_decision(
                    &mut batch,
                    RejectedDecision {
                        side: order_side(exit.side).opposite(),
                        quantity: None,
                        reason: FillReason::Exit,
                        submitted_bar: index,
                        submitted_time: time,
                        reject_reason: RejectReason::NoPosition,
                    },
                ),
            },
        }
        Ok(batch)
    }

    /// Rejects a pending order that has no next bar.
    ///
    /// Call this after the last bar. An open position is left open: the first
    /// backtest has no end-of-data exit. Calling it twice is harmless.
    pub fn finish(&mut self) -> ExecutionBatch {
        let mut batch = ExecutionBatch::default();
        if let Some(pending) = self.pending.take() {
            let bar = pending.order.submitted_bar;
            let time = pending.order.submitted_time;
            self.reject_pending(&mut batch, pending, bar, time, RejectReason::NoNextBar);
        }
        batch
    }

    /// The current position state.
    #[must_use]
    pub const fn position(&self) -> PositionState {
        self.position
    }

    fn next_order_id(&mut self) -> u64 {
        let order_id = self.next_order_id;
        self.next_order_id += 1;
        order_id
    }

    fn next_fill_id(&mut self) -> u64 {
        let fill_id = self.next_fill_id;
        self.next_fill_id += 1;
        fill_id
    }

    fn validate_decision(
        &self,
        decision: Decision,
        index: usize,
        time: i64,
    ) -> Result<(), ExecutionError> {
        let bar = match decision {
            Decision::Hold => return Ok(()),
            Decision::Enter(entry) => entry.bar,
            Decision::Exit(exit) => exit.bar,
        };
        if bar.bar_index != index {
            return Err(ExecutionError::DecisionBarMismatch {
                expected: index,
                found: bar.bar_index,
            });
        }
        if bar.open_time != time {
            return Err(ExecutionError::DecisionTimeMismatch {
                expected: time,
                found: bar.open_time,
            });
        }
        Ok(())
    }

    fn apply_pending(
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

    fn enter(
        &mut self,
        pending: PendingOrder,
        bar: &Candle,
        index: usize,
        time: i64,
        batch: &mut ExecutionBatch,
    ) -> Result<(), ExecutionError> {
        let Some(atr14) = usable_atr(pending.atr14) else {
            self.reject_pending(batch, pending, index, time, RejectReason::MissingAtr);
            return Ok(());
        };

        let fill = self.fill(
            pending.order.side,
            pending.order.quantity,
            bar.open(),
            index,
            time,
            FillReason::Entry,
        )?;
        let side = match pending.order.side {
            OrderSide::Buy => Side::Long,
            OrderSide::Sell => Side::Short,
        };
        let (stop_price, target_price) = bracket(
            side,
            fill.price,
            atr14,
            self.config.stop_atr_multiple(),
            self.config.target_atr_multiple(),
        );
        if !is_valid_bracket(side, fill.price, stop_price, target_price) {
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
        .map_err(ExecutionError::Position)?;
        let fill_id = self.next_fill_id();
        self.position = PositionState::Open(position);
        batch.orders.push(filled_record(
            pending.order_id,
            &pending.order,
            index,
            time,
            fill_id,
        ));
        batch.fills.push(fill);
        Ok(())
    }

    fn apply_bracket(
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

        let (reference, reason, trigger_price, side) = match position.side {
            Side::Long => {
                if long_stop_hit(bar, position.stop_price) {
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
                if short_stop_hit(bar, position.stop_price) {
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
        batch
            .orders
            .push(filled_record(order_id, &order, index, time, fill_id));
        batch.fills.push(fill);
        Ok(())
    }

    fn fill(
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

    /// Records a decision refused before an order was accepted.
    fn reject_decision(&mut self, batch: &mut ExecutionBatch, decision: RejectedDecision) {
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
    fn reject_pending(
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

fn pending_record(order_id: u64, order: &Order) -> OrderRecord {
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

fn filled_record(order_id: u64, order: &Order, bar: usize, time: i64, fill_id: u64) -> OrderRecord {
    OrderRecord {
        order_id,
        submitted_bar: order.submitted_bar,
        submitted_time: order.submitted_time,
        side: order.side,
        quantity: Some(order.quantity),
        reason: order.reason,
        trigger: order.trigger,
        state: OrderState::Filled,
        filled_bar: Some(bar),
        filled_time: Some(time),
        fill_id: Some(fill_id),
        rejection_reason: None,
    }
}

fn is_tradable(bar: &Candle) -> bool {
    // An absent volume is not zero: the source did not supply it, so the bar
    // is treated as tradable rather than as an empty one.
    bar.volume() != Some(0.0)
}

fn usable_atr(atr14: Option<f64>) -> Option<f64> {
    atr14.filter(|value| value.is_finite() && *value > 0.0)
}

/// The quantity an entry may be submitted with.
///
/// A missing quantity and an unusable one are distinct rejections: the first
/// says sizing produced nothing, the second says it produced a value the engine
/// cannot trade.
fn usable_quantity(quantity: Option<f64>) -> Result<f64, RejectReason> {
    match quantity {
        None => Err(RejectReason::MissingQuantity),
        Some(value) if value.is_finite() && value > 0.0 => Ok(value),
        Some(_) => Err(RejectReason::NonPositiveQuantity),
    }
}

fn bracket(
    side: Side,
    entry: f64,
    atr: f64,
    stop_multiple: f64,
    target_multiple: f64,
) -> (f64, f64) {
    match side {
        Side::Long => (entry - stop_multiple * atr, entry + target_multiple * atr),
        Side::Short => (entry + stop_multiple * atr, entry - target_multiple * atr),
    }
}

fn is_valid_bracket(side: Side, entry: f64, stop: f64, target: f64) -> bool {
    if !stop.is_finite() || stop <= 0.0 || !target.is_finite() || target <= 0.0 {
        return false;
    }
    match side {
        Side::Long => stop < entry && entry < target,
        Side::Short => target < entry && entry < stop,
    }
}

fn long_stop_hit(bar: &Candle, stop: f64) -> bool {
    bar.open() <= stop || bar.low() <= stop
}

fn long_stop_reference(bar: &Candle, stop: f64) -> f64 {
    if bar.open() <= stop { bar.open() } else { stop }
}

fn long_target_hit(bar: &Candle, target: f64) -> bool {
    bar.open() >= target || bar.high() >= target
}

fn long_target_reference(bar: &Candle, target: f64) -> f64 {
    if bar.open() >= target {
        bar.open()
    } else {
        target
    }
}

fn short_stop_hit(bar: &Candle, stop: f64) -> bool {
    bar.open() >= stop || bar.high() >= stop
}

fn short_stop_reference(bar: &Candle, stop: f64) -> f64 {
    if bar.open() >= stop { bar.open() } else { stop }
}

fn short_target_hit(bar: &Candle, target: f64) -> bool {
    bar.open() <= target || bar.low() <= target
}

fn short_target_reference(bar: &Candle, target: f64) -> f64 {
    if bar.open() <= target {
        bar.open()
    } else {
        target
    }
}

fn order_side(side: Side) -> OrderSide {
    match side {
        Side::Long => OrderSide::Buy,
        Side::Short => OrderSide::Sell,
    }
}

/// A fill or decision could not be simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    /// A decision was submitted before any bar had been advanced.
    SubmitBeforeAdvance,
    /// The decision carries a different bar index than the bar it was given
    /// with.
    DecisionBarMismatch {
        /// The engine's next bar index.
        expected: usize,
        /// The decision's bar index.
        found: usize,
    },
    /// The decision carries a different open time than the bar it was given
    /// with.
    DecisionTimeMismatch {
        /// The bar's open time.
        expected: i64,
        /// The decision's open time.
        found: i64,
    },
    /// The configured slippage is at least 100% of the reference price.
    SlippageAtOrAboveOne,
    /// A bracket fill was labelled as an entry or exit, which cannot happen.
    UnexpectedFillReason,
    /// An order could not be constructed.
    Order(OrderError),
    /// A fill could not be constructed.
    Fill(FillError),
    /// A position could not be constructed.
    Position(PositionError),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubmitBeforeAdvance => {
                write!(
                    formatter,
                    "a decision cannot be submitted before a bar is advanced"
                )
            }
            Self::DecisionBarMismatch { expected, found } => write!(
                formatter,
                "the decision is for bar {found}, but the engine expects bar {expected}"
            ),
            Self::DecisionTimeMismatch { expected, found } => write!(
                formatter,
                "the decision is for time {found}, but the bar opens at {expected}"
            ),
            Self::SlippageAtOrAboveOne => {
                write!(formatter, "the execution slippage must be below 100%")
            }
            Self::UnexpectedFillReason => {
                write!(formatter, "a bracket fill must be a stop or a target")
            }
            Self::Order(source) => source.fmt(formatter),
            Self::Fill(source) => source.fmt(formatter),
            Self::Position(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Order(source) => Some(source),
            Self::Fill(source) => Some(source),
            Self::Position(source) => Some(source),
            _ => None,
        }
    }
}
