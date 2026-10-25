//! The per-bar cross-axis consensus.
//!
//! The model publishes [`TimeframeConsensus`] from the three voting bars of one
//! closed bar. This module copies that record once per bar; it does not
//! recompute the counts. Rows are ordered by bar, in the same order as Layer 1.
//!
//! The module is pure: it reads the series it is given and performs no I/O,
//! reads no clock, and uses no randomness.

use quantrule_model::state::TimeframeConsensus;

use super::series::TimeframeSeries;

/// One stored consensus row: the closed bar and the model's counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsensusRow {
    /// The closed bar index.
    pub bar_index: usize,
    /// The model's consensus for that bar.
    pub consensus: TimeframeConsensus,
}

impl ConsensusRow {
    /// Builds one row per closed bar of `series`, in bar order.
    pub(crate) fn rows(series: &TimeframeSeries<'_>) -> Vec<Self> {
        (0..series.len())
            .filter_map(|bar| {
                series.consensus_at(bar).map(|consensus| Self {
                    bar_index: bar,
                    consensus,
                })
            })
            .collect()
    }
}
