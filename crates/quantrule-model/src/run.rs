//! The per-timeframe model run orchestration.
//!
//! A [`ModelRun`] computes each timeframe once and keeps the per-timeframe
//! series, the full event batch, the confirmed levels, and the latest
//! snapshot. It is the single orchestration for analytics and the shell; the
//! shell only selects a display tail and renders it.
//!
//! The module is pure: it reads the candles and configuration it is given. It
//! performs no I/O, reads no clock, and uses no randomness.

use crate::aggregate::TimeframeSnapshot;
use crate::consensus::TimeframeConsensus;
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::timeframe::{ModelBar, TimeframeModel};

/// The complete model run across the timeframe ladder.
///
/// The run publishes per-timeframe models and consensus diagnostics. It does
/// not publish cross-timeframe aggregates; reporting and analytics compute that
/// descriptive view explicitly from [`Self::snapshots`].
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRun {
    timeframes: Vec<TimeframeModel>,
    consensus: Vec<TimeframeConsensus>,
}

impl ModelRun {
    /// Computes every non-empty timeframe once.
    #[must_use]
    pub fn compute(inputs: &[TimeframeInput<'_>], config: &MarketStateConfig) -> Self {
        let mut timeframes: Vec<TimeframeModel> = inputs
            .iter()
            .filter_map(|input| TimeframeModel::compute(input, config))
            .collect();
        timeframes.sort_by_key(TimeframeModel::timeframe_secs);

        let snapshots: Vec<TimeframeSnapshot> = timeframes
            .iter()
            .map(|timeframe| timeframe.snapshot().clone())
            .collect();
        let consensus = snapshots.iter().map(TimeframeConsensus::compute).collect();

        Self {
            timeframes,
            consensus,
        }
    }

    /// The per-timeframe models, fastest first.
    #[must_use]
    pub fn timeframes(&self) -> &[TimeframeModel] {
        &self.timeframes
    }

    /// One timeframe, or `None` when it is absent.
    #[must_use]
    pub fn timeframe(&self, timeframe_secs: u64) -> Option<&TimeframeModel> {
        self.timeframes
            .iter()
            .find(|timeframe| timeframe.timeframe_secs() == timeframe_secs)
    }

    /// The latest per-timeframe snapshots, fastest first.
    ///
    /// Reporting and analytics pass these snapshots to the explicit
    /// cross-timeframe view. The model run does not publish the view itself.
    #[must_use]
    pub fn snapshots(&self) -> Vec<TimeframeSnapshot> {
        self.timeframes
            .iter()
            .map(|timeframe| timeframe.snapshot().clone())
            .collect()
    }

    /// The consensus diagnostics, fastest first.
    #[must_use]
    pub fn consensus(&self) -> &[TimeframeConsensus] {
        &self.consensus
    }

    /// The latest complete bars across the ladder, fastest first.
    #[must_use]
    pub fn latest_bars(&self) -> Vec<ModelBar> {
        self.timeframes
            .iter()
            .filter_map(TimeframeModel::latest_bar)
            .collect()
    }
}
