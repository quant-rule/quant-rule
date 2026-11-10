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
//! A two-sided stop entry is submitted as two pending orders, one at each
//! boundary. The first fill cancels the other leg; a bar that touches both
//! boundaries with the open between them is ambiguous and fills neither,
//! because OHLC cannot order the two intrabar triggers. The filled leg is then
//! bracketed by the same stop/target resolution as a market entry.
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
//! An entry carries the strategy's [`TradeIntent`]. Execution resolves its stop
//! and target against the entry fill price and the decision bar's ATR(14):
//!
//! | Stop spec | Long | Short |
//! |---|---|---|
//! | `AtrMultiple { multiple }` | `fill - multiple * ATR` | `fill + multiple * ATR` |
//! | `Price { price }` | `price` | `price` |
//!
//! A `RewardToRisk { multiple }` target is the stop distance times `multiple`,
//! projected on the profitable side of the fill. A `Price { price }` target is
//! used as given. Execution never chooses a distance.
//!
//! An ATR is required only when the stop is ATR-based. A missing, zero, or
//! non-finite ATR rejects that entry with [`RejectReason::MissingAtr`]. A stop
//! or target that is not a valid positive bracket around the fill price
//! rejects it with [`RejectReason::InvalidBracket`]. An unsupported entry
//! style, an expired intent, and an intent whose slippage limit is below
//! execution's configured slippage are rejected with
//! [`RejectReason::UnsupportedEntryStyle`], [`RejectReason::ExpiredIntent`], and
//! [`RejectReason::SlippageLimit`]. A rejection leaves the position unchanged.
//!
//! The bracket is live from the entry bar onward, so one bar can fill the entry
//! at its open and trigger the stop or target later in the same bar. A gap at
//! the open is resolved first; if the open is between both levels and both are
//! touched, the stop wins.
//!
//! A bar with zero volume cannot fill a market order or trigger a bracket.
//! A zero-volume next bar rejects the pending market order with
//! [`RejectReason::ZeroVolume`]; a zero-volume bar while a position is open
//! leaves the bracket active for the next bar. An absent volume is not zero:
//! it is treated as tradable.
//!
//! A bar that does not immediately follow the previous bar at the execution
//! timeframe's cadence cannot fill a pending market order. The order is
//! rejected with [`RejectReason::NonContiguousBar`], and an open position's
//! bracket is still evaluated on that bar.
//!
//! # End of data
//!
//! [`ExecutionEngine::finish`] rejects an order that has no next bar with
//! [`RejectReason::NoNextBar`]. It does not close an open position: the first
//! backtest has no end-of-data exit.

mod bracket_entry;

use quantrule_data::Candle;

use crate::intent::{BracketIntent, StrategyPosition, TradeIntent};
use crate::portfolio::floored_steps;
use crate::side::Side;

use super::{
    ExecutionBatch, ExecutionConfig, ExecutionError, Fill, FillReason, Order, OrderIntent,
    OrderRecord, OrderSide, OrderState, Position, PositionState, RejectReason, Rejection, Trigger,
    bracket,
};

/// A pending market order and the decision that produced it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingOrder {
    order_id: u64,
    order: Order,
    /// The strategy entry request; `None` for a market exit.
    intent: Option<TradeIntent>,
    atr14: Option<f64>,
    max_notional: Option<f64>,
    quantity_step: Option<f64>,
}

/// A pending two-sided stop-entry bracket.
#[derive(Debug, Clone, Copy, PartialEq)]
struct PendingBracket {
    intent: BracketIntent,
    quantity: f64,
    atr14: Option<f64>,
    max_notional: Option<f64>,
    quantity_step: Option<f64>,
    upper_order_id: u64,
    lower_order_id: u64,
    submitted_bar: usize,
    submitted_time: i64,
    remaining_bars: u32,
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

/// The causal fill simulator for one instrument and one position.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{ExecutionConfig, ExecutionEngine, OrderIntent};
/// use quantrule_backtest::intent::RiskPolicy;
/// use quantrule_backtest::side::Side;
/// use quantrule_data::Candle;
///
/// let config = ExecutionConfig::new(0.001, 2.0)?;
/// let mut engine = ExecutionEngine::new(config)?;
/// let first = Candle::new(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0))?;
/// let second = Candle::new(1, 900, 100.0, 102.0, 99.5, 101.0, Some(10.0))?;
///
/// assert!(engine.advance(&first, true)?.is_empty());
/// engine.submit(
///     OrderIntent::Enter {
///         intent: RiskPolicy::default().intent(Side::Long),
///         bar_index: 0,
///         open_time: 0,
///         max_notional: None,
///     },
///     Some(1.0),
///     Some(1.0),
/// )?;
/// let batch = engine.advance(&second, true)?;
/// assert_eq!(batch.fills.len(), 1);
/// assert_eq!(batch.fills[0].reference_price, 100.0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct ExecutionEngine {
    config: ExecutionConfig,
    next_bar: usize,
    last_bar: Option<(usize, i64)>,
    last_close: Option<f64>,
    position: PositionState,
    pending: Option<PendingOrder>,
    pending_bracket: Option<PendingBracket>,
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
            last_close: None,
            position: PositionState::Flat,
            pending: None,
            pending_bracket: None,
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
    /// `contiguous` is false when this bar does not immediately follow the
    /// previous bar at the execution timeframe's cadence. A non-contiguous bar
    /// rejects a pending market order with
    /// [`RejectReason::NonContiguousBar`]; it never fills across a gap. An open
    /// position's bracket is still evaluated on the bar.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError`] when a validated value cannot be
    /// constructed.
    pub fn advance(
        &mut self,
        bar: &Candle,
        contiguous: bool,
    ) -> Result<ExecutionBatch, ExecutionError> {
        let index = self.next_bar;
        let time = bar.timestamp();
        self.next_bar += 1;
        self.last_bar = Some((index, time));
        self.last_close = Some(bar.close());

        let mut batch = ExecutionBatch::default();

        if let Some(pending) = self.pending.take() {
            if contiguous {
                self.apply_pending(pending, bar, index, time, &mut batch)?;
            } else {
                self.reject_pending(
                    &mut batch,
                    pending,
                    index,
                    time,
                    RejectReason::NonContiguousBar,
                );
            }
        }

        if let Some(bracket) = self.pending_bracket.take() {
            if contiguous {
                self.apply_bracket_entry(bracket, bar, index, time, &mut batch)?;
            } else {
                self.reject_bracket(
                    &mut batch,
                    bracket,
                    index,
                    time,
                    RejectReason::NonContiguousBar,
                );
            }
        }

        if self.position.is_open() {
            self.apply_bracket(bar, index, time, &mut batch)?;
            if let PositionState::Open(position) = &mut self.position {
                update_excursions(position, bar);
            }
        }

        Ok(batch)
    }

    /// Schedules one decision for the next bar.
    ///
    /// `quantity` is the entry quantity. It is `None` when the caller could not
    /// size the entry; the decision is then rejected with
    /// [`RejectReason::MissingQuantity`]. An exit always closes the open
    /// position with the position's quantity, so its quantity is ignored. An
    /// exit whose side does not match the open position is rejected with
    /// [`RejectReason::WrongExitSide`]; the engine never closes the opposite
    /// side on a mismatched intent.
    ///
    /// `atr14` is the decision bar's ATR(14). It is used only when the entry's
    /// stop is ATR-based. The decision must carry the last advanced bar's index
    /// and open time; a mismatch is a caller error, not a rejection.
    ///
    /// An entry whose style execution does not fill, whose `valid_until` bar
    /// has passed, whose slippage limit is below execution's configured
    /// slippage, or whose stop or target specification is invalid is rejected
    /// before an order is accepted. A rejected entry leaves the position
    /// unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`ExecutionError::SubmitBeforeAdvance`] when no bar has been
    /// advanced, [`ExecutionError`] when the decision does not describe the
    /// last advanced bar, or when a validated value cannot be constructed.
    pub fn submit(
        &mut self,
        intent: OrderIntent,
        quantity: Option<f64>,
        atr14: Option<f64>,
    ) -> Result<ExecutionBatch, ExecutionError> {
        self.submit_with_step(intent, quantity, atr14, None)
    }

    /// Schedules one decision with an explicit quantity step for a risk cap.
    ///
    /// The step is used only when a fill notional is capped; the uncapped
    /// quantity is supplied by the caller exactly as for [`Self::submit`].
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::submit`].
    pub fn submit_with_step(
        &mut self,
        intent: OrderIntent,
        quantity: Option<f64>,
        atr14: Option<f64>,
        quantity_step: Option<f64>,
    ) -> Result<ExecutionBatch, ExecutionError> {
        let Some((index, time)) = self.last_bar else {
            return Err(ExecutionError::SubmitBeforeAdvance);
        };
        self.validate_intent(intent, index, time)?;
        if (self.pending.is_some() || self.pending_bracket.is_some())
            && !matches!(intent, OrderIntent::Hold)
        {
            return Err(ExecutionError::PendingOrderExists);
        }

        let mut batch = ExecutionBatch::default();
        match intent {
            OrderIntent::Hold => {}
            OrderIntent::Enter {
                intent,
                max_notional,
                ..
            } => {
                let side = order_side(intent.side);
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
                } else if let Some(reject_reason) = self.entry_reject_reason(&intent, index) {
                    self.reject_decision(
                        &mut batch,
                        RejectedDecision {
                            side,
                            quantity,
                            reason: FillReason::Entry,
                            submitted_bar: index,
                            submitted_time: time,
                            reject_reason,
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
                                intent: Some(intent),
                                atr14,
                                max_notional,
                                quantity_step,
                            });
                        }
                    }
                }
            }
            OrderIntent::EnterBracket {
                intent,
                max_notional,
                ..
            } => {
                if self.position.is_open() {
                    self.reject_bracket_decision(
                        &mut batch,
                        &intent,
                        quantity,
                        index,
                        time,
                        RejectReason::PositionAlreadyOpen,
                    );
                } else if let Some(reject_reason) = self.bracket_entry_reject_reason(&intent) {
                    self.reject_bracket_decision(
                        &mut batch,
                        &intent,
                        quantity,
                        index,
                        time,
                        reject_reason,
                    );
                } else {
                    match usable_quantity(quantity) {
                        Err(reject_reason) => self.reject_bracket_decision(
                            &mut batch,
                            &intent,
                            quantity,
                            index,
                            time,
                            reject_reason,
                        ),
                        Ok(quantity) => {
                            let upper = Order::new(
                                OrderSide::Buy,
                                quantity,
                                FillReason::Entry,
                                Trigger::Stop {
                                    price: intent.upper_price,
                                },
                                index,
                                time,
                            )
                            .map_err(ExecutionError::Order)?;
                            let lower = Order::new(
                                OrderSide::Sell,
                                quantity,
                                FillReason::Entry,
                                Trigger::Stop {
                                    price: intent.lower_price,
                                },
                                index,
                                time,
                            )
                            .map_err(ExecutionError::Order)?;
                            let upper_order_id = self.next_order_id();
                            let lower_order_id = self.next_order_id();
                            batch.orders.push(pending_record(upper_order_id, &upper));
                            batch.orders.push(pending_record(lower_order_id, &lower));
                            self.pending_bracket = Some(PendingBracket {
                                intent,
                                quantity,
                                atr14,
                                max_notional,
                                quantity_step,
                                upper_order_id,
                                lower_order_id,
                                submitted_bar: index,
                                submitted_time: time,
                                remaining_bars: intent.valid_for_bars,
                            });
                        }
                    }
                }
            }
            OrderIntent::Exit { side, .. } => match self.position {
                PositionState::Open(position) if position.side != side => self.reject_decision(
                    &mut batch,
                    RejectedDecision {
                        side: order_side(position.side).opposite(),
                        quantity: Some(position.quantity),
                        reason: FillReason::Exit,
                        submitted_bar: index,
                        submitted_time: time,
                        reject_reason: RejectReason::WrongExitSide,
                    },
                ),
                PositionState::Open(position) => {
                    // The engine closes the position it holds; the intent's
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
                        intent: None,
                        atr14,
                        max_notional: None,
                        quantity_step,
                    });
                }
                PositionState::Flat => self.reject_decision(
                    &mut batch,
                    RejectedDecision {
                        side: order_side(side).opposite(),
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
        if let Some(bracket) = self.pending_bracket.take() {
            let bar = bracket.submitted_bar;
            let time = bracket.submitted_time;
            self.reject_bracket(&mut batch, bracket, bar, time, RejectReason::NoNextBar);
        }
        batch
    }

    /// The current position state.
    #[must_use]
    pub const fn position(&self) -> PositionState {
        self.position
    }

    /// The open position's side, or `None` while flat.
    #[must_use]
    pub const fn position_side(&self) -> Option<Side> {
        match self.position {
            PositionState::Open(position) => Some(position.side),
            PositionState::Flat => None,
        }
    }

    /// The position as the strategy sees it.
    ///
    /// The strategy receives the side, the entry fill bar, and the number of
    /// closed bars since entry. It never receives the stop, target, quantity,
    /// or P&L, and it cannot change the position.
    #[must_use]
    pub fn position_view(&self) -> StrategyPosition {
        match self.position {
            PositionState::Open(position) => {
                let bars_held = self
                    .last_bar
                    .map(|(bar, _)| bar.saturating_sub(position.entry_bar) as u64)
                    .unwrap_or(0);
                let unrealized_atr = match (self.last_close, position.entry_atr) {
                    (Some(close), Some(atr)) if atr > 0.0 => {
                        Some((close - position.entry_price) / atr * f64::from(position.side.code()))
                    }
                    _ => None,
                };
                StrategyPosition {
                    side: Some(position.side),
                    entry_bar: Some(position.entry_bar),
                    bars_held,
                    entry_price: Some(position.entry_price),
                    entry_atr: position.entry_atr,
                    unrealized_atr,
                    mfe_atr: Some(position.mfe_atr),
                    mae_atr: Some(position.mae_atr),
                }
            }
            PositionState::Flat => StrategyPosition::flat(),
        }
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
}

fn update_excursions(position: &mut Position, bar: &Candle) {
    let Some(atr) = position.entry_atr.filter(|atr| *atr > 0.0) else {
        return;
    };
    let favorable = if position.is_long() {
        bar.high() - position.entry_price
    } else {
        position.entry_price - bar.low()
    } / atr;
    let adverse = if position.is_long() {
        position.entry_price - bar.low()
    } else {
        bar.high() - position.entry_price
    } / atr;
    position.mfe_atr = position.mfe_atr.max(favorable.max(0.0));
    position.mae_atr = position.mae_atr.max(adverse.max(0.0));
}

mod entry;
mod exit;
mod records;

use self::records::{order_side, pending_record, usable_quantity};
