//! Fill records and the reason a fill happened.
//!
//! A fill is the executed result of an order. It carries the price after
//! slippage, the unslipped reference price, the fee, and the bar that produced
//! it. It carries no cash, position, or P&L; the portfolio module owns those.

use std::fmt;

use super::order::{OrderSide, Trigger};

/// Why a fill happened.
///
/// The reason is the order's intent, carried through to the fill so a report
/// can separate a strategy exit from a stop or a target.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{FillReason, Trigger};
///
/// assert!(FillReason::Entry.matches_trigger(Trigger::Market));
/// assert!(FillReason::StopLoss.matches_trigger(Trigger::Stop { price: 95.0 }));
/// assert!(FillReason::TakeProfit.is_exit());
/// assert!(!FillReason::Entry.is_exit());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillReason {
    /// A strategy entry.
    Entry,
    /// A strategy exit.
    Exit,
    /// A stop-loss exit.
    StopLoss,
    /// A take-profit exit.
    TakeProfit,
}

impl FillReason {
    /// Whether the reason closes a position.
    #[must_use]
    pub const fn is_exit(self) -> bool {
        matches!(self, Self::Exit | Self::StopLoss | Self::TakeProfit)
    }

    /// Whether the reason is compatible with the supplied trigger.
    ///
    /// A market order carries an entry or exit reason; a stop carries a
    /// stop-loss reason; a target carries a take-profit reason.
    #[must_use]
    pub const fn matches_trigger(self, trigger: Trigger) -> bool {
        matches!(
            (self, trigger),
            (Self::Entry | Self::Exit, Trigger::Market)
                | (Self::StopLoss, Trigger::Stop { .. })
                | (Self::TakeProfit, Trigger::Target { .. })
        )
    }
}

/// One executed fill.
///
/// `reference_price` is the price selected by the fill policy before costs.
/// `slippage` is the non-negative per-unit adjustment applied against the
/// position, so a buy pays `reference_price + slippage` and a sell receives
/// `reference_price - slippage`. `fee` is the total quote-currency fee for the
/// fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{Fill, FillReason, OrderSide};
///
/// let fill = Fill::new(
///     OrderSide::Buy,
///     2.0,
///     100.0,
///     0.10,
///     0.05,
///     11,
///     1_700_000_900,
///     FillReason::Entry,
/// )?;
/// assert_eq!(fill.price, 100.05);
/// assert_eq!(fill.notional(), 200.1);
/// assert_eq!(fill.side, OrderSide::Buy);
/// # Ok::<(), quantrule_backtest::execution::FillError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fill {
    /// The transaction side.
    pub side: OrderSide,
    /// The quantity traded.
    pub quantity: f64,
    /// The executed price after slippage: `reference_price + slippage` for a
    /// buy and `reference_price - slippage` for a sell.
    pub price: f64,
    /// The unslipped price selected by the fill policy.
    pub reference_price: f64,
    /// The total quote-currency fee for the fill.
    pub fee: f64,
    /// The non-negative per-unit adverse price adjustment.
    pub slippage: f64,
    /// The zero-based bar index the fill happened on.
    pub bar: usize,
    /// The fill bar's open time as UTC Unix seconds.
    pub time: i64,
    /// Why the fill happened.
    pub reason: FillReason,
}

impl Fill {
    /// Builds a fill, applies slippage to the reference price, and validates
    /// the result.
    ///
    /// # Errors
    ///
    /// Returns [`FillError`] when the quantity or reference price is not finite
    /// or not positive, when the fee or slippage is not finite or negative, or
    /// when the computed executed price is not finite or not positive.
    // A fill is a flat record; each argument is one of its fields, and a
    // builder would add a parallel type without enforcing a new invariant.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        side: OrderSide,
        quantity: f64,
        reference_price: f64,
        fee: f64,
        slippage: f64,
        bar: usize,
        time: i64,
        reason: FillReason,
    ) -> Result<Self, FillError> {
        if !quantity.is_finite() {
            return Err(FillError::NonFiniteQuantity);
        }
        if quantity <= 0.0 {
            return Err(FillError::NonPositiveQuantity);
        }
        if !reference_price.is_finite() {
            return Err(FillError::NonFiniteReferencePrice);
        }
        if reference_price <= 0.0 {
            return Err(FillError::NonPositiveReferencePrice);
        }
        if !fee.is_finite() {
            return Err(FillError::NonFiniteFee);
        }
        if fee < 0.0 {
            return Err(FillError::NegativeFee);
        }
        if !slippage.is_finite() {
            return Err(FillError::NonFiniteSlippage);
        }
        if slippage < 0.0 {
            return Err(FillError::NegativeSlippage);
        }

        let price = match side {
            OrderSide::Buy => reference_price + slippage,
            OrderSide::Sell => reference_price - slippage,
        };
        if !price.is_finite() {
            return Err(FillError::NonFinitePrice);
        }
        if price <= 0.0 {
            return Err(FillError::NonPositivePrice);
        }

        Ok(Self {
            side,
            quantity,
            price,
            reference_price,
            fee,
            slippage,
            bar,
            time,
            reason,
        })
    }

    /// The quote-currency value traded before fees.
    #[must_use]
    pub fn notional(&self) -> f64 {
        self.price * self.quantity
    }
}

/// A fill could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillError {
    /// The quantity is not a finite number.
    NonFiniteQuantity,
    /// The quantity is zero or negative.
    NonPositiveQuantity,
    /// The executed price is not a finite number.
    NonFinitePrice,
    /// The executed price is zero or negative.
    NonPositivePrice,
    /// The reference price is not a finite number.
    NonFiniteReferencePrice,
    /// The reference price is zero or negative.
    NonPositiveReferencePrice,
    /// The fee is not a finite number.
    NonFiniteFee,
    /// The fee is negative.
    NegativeFee,
    /// The slippage is not a finite number.
    NonFiniteSlippage,
    /// The slippage is negative.
    NegativeSlippage,
}

impl fmt::Display for FillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteQuantity => write!(formatter, "the fill quantity must be finite"),
            Self::NonPositiveQuantity => write!(formatter, "the fill quantity must be positive"),
            Self::NonFinitePrice => write!(formatter, "the fill price must be finite"),
            Self::NonPositivePrice => write!(formatter, "the fill price must be positive"),
            Self::NonFiniteReferencePrice => {
                write!(formatter, "the reference price must be finite")
            }
            Self::NonPositiveReferencePrice => {
                write!(formatter, "the reference price must be positive")
            }
            Self::NonFiniteFee => write!(formatter, "the fill fee must be finite"),
            Self::NegativeFee => write!(formatter, "the fill fee must not be negative"),
            Self::NonFiniteSlippage => write!(formatter, "the fill slippage must be finite"),
            Self::NegativeSlippage => {
                write!(formatter, "the fill slippage must not be negative")
            }
        }
    }
}

impl std::error::Error for FillError {}
