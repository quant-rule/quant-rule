//! The strategy walk over one timeframe.
//!
//! The walk records the per-bar strategy input, the decision it produced, the
//! position execution actually held on that bar, the agreement counts, and the
//! final position. It is computed once by the runner and reused by the reports;
//! the shell never re-walks the strategy.

use crate::intent::StrategyPosition;
use crate::side::Side;

use super::decision::Decision;
use super::input::StrategyInput;

/// The agreement classification counts for one walk.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AgreementCounts {
    /// Bars where all three readings agreed long.
    pub long: usize,
    /// Bars where all three readings agreed short.
    pub short: usize,
    /// Bars where the readings were available but neutral.
    pub neutral: usize,
    /// Bars where at least one reading was unavailable.
    pub unavailable: usize,
}

/// One deterministic strategy walk.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StrategyWalk {
    inputs: Vec<StrategyInput>,
    decisions: Vec<Decision>,
    positions: Vec<StrategyPosition>,
    agreements: AgreementCounts,
    final_position: StrategyPosition,
}

impl StrategyWalk {
    pub(crate) fn new(
        inputs: Vec<StrategyInput>,
        decisions: Vec<Decision>,
        positions: Vec<StrategyPosition>,
        agreements: AgreementCounts,
        final_position: StrategyPosition,
    ) -> Self {
        Self {
            inputs,
            decisions,
            positions,
            agreements,
            final_position,
        }
    }

    /// The per-bar strategy inputs, oldest first.
    #[must_use]
    pub fn inputs(&self) -> &[StrategyInput] {
        &self.inputs
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

    /// The agreement classification counts.
    #[must_use]
    pub const fn agreements(&self) -> AgreementCounts {
        self.agreements
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
