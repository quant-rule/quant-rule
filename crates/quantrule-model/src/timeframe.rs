//! One timeframe's complete model series.
//!
//! The module owns the per-bar readings, axes, context, events, confirmed
//! levels, and the latest snapshot. It reads only the candles and configuration
//! it is given; it performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::aggregate::events::EVENT_WINDOW_BARS;
use crate::aggregate::{ReferenceSet, TimeframeSnapshot};
use crate::axes::direction::{DirectionBar, DirectionSeries};
use crate::axes::momentum::{MomentumBar, MomentumSeries};
use crate::axes::progression::{
    ProgressionBar, ProgressionConfig, ProgressionLeg, ProgressionSeries,
};
use crate::axes::structure::registry::{ConfirmedLevel, ConfirmedLevelRegistry};
use crate::axes::structure::{StructureBar, StructureConfig, StructureSeries};
use crate::consensus::TimeframeConsensus;
use crate::context::ContextBar;
use crate::context::ContextSeries;
use crate::events::EventRecord;
use crate::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use crate::events::level::{LevelEventConfig, LevelEventSeries};
use crate::events::signal::{SignalConfig, SignalEventSeries};
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::readings::{PivotKind, PivotScale, PivotSeries, Readings};
use crate::retention::{EventBatch, EventHistory};

/// The six scalar readings for one closed bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelReadings {
    /// EMA 20.
    pub ema20: Option<f64>,
    /// EMA 50.
    pub ema50: Option<f64>,
    /// EMA 200.
    pub ema200: Option<f64>,
    /// RSI 14.
    pub rsi14: Option<f64>,
    /// ATR 14.
    pub atr14: Option<f64>,
    /// Relative volume.
    pub rvol20: Option<f64>,
}

impl ModelReadings {
    fn at(readings: &Readings, bar: usize) -> Self {
        Self {
            ema20: readings.ema20.get(bar),
            ema50: readings.ema50.get(bar),
            ema200: readings.ema200.get(bar),
            rsi14: readings.rsi14.get(bar),
            atr14: readings.atr14.get(bar),
            rvol20: readings.rvol20.get(bar),
        }
    }
}

/// The complete published model state for one closed bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelBar {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position in the timeframe series.
    pub bar_index: usize,
    /// The candle.
    pub candle: Candle,
    /// The six scalar readings.
    pub readings: ModelReadings,
    /// Direction.
    pub direction: DirectionBar,
    /// Momentum.
    pub momentum: MomentumBar,
    /// Structure.
    pub structure: StructureBar,
    /// Progression.
    pub progression: ProgressionBar,
    /// Context.
    pub context: ContextBar,
    /// The cross-axis consensus for this bar.
    pub consensus: TimeframeConsensus,
}

/// One timeframe's complete model series and records.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeModel {
    timeframe_secs: u64,
    candles: Vec<Candle>,
    open_times: Vec<i64>,
    readings: Readings,
    direction: DirectionSeries,
    momentum: MomentumSeries,
    structure: StructureSeries,
    progression: ProgressionSeries,
    context: ContextSeries,
    events: EventBatch,
    event_history: EventHistory,
    levels: Vec<ConfirmedLevel>,
    snapshot: TimeframeSnapshot,
}

impl TimeframeModel {
    /// Computes the model for one non-empty timeframe.
    ///
    /// Returns `None` when the timeframe is zero or the candle series is empty.
    #[must_use]
    pub fn compute(input: &TimeframeInput<'_>, config: &MarketStateConfig) -> Option<Self> {
        let timeframe_secs = NonZeroU64::new(input.timeframe_secs)?;
        if input.candles.is_empty() {
            return None;
        }

        let readings_config = config.readings_for(input.timeframe_secs);
        let readings = Readings::compute_with_config(input.candles, &readings_config);
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

        let level_events = LevelEventSeries::compute(
            input.candles,
            &readings,
            &LevelEventConfig::new(timeframe_secs),
        );
        let divergence_events = DivergenceEventSeries::compute(
            &readings,
            &DivergenceConfig::new(timeframe_secs, config.structure_scale),
        );
        let signal_events = SignalEventSeries::compute(
            &readings,
            &direction,
            &momentum,
            &structure,
            &SignalConfig::new(timeframe_secs),
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

        let events = EventBatch::new(events);
        let event_history = EventHistory::from_batch(&events);
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let levels: Vec<ConfirmedLevel> = registry.levels().to_vec();
        let latest = input.candles.len().checked_sub(1)?;
        let snapshot = snapshot_at(
            SnapshotInputs {
                timeframe_secs: input.timeframe_secs,
                candles: input.candles,
                readings: &readings,
                direction: &direction,
                momentum: &momentum,
                structure: &structure,
                progression: &progression,
                context: &context,
                events: event_history.as_slice(),
                levels: &levels,
            },
            latest,
            true,
        )?;

        let open_times = input.candles.iter().map(Candle::timestamp).collect();

        Some(Self {
            timeframe_secs: input.timeframe_secs,
            candles: input.candles.to_vec(),
            open_times,
            readings,
            direction,
            momentum,
            structure,
            progression,
            context,
            events,
            event_history,
            levels,
            snapshot,
        })
    }

    /// The source timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(&self) -> u64 {
        self.timeframe_secs
    }

    /// The number of closed bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candles.len()
    }

    /// Whether the timeframe has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    /// The candle at `bar`, or `None` when out of range.
    #[must_use]
    pub fn candle(&self, bar: usize) -> Option<Candle> {
        self.candles.get(bar).copied()
    }

    /// The pivot series computed for this timeframe.
    #[must_use]
    pub fn pivots(&self) -> &PivotSeries {
        &self.readings.pivots
    }

    /// The Direction series for this timeframe.
    ///
    /// The dashboard reads the published bars to derive bias tenure; the axis
    /// itself is not recomputed.
    #[must_use]
    pub const fn direction(&self) -> &DirectionSeries {
        &self.direction
    }

    /// The Progression series for this timeframe.
    ///
    /// The dashboard reads the published bars to derive recent pace; the axis
    /// itself is not recomputed.
    #[must_use]
    pub const fn progression(&self) -> &ProgressionSeries {
        &self.progression
    }

    /// The Momentum series for this timeframe.
    ///
    /// The market report reads the published bars for its statistics; the axis
    /// itself is not recomputed.
    #[must_use]
    pub const fn momentum(&self) -> &MomentumSeries {
        &self.momentum
    }

    /// The Structure series for this timeframe.
    ///
    /// The market report reads the published bars for its statistics; the axis
    /// itself is not recomputed.
    #[must_use]
    pub const fn structure(&self) -> &StructureSeries {
        &self.structure
    }

    /// The context series for this timeframe.
    ///
    /// The market report reads the latest published context; it is not
    /// recomputed.
    #[must_use]
    pub const fn context(&self) -> &ContextSeries {
        &self.context
    }

    /// The retained progression legs for this timeframe.
    #[must_use]
    pub fn legs(&self) -> &[ProgressionLeg] {
        self.progression.legs()
    }

    /// The bar open times, for the as-of search.
    #[must_use]
    pub fn open_times(&self) -> &[i64] {
        &self.open_times
    }

    /// The complete model bar at `bar`, or `None` when out of range.
    #[must_use]
    pub fn bar(&self, bar: usize) -> Option<ModelBar> {
        let candle = self.candles.get(bar).copied()?;
        let direction = self.direction.get(bar)?;
        let momentum = self.momentum.get(bar)?;
        let structure = self.structure.get(bar)?;
        let progression = self.progression.get(bar)?;
        let context = self.context.get(bar)?;
        Some(ModelBar {
            timeframe_secs: self.timeframe_secs,
            bar_index: bar,
            candle,
            readings: ModelReadings::at(&self.readings, bar),
            direction,
            momentum,
            structure,
            progression,
            context,
            consensus: TimeframeConsensus::from_bars(
                self.timeframe_secs,
                &direction,
                &momentum,
                &structure,
            ),
        })
    }

    /// The latest complete model bar.
    #[must_use]
    pub fn latest_bar(&self) -> Option<ModelBar> {
        self.len().checked_sub(1).and_then(|bar| self.bar(bar))
    }

    /// The full event batch, in availability order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        self.events.as_slice()
    }

    /// The full event batch wrapper.
    #[must_use]
    pub const fn event_batch(&self) -> &EventBatch {
        &self.events
    }

    /// The bounded event history published with the latest snapshot.
    #[must_use]
    pub const fn event_history(&self) -> &EventHistory {
        &self.event_history
    }

    /// The confirmed levels, in confirmation order.
    #[must_use]
    pub fn levels(&self) -> &[ConfirmedLevel] {
        &self.levels
    }

    /// The latest published snapshot.
    ///
    /// The snapshot carries the full event batch; aggregate consumers should
    /// use [`Self::snapshot_at`], which keeps only the five-bar window and the
    /// latest event.
    #[must_use]
    pub const fn snapshot(&self) -> &TimeframeSnapshot {
        &self.snapshot
    }

    /// The aggregate input at `bar`, or `None` when out of range.
    ///
    /// The event list is bounded to the five-bar window plus the latest
    /// earlier event, which is everything the cross-timeframe aggregate reads.
    #[must_use]
    pub fn snapshot_at(&self, bar: usize) -> Option<TimeframeSnapshot> {
        snapshot_at(
            SnapshotInputs {
                timeframe_secs: self.timeframe_secs,
                candles: &self.candles,
                readings: &self.readings,
                direction: &self.direction,
                momentum: &self.momentum,
                structure: &self.structure,
                progression: &self.progression,
                context: &self.context,
                events: self.events.as_slice(),
                levels: &self.levels,
            },
            bar,
            false,
        )
    }

    /// The last `count` complete model bars, oldest first.
    #[must_use]
    pub fn tail(&self, count: usize) -> Vec<ModelBar> {
        let start = self.len().saturating_sub(count);
        (start..self.len())
            .filter_map(|bar| self.bar(bar))
            .collect()
    }
}

/// The data a snapshot reads from one timeframe.
struct SnapshotInputs<'a> {
    timeframe_secs: u64,
    candles: &'a [Candle],
    readings: &'a Readings,
    direction: &'a DirectionSeries,
    momentum: &'a MomentumSeries,
    structure: &'a StructureSeries,
    progression: &'a ProgressionSeries,
    context: &'a ContextSeries,
    events: &'a [EventRecord],
    levels: &'a [ConfirmedLevel],
}

fn snapshot_at(
    inputs: SnapshotInputs<'_>,
    bar: usize,
    full_events: bool,
) -> Option<TimeframeSnapshot> {
    let SnapshotInputs {
        timeframe_secs,
        candles,
        readings,
        direction,
        momentum,
        structure,
        progression,
        context,
        events,
        levels,
    } = inputs;
    let direction = direction.get(bar)?;
    let momentum = momentum.get(bar)?;
    let structure = structure.get(bar)?;
    let progression = progression.get(bar)?;
    let context = context.get(bar)?;
    let (minor_high, minor_low, major_high, major_low) = latest_swings_at(levels, bar);

    let references = ReferenceSet::from_parts(
        readings.ema20.get(bar),
        readings.ema50.get(bar),
        readings.ema200.get(bar),
        minor_high.map(|level| level.price),
        minor_low.map(|level| level.price),
        major_high.map(|level| level.price),
        major_low.map(|level| level.price),
    );

    let retained = if full_events {
        events.to_vec()
    } else {
        let window_start = bar.saturating_sub(EVENT_WINDOW_BARS - 1);
        let begin = events.partition_point(|event| event.available_bar_index < window_start);
        let end = events.partition_point(|event| event.available_bar_index <= bar);
        let mut retained = Vec::with_capacity(end.saturating_sub(begin) + 1);
        if end > 0 && end - 1 < begin {
            retained.push(events[end - 1]);
        }
        retained.extend_from_slice(&events[begin..end]);
        retained
    };

    let minor_origin_time_secs = progression
        .minor_leg
        .and_then(|leg| candles.get(leg.origin_occurred_at).map(Candle::timestamp));
    let major_origin_time_secs = progression
        .major_leg
        .and_then(|leg| candles.get(leg.origin_occurred_at).map(Candle::timestamp));

    Some(TimeframeSnapshot {
        timeframe_secs,
        latest_bar_index: bar,
        direction,
        momentum,
        structure,
        progression,
        context,
        atr: readings.atr14.get(bar),
        references,
        events: retained,
        minor_origin_time_secs,
        major_origin_time_secs,
    })
}

fn latest_swings_at(
    levels: &[ConfirmedLevel],
    bar: usize,
) -> (
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
    Option<ConfirmedLevel>,
) {
    let mut minor_high = None;
    let mut minor_low = None;
    let mut major_high = None;
    let mut major_low = None;
    let end = levels.partition_point(|level| level.available_at <= bar);

    // Levels are in confirmation order. Walking back from the latest one
    // finds the four latest swings without scanning the whole registry for
    // every M1 bar.
    for level in levels[..end].iter().rev() {
        match (level.scale, level.kind) {
            (PivotScale::Minor, PivotKind::High) if minor_high.is_none() => {
                minor_high = Some(*level);
            }
            (PivotScale::Minor, PivotKind::Low) if minor_low.is_none() => {
                minor_low = Some(*level);
            }
            (PivotScale::Major, PivotKind::High) if major_high.is_none() => {
                major_high = Some(*level);
            }
            (PivotScale::Major, PivotKind::Low) if major_low.is_none() => {
                major_low = Some(*level);
            }
            _ => {}
        }
        if minor_high.is_some()
            && minor_low.is_some()
            && major_high.is_some()
            && major_low.is_some()
        {
            break;
        }
    }
    (minor_high, minor_low, major_high, major_low)
}
