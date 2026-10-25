//! The per-timeframe model series and the per-bar snapshot assembly.
//!
//! Layer 3's cross-timeframe aggregates are defined over the model's
//! [`TimeframeSnapshot`]. This module builds a snapshot at any bar from the
//! model's public per-bar series. It reads only bars at or before the requested
//! one, and it bounds the snapshot's event list to the five-bar event window
//! plus the latest event, which is everything the cross-timeframe aggregate
//! reads. The full event history is not cloned per row.

use std::num::NonZeroU64;

use quantrule_data::Candle;
use quantrule_model::aggregate::events::EVENT_WINDOW_BARS;
use quantrule_model::aggregate::{ReferenceSet, TimeframeSnapshot};
use quantrule_model::axes::direction::DirectionSeries;
use quantrule_model::axes::momentum::MomentumSeries;
use quantrule_model::axes::progression::{ProgressionConfig, ProgressionLeg, ProgressionSeries};
use quantrule_model::axes::structure::registry::{ConfirmedLevel, ConfirmedLevelRegistry};
use quantrule_model::axes::structure::{StructureConfig, StructureSeries};
use quantrule_model::context::ContextSeries;
use quantrule_model::events::EventRecord;
use quantrule_model::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use quantrule_model::events::level::{LevelEventConfig, LevelEventSeries};
use quantrule_model::events::signal::{SignalConfig, SignalEventSeries};
use quantrule_model::readings::{PivotKind, PivotScale, PivotSeries, Readings};
use quantrule_model::state::{TimeframeConsensus, TimeframeInput};

use super::metrics::{Metrics, ReadingsSnapshot};
use super::{LayerConfig, LayerError};

/// One timeframe's model series, with a monotone snapshot cursor.
///
/// The level cursor assumes snapshot requests arrive with non-decreasing bar
/// indices, which is how the layer builder walks a timeframe. A request for an
/// earlier bar after a later one would leave the cursor ahead of the bar.
pub(crate) struct TimeframeSeries<'a> {
    timeframe_secs: u64,
    candles: &'a [Candle],
    open_times: Vec<i64>,
    readings: Readings,
    direction: DirectionSeries,
    momentum: MomentumSeries,
    structure: StructureSeries,
    progression: ProgressionSeries,
    context: ContextSeries,
    levels: Vec<ConfirmedLevel>,
    events: Vec<EventRecord>,
    level_cursor: usize,
    swings: LatestSwings,
}

/// The four latest confirmed swing levels knowable at the cursor.
#[derive(Debug, Clone, Copy, Default)]
struct LatestSwings {
    minor_high: Option<ConfirmedLevel>,
    minor_low: Option<ConfirmedLevel>,
    major_high: Option<ConfirmedLevel>,
    major_low: Option<ConfirmedLevel>,
}

impl<'a> TimeframeSeries<'a> {
    /// Computes every model series for one timeframe.
    pub(crate) fn build(
        input: &TimeframeInput<'a>,
        config: &LayerConfig,
    ) -> Result<Self, LayerError> {
        let timeframe = NonZeroU64::new(input.timeframe_secs).ok_or(LayerError::ZeroTimeframe)?;
        let readings = Readings::compute(input.candles);
        let direction = DirectionSeries::compute(&readings);
        let momentum = MomentumSeries::compute(&readings);
        let structure = StructureSeries::compute_with_config(
            input.candles,
            &readings,
            StructureConfig::new(config.structure_scale),
        );
        let progression = ProgressionSeries::compute_with_config(
            input.candles,
            &readings,
            ProgressionConfig::new(input.timeframe_secs),
        );
        let context = ContextSeries::compute(&readings);

        let levels = ConfirmedLevelRegistry::from_pivots(&readings.pivots)
            .levels()
            .to_vec();

        let level_events =
            LevelEventSeries::compute(input.candles, &readings, &LevelEventConfig::new(timeframe));
        let divergence_events = DivergenceEventSeries::compute(
            &readings,
            &DivergenceConfig::new(timeframe, config.structure_scale),
        );
        let signal_events = SignalEventSeries::compute(
            &readings,
            &direction,
            &momentum,
            &structure,
            &SignalConfig::new(timeframe),
        );
        let mut events: Vec<EventRecord> = level_events
            .events()
            .iter()
            .chain(divergence_events.events())
            .chain(signal_events.events())
            .copied()
            .collect();
        events.sort_by_key(|event| {
            (
                event.available_bar_index,
                event.occurred_bar_index,
                event.event_type.code(),
                event.event_subtype_code,
                event.source_code,
                event.reference_code.unwrap_or(0),
            )
        });

        let open_times = input.candles.iter().map(Candle::timestamp).collect();

        Ok(Self {
            timeframe_secs: input.timeframe_secs,
            candles: input.candles,
            open_times,
            readings,
            direction,
            momentum,
            structure,
            progression,
            context,
            levels,
            events,
            level_cursor: 0,
            swings: LatestSwings::default(),
        })
    }

    /// The timeframe in seconds.
    pub(crate) fn timeframe_secs(&self) -> u64 {
        self.timeframe_secs
    }

    /// The number of bars.
    pub(crate) fn len(&self) -> usize {
        self.candles.len()
    }

    /// The pivot series computed for this timeframe.
    pub(crate) fn pivots(&self) -> &PivotSeries {
        &self.readings.pivots
    }

    /// The confirmed-level registry for this timeframe.
    pub(crate) fn levels(&self) -> &[ConfirmedLevel] {
        &self.levels
    }

    /// The event records computed for this timeframe.
    pub(crate) fn events(&self) -> &[EventRecord] {
        &self.events
    }

    /// The retained progression legs for this timeframe.
    pub(crate) fn legs(&self) -> &[ProgressionLeg] {
        self.progression.legs()
    }

    /// The candle at `bar`.
    pub(crate) fn candle(&self, bar: usize) -> Option<Candle> {
        self.candles.get(bar).copied()
    }

    /// The readings at `bar`.
    pub(crate) fn readings_at(&self, bar: usize) -> Option<ReadingsSnapshot> {
        (bar < self.len()).then(|| ReadingsSnapshot::at(&self.readings, bar))
    }

    /// The published metrics at `bar`.
    pub(crate) fn metrics_at(&self, bar: usize) -> Option<Metrics> {
        Some(Metrics::from_bars(
            &self.direction.get(bar)?,
            &self.momentum.get(bar)?,
            &self.structure.get(bar)?,
            &self.progression.get(bar)?,
            &self.context.get(bar)?,
        ))
    }

    /// The model's cross-axis consensus at `bar`.
    pub(crate) fn consensus_at(&self, bar: usize) -> Option<TimeframeConsensus> {
        Some(TimeframeConsensus::from_bars(
            self.timeframe_secs,
            &self.direction.get(bar)?,
            &self.momentum.get(bar)?,
            &self.structure.get(bar)?,
        ))
    }

    /// The bar open times, for the as-of search.
    pub(crate) fn open_times(&self) -> &[i64] {
        &self.open_times
    }

    /// Builds the aggregate input at `bar`.
    ///
    /// The snapshot carries only the events the cross-timeframe aggregate
    /// reads: those that became knowable in the five-bar window, plus the
    /// latest event when it is older. The per-timeframe event summary's
    /// `latest_event_*` fields therefore stay correct while the full history is
    /// never cloned.
    pub(crate) fn snapshot_at(&mut self, bar: usize) -> Option<TimeframeSnapshot> {
        self.advance_levels(bar);
        let direction = self.direction.get(bar)?;
        let momentum = self.momentum.get(bar)?;
        let structure = self.structure.get(bar)?;
        let progression = self.progression.get(bar)?;
        let context = self.context.get(bar)?;

        let references = ReferenceSet::from_parts(
            self.readings.ema20.get(bar),
            self.readings.ema50.get(bar),
            self.readings.ema200.get(bar),
            self.swings.minor_high.map(|level| level.price),
            self.swings.minor_low.map(|level| level.price),
            self.swings.major_high.map(|level| level.price),
            self.swings.major_low.map(|level| level.price),
        );

        let minor_origin_time_secs = progression.minor_leg.and_then(|leg| {
            self.candles
                .get(leg.origin_occurred_at)
                .map(Candle::timestamp)
        });
        let major_origin_time_secs = progression.major_leg.and_then(|leg| {
            self.candles
                .get(leg.origin_occurred_at)
                .map(Candle::timestamp)
        });

        Some(TimeframeSnapshot {
            timeframe_secs: self.timeframe_secs,
            latest_bar_index: bar,
            direction,
            momentum,
            structure,
            progression,
            context,
            atr: self.readings.atr14.get(bar),
            references,
            events: self.retained_events(bar),
            minor_origin_time_secs,
            major_origin_time_secs,
        })
    }

    /// Advances the level cursor to every level knowable at `bar`.
    fn advance_levels(&mut self, bar: usize) {
        while let Some(level) = self.levels.get(self.level_cursor).copied() {
            if level.available_at > bar {
                break;
            }
            match (level.scale, level.kind) {
                (PivotScale::Minor, PivotKind::High) => self.swings.minor_high = Some(level),
                (PivotScale::Minor, PivotKind::Low) => self.swings.minor_low = Some(level),
                (PivotScale::Major, PivotKind::High) => self.swings.major_high = Some(level),
                (PivotScale::Major, PivotKind::Low) => self.swings.major_low = Some(level),
            }
            self.level_cursor += 1;
        }
    }

    /// The events that can contribute to the aggregate at `bar`.
    fn retained_events(&self, bar: usize) -> Vec<EventRecord> {
        let window_start = bar.saturating_sub(EVENT_WINDOW_BARS - 1);
        let begin = self
            .events
            .partition_point(|event| event.available_bar_index < window_start);
        let end = self
            .events
            .partition_point(|event| event.available_bar_index <= bar);

        let mut retained = Vec::with_capacity(end.saturating_sub(begin) + 1);
        if end > 0 && end - 1 < begin {
            retained.push(self.events[end - 1]);
        }
        retained.extend_from_slice(&self.events[begin..end]);
        retained
    }
}
