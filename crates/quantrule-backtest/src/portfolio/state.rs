//! Portfolio state and the open position it carries.
//!
//! The state is a snapshot of capital at one strategy bar: cash, the optional
//! open position, equity, peak equity, drawdown, and the halt flag. The ledger
//! owns the transitions; this module defines the values and their validation so
//! the ledger cannot store a negative quantity or a negative cost.

use std::fmt;

use crate::strategy::Side;

/// One open position as the portfolio sees it.
///
/// The entry equity is the denominator of the trade return. The entry fees and
/// commission are the costs already paid to open the position; funding is
/// accumulated while it stays open. Execution's [`crate::execution::Position`]
/// carries the stop and target; this record carries the capital.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::OpenPosition;
/// use quantrule_backtest::strategy::Side;
///
/// let position = OpenPosition::new(Side::Long, 2.0, 100.0, 10, 1_700_000_000, 10_000.0, 1.0, 10.0)?;
/// assert!(position.is_long());
/// assert_eq!(position.quantity, 2.0);
/// # Ok::<(), quantrule_backtest::portfolio::OpenPositionError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenPosition {
    /// The position's direction.
    pub side: Side,
    /// The quantity held.
    pub quantity: f64,
    /// The entry fill price.
    pub entry_price: f64,
    /// The zero-based strategy bar of the entry fill.
    pub entry_bar: usize,
    /// The entry bar's open time as UTC Unix seconds.
    pub entry_time: i64,
    /// The account equity before the entry fill.
    pub entry_equity: f64,
    /// The execution fees paid to open the position.
    pub entry_fees: f64,
    /// The commission paid to open the position.
    pub entry_commission: f64,
    /// Funding paid while the position has been open.
    pub funding_paid: f64,
    /// Funding received while the position has been open.
    pub funding_received: f64,
}

impl OpenPosition {
    /// Builds an open position and validates its values.
    ///
    /// # Errors
    ///
    /// Returns an error when the quantity or entry price is not finite or not
    /// positive, when the entry equity is not finite or negative, or when a
    /// cost is not finite or negative.
    // A position is a flat record; each argument is one of its fields, and a
    // builder would add a parallel type without enforcing a new invariant.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        side: Side,
        quantity: f64,
        entry_price: f64,
        entry_bar: usize,
        entry_time: i64,
        entry_equity: f64,
        entry_fees: f64,
        entry_commission: f64,
    ) -> Result<Self, OpenPositionError> {
        if !quantity.is_finite() {
            return Err(OpenPositionError::NonFiniteQuantity);
        }
        if quantity <= 0.0 {
            return Err(OpenPositionError::NonPositiveQuantity);
        }
        if !entry_price.is_finite() {
            return Err(OpenPositionError::NonFiniteEntryPrice);
        }
        if entry_price <= 0.0 {
            return Err(OpenPositionError::NonPositiveEntryPrice);
        }
        if !entry_equity.is_finite() {
            return Err(OpenPositionError::NonFiniteEntryEquity);
        }
        if entry_equity < 0.0 {
            return Err(OpenPositionError::NegativeEntryEquity);
        }
        validate_cost(entry_fees)?;
        validate_cost(entry_commission)?;

        Ok(Self {
            side,
            quantity,
            entry_price,
            entry_bar,
            entry_time,
            entry_equity,
            entry_fees,
            entry_commission,
            funding_paid: 0.0,
            funding_received: 0.0,
        })
    }

    /// Whether the position is long.
    #[must_use]
    pub fn is_long(&self) -> bool {
        matches!(self.side, Side::Long)
    }

    /// Whether the position is short.
    #[must_use]
    pub fn is_short(&self) -> bool {
        matches!(self.side, Side::Short)
    }
}

fn validate_cost(cost: f64) -> Result<(), OpenPositionError> {
    if !cost.is_finite() {
        return Err(OpenPositionError::NonFiniteCost);
    }
    if cost < 0.0 {
        return Err(OpenPositionError::NegativeCost);
    }

    Ok(())
}

/// An open position could not be constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenPositionError {
    /// The quantity is not a finite number.
    NonFiniteQuantity,
    /// The quantity is zero or negative.
    NonPositiveQuantity,
    /// The entry price is not a finite number.
    NonFiniteEntryPrice,
    /// The entry price is zero or negative.
    NonPositiveEntryPrice,
    /// The entry equity is not a finite number.
    NonFiniteEntryEquity,
    /// The entry equity is negative.
    NegativeEntryEquity,
    /// A cost is not a finite number.
    NonFiniteCost,
    /// A cost is negative.
    NegativeCost,
}

impl fmt::Display for OpenPositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteQuantity => write!(formatter, "the position quantity must be finite"),
            Self::NonPositiveQuantity => {
                write!(formatter, "the position quantity must be positive")
            }
            Self::NonFiniteEntryPrice => write!(formatter, "the entry price must be finite"),
            Self::NonPositiveEntryPrice => write!(formatter, "the entry price must be positive"),
            Self::NonFiniteEntryEquity => write!(formatter, "the entry equity must be finite"),
            Self::NegativeEntryEquity => {
                write!(formatter, "the entry equity must not be negative")
            }
            Self::NonFiniteCost => write!(formatter, "a position cost must be finite"),
            Self::NegativeCost => write!(formatter, "a position cost must not be negative"),
        }
    }
}

impl std::error::Error for OpenPositionError {}

/// The portfolio's capital at one strategy bar.
///
/// `equity` is `cash` plus the position's marked value; the ledger recomputes
/// both on every bar. `drawdown` is `1 - equity / peak_equity`. The halt flag
/// latches once the configured limit is reached and never clears during a run.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::portfolio::PortfolioState;
///
/// let state = PortfolioState::new(10_000.0)?;
/// assert!(state.is_flat());
/// assert_eq!(state.equity, 10_000.0);
/// assert_eq!(state.drawdown, 0.0);
/// assert!(!state.halted);
/// # Ok::<(), quantrule_backtest::portfolio::PortfolioStateError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortfolioState {
    /// The cash balance.
    pub cash: f64,
    /// The open position, or `None` while flat.
    pub position: Option<OpenPosition>,
    /// Cash plus the marked position value.
    pub equity: f64,
    /// The highest equity seen so far.
    pub peak_equity: f64,
    /// `1 - equity / peak_equity`.
    pub drawdown: f64,
    /// Whether the drawdown limit has latched a halt.
    pub halted: bool,
}

impl PortfolioState {
    /// Builds the initial state from the starting cash.
    ///
    /// # Errors
    ///
    /// Returns an error when the cash is not finite or not positive.
    pub fn new(initial_cash: f64) -> Result<Self, PortfolioStateError> {
        if !initial_cash.is_finite() {
            return Err(PortfolioStateError::NonFiniteInitialCash);
        }
        if initial_cash <= 0.0 {
            return Err(PortfolioStateError::NonPositiveInitialCash);
        }

        Ok(Self {
            cash: initial_cash,
            position: None,
            equity: initial_cash,
            peak_equity: initial_cash,
            drawdown: 0.0,
            halted: false,
        })
    }

    /// Whether no position is open.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.position.is_none()
    }

    /// Whether a long position is open.
    #[must_use]
    pub fn is_long(&self) -> bool {
        self.position.is_some_and(|position| position.is_long())
    }

    /// Whether a short position is open.
    #[must_use]
    pub fn is_short(&self) -> bool {
        self.position.is_some_and(|position| position.is_short())
    }
}

/// The initial portfolio state was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioStateError {
    /// The initial cash is not a finite number.
    NonFiniteInitialCash,
    /// The initial cash is zero or negative.
    NonPositiveInitialCash,
}

impl fmt::Display for PortfolioStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteInitialCash => write!(formatter, "the initial cash must be finite"),
            Self::NonPositiveInitialCash => write!(formatter, "the initial cash must be positive"),
        }
    }
}

impl std::error::Error for PortfolioStateError {}
