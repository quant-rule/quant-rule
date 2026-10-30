//! The strategy walk over one execution timeframe.
//!
//! The walk records the per-bar execution snapshot, the as-of context, the
//! decision it produced, the position execution actually held on that bar, and
//! the final position. It is computed once by the runner and reused by the
//! reports; the shell never re-walks the strategy.

use crate::context::LadderContext;
use crate::intent::StrategyPosition;
use crate::side::Side;

use super::decision::Decision;
use super::input::StrategyInput;

/// One deterministic strategy walk.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrategyWalk {
    inputs: Vec<StrategyInput>,
    contexts: Vec<LadderContext>,
    decisions: Vec<Decision>,
    positions: Vec<StrategyPosition>,
    final_position: StrategyPosition,
}

impl StrategyWalk {
    pub(crate) fn new(
        inputs: Vec<StrategyInput>,
        contexts: Vec<LadderContext>,
        decisions: Vec<Decision>,
        positions: Vec<StrategyPosition>,
        final_position: StrategyPosition,
    ) -> Self {
        Self {
            inputs,
            contexts,
            decisions,
            positions,
            final_position,
        }
    }

    /// The per-bar execution snapshots, oldest first.
    #[must_use]
    pub fn inputs(&self) -> &[StrategyInput] {
        &self.inputs
    }

    /// The per-bar as-of contexts, oldest first.
    #[must_use]
    pub fn contexts(&self) -> &[LadderContext] {
        &self.contexts
    }

    /// One decision per bar, oldest first.
    #[must_use]
    pub fn decisions(&self) -> &[Decision] {
        &self.decisions
    }

    /// The position execution supplied to the rule on each bar, oldest first.
    ///
    /// This is the execution position in a backtest walk and the virtual
    /// position in the diagnostic walk. The strategy never owns either one.
    #[must_use]
    pub fn positions(&self) -> &[StrategyPosition] {
        &self.positions
    }

    /// The final position, or flat when the run ended flat.
    #[must_use]
    pub const fn final_position(&self) -> StrategyPosition {
        self.final_position
    }

    /// The final position's side, or `None` when flat.
    ///
    /// In a backtest walk this is the side execution actually holds. In the
    /// diagnostic walk it is the virtual side produced by assuming every
    /// decision filled immediately.
    #[must_use]
    pub const fn open_position(&self) -> Option<Side> {
        self.final_position.side
    }

    /// The number of bars in the walk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.decisions.len()
    }

    /// Whether the walk has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty()
    }
}
