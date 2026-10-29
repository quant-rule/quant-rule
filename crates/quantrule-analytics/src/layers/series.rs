//! The analytics view of one timeframe's model run.
//!
//! The model owns the per-bar calculation. This module only adapts the public
//! [`TimeframeModel`] surface to the layer builder: it exposes candles, pivots,
//! levels, events, legs, metrics, consensus, and the aggregate snapshot. It
//! must not call the reading or axis constructors itself.
//!
//! The module is pure: it reads the model it is given and performs no I/O.

use std::num::NonZeroU64;

use quantrule_data::Candle;
use quantrule_model::aggregate::TimeframeSnapshot;
use quantrule_model::axes::progression::ProgressionLeg;
use quantrule_model::axes::structure::registry::ConfirmedLevel;
use quantrule_model::events::EventRecord;
use quantrule_model::readings::PivotSeries;
use quantrule_model::run::TimeframeModel;
use quantrule_model::state::TimeframeConsensus;

use super::LayerError;
use super::metrics::{Metrics, ReadingsSnapshot};

/// One timeframe's model series, adapted for the layer builder.
///
/// The series borrows the model rather than owning it: the layers are a view of
/// the run, and a clone would double the peak memory of a full-year build.
pub(crate) struct TimeframeSeries<'a> {
    model: &'a TimeframeModel,
}

impl<'a> TimeframeSeries<'a> {
    /// Wraps an already computed model timeframe.
    pub(crate) fn new(model: &'a TimeframeModel) -> Result<Self, LayerError> {
        if NonZeroU64::new(model.timeframe_secs()).is_none() {
            return Err(LayerError::ZeroTimeframe);
        }
        Ok(Self { model })
    }

    /// The timeframe in seconds.
    pub(crate) fn timeframe_secs(&self) -> u64 {
        self.model.timeframe_secs()
    }

    /// The number of bars.
    pub(crate) fn len(&self) -> usize {
        self.model.len()
    }

    /// The pivot series computed for this timeframe.
    pub(crate) fn pivots(&self) -> &PivotSeries {
        self.model.pivots()
    }

    /// The confirmed levels for this timeframe.
    pub(crate) fn levels(&self) -> &[ConfirmedLevel] {
        self.model.levels()
    }

    /// The full event batch for this timeframe.
    pub(crate) fn events(&self) -> &[EventRecord] {
        self.model.events()
    }

    /// The retained progression legs for this timeframe.
    pub(crate) fn legs(&self) -> &[ProgressionLeg] {
        self.model.legs()
    }

    /// The candle at `bar`.
    pub(crate) fn candle(&self, bar: usize) -> Option<Candle> {
        self.model.candle(bar)
    }

    /// The readings at `bar`.
    pub(crate) fn readings_at(&self, bar: usize) -> Option<ReadingsSnapshot> {
        Some(ReadingsSnapshot::from_model(self.model.bar(bar)?.readings))
    }

    /// The published metrics at `bar`.
    pub(crate) fn metrics_at(&self, bar: usize) -> Option<Metrics> {
        let bar = self.model.bar(bar)?;
        Some(Metrics::from_bars(
            &bar.direction,
            &bar.momentum,
            &bar.structure,
            &bar.progression,
            &bar.context,
        ))
    }

    /// The model's cross-axis consensus at `bar`.
    pub(crate) fn consensus_at(&self, bar: usize) -> Option<TimeframeConsensus> {
        Some(self.model.bar(bar)?.consensus)
    }

    /// The bar open times, for the as-of search.
    pub(crate) fn open_times(&self) -> &[i64] {
        self.model.open_times()
    }

    /// The aggregate input at `bar`.
    pub(crate) fn snapshot_at(&self, bar: usize) -> Option<TimeframeSnapshot> {
        self.model.snapshot_at(bar)
    }
}
