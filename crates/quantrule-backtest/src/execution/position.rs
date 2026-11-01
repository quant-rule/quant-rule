//! Open positions and the position lifecycle.
//!
//! A position is what remains after an entry fill: a side, a quantity, an entry
//! price, and the stop and target that frame its risk. The lifecycle state
//! says whether the engine is flat, holding a position, or has just closed one.
//! Cash, equity, and P&L belong to the portfolio module.

use std::fmt;

use crate::side::Side;

/// One open position.
///
/// The stop and target are required because the first execution policy always
/// brackets a position with ATR-derived levels. They are prices, not orders;
/// execution creates the exit orders from them.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::Position;
/// use quantrule_backtest::side::Side;
///
/// let position = Position::new(Side::Long, 2.0, 100.0, 10, 1_700_000_000, 95.0, 110.0)?;
/// assert!(position.is_long());
/// assert_eq!(position.quantity, 2.0);
/// # Ok::<(), quantrule_backtest::execution::PositionError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position {
    /// The position's direction.
    pub side: Side,
    /// The quantity held.
    pub quantity: f64,
    /// The entry fill price.
    pub entry_price: f64,
    /// The zero-based bar index of the entry fill.
    pub entry_bar: usize,
    /// The entry bar's open time as UTC Unix seconds.
    pub entry_time: i64,
    /// The stop price.
    pub stop_price: f64,
    /// The target price.
    pub target_price: f64,
}

impl Position {
    /// Builds an open position and validates its values.
    ///
    /// # Errors
    ///
    /// Returns [`PositionError`] when the quantity, entry price, stop, or
    /// target is not finite or not positive, or when the stop and target do
    /// not bracket the entry price on the correct side. A long needs
    /// `stop < entry < target`; a short needs `target < entry < stop`.
    pub fn new(
        side: Side,
        quantity: f64,
        entry_price: f64,
        entry_bar: usize,
        entry_time: i64,
        stop_price: f64,
        target_price: f64,
    ) -> Result<Self, PositionError> {
        if !quantity.is_finite() {
            return Err(PositionError::NonFiniteQuantity);
        }
        if quantity <= 0.0 {
            return Err(PositionError::NonPositiveQuantity);
        }
        if !entry_price.is_finite() {
            return Err(PositionError::NonFiniteEntryPrice);
        }
        if entry_price <= 0.0 {
            return Err(PositionError::NonPositiveEntryPrice);
        }
        if !stop_price.is_finite() {
            return Err(PositionError::NonFiniteStopPrice);
        }
        if stop_price <= 0.0 {
            return Err(PositionError::NonPositiveStopPrice);
        }
        if !target_price.is_finite() {
            return Err(PositionError::NonFiniteTargetPrice);
        }
        if target_price <= 0.0 {
            return Err(PositionError::NonPositiveTargetPrice);
        }

        match side {
            Side::Long => {
                if stop_price >= entry_price {
                    return Err(PositionError::StopNotBelowEntry);
                }
                if target_price <= entry_price {
                    return Err(PositionError::TargetNotAboveEntry);
                }
            }
            Side::Short => {
                if stop_price <= entry_price {
                    return Err(PositionError::StopNotAboveEntry);
                }
                if target_price >= entry_price {
                    return Err(PositionError::TargetNotBelowEntry);
                }
            }
        }

        Ok(Self {
            side,
            quantity,
            entry_price,
            entry_bar,
            entry_time,
            stop_price,
            target_price,
        })
    }

    /// Whether the position is long.
    #[must_use]
    pub const fn is_long(&self) -> bool {
        matches!(self.side, Side::Long)
    }

    /// Whether the position is short.
    #[must_use]
    pub const fn is_short(&self) -> bool {
        matches!(self.side, Side::Short)
    }
}

/// The lifecycle state of a position.
///
/// `Flat` is the initial state and the state after a close has been accounted
/// for; the trade record and P&L belong to the portfolio module. `Open` carries
/// the position. There is no separate just-closed state: the engine returns to
/// `Flat` on the bar the position closes.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::execution::{Position, PositionState};
/// use quantrule_backtest::side::Side;
///
/// let position = Position::new(Side::Long, 1.0, 100.0, 0, 0, 95.0, 110.0)?;
/// let state = PositionState::Open(position);
/// assert!(state.is_open());
/// assert!(PositionState::Flat.is_flat());
/// # Ok::<(), quantrule_backtest::execution::PositionError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionState {
    /// No position is held.
    Flat,
    /// A position is open.
    Open(Position),
}

impl PositionState {
    /// Whether no position is held.
    #[must_use]
    pub const fn is_flat(self) -> bool {
        matches!(self, Self::Flat)
    }

    /// Whether a position is open.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Open(_))
    }
}

/// A position could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionError {
    /// The quantity is not a finite number.
    NonFiniteQuantity,
    /// The quantity is zero or negative.
    NonPositiveQuantity,
    /// The entry price is not a finite number.
    NonFiniteEntryPrice,
    /// The entry price is zero or negative.
    NonPositiveEntryPrice,
    /// The stop price is not a finite number.
    NonFiniteStopPrice,
    /// The stop price is zero or negative.
    NonPositiveStopPrice,
    /// The target price is not a finite number.
    NonFiniteTargetPrice,
    /// The target price is zero or negative.
    NonPositiveTargetPrice,
    /// A long stop is not below its entry.
    StopNotBelowEntry,
    /// A long target is not above its entry.
    TargetNotAboveEntry,
    /// A short stop is not above its entry.
    StopNotAboveEntry,
    /// A short target is not below its entry.
    TargetNotBelowEntry,
}

impl fmt::Display for PositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteQuantity => write!(formatter, "the position quantity must be finite"),
            Self::NonPositiveQuantity => {
                write!(formatter, "the position quantity must be positive")
            }
            Self::NonFiniteEntryPrice => {
                write!(formatter, "the entry price must be finite")
            }
            Self::NonPositiveEntryPrice => {
                write!(formatter, "the entry price must be positive")
            }
            Self::NonFiniteStopPrice => write!(formatter, "the stop price must be finite"),
            Self::NonPositiveStopPrice => write!(formatter, "the stop price must be positive"),
            Self::NonFiniteTargetPrice => write!(formatter, "the target price must be finite"),
            Self::NonPositiveTargetPrice => {
                write!(formatter, "the target price must be positive")
            }
            Self::StopNotBelowEntry => {
                write!(formatter, "a long stop must be below the entry price")
            }
            Self::TargetNotAboveEntry => {
                write!(formatter, "a long target must be above the entry price")
            }
            Self::StopNotAboveEntry => {
                write!(formatter, "a short stop must be above the entry price")
            }
            Self::TargetNotBelowEntry => {
                write!(formatter, "a short target must be below the entry price")
            }
        }
    }
}

impl std::error::Error for PositionError {}
