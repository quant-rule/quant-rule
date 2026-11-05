//! The flat per-timeframe indicator/event report.
//!
//! This projection is the model v3 view: readings, discrete states, confirmed
//! levels, events, progression, and context. It carries no axis value, no
//! consensus, and no cross-timeframe aggregate. The report is a pure projection
//! of a completed [`crate::run::ModelRun`]; it never recomputes a reading or an
//! event.

use crate::axes::progression::ProgressionLeg;
use crate::axes::structure::registry::ConfirmedLevel;
use crate::events::{EventRecord, EventType};
use crate::readings::{PivotKind, PivotScale};
use crate::run::ModelRun;
use crate::timeframe::TimeframeModel;

/// The flat report across the configured ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatReport {
    /// One timeframe report per ladder member, ascending in seconds.
    pub timeframes: Vec<FlatTimeframe>,
}

/// One timeframe's flat report.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatTimeframe {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The number of closed bars.
    pub bars: usize,
    /// The expected bars over the closed span.
    pub expected_bars: usize,
    /// The missing bars over the closed span.
    pub missing_bars: usize,
    /// `bars / expected_bars` in `0..=1`, or `None`.
    pub coverage_share: Option<f64>,
    /// The first closed bar's open time, or `None`.
    pub first_open_time: Option<i64>,
    /// The last closed bar's open time, or `None`.
    pub last_open_time: Option<i64>,
    /// The latest published reading/state/context snapshot, or `None`.
    pub latest: Option<FlatLatest>,
    /// Every confirmed level, in confirmation order.
    pub levels: Vec<FlatLevel>,
    /// Event counts and the most recent records.
    pub events: FlatEventSeries,
    /// The latest progression leg at each scale.
    pub progression: Option<FlatProgression>,
}

/// The latest flat snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatLatest {
    /// The zero-based bar position.
    pub bar_index: usize,
    /// The bar's UTC open time.
    pub open_time: i64,
    /// The scalar readings.
    pub readings: FlatReadings,
    /// The discrete indicator states.
    pub states: FlatStates,
    /// Volatility and participation context.
    pub context: FlatContext,
}

/// The scalar readings at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatReadings {
    /// The fast EMA.
    pub ema_fast: Option<f64>,
    /// The middle EMA.
    pub ema_mid: Option<f64>,
    /// The slow EMA.
    pub ema_slow: Option<f64>,
    /// The RSI.
    pub rsi: Option<f64>,
    /// The ATR.
    pub atr: Option<f64>,
    /// The relative volume.
    pub rvol: Option<f64>,
    /// The MACD line.
    pub macd_line: Option<f64>,
    /// The MACD signal.
    pub macd_signal: Option<f64>,
    /// The MACD histogram.
    pub macd_histogram: Option<f64>,
    /// Average Directional Index.
    pub adx: Option<f64>,
    /// Positive directional indicator.
    pub di_plus: Option<f64>,
    /// Negative directional indicator.
    pub di_minus: Option<f64>,
}

/// The discrete indicator states at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatStates {
    /// +1 fast > mid > slow, -1 fast < mid < slow, 0 otherwise.
    pub ema_stack: Option<i8>,
    /// -1 rsi < 30, +1 rsi > 70, 0 otherwise.
    pub rsi_zone: Option<i8>,
    /// +1 line > signal, -1 line < signal, 0 equal.
    pub macd_state: Option<i8>,
    /// +1 histogram > 0, -1 histogram < 0, 0 equal.
    pub macd_histogram_state: Option<i8>,
    /// 1 ADX at or above its threshold, 0 below, null unavailable.
    pub adx_regime: Option<i8>,
    /// Sign of the ADX change over the configured lookback.
    pub adx_slope: Option<i8>,
    /// Sign of `+DI - -DI`.
    pub di_state: Option<i8>,
}

/// The volatility and participation context.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatContext {
    /// ATR / SMA(ATR, 20).
    pub atr_ratio: Option<f64>,
    /// Volume / SMA(volume, 20).
    pub rvol_ratio: Option<f64>,
    /// 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation.
    pub context_code: Option<u8>,
}

/// One confirmed level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatLevel {
    /// The stable reference code.
    pub reference_code: u32,
    /// 1 minor, 2 major.
    pub scale_code: u8,
    /// +1 swing high, -1 swing low.
    pub kind_code: i8,
    /// The level price.
    pub price: f64,
    /// The bar the extreme occurred on.
    pub occurred_bar_index: usize,
    /// The first bar the level became knowable.
    pub available_bar_index: usize,
    /// `(latest close - level) / latest ATR`, or `None`.
    pub distance_atr: Option<f64>,
}

/// Event counts and the most recent event records.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatEventSeries {
    /// The total event count.
    pub total: usize,
    /// Counts by type in Level, Divergence, Signal order.
    pub by_type: [usize; 3],
    /// Counts by subtype.
    pub by_subtype: FlatSubtypeCounts,
    /// The latest event per type and subtype, sorted by type then subtype.
    pub latest: Vec<FlatEvent>,
    /// The most recent records, oldest first.
    pub recent: Vec<FlatEvent>,
}

/// Counts by event subtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlatSubtypeCounts {
    /// Level subtypes 1..3: test, cross, break.
    pub level: [usize; 3],
    /// Divergence subtypes 1..4.
    pub divergence: [usize; 4],
    /// Signal subtypes 1..5: threshold, series, state, histogram flip, zero cross.
    pub signal: [usize; 5],
    /// A record with an unknown subtype.
    pub other: usize,
}

/// One flat event record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatEvent {
    /// The event type code.
    pub event_type_code: u8,
    /// The event subtype code.
    pub subtype_code: u8,
    /// The stable source code.
    pub source_code: u16,
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The bar the relationship describes.
    pub occurred_bar_index: usize,
    /// The first bar the event became knowable.
    pub available_bar_index: usize,
    /// -1 bearish, 0 neutral, +1 bullish.
    pub direction_code: i8,
    /// The level or series reference, when applicable.
    pub reference_code: Option<u32>,
    /// The main measured value.
    pub primary_value: Option<f64>,
    /// The compared value.
    pub reference_value: Option<f64>,
    /// Fraction of the event's required measured values that are present.
    pub data_completeness: Option<f64>,
}

/// The latest progression leg at each scale.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatProgression {
    /// The published leg's progress.
    pub progress: Option<f64>,
    /// The published leg's stage.
    pub stage_code: Option<i8>,
    /// The published leg's direction.
    pub leg_direction: Option<i8>,
    /// The published leg's scale.
    pub leg_scale_code: Option<u8>,
    /// The published leg.
    pub published: Option<FlatLeg>,
    /// The minor leg.
    pub minor: Option<FlatLeg>,
    /// The major leg.
    pub major: Option<FlatLeg>,
}

/// One progression leg.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlatLeg {
    /// The creation-order identifier.
    pub leg_id: u64,
    /// The timeframe in seconds.
    pub timeframe_secs: Option<u64>,
    /// 1 minor, 2 major.
    pub scale_code: u8,
    /// The origin price.
    pub origin_price: f64,
    /// The origin bar.
    pub origin_occurred_at: usize,
    /// The origin availability bar.
    pub origin_available_at: Option<usize>,
    /// 1 when the origin is confirmed.
    pub origin_confirmed_code: u8,
    /// +1 upward, -1 downward.
    pub direction_code: i8,
    /// The target price.
    pub target_price: Option<f64>,
    /// 1 structural, 2 ATR fallback.
    pub target_basis_code: Option<u8>,
    /// The target revision count.
    pub target_version: u32,
    /// 1 when the target is stable.
    pub target_stable_code: u8,
    /// The progress fraction.
    pub progress: Option<f64>,
    /// The price displacement.
    pub displacement_price: Option<f64>,
    /// The ATR displacement.
    pub displacement_atr: Option<f64>,
    /// 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded.
    pub lifecycle_code: u8,
    /// The ordinal stage.
    pub stage_code: Option<i8>,
    /// 1 when the target was reached.
    pub target_reached_code: Option<u8>,
}

/// The maximum number of recent events kept in the JSON report.
pub const RECENT_EVENT_LIMIT: usize = 200;

impl FlatReport {
    /// Projects a completed model run onto the flat report.
    #[must_use]
    pub fn compute(run: &ModelRun, ladder: &[u64]) -> Self {
        let mut ladder = ladder.to_vec();
        ladder.sort_unstable();
        ladder.dedup();
        let timeframes = ladder
            .iter()
            .map(|&timeframe_secs| {
                run.timeframe(timeframe_secs).map_or_else(
                    || FlatTimeframe::absent(timeframe_secs),
                    FlatTimeframe::compute,
                )
            })
            .collect();
        Self { timeframes }
    }
}

impl FlatTimeframe {
    fn compute(timeframe: &TimeframeModel) -> Self {
        let bars = timeframe.len();
        let first_open_time = timeframe.candle(0).map(|candle| candle.timestamp());
        let last_open_time = timeframe
            .candle(bars.saturating_sub(1))
            .map(|candle| candle.timestamp());
        let (expected_bars, missing_bars, coverage_share) = super::assemble::coverage(
            bars,
            first_open_time,
            last_open_time,
            timeframe.timeframe_secs(),
        );
        let latest = bars.checked_sub(1).and_then(|bar| {
            let candle = timeframe.candle(bar)?;
            let readings = timeframe.readings();
            let context = timeframe.context().get(bar)?;
            let macd = timeframe.macd();
            let adx = timeframe.adx();
            let adx_state = timeframe.adx_state(bar);
            Some(FlatLatest {
                bar_index: bar,
                open_time: candle.timestamp(),
                readings: FlatReadings {
                    ema_fast: readings.ema20.get(bar),
                    ema_mid: readings.ema50.get(bar),
                    ema_slow: readings.ema200.get(bar),
                    rsi: readings.rsi14.get(bar),
                    atr: readings.atr14.get(bar),
                    rvol: readings.rvol20.get(bar),
                    macd_line: macd.line.get(bar),
                    macd_signal: macd.signal.get(bar),
                    macd_histogram: macd.histogram.get(bar),
                    adx: adx.adx.get(bar),
                    di_plus: adx.di_plus.get(bar),
                    di_minus: adx.di_minus.get(bar),
                },
                states: FlatStates {
                    ema_stack: ema_stack(
                        readings.ema20.get(bar),
                        readings.ema50.get(bar),
                        readings.ema200.get(bar),
                    ),
                    rsi_zone: readings.rsi14.get(bar).map(rsi_zone),
                    macd_state: macd_stack(macd.line.get(bar), macd.signal.get(bar)),
                    macd_histogram_state: macd.histogram.get(bar).map(sign_state),
                    adx_regime: adx_state.adx_regime,
                    adx_slope: adx_state.adx_slope,
                    di_state: adx_state.di_state,
                },
                context: FlatContext {
                    atr_ratio: context.atr_ratio,
                    rvol_ratio: context.rvol_ratio,
                    context_code: context.context_code,
                },
            })
        });
        let latest_close = latest.as_ref().and_then(|latest| {
            timeframe
                .candle(latest.bar_index)
                .map(|candle| candle.close())
        });
        let latest_atr = latest.as_ref().and_then(|latest| latest.readings.atr);
        let levels = timeframe
            .levels()
            .iter()
            .map(|level| FlatLevel::from_level(level, latest_close, latest_atr))
            .collect();
        // The flat report keeps the v4 event domain; the extended sources
        // (19+) have their own surface. DI, trend, and compression events stay.
        let events: Vec<EventRecord> = timeframe
            .events()
            .iter()
            .filter(|event| event.source_code < 19)
            .copied()
            .collect();
        let events = FlatEventSeries::compute(&events);
        let progression = bars
            .checked_sub(1)
            .and_then(|bar| timeframe.progression().get(bar))
            .map(FlatProgression::from);
        Self {
            timeframe_secs: timeframe.timeframe_secs(),
            bars,
            expected_bars,
            missing_bars,
            coverage_share,
            first_open_time,
            last_open_time,
            latest,
            levels,
            events,
            progression,
        }
    }

    fn absent(timeframe_secs: u64) -> Self {
        Self {
            timeframe_secs,
            bars: 0,
            expected_bars: 0,
            missing_bars: 0,
            coverage_share: None,
            first_open_time: None,
            last_open_time: None,
            latest: None,
            levels: Vec::new(),
            events: FlatEventSeries::empty(),
            progression: None,
        }
    }
}

impl FlatLevel {
    fn from_level(level: &ConfirmedLevel, close: Option<f64>, atr: Option<f64>) -> Self {
        let distance_atr = match (close, atr) {
            (Some(close), Some(atr)) if atr > 0.0 => Some((close - level.price) / atr),
            _ => None,
        };
        Self {
            reference_code: level.reference_code,
            scale_code: match level.scale {
                PivotScale::Minor => 1,
                PivotScale::Major => 2,
            },
            kind_code: match level.kind {
                PivotKind::High => 1,
                PivotKind::Low => -1,
            },
            price: level.price,
            occurred_bar_index: level.occurred_at,
            available_bar_index: level.available_at,
            distance_atr,
        }
    }
}

impl FlatEventSeries {
    fn compute(events: &[EventRecord]) -> Self {
        let mut by_type = [0_usize; 3];
        let mut by_subtype = FlatSubtypeCounts {
            level: [0; 3],
            divergence: [0; 4],
            signal: [0; 5],
            other: 0,
        };
        let mut latest: std::collections::BTreeMap<(u8, u8), FlatEvent> =
            std::collections::BTreeMap::new();
        for event in events {
            let type_code = event.event_type.code();
            if let Some(count) = by_type.get_mut(usize::from(type_code).saturating_sub(1)) {
                *count += 1;
            }
            let subtype = usize::from(event.event_subtype_code).saturating_sub(1);
            match event.event_type {
                EventType::Level => {
                    if let Some(count) = by_subtype.level.get_mut(subtype) {
                        *count += 1;
                    } else {
                        by_subtype.other += 1;
                    }
                }
                EventType::Divergence => {
                    if let Some(count) = by_subtype.divergence.get_mut(subtype) {
                        *count += 1;
                    } else {
                        by_subtype.other += 1;
                    }
                }
                EventType::Signal => {
                    if let Some(count) = by_subtype.signal.get_mut(subtype) {
                        *count += 1;
                    } else {
                        by_subtype.other += 1;
                    }
                }
            }
            latest.insert(
                (type_code, event.event_subtype_code),
                FlatEvent::from_record(event),
            );
        }
        let recent = events
            .iter()
            .rev()
            .take(RECENT_EVENT_LIMIT)
            .rev()
            .map(FlatEvent::from_record)
            .collect();
        Self {
            total: events.len(),
            by_type,
            by_subtype,
            latest: latest.into_values().collect(),
            recent,
        }
    }

    fn empty() -> Self {
        Self {
            total: 0,
            by_type: [0; 3],
            by_subtype: FlatSubtypeCounts {
                level: [0; 3],
                divergence: [0; 4],
                signal: [0; 5],
                other: 0,
            },
            latest: Vec::new(),
            recent: Vec::new(),
        }
    }
}

impl FlatEvent {
    fn from_record(event: &EventRecord) -> Self {
        let required = match event.event_type {
            EventType::Level | EventType::Divergence => 2_u8,
            EventType::Signal => 1_u8,
        };
        let present =
            u8::from(event.primary_value.is_some()) + u8::from(event.reference_value.is_some());
        Self {
            event_type_code: event.event_type.code(),
            subtype_code: event.event_subtype_code,
            source_code: event.source_code,
            timeframe_secs: event.timeframe_secs,
            occurred_bar_index: event.occurred_bar_index,
            available_bar_index: event.available_bar_index,
            direction_code: event.direction_code,
            reference_code: event.reference_code,
            primary_value: event.primary_value,
            reference_value: event.reference_value,
            data_completeness: Some(f64::from(present.min(required)) / f64::from(required)),
        }
    }
}

impl FlatProgression {
    fn from(bar: crate::axes::progression::ProgressionBar) -> Self {
        Self {
            progress: bar.progress,
            stage_code: bar.stage_code,
            leg_direction: bar.leg_direction,
            leg_scale_code: bar.leg_scale_code,
            published: bar.published_leg.map(FlatLeg::from),
            minor: bar.minor_leg.map(FlatLeg::from),
            major: bar.major_leg.map(FlatLeg::from),
        }
    }
}

impl From<ProgressionLeg> for FlatLeg {
    fn from(leg: ProgressionLeg) -> Self {
        Self {
            leg_id: leg.leg_id,
            timeframe_secs: leg.timeframe_secs,
            scale_code: leg.scale_code,
            origin_price: leg.origin_price,
            origin_occurred_at: leg.origin_occurred_at,
            origin_available_at: leg.origin_available_at,
            origin_confirmed_code: leg.origin_confirmed_code,
            direction_code: leg.direction_code,
            target_price: leg.target_price,
            target_basis_code: leg.target_basis_code,
            target_version: leg.target_version,
            target_stable_code: leg.target_stable_code,
            progress: leg.progress,
            displacement_price: leg.displacement_price,
            displacement_atr: leg.displacement_atr,
            lifecycle_code: leg.lifecycle_code,
            stage_code: leg.stage_code,
            target_reached_code: leg.target_reached_code,
        }
    }
}

fn ema_stack(fast: Option<f64>, mid: Option<f64>, slow: Option<f64>) -> Option<i8> {
    let (fast, mid, slow) = (fast?, mid?, slow?);
    if fast > mid && mid > slow {
        Some(1)
    } else if fast < mid && mid < slow {
        Some(-1)
    } else {
        Some(0)
    }
}

fn macd_stack(line: Option<f64>, signal: Option<f64>) -> Option<i8> {
    let (line, signal) = (line?, signal?);
    if line > signal {
        Some(1)
    } else if line < signal {
        Some(-1)
    } else {
        Some(0)
    }
}

fn sign_state(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

fn rsi_zone(rsi: f64) -> i8 {
    if rsi < 30.0 {
        -1
    } else if rsi > 70.0 {
        1
    } else {
        0
    }
}
