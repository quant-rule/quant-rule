//! The strategy walk over one timeframe.
//!
//! The walk records the per-bar strategy input, the decision it produced, the
//! agreement counts, and the strategy's final position. It is computed once by
//! the runner and reused by the reports; the shell never re-walks the strategy.

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
    positions: Vec<Option<Side>>,
    agreements: AgreementCounts,
    open_position: Option<Side>,
}

impl StrategyWalk {
    pub(crate) fn new(
        inputs: Vec<StrategyInput>,
        decisions: Vec<Decision>,
        positions: Vec<Option<Side>>,
        agreements: AgreementCounts,
        open_position: Option<Side>,
    ) -> Self {
        Self {
            inputs,
            decisions,
            positions,
            agreements,
            open_position,
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

    /// The position supplied to the rule on each bar, oldest first.
    ///
    /// This is the execution position in a backtest walk and the virtual
    /// position in the diagnostic walk.
    #[must_use]
    pub fn positions(&self) -> &[Option<Side>] {
        &self.positions
    }

    /// The agreement classification counts.
    #[must_use]
    pub const fn agreements(&self) -> AgreementCounts {
        self.agreements
    }

    /// The final position, or `None` when flat.
    ///
    /// In a backtest walk this is the position execution actually holds. In the
    /// diagnostic walk it is the virtual position produced by assuming every
    /// decision filled immediately.
    #[must_use]
    pub const fn open_position(&self) -> Option<Side> {
        self.open_position
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
