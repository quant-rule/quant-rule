//! The full-period descriptive market report.
//!
//! [`MarketReport`] summarizes a completed [`crate::run::ModelRun`] over the
//! configured timeframe ladder: coverage, per-axis statistics and ordinal
//! histograms, full-period event counts, the latest published state, the
//! existing cross-timeframe aggregates, and a deterministic chart projection.
//!
//! The view is descriptive. It publishes no order, fill, position, trade,
//! equity, drawdown, or P&L field, and it applies no strategy. It is a pure
//! projection of the run and the ladder: no I/O, no clock, no randomness, and
//! no dependence on unordered iteration.
//!
//! The contract is `docs/model/2026-10-27/market-report.md`. The shell renders
//! this type; it does not compute any statistic in it.
//!
//! # Examples
//!
//! ```
//! use quantrule_model::report::MarketReport;
//! use quantrule_model::run::ModelRun;
//! use quantrule_model::state::MarketStateConfig;
//!
//! let run = ModelRun::compute(&[], &MarketStateConfig::default());
//! let report = MarketReport::compute(&run, &[]);
//!
//! assert!(report.ladder.is_empty());
//! assert!(report.timeframes.is_empty());
//! assert_eq!(report.period.first_open_time, None);
//! assert_eq!(report.charts.timeframe_secs, None);
//! ```

mod assemble;
mod chart;
mod order;
#[cfg(test)]
mod tests;

use crate::aggregate::{CrossTimeframeAggregates, EventSubtypeCounts};
use crate::readings::roles::AxisCode;

/// The maximum number of buckets in the report chart.
pub const CHART_BUCKET_LIMIT: usize = 240;

/// The timeframe the report chart prefers, in seconds.
pub const CHART_TIMEFRAME_SECS: u64 = 3_600;

/// The full-period descriptive view of one model run.
///
/// The constructor sorts and deduplicates the ladder, so the input order
/// cannot change the output. A ladder entry with no series in the run produces
/// a zero-bar [`TimeframeReport`]; it is absent, not neutral.
#[derive(Debug, Clone, PartialEq)]
pub struct MarketReport {
    /// The earliest first bar and latest last bar across present timeframes.
    pub period: ReportPeriod,
    /// The normalized configured ladder, ascending in seconds.
    pub ladder: Vec<u64>,
    /// One report per ladder member, in ascending timeframe order.
    pub timeframes: Vec<TimeframeReport>,
    /// The existing cross-timeframe aggregates at the run's latest bar.
    pub cross_timeframe: CrossTimeframeAggregates,
    /// The full-period event counts, separate from the five-bar aggregates.
    pub events: EventReport,
    /// The deterministic chart projection.
    pub charts: ChartReport,
}

/// The actual coverage bounds of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportPeriod {
    /// The earliest first bar across present timeframes, or `None` when the
    /// report has no present timeframe.
    pub first_open_time: Option<i64>,
    /// The latest last bar across present timeframes, or `None` when the
    /// report has no present timeframe.
    pub last_open_time: Option<i64>,
}

/// Coverage and statistics for one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeReport {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The number of closed bars in the model series.
    pub bars: usize,
    /// `(last_open_time - first_open_time) / timeframe_secs + 1` when
    /// `bars > 0`, and zero otherwise.
    pub expected_bars: usize,
    /// `expected_bars - bars`, saturating at zero.
    pub missing_bars: usize,
    /// `bars / expected_bars` in `0..=1`, or `None` when `expected_bars = 0`.
    pub coverage_share: Option<f64>,
    /// The first closed bar's open time, or `None` when `bars = 0`.
    pub first_open_time: Option<i64>,
    /// The last closed bar's open time, or `None` when `bars = 0`.
    pub last_open_time: Option<i64>,
    /// Exactly four axis reports, in [`AxisCode::ALL`] order.
    pub axes: Vec<AxisReport>,
    /// The complete published state at the last closed bar, or `None` when the
    /// timeframe has no bars.
    pub latest: Option<LatestState>,
}

/// Present/null counts, range, mean, and ordinal histogram for one axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisReport {
    /// The axis this report describes.
    pub axis: AxisCode,
    /// Bars where the primary value is non-null.
    pub present_count: usize,
    /// Bars where the primary value is null.
    pub null_count: usize,
    /// The minimum non-null primary value, or `None` when none is present.
    pub min: Option<f64>,
    /// The maximum non-null primary value, or `None` when none is present.
    pub max: Option<f64>,
    /// The mean of the non-null primary values in ascending bar order, or
    /// `None` when none is present.
    pub mean: Option<f64>,
    /// The ordinal code histogram, indexed by the mapping in the contract.
    pub code_counts: Vec<u32>,
    /// Bars where the ordinal code is null.
    pub code_null_count: usize,
}

/// The model's complete published state at the last closed bar.
#[derive(Debug, Clone, PartialEq)]
pub struct LatestState {
    /// The zero-based bar position in the timeframe series.
    pub bar_index: usize,
    /// The bar's UTC open time in Unix seconds.
    pub open_time: i64,
    /// The six scalar readings.
    pub readings: ReportReadings,
    /// Direction.
    pub direction: ReportDirection,
    /// Momentum.
    pub momentum: ReportMomentum,
    /// Structure.
    pub structure: ReportStructure,
    /// Progression.
    pub progression: ReportProgression,
    /// Volatility and participation context.
    pub context: ReportContext,
    /// The per-timeframe directional consensus counts.
    pub consensus: ReportConsensus,
}

/// The six scalar readings at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportReadings {
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

/// The published Direction state at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportDirection {
    /// Directional alignment in `[-1, +1]`, or `None`.
    pub direction: Option<f64>,
    /// Compact state -1, 0, or +1, or `None`.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute magnitude, or `None`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Unavailable reason code 1..6, or `None` when available.
    pub reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The published Momentum state at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportMomentum {
    /// Signed RSI force in `[-1, +1]`, or `None`.
    pub force: Option<f64>,
    /// Signed change in force over three bars, or `None`.
    pub acceleration: Option<f64>,
    /// Signed change in the force magnitude over three bars, or `None`.
    pub intensification: Option<f64>,
    /// Compact state -1, 0, or +1, or `None`.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute magnitude, or `None`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Unavailable reason code 1..6, or `None` when available.
    pub reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The published Structure state at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportStructure {
    /// Relation of the latest two confirmed swing highs, or `None`.
    pub high_relation_code: Option<i8>,
    /// Relation of the latest two confirmed swing lows, or `None`.
    pub low_relation_code: Option<i8>,
    /// The swing score in `[-1, +1]`, or `None`.
    pub score: Option<f64>,
    /// Compact swing state -2..=+2, or `None`.
    pub state_code: Option<i8>,
    /// Pivot scale: 1 minor, 2 major.
    pub scale_code: u8,
    /// `(Close - EMA 20) / ATR 14`, or `None`.
    pub ema_distance_atr_20: Option<f64>,
    /// `(Close - EMA 50) / ATR 14`, or `None`.
    pub ema_distance_atr_50: Option<f64>,
    /// `(Close - EMA 200) / ATR 14`, or `None`.
    pub ema_distance_atr_200: Option<f64>,
    /// Close/EMA 20 cross code, or `None`.
    pub ema_cross_code_20: Option<i8>,
    /// Close/EMA 50 cross code, or `None`.
    pub ema_cross_code_50: Option<i8>,
    /// Close/EMA 200 cross code, or `None`.
    pub ema_cross_code_200: Option<i8>,
    /// EMA 20 zone retest code, or `None`.
    pub ema_retest_code_20: Option<i8>,
    /// EMA 50 zone retest code, or `None`.
    pub ema_retest_code_50: Option<i8>,
    /// EMA 200 zone retest code, or `None`.
    pub ema_retest_code_200: Option<i8>,
    /// Empirical percentile of the absolute score, or `None`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Unavailable reason code 1..6, or `None` when available.
    pub reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The published Progression state at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportProgression {
    /// The published leg's progress, unclamped, or `None`.
    pub progress: Option<f64>,
    /// The published leg's ordinal stage 0..=5, or `None`.
    pub stage_code: Option<i8>,
    /// The published leg's direction +1 or -1, or `None`.
    pub leg_direction: Option<i8>,
    /// The published leg's scale 1 or 2, or `None`.
    pub leg_scale_code: Option<u8>,
    /// Empirical percentile of the published displacement, or `None`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Unavailable reason code 1..6, or `None` when available.
    pub reason_code: Option<u8>,
    /// Closed bars the published stage has held.
    pub state_age_bars: u64,
    /// Published stage changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
    /// The published leg record, or `None`.
    pub leg: Option<ReportLeg>,
}

/// The published progression leg at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportLeg {
    /// The per-series creation-order identifier.
    pub leg_id: u64,
    /// The timeframe in seconds, or `None`.
    pub timeframe_secs: Option<u64>,
    /// The pivot scale: 1 minor, 2 major.
    pub scale_code: u8,
    /// The candidate price that became the origin.
    pub origin_price: f64,
    /// The bar the origin pivot occurred on.
    pub origin_occurred_at: usize,
    /// The bar the origin became knowable, or `None` while forming.
    pub origin_available_at: Option<usize>,
    /// 1 when the origin pivot is confirmed, 0 while forming.
    pub origin_confirmed_code: u8,
    /// +1 for an upward leg, -1 for a downward leg.
    pub direction_code: i8,
    /// The selected target price, or `None`.
    pub target_price: Option<f64>,
    /// 1 confirmed structural level, 2 ATR fallback, or `None`.
    pub target_basis_code: Option<u8>,
    /// The number of target revisions.
    pub target_version: u32,
    /// 1 when the target is stable, 0 otherwise.
    pub target_stable_code: u8,
    /// `(Close - origin) / (target - origin)`, unclamped.
    pub progress: Option<f64>,
    /// `Close - origin`, or `None`.
    pub displacement_price: Option<f64>,
    /// `(Close - origin) / ATR_at_origin`, or `None`.
    pub displacement_atr: Option<f64>,
    /// 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded.
    pub lifecycle_code: u8,
    /// The ordinal stage 0..=5, or `None`.
    pub stage_code: Option<i8>,
    /// 1 when progress reached the target, 0 when it has not, or `None`.
    pub target_reached_code: Option<u8>,
}

/// The volatility and participation context at the latest bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportContext {
    /// `ATR14 / SMA20(ATR14)`, or `None`.
    pub atr_ratio: Option<f64>,
    /// `RVOL20`, or `None`.
    pub rvol_ratio: Option<f64>,
    /// 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation, or
    /// `None`.
    pub context_code: Option<u8>,
}

/// The per-timeframe directional consensus counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportConsensus {
    /// Number of voting axes above +0.1.
    pub bullish_axis_count: u8,
    /// Number of voting axes below -0.1.
    pub bearish_axis_count: u8,
    /// Number of voting axes in the inclusive ±0.1 band.
    pub neutral_axis_count: u8,
    /// The lowest valid size percentile's axis code: 1 Direction, 2 Momentum,
    /// 3 Structure, or `None`.
    pub weakest_directional_axis_code: Option<u8>,
}

/// Full-period event counts per timeframe and overall.
#[derive(Debug, Clone, PartialEq)]
pub struct EventReport {
    /// The sum of the three type counts.
    pub total: u32,
    /// Overall counts by event type.
    pub by_type: EventTypeCounts,
    /// Overall counts by subtype, using the existing aggregate layout.
    pub by_subtype: EventSubtypeCounts,
    /// One entry per ladder member, in ladder order.
    pub timeframes: Vec<TimeframeEventReport>,
}

/// Counts by event type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EventTypeCounts {
    /// Level events.
    pub level: u32,
    /// Divergence events.
    pub divergence: u32,
    /// Signal events.
    pub signal: u32,
}

impl EventTypeCounts {
    /// The sum of the three type counts, saturating only beyond `u32`.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.level
            .saturating_add(self.divergence)
            .saturating_add(self.signal)
    }

    /// Adds one event-type code.
    pub(crate) fn add_code(&mut self, code: u8) {
        match code {
            1 => self.level = self.level.saturating_add(1),
            2 => self.divergence = self.divergence.saturating_add(1),
            3 => self.signal = self.signal.saturating_add(1),
            _ => {}
        }
    }

    /// Adds every count of `other` into this record.
    pub(crate) fn add_counts(&mut self, other: Self) {
        self.level = self.level.saturating_add(other.level);
        self.divergence = self.divergence.saturating_add(other.divergence);
        self.signal = self.signal.saturating_add(other.signal);
    }
}

/// Full-period event counts for one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeEventReport {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The sum of the three type counts.
    pub total: u32,
    /// Counts by event type.
    pub by_type: EventTypeCounts,
    /// Counts by subtype, using the existing aggregate layout.
    pub by_subtype: EventSubtypeCounts,
    /// The event with the greatest availability key, or `None`.
    pub latest: Option<LatestEvent>,
}

/// The latest event of one timeframe, at its availability bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatestEvent {
    /// 1 level, 2 divergence, 3 signal.
    pub event_type_code: u8,
    /// The type's compact subtype code.
    pub event_subtype_code: u8,
    /// The stable source code.
    pub source_code: u16,
    /// The bar the underlying relationship occurred on.
    pub occurred_bar_index: usize,
    /// The first closed bar the event became knowable on.
    pub available_bar_index: usize,
    /// -1 bearish, 0 neutral, +1 bullish.
    pub direction_code: i8,
    /// The identified reference, or `None`.
    pub reference_code: Option<u32>,
    /// The main measured value, or `None`.
    pub primary_value: Option<f64>,
    /// The compared value, or `None`.
    pub reference_value: Option<f64>,
}

/// The deterministic chart projection.
///
/// The selected series is H1 when present, otherwise the slowest present
/// timeframe. It is reduced to at most [`CHART_BUCKET_LIMIT`] buckets; each
/// bucket carries the last published axis value, never an average.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartReport {
    /// The selected timeframe in seconds, or `None` when no timeframe is
    /// present.
    pub timeframe_secs: Option<u64>,
    /// The number of closed bars in the selected series.
    pub source_bars: usize,
    /// The bucket size, or 1 when no series is selected.
    pub bucket_size: usize,
    /// The buckets, oldest first, at most [`CHART_BUCKET_LIMIT`].
    pub bars: Vec<ReportChartBar>,
    /// Every event of the selected timeframe, in the fixed event order.
    pub events: Vec<ReportChartEvent>,
}

/// One chart bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportChartBar {
    /// The absolute index of the first source bar in the bucket.
    pub first_bar_index: usize,
    /// The absolute index of the last source bar in the bucket.
    pub last_bar_index: usize,
    /// The first source bar's UTC open time.
    pub open_time: i64,
    /// The last source bar's UTC open time.
    pub last_open_time: i64,
    /// The first source bar's open.
    pub open: f64,
    /// The maximum high in the bucket.
    pub high: f64,
    /// The minimum low in the bucket.
    pub low: f64,
    /// The last source bar's close.
    pub close: f64,
    /// The sum of the bucket volumes, or `None` when any volume is missing.
    pub volume: Option<f64>,
    /// The last non-null Direction primary value, or `None`.
    pub direction: Option<f64>,
    /// The last non-null Momentum primary value, or `None`.
    pub momentum: Option<f64>,
    /// The last non-null Structure primary value, or `None`.
    pub structure: Option<f64>,
    /// The last non-null Progression stage code, or `None`.
    pub progression_stage_code: Option<i8>,
}

/// One event marker in the chart projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportChartEvent {
    /// The bucket the event is drawn in, from its availability bar.
    pub bucket_index: usize,
    /// The first closed bar the event became knowable on.
    pub available_bar_index: usize,
    /// The bar the underlying relationship occurred on.
    pub occurred_bar_index: usize,
    /// 1 level, 2 divergence, 3 signal.
    pub event_type_code: u8,
    /// The type's compact subtype code.
    pub event_subtype_code: u8,
    /// -1 bearish, 0 neutral, +1 bullish.
    pub direction_code: i8,
}
