//! The typed four-axis dashboard view.
//!
//! This module owns the value types for the terminal dashboard. The pure
//! assembler in `assemble` fills them from a [`crate::run::ModelRun`]; this
//! file deliberately contains no I/O, candle computation, axis formula, or
//! shell rendering.
//!
//! The view is one instrument over one timeframe ladder. It is not a second
//! market model and does not replace [`crate::state::MarketState`]. The
//! contract is `docs/model/2026-10-25/dashboard-state.md`.
//!
//! # Examples
//!
//! ```
//! use quantrule_model::dashboard::{AxisValue, Domain};
//!
//! let value = AxisValue::signed(-0.42, Domain::DirectionalBias);
//! assert_eq!(value.as_signed(), Some((-0.42, Domain::DirectionalBias)));
//! assert!(!value.is_undefined());
//! ```

use crate::aggregate::CrossTimeframeAggregates;
use crate::kernel::{Readiness, UnavailableReason};
use crate::readings::roles::AxisCode;

/// The number of bars used by the view-only recent progression pace.
///
/// The value is fixed by the dashboard contract. It is not a command flag.
pub const RECENT_PACE_WINDOW_BARS: u8 = 3;

/// The default number of state changes in 50 code-bearing bars that marks
/// churn as unstable.
pub const DEFAULT_CHURN_THRESHOLD: u32 = 6;

/// The default minimum number of valid timeframe readings required before a
/// cross-timeframe majority is meaningful.
pub const DEFAULT_MIN_CROSS_TF_MEMBERS: u8 = 2;

/// The configuration of the dashboard projection.
///
/// The two fields are explicit inputs so that the thresholds can be calibrated
/// without changing the command contract. The shell uses the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardConfig {
    /// State changes in 50 code-bearing bars at or above which churn is
    /// `Unstable`.
    pub churn_threshold: u32,
    /// Minimum number of valid timeframe readings before a cross-timeframe
    /// majority is considered meaningful.
    pub min_cross_tf_members: u8,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            churn_threshold: DEFAULT_CHURN_THRESHOLD,
            min_cross_tf_members: DEFAULT_MIN_CROSS_TF_MEMBERS,
        }
    }
}

impl DashboardConfig {
    /// Builds a configuration with explicit thresholds.
    #[must_use]
    pub const fn new(churn_threshold: u32, min_cross_tf_members: u8) -> Self {
        Self {
            churn_threshold,
            min_cross_tf_members,
        }
    }
}

/// One complete dashboard document for one instrument and one ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardState {
    /// The configured timeframe ladder, ascending in seconds.
    pub ladder: Vec<u64>,
    /// One state per ladder member, in the same ascending order.
    pub timeframes: Vec<TimeframeState>,
    /// The cross-timeframe reduction and its view-only diagnostics.
    pub cross_tf_state: CrossTfState,
}

/// The dashboard state of one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeState {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// Whether the timeframe has at least one closed bar.
    pub present: bool,
    /// The newest closed bar, or `None` when the timeframe is absent.
    pub latest_bar: Option<usize>,
    /// The UTC open time of the newest closed bar, or `None` when absent.
    pub latest_open_time: Option<i64>,
    /// The first bar position in the loaded window.
    pub window_start: usize,
    /// The number of bars in the loaded window.
    pub window_len: usize,
    /// Exactly four axis states, in [`AxisCode::ALL`] order.
    pub axes: Vec<AxisState>,
    /// The volatility and participation diagnostic. It never votes.
    pub diagnostics: DiagnosticsView,
    /// The per-timeframe derived diagnostics.
    pub derived: TimeframeDerived,
}

/// The volatility and participation context of one timeframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagnosticsView {
    /// `ATR14 / SMA20(ATR14)`, or `None` when unavailable.
    pub atr_ratio: Option<f64>,
    /// `RVOL20`, or `None` when unavailable.
    pub rvol_ratio: Option<f64>,
    /// 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation, or
    /// `None` when either ratio is unavailable.
    pub context_code: Option<u8>,
}

/// The per-timeframe derived diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeframeDerived {
    /// The existing per-timeframe consensus counts. This is not the
    /// cross-timeframe agreement in [`CrossTfState::agreement`].
    pub timeframe_consensus: TimeframeConsensusView,
}

/// The existing per-timeframe directional consensus counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeframeConsensusView {
    /// Number of voting axes above +0.1.
    pub bullish_axis_count: u8,
    /// Number of voting axes below -0.1.
    pub bearish_axis_count: u8,
    /// Number of voting axes in the inclusive ±0.1 band.
    pub neutral_axis_count: u8,
    /// The lowest valid size percentile's axis code, or `None`.
    pub weakest_directional_axis_code: Option<u8>,
}

/// One axis's typed state for one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisState {
    /// The axis this state belongs to.
    pub axis: AxisCode,
    /// The axis's positional variable.
    pub position: AxisValue,
    /// The axis's magnitude variable.
    pub size: AxisValue,
    /// The visual readiness flag: solid for ready, dashed otherwise.
    pub readiness: AxisReadiness,
    /// The exact model readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// The dated Event and Transition readings, oldest first, at most 50.
    pub readings: Vec<DatedReading>,
    /// Axis-specific evidence.
    pub detail: AxisDetail,
}

/// A typed axis value.
///
/// # Examples
///
/// ```
/// use quantrule_model::dashboard::{AxisValue, Domain, Stage};
/// use quantrule_model::kernel::UnavailableReason;
///
/// let signed = AxisValue::signed(0.5, Domain::Acceleration);
/// let rank = AxisValue::rank(70.0);
/// let ordinal = AxisValue::ordinal(Stage::EarlyMid);
/// let absent = AxisValue::undefined(UnavailableReason::InsufficientBars);
///
/// assert_eq!(signed.as_signed(), Some((0.5, Domain::Acceleration)));
/// assert_eq!(rank.as_rank(), Some(70.0));
/// assert_eq!(ordinal.as_stage(), Some(Stage::EarlyMid));
/// assert!(absent.is_undefined());
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AxisValue {
    /// A continuous value with its meaning attached.
    Signed {
        /// The continuous value.
        value: f64,
        /// What the value means on its axis.
        domain: Domain,
    },
    /// A percentile against the axis's own history.
    Rank {
        /// The percentile in `0..=100`.
        pct: f64,
    },
    /// One of the five progression anchors plus the `against` pre-anchor.
    Ordinal {
        /// The snapped anchor.
        stage: Stage,
    },
    /// An absent value with its reason.
    Undefined {
        /// Why the value is absent.
        reason: UnavailableReason,
    },
}

impl AxisValue {
    /// Builds a signed value.
    #[must_use]
    pub const fn signed(value: f64, domain: Domain) -> Self {
        Self::Signed { value, domain }
    }

    /// Builds a rank value.
    #[must_use]
    pub const fn rank(pct: f64) -> Self {
        Self::Rank { pct }
    }

    /// Builds an ordinal value.
    #[must_use]
    pub const fn ordinal(stage: Stage) -> Self {
        Self::Ordinal { stage }
    }

    /// Builds an absent value.
    #[must_use]
    pub const fn undefined(reason: UnavailableReason) -> Self {
        Self::Undefined { reason }
    }

    /// Whether this is an absent value.
    #[must_use]
    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined { .. })
    }

    /// The signed value and its domain, if this is a signed value.
    #[must_use]
    pub const fn as_signed(&self) -> Option<(f64, Domain)> {
        match self {
            Self::Signed { value, domain } => Some((*value, *domain)),
            _ => None,
        }
    }

    /// The percentile, if this is a rank value.
    #[must_use]
    pub const fn as_rank(&self) -> Option<f64> {
        match self {
            Self::Rank { pct } => Some(*pct),
            _ => None,
        }
    }

    /// The anchor, if this is an ordinal value.
    #[must_use]
    pub const fn as_stage(&self) -> Option<Stage> {
        match self {
            Self::Ordinal { stage } => Some(*stage),
            _ => None,
        }
    }
}

/// The meaning attached to an [`AxisValue::Signed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    /// Direction's signed alignment.
    DirectionalBias,
    /// Momentum's signed acceleration.
    Acceleration,
    /// Structure's signed swing relation.
    StructureRelation,
}

/// The six progression anchors, in order.
///
/// The first anchor is the `against` pre-stage; the remaining five are the
/// contract's Early through Terminal anchors. No arithmetic operator exists for
/// this type, and none is added here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    /// Progress below zero: price is against the leg.
    Against,
    /// Progress from 0 up to, but not including, 0.25.
    Early,
    /// Progress from 0.25 up to, but not including, 0.50.
    EarlyMid,
    /// Progress from 0.50 up to, but not including, 0.75.
    Mid,
    /// Progress of at least 0.75, or a late active leg.
    Late,
    /// The leg lifecycle is terminal.
    Terminal,
}

impl Stage {
    /// Every stage, in ordinal order.
    pub const ALL: [Self; 6] = [
        Self::Against,
        Self::Early,
        Self::EarlyMid,
        Self::Mid,
        Self::Late,
        Self::Terminal,
    ];

    /// The ordinal code, 0 through 5.
    #[must_use]
    pub const fn code(self) -> i8 {
        self as i8
    }

    /// Resolves an ordinal code.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::dashboard::Stage;
    ///
    /// assert_eq!(Stage::from_code(4), Some(Stage::Late));
    /// assert_eq!(Stage::from_code(6), None);
    /// ```
    #[must_use]
    pub const fn from_code(code: i8) -> Option<Self> {
        match code {
            0 => Some(Self::Against),
            1 => Some(Self::Early),
            2 => Some(Self::EarlyMid),
            3 => Some(Self::Mid),
            4 => Some(Self::Late),
            5 => Some(Self::Terminal),
            _ => None,
        }
    }
}

/// The visual readiness flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisReadiness {
    /// Every input is present.
    Solid,
    /// At least one input is absent or still moving.
    Dashed,
}

impl AxisReadiness {
    /// The stable visual code: 0 solid, 1 dashed.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Solid => 0,
            Self::Dashed => 1,
        }
    }
}

impl From<Readiness> for AxisReadiness {
    /// `Ready` is solid; `Provisional` and `Unavailable` are dashed.
    fn from(readiness: Readiness) -> Self {
        match readiness {
            Readiness::Ready => Self::Solid,
            Readiness::Provisional | Readiness::Unavailable => Self::Dashed,
        }
    }
}

/// The current directional bias state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiasState {
    /// Above +0.1.
    Bullish,
    /// Below -0.1.
    Bearish,
    /// Inside the inclusive ±0.1 corridor.
    Neutral,
    /// The direction value is unavailable.
    Undefined,
}

/// Whether bias tenure was continuous across the observed bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BiasContinuity {
    /// No unavailable interval interrupted the counter.
    Continuous,
    /// An unavailable interval interrupted the counter.
    Interrupted,
    /// Continuity cannot be established because the value is unavailable.
    Undefined,
}

/// The origin of the current bias tenure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TenureOriginKind {
    /// No bias has been established.
    None,
    /// The first established bias.
    FirstEstablishment,
    /// A genuine bullish/bearish reversal.
    Reversal,
    /// A new origin observed after an unavailable interval; the actual
    /// reversal time during the gap is unknown.
    NewOriginAfterUnavailable,
}

/// The sign of the recent progression pace relative to the leg direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaceDirection {
    /// The price moved with the leg.
    WithLeg,
    /// The price moved against the leg.
    AgainstLeg,
    /// The net displacement change was zero.
    Flat,
}

/// The kind of compact state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransitionKind {
    /// A compact Direction, Momentum, or Structure state change.
    CompactState,
    /// A Progression stage change.
    ProgressionStage,
}

/// A current persistent reference price.
///
/// The reference kinds are fixed by the model's reference order. `available_bar`
/// is the view bar at which the current reference is knowable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelReading {
    /// 1 EMA20, 2 EMA50, 3 EMA200, 4 minor high, 5 minor low, 6 major high,
    /// 7 major low.
    pub reference_kind_code: u8,
    /// The reference price.
    pub price: f64,
    /// The view bar at which the reference is knowable.
    pub available_bar: usize,
}

/// A dated reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DatedReading {
    /// A dated occurrence from the event batch.
    Event(EventReading),
    /// A compact-state or progression-stage change.
    Transition(TransitionReading),
}

/// A dated event occurrence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventReading {
    /// 1 Level, 2 Divergence, 3 Signal.
    pub event_type_code: u8,
    /// The event subtype code.
    pub event_subtype_code: u8,
    /// The event source code.
    pub source_code: u16,
    /// The bar the underlying occurrence happened on.
    pub occurred_bar: usize,
    /// The first bar the event became knowable on.
    pub available_bar: usize,
    /// -1 bearish, 0 neutral, +1 bullish.
    pub direction_code: i8,
    /// The reference code, when the event names one.
    pub reference_code: Option<u32>,
    /// The primary measured value.
    pub primary_value: Option<f64>,
    /// The compared reference value.
    pub reference_value: Option<f64>,
}

/// A dated state transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionReading {
    /// The axis whose state changed.
    pub axis: AxisCode,
    /// Whether this is a compact state or a progression stage change.
    pub kind: TransitionKind,
    /// The prior code, when one was known.
    pub from_code: Option<i8>,
    /// The new code.
    pub to_code: Option<i8>,
    /// The bar the transition happened on.
    pub occurred_bar: usize,
    /// The first bar the transition became knowable on.
    pub available_bar: usize,
}

/// The axis-specific evidence carried by an [`AxisState`].
#[derive(Debug, Clone, PartialEq)]
pub enum AxisDetail {
    /// Direction evidence.
    Direction(DirectionDetail),
    /// Momentum evidence.
    Momentum(MomentumDetail),
    /// Structure evidence.
    Structure(StructureDetail),
    /// Progression evidence.
    Progression(ProgressionDetail),
}

/// Direction evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionDetail {
    /// The current bias state.
    pub bias_state: BiasState,
    /// Whether tenure was interrupted by an unavailable interval.
    pub bias_continuity: BiasContinuity,
    /// Bars since the last establishment or reversal, including neutral bars.
    pub bias_tenure_bars: Option<u64>,
    /// The origin kind.
    pub tenure_origin_kind: TenureOriginKind,
    /// The origin bar.
    pub tenure_origin_bar: Option<usize>,
    /// The existing magnitude percentile, retained as detail.
    pub magnitude_pct: Option<f64>,
    /// The compact-state age in bars.
    pub compact_state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub compact_state_changes_50: u64,
}

/// Momentum evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MomentumDetail {
    /// The signed RSI force.
    pub force: Option<f64>,
    /// The signed change in force.
    pub acceleration: Option<f64>,
    /// The signed change in force magnitude.
    pub intensification: Option<f64>,
    /// The compact-state age in bars.
    pub compact_state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub compact_state_changes_50: u64,
}

/// Structure evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureDetail {
    /// The signed swing-relation score.
    pub score: Option<f64>,
    /// The latest high relation: -1, 0, or +1.
    pub high_relation_code: Option<i8>,
    /// The latest low relation: -1, 0, or +1.
    pub low_relation_code: Option<i8>,
    /// 1 minor, 2 major.
    pub scale_code: u8,
    /// `(Close - EMA20) / ATR14`.
    pub ema_distance_atr_20: Option<f64>,
    /// `(Close - EMA50) / ATR14`.
    pub ema_distance_atr_50: Option<f64>,
    /// `(Close - EMA200) / ATR14`.
    pub ema_distance_atr_200: Option<f64>,
    /// Close/EMA20 cross code.
    pub ema_cross_code_20: Option<i8>,
    /// Close/EMA50 cross code.
    pub ema_cross_code_50: Option<i8>,
    /// Close/EMA200 cross code.
    pub ema_cross_code_200: Option<i8>,
    /// EMA20 retest code.
    pub ema_retest_code_20: Option<i8>,
    /// EMA50 retest code.
    pub ema_retest_code_50: Option<i8>,
    /// EMA200 retest code.
    pub ema_retest_code_200: Option<i8>,
    /// Current reference prices, in the fixed EMA/swing order.
    pub level_references: Vec<LevelReading>,
    /// The number of confirmed levels available at the view bar.
    pub confirmed_level_count: usize,
    /// Structure-mapped events in the recent window.
    pub activity_count: u32,
    /// The activity window length in bars.
    pub activity_window: usize,
    /// `activity_count / activity_window`, or `None` when the window is empty.
    pub activity_per_bar: Option<f64>,
    /// The compact-state age in bars.
    pub compact_state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub compact_state_changes_50: u64,
}

/// Progression evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressionDetail {
    /// The unclamped progress toward the target.
    pub progress: Option<f64>,
    /// The ordinal stage code, 0 through 5.
    pub stage_code: Option<i8>,
    /// +1 upward leg, -1 downward leg.
    pub leg_direction: Option<i8>,
    /// 1 minor, 2 major.
    pub leg_scale_code: Option<u8>,
    /// 0 forming, 1 active, 2 completed, 3 invalidated, 4 superseded.
    pub lifecycle_code: Option<u8>,
    /// The target revision count.
    pub target_version: Option<u32>,
    /// Whether the target is stable.
    pub target_stable: Option<bool>,
    /// The bar the leg origin occurred on.
    pub origin_bar: Option<usize>,
    /// The bar the origin became knowable.
    pub origin_available_bar: Option<usize>,
    /// The existing displacement percentile, retained as detail.
    pub displacement_pct: Option<f64>,
    /// The signed 3-bar displacement change per bar.
    pub recent_pace_signed_atr_per_bar: Option<f64>,
    /// Whether the recent pace is with, against, or flat relative to the leg.
    pub recent_pace_direction: Option<PaceDirection>,
    /// The recent-pace window in bars.
    pub recent_pace_window_bars: u8,
    /// Net displacement per elapsed bar since the origin.
    pub average_net_displacement_atr_per_bar: Option<f64>,
    /// Whether the terminal pace has been frozen.
    pub recent_pace_frozen_terminal: bool,
}

/// The cross-timeframe reduction and its view-only diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTfState {
    /// The configured ladder, ascending.
    pub configured_timeframes: Vec<u64>,
    /// The ladder members that have a closed bar.
    pub present_timeframes: Vec<u64>,
    /// The existing cross-timeframe aggregates, carried unchanged.
    pub aggregates: CrossTimeframeAggregates,
    /// The cross-timeframe directional majority summary.
    pub agreement: CrossTfAgreement,
    /// The cross-timeframe state-change churn summary.
    pub churn: CrossTfChurn,
    /// The published leg direction per timeframe, independently of the
    /// directional agreement.
    pub progression_leg_direction: Vec<LegDirectionReading>,
}

/// The published leg direction for one timeframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegDirectionReading {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// +1 or -1 when a published leg exists.
    pub direction: Option<i8>,
}

/// The cross-timeframe directional agreement summary.
#[derive(Debug, Clone, PartialEq)]
pub struct CrossTfAgreement {
    /// The number of voting axes: three.
    pub of: u8,
    /// The number of axes with a strict majority.
    pub agreeing_axes: u8,
    /// One entry per voting axis.
    pub axes: Vec<AxisAgreement>,
    /// The timeframe coverage.
    pub coverage: Coverage,
    /// The minimum number of valid timeframe readings required.
    pub min_valid_timeframes: u8,
}

/// One axis's cross-timeframe majority.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisAgreement {
    /// Direction, Momentum, or Structure.
    pub axis: AxisCode,
    /// The number of timeframes with a defined value.
    pub valid_timeframes: u8,
    /// Bullish timeframe count.
    pub bullish_count: u8,
    /// Bearish timeframe count.
    pub bearish_count: u8,
    /// Neutral timeframe count.
    pub neutral_count: u8,
    /// The strict majority class, if there is one.
    pub majority: Option<MajorityClass>,
    /// The majority count divided by the valid timeframe count.
    pub majority_share: Option<f64>,
    /// Whether a strict majority exists with sufficient coverage.
    pub agrees: bool,
    /// Whether there were too few valid timeframe readings.
    pub insufficient_coverage: bool,
}

/// A strict majority class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MajorityClass {
    /// Bullish.
    Bullish,
    /// Bearish.
    Bearish,
    /// Neutral.
    Neutral,
}

/// The number of present and configured timeframes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Coverage {
    /// The number of present timeframes.
    pub present: u32,
    /// The number of configured timeframes.
    pub configured: u32,
}

/// The cross-timeframe state-change churn summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossTfChurn {
    /// The maximum state changes in 50 code-bearing bars.
    pub max_changes_50: u32,
    /// Every timeframe and axis tied at the maximum.
    pub sources: Vec<ChurnSource>,
    /// The threshold used for `Unstable`.
    pub threshold: u32,
    /// The churn window in bars.
    pub window: u32,
    /// Whether every required timeframe has enough code-bearing history.
    pub history_complete: bool,
    /// The churn class.
    pub churn_class: ChurnClass,
    /// The timeframe coverage.
    pub coverage: Coverage,
}

/// One source of the maximum churn count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChurnSource {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The source axis.
    pub axis: AxisCode,
}

/// The churn class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChurnClass {
    /// History is incomplete.
    Building,
    /// History is complete and churn is below the threshold.
    Holding,
    /// History is complete and churn is at or above the threshold.
    Unstable,
}

mod assemble;

#[cfg(test)]
mod tests;
