//! The strategy decision records.
//!
//! An entry or an exit is a decision, not a fill. The record carries the bar
//! that produced the decision and the values that justified it; it carries no
//! fill price, quantity, fee, position, or P&L. Execution owns those.

use super::input::StrategyInput;

/// The direction of a position or a decision.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// A long position.
    Long = 1,
    /// A short position.
    Short = -1,
}

impl Side {
    /// The contract's numeric code: `+1` for long and `-1` for short.
    #[must_use]
    pub const fn code(self) -> i8 {
        self as i8
    }

    /// The opposite side.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::Long => Self::Short,
            Self::Short => Self::Long,
        }
    }
}

/// An entry decision.
///
/// The strategy constructs an entry only while flat, and only when the bar's
/// agreement is [`super::input::Agreement::Long`] for [`Side::Long`] or
/// [`super::input::Agreement::Short`] for [`Side::Short`]. The bar snapshot is
/// the evidence for the decision; it is not a fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::{Entry, Side, StrategyInput};
///
/// let bar = StrategyInput::new(900, 0, 0, Some(0.4), Some(0.2), Some(1.0));
/// let entry = Entry { side: Side::Long, bar };
/// assert_eq!(entry.side, Side::Long);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    /// The side to enter.
    pub side: Side,
    /// The bar whose agreement produced the entry.
    pub bar: StrategyInput,
}

/// An exit decision.
///
/// The strategy constructs an exit only while a position is open. The bar
/// snapshot is the bar on which the exit condition became true; it is not a
/// fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::{Exit, ExitReason, Side, StrategyInput};
///
/// let bar = StrategyInput::new(900, 1, 900, Some(0.0), Some(0.0), Some(0.0));
/// let exit = Exit { side: Side::Long, bar, reason: ExitReason::AgreementLost };
/// assert_eq!(exit.reason, ExitReason::AgreementLost);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exit {
    /// The side that was open and is now closed.
    pub side: Side,
    /// The bar on which the exit condition became true.
    pub bar: StrategyInput,
    /// Why the position is closed.
    pub reason: ExitReason,
}

/// Why a strategy closed a position.
///
/// The first rule has one exit condition. A later rule adds its own reason
/// rather than overloading this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// The three readings no longer agree in the position's direction.
    AgreementLost,
}

/// The decision a strategy makes for one closed bar.
///
/// A strategy returns exactly one decision per bar. `Hold` means no change;
/// `Enter` and `Exit` are requests for the next execution step, not fills.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decision {
    /// Keep the current position, or stay flat.
    Hold,
    /// Open a position.
    Enter(Entry),
    /// Close the open position.
    Exit(Exit),
}

/// A causal strategy over one timeframe.
///
/// The runner calls [`Strategy::on_bar`] once per closed bar, in ascending
/// open-time order. An implementation may keep position state between calls,
/// but it must not read a bar it has not been given, the clock, or any random
/// source. A strategy emits `Enter` only while flat and `Exit` only while a
/// position is open.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::strategy::{Decision, Strategy, StrategyInput};
///
/// struct NeverTrades;
///
/// impl Strategy for NeverTrades {
///     fn on_bar(&mut self, _input: &StrategyInput) -> Decision {
///         Decision::Hold
///     }
/// }
///
/// let mut strategy = NeverTrades;
/// let bar = StrategyInput::new(900, 0, 0, None, None, None);
/// assert_eq!(strategy.on_bar(&bar), Decision::Hold);
/// ```
pub trait Strategy {
    /// Feeds one closed bar and returns the decision for that bar.
    fn on_bar(&mut self, input: &StrategyInput) -> Decision;
}
