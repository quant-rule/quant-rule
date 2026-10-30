//! The strategy decision records.
//!
//! An entry or an exit is a decision, not a fill. The record carries the bar
//! that produced the decision and the values that justified it; it carries no
//! fill price, quantity, fee, position, or P&L. Execution owns those.
//!
//! An entry carries a [`TradeIntent`], which is the strategy's request for a
//! side and a risk bracket. Execution resolves the request; the strategy never
//! sets a fill price, fee, or slippage.

use crate::intent::{StrategyPosition, TradeIntent};
use crate::side::Side;

use super::input::StrategyInput;

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
/// use quantrule_backtest::intent::RiskPolicy;
/// use quantrule_backtest::strategy::{Entry, Side, StrategyInput};
///
/// let bar = StrategyInput::new(900, 0, 0, Some(0.4), Some(0.2), Some(1.0));
/// let entry = Entry {
///     bar,
///     intent: RiskPolicy::default().intent(Side::Long),
/// };
/// assert_eq!(entry.side(), Side::Long);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    /// The bar whose agreement produced the entry.
    pub bar: StrategyInput,
    /// The strategy's complete entry request.
    pub intent: TradeIntent,
}

impl Entry {
    /// The side to enter.
    #[must_use]
    pub const fn side(&self) -> Side {
        self.intent.side
    }
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

/// A causal, stateless strategy over one timeframe.
///
/// The runner calls [`Strategy::decide`] once per closed bar, in ascending
/// open-time order. The position execution actually holds is passed in by the
/// caller; execution and portfolio own it. A strategy must not read a bar it
/// has not been given, the clock, or any random source. It emits `Enter` only
/// when the position is flat and `Exit` only when the position is open.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::StrategyPosition;
/// use quantrule_backtest::strategy::{Decision, Strategy, StrategyInput};
///
/// struct NeverTrades;
///
/// impl Strategy for NeverTrades {
///     fn decide(&self, _input: &StrategyInput, _position: StrategyPosition) -> Decision {
///         Decision::Hold
///     }
/// }
///
/// let strategy = NeverTrades;
/// let bar = StrategyInput::new(900, 0, 0, None, None, None);
/// assert_eq!(strategy.decide(&bar, StrategyPosition::flat()), Decision::Hold);
/// ```
pub trait Strategy {
    /// Decides for one closed bar given the position execution currently holds.
    fn decide(&self, input: &StrategyInput, position: StrategyPosition) -> Decision;
}
