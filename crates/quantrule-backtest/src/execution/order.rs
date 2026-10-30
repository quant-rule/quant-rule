//! Orders and their lifecycle states.
//!
//! An order is a request, not a fill. It records the transaction side, the
//! quantity, why it was submitted, the trigger that decides its reference
//! price, and the bar it was submitted on. Execution resolves it from
//! [`OrderState::Pending`] to exactly one terminal state; the fill record is
//! separate and belongs to [`super::Fill`].
//!
//! [`OrderRecord`] is the published lifecycle record: the engine emits a
//! pending record when it accepts an order and a filled or rejected record with
//! the same id when the order resolves.

use std::fmt;

use super::fill::FillReason;

/// The transaction side of an order or a fill.
///
/// This is the direction of the trade, not the direction of the position. A
/// long entry is a buy, and closing that long is a sell. The position's own
/// long/short direction is [`crate::strategy::Side`].
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::OrderSide;
///
/// assert_eq!(OrderSide::Buy.code(), 1);
/// assert_eq!(OrderSide::Sell.code(), -1);
/// assert_eq!(OrderSide::Buy.opposite(), OrderSide::Sell);
/// ```
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Buy the instrument.
    Buy = 1,
    /// Sell the instrument.
    Sell = -1,
}

impl OrderSide {
    /// The contract's numeric code: `+1` for a buy and `-1` for a sell.
    #[must_use]
    pub const fn code(self) -> i8 {
        self as i8
    }

    /// The opposite transaction side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }

    /// Whether this side buys the instrument.
    #[must_use]
    pub const fn is_buy(self) -> bool {
        matches!(self, Self::Buy)
    }

    /// Whether this side sells the instrument.
    #[must_use]
    pub const fn is_sell(self) -> bool {
        matches!(self, Self::Sell)
    }
}

/// What causes an order to fill.
///
/// A market order fills at the next bar's open. A stop or target order waits
/// for its trigger price to be reached; the module policy defines the
/// gap and tie-break rules.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::Trigger;
///
/// assert_eq!(Trigger::Market.price(), None);
/// assert_eq!(Trigger::Stop { price: 95.0 }.price(), Some(95.0));
/// assert_eq!(Trigger::Target { price: 110.0 }.price(), Some(110.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Trigger {
    /// Fill at the next bar's open.
    Market,
    /// Fill when the price reaches the stop.
    Stop {
        /// The stop trigger price.
        price: f64,
    },
    /// Fill when the price reaches the target.
    Target {
        /// The target trigger price.
        price: f64,
    },
}

impl Trigger {
    /// The trigger price, or `None` for a market order.
    #[must_use]
    pub const fn price(self) -> Option<f64> {
        match self {
            Self::Market => None,
            Self::Stop { price } | Self::Target { price } => Some(price),
        }
    }
}

/// The lifecycle state of an order.
///
/// `Pending` is the only non-terminal state. A filled order has a separate
/// [`super::Fill`] record; the state records only that the fill happened. A
/// rejected order carries its reason on [`OrderRecord::rejection_reason`].
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::OrderState;
///
/// assert!(!OrderState::Pending.is_terminal());
/// assert!(OrderState::Filled.is_terminal());
/// assert!(OrderState::Rejected.is_terminal());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderState {
    /// Submitted and waiting for its trigger.
    Pending,
    /// Filled; the fill record carries the price and costs.
    Filled,
    /// Refused before it could fill.
    Rejected,
}

impl OrderState {
    /// Whether the order can no longer change state.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }
}

/// Why an order was rejected before it could fill.
///
/// These are execution conditions, not construction errors. A malformed order
/// is refused by [`Order::new`]; a decision refused before an order exists is
/// recorded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// The order was submitted on the last available bar.
    NoNextBar,
    /// An exit order was submitted while no position was open.
    NoPosition,
    /// An exit order named a side different from the open position.
    WrongExitSide,
    /// An entry order was submitted while a position was already open.
    PositionAlreadyOpen,
    /// The next bar has zero volume, so no fill is possible.
    ZeroVolume,
    /// The decision bar had no usable ATR(14), so no stop or target could be
    /// derived.
    MissingAtr,
    /// The ATR-derived stop or target was not a valid positive bracket around
    /// the entry price.
    InvalidBracket,
    /// An entry decision carried no quantity, because sizing produced none.
    MissingQuantity,
    /// An entry decision carried a quantity that is zero, negative, or not
    /// finite.
    NonPositiveQuantity,
    /// The fill notional exceeded the caller's entry risk limit.
    RiskLimit,
}

/// A request to trade one quantity of one instrument.
///
/// The order carries the bar it was submitted on, not the bar it fills on.
/// Execution may fill it on the next bar or leave it pending. It carries no
/// lifecycle state: that belongs to [`OrderRecord`].
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{FillReason, Order, OrderSide, Trigger};
///
/// let order = Order::new(
///     OrderSide::Buy,
///     1.5,
///     FillReason::Entry,
///     Trigger::Market,
///     10,
///     1_700_000_000,
/// )?;
/// assert_eq!(order.side, OrderSide::Buy);
/// assert_eq!(order.quantity, 1.5);
/// # Ok::<(), quantrule_backtest::execution::OrderError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Order {
    /// The transaction side.
    pub side: OrderSide,
    /// The quantity to trade.
    pub quantity: f64,
    /// Why the order was submitted.
    pub reason: FillReason,
    /// What causes the order to fill.
    pub trigger: Trigger,
    /// The zero-based bar index the order was submitted on.
    pub submitted_bar: usize,
    /// The submission bar's open time as UTC Unix seconds.
    pub submitted_time: i64,
}

impl Order {
    /// Builds a pending order and validates its values.
    ///
    /// # Errors
    ///
    /// Returns [`OrderError`] when the quantity is not finite or not positive,
    /// a stop or target price is not finite or not positive, or the reason
    /// does not match the trigger. A market order carries an entry or exit
    /// reason; a stop carries a stop-loss reason; a target carries a
    /// take-profit reason.
    pub fn new(
        side: OrderSide,
        quantity: f64,
        reason: FillReason,
        trigger: Trigger,
        submitted_bar: usize,
        submitted_time: i64,
    ) -> Result<Self, OrderError> {
        if !quantity.is_finite() {
            return Err(OrderError::NonFiniteQuantity);
        }
        if quantity <= 0.0 {
            return Err(OrderError::NonPositiveQuantity);
        }
        if let Some(price) = trigger.price() {
            if !price.is_finite() {
                return Err(OrderError::NonFiniteTriggerPrice);
            }
            if price <= 0.0 {
                return Err(OrderError::NonPositiveTriggerPrice);
            }
        }
        if !reason.matches_trigger(trigger) {
            return Err(OrderError::ReasonTriggerMismatch);
        }

        Ok(Self {
            side,
            quantity,
            reason,
            trigger,
            submitted_bar,
            submitted_time,
        })
    }
}

/// The published lifecycle record of one order.
///
/// The engine emits a pending record when it accepts an order and a filled or
/// rejected record with the same [`order_id`](Self::order_id) when the order
/// resolves. A report keeps the last record per id.
///
/// A bracket order is recorded when its stop or target is reached. Its
/// submission bar and time are the entry fill's, because that is when the
/// bracket became live. The level that never triggers is not an order and is
/// not recorded.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{ExecutionConfig, ExecutionEngine, OrderIntent, OrderState};
/// use quantrule_backtest::side::Side;
/// use quantrule_data::Candle;
///
/// let config = ExecutionConfig::new(0.001, 2.0)?;
/// let mut engine = ExecutionEngine::new(config)?;
/// let first = Candle::new(0, 0, 100.0, 101.0, 99.0, 100.0, Some(10.0))?;
/// engine.advance(&first)?;
/// let batch = engine.submit(
///     OrderIntent::Enter {
///         side: Side::Long,
///         bar_index: 0,
///         open_time: 0,
///         max_notional: None,
///     },
///     Some(1.0),
///     Some(1.0),
/// )?;
/// assert_eq!(batch.orders.len(), 1);
/// assert_eq!(batch.orders[0].order_id, 0);
/// assert_eq!(batch.orders[0].state, OrderState::Pending);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrderRecord {
    /// The deterministic order id, assigned in creation order from zero.
    pub order_id: u64,
    /// The zero-based bar index the order was submitted on.
    pub submitted_bar: usize,
    /// The submission bar's open time as UTC Unix seconds.
    pub submitted_time: i64,
    /// The transaction side.
    pub side: OrderSide,
    /// The quantity to trade, or `None` when the decision carried none.
    pub quantity: Option<f64>,
    /// Why the order was submitted.
    pub reason: FillReason,
    /// What causes the order to fill.
    pub trigger: Trigger,
    /// The lifecycle state at the time of the record.
    pub state: OrderState,
    /// The bar the order filled on, when it filled.
    pub filled_bar: Option<usize>,
    /// The fill bar's open time as UTC Unix seconds, when it filled.
    pub filled_time: Option<i64>,
    /// The id of the fill that resolved the order, when it filled.
    pub fill_id: Option<u64>,
    /// Why the order was refused, when it was refused.
    pub rejection_reason: Option<RejectReason>,
}

/// An order could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderError {
    /// The quantity is not a finite number.
    NonFiniteQuantity,
    /// The quantity is zero or negative.
    NonPositiveQuantity,
    /// A stop or target price is not a finite number.
    NonFiniteTriggerPrice,
    /// A stop or target price is zero or negative.
    NonPositiveTriggerPrice,
    /// The reason does not describe the supplied trigger.
    ReasonTriggerMismatch,
}

impl fmt::Display for OrderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteQuantity => write!(formatter, "the order quantity must be finite"),
            Self::NonPositiveQuantity => write!(formatter, "the order quantity must be positive"),
            Self::NonFiniteTriggerPrice => {
                write!(formatter, "the trigger price must be finite")
            }
            Self::NonPositiveTriggerPrice => {
                write!(formatter, "the trigger price must be positive")
            }
            Self::ReasonTriggerMismatch => {
                write!(formatter, "the order reason does not match its trigger")
            }
        }
    }
}

impl std::error::Error for OrderError {}
