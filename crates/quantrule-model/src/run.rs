//! The per-timeframe model run orchestration.
//!
//! A [`ModelRun`] computes each timeframe once and keeps the per-timeframe
//! series. It is the single orchestration for the shell; the shell only selects
//! a display tail and renders it.
//!
//! The run publishes per-timeframe results only. It does not publish a
//! cross-timeframe aggregate or a consensus vote: those were the retired
//! composite architecture. The module is pure: it reads the candles and
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

#[cfg(test)]
mod tests {
    use super::ModelRun;
    use crate::input::{MarketStateConfig, TimeframeInput};
    use quantrule_data::Candle;

    fn candles(count: usize, secs: u64, step: f64) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let close = 100.0 + index as f64 * step;
                Candle::new(
                    index as u64,
                    index as i64 * secs as i64,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0),
                )
                .expect("the test candle is valid")
            })
            .collect()
    }

    #[test]
    fn a_selected_timeframe_is_independent_of_the_model_ladder() {
        let m1 = candles(600, 60, 0.01);
        let h1 = candles(300, 3_600, 0.5);
        let full = ModelRun::compute(
            &[
                TimeframeInput::new(60, &m1),
                TimeframeInput::new(3_600, &h1),
            ],
            &MarketStateConfig::default(),
        );
        let selected = ModelRun::compute(
            &[TimeframeInput::new(3_600, &h1)],
            &MarketStateConfig::default(),
        );
        assert_eq!(full.timeframe(3_600), selected.timeframe(3_600));
    }
}
