//! The strategy decision records.
//!
//! An entry or an exit is a decision, not a fill. The record carries the bar
//! that produced the decision and the values that justified it; it carries no
//! fill price, quantity, fee, position, or P&L. Execution owns those.
//!
//! An entry carries a [`TradeIntent`], which is the strategy's request for a
//! side and a risk bracket. Execution resolves the request; the strategy never
//! sets a fill price, fee, or slippage.

use crate::context::LadderContext;
use crate::intent::{StrategyPosition, TradeIntent};
use crate::side::Side;

use super::input::StrategyInput;

/// An entry decision.
///
/// The strategy constructs an entry only while flat. The bar snapshot is the
/// evidence for the decision; it is not a fill.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::intent::RiskPolicy;
/// use quantrule_backtest::strategy::{Entry, Side, StrategyInput};
///
/// let bar = StrategyInput::new(300, 0, 0, Some(0.4), Some(0.2), Some(1.0));
/// let entry = Entry {
///     bar,
///     intent: RiskPolicy::default().intent(Side::Long),
/// };
/// assert_eq!(entry.side(), Side::Long);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entry {
    /// The bar whose readings produced the entry.
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
/// let bar = StrategyInput::new(300, 1, 300, Some(0.0), Some(0.0), Some(0.0));
/// let exit = Exit { side: Side::Long, bar, reason: ExitReason::ContextLost };
/// assert_eq!(exit.reason, ExitReason::ContextLost);
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    /// The higher-timeframe context that justified the entry no longer holds.
    ContextLost,
    /// The execution Structure state flipped against the position.
    StructureLost,
    /// The nearest confirmed structural level was lost.
    LevelLost,
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

/// A causal, stateless strategy over one execution timeframe.
///
/// The runner calls [`Strategy::decide`] once per closed execution bar, in
/// ascending open-time order, with the as-of [`LadderContext`] and the position
/// execution actually holds. Execution and portfolio own the position. A
/// strategy must not read a bar it has not been given, the clock, or any random
/// source. It emits `Enter` only when the position is flat and `Exit` only when
/// the position is open.
///
/// # Examples
///
/// ```
/// use quantrule_backtest::context::LadderContext;
/// use quantrule_backtest::intent::StrategyPosition;
/// use quantrule_backtest::strategy::{Decision, Strategy, StrategyInput};
///
/// struct NeverTrades;
///
/// impl Strategy for NeverTrades {
///     fn decide(
///         &self,
///         _input: &StrategyInput,
///         _context: &LadderContext,
///         _position: StrategyPosition,
///     ) -> Decision {
///         Decision::Hold
///     }
/// }
///
/// let strategy = NeverTrades;
/// let bar = StrategyInput::new(300, 0, 0, None, None, None);
/// let context = LadderContext {
///     execution_timeframe_secs: 300,
///     execution_bar_index: 0,
///     execution_open_time: 0,
///     execution_close_time: 300,
///     context: Vec::new(),
///     missing_timeframes: vec![900],
///     direction: quantrule_backtest::context::AxisContext::unavailable(),
///     momentum: quantrule_backtest::context::AxisContext::unavailable(),
///     structure: quantrule_backtest::context::StructureContext::unavailable(),
///     progression: quantrule_backtest::context::ProgressionContext::unavailable(),
///     levels: quantrule_backtest::context::LevelContext::unavailable(),
///     agreement: quantrule_backtest::context::AgreementContext::unavailable(),
/// };
/// assert_eq!(
///     strategy.decide(&bar, &context, StrategyPosition::flat()),
///     Decision::Hold
/// );
/// ```
pub trait Strategy {
    /// Decides for one closed execution bar.
    fn decide(
        &self,
        input: &StrategyInput,
        context: &LadderContext,
        position: StrategyPosition,
    ) -> Decision;
}
