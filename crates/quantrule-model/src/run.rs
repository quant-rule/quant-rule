//! The per-timeframe model run orchestration.
//!
//! A [`ModelRun`] computes each timeframe once and keeps the per-timeframe
//! series. It is the single orchestration for the shell; the shell only selects
//! a display tail and renders it.
//!
//! The run publishes per-timeframe results only. It does not publish a
//! cross-timeframe aggregate or a consensus vote: those were the legacy
//! four-axis architecture. The module is pure: it reads the candles and
//! configuration it is given. It performs no I/O, reads no clock, and uses no
//! randomness.

use crate::input::{MarketStateConfig, TimeframeInput};
use crate::timeframe::{ModelBar, TimeframeModel};

/// The complete model run across the timeframe ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelRun {
    timeframes: Vec<TimeframeModel>,
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

        Self { timeframes }
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

    /// The latest complete bars across the ladder, fastest first.
    #[must_use]
    pub fn latest_bars(&self) -> Vec<ModelBar> {
        self.timeframes
            .iter()
            .filter_map(TimeframeModel::latest_bar)
            .collect()
    }
}
