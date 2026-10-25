//! The published per-bar record: readings, metrics, and their reasons.
//!
//! This is Layer 1's payload in the research contract's numeric form. Codes are
//! the contract's codes; a missing value is `None` with its reason code, never
//! zero and never omitted. The resolution columns are added by
//! [`super::Layer2Row`].

use quantrule_model::axes::direction::DirectionBar;
use quantrule_model::axes::momentum::MomentumBar;
use quantrule_model::axes::progression::ProgressionBar;
use quantrule_model::axes::structure::StructureBar;
use quantrule_model::context::ContextBar;
use quantrule_model::kernel::UnavailableReason;
use quantrule_model::run::ModelReadings;

/// The six scalar indicator readings for one bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReadingsSnapshot {
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
    /// RVOL 20.
    pub rvol20: Option<f64>,
}

impl ReadingsSnapshot {
    /// Copies the model's scalar readings for one bar.
    #[must_use]
    pub const fn from_model(readings: ModelReadings) -> Self {
        Self {
            ema20: readings.ema20,
            ema50: readings.ema50,
            ema200: readings.ema200,
            rsi14: readings.rsi14,
            atr14: readings.atr14,
            rvol20: readings.rvol20,
        }
    }
}

/// Direction's published per-bar metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectionMetrics {
    /// Directional alignment in `[-1, +1]`.
    pub direction: Option<f64>,
    /// Compact state: -1, 0, or +1.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute value, in `0..=100`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Why the value is null; null when the value is present.
    pub unavailable_reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub state_changes_50: u64,
}

/// Momentum's published per-bar metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MomentumMetrics {
    /// Signed force in `[-1, +1]`.
    pub force: Option<f64>,
    /// Signed change in force over three bars.
    pub acceleration: Option<f64>,
    /// Signed change in the force magnitude over three bars.
    pub intensification: Option<f64>,
    /// Compact state of `force`: -1, 0, or +1.
    pub state_code: Option<i8>,
    /// Empirical percentile of the absolute force, in `0..=100`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Why the force is null; null when it is present.
    pub unavailable_reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub state_changes_50: u64,
}

/// Structure's published per-bar metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureMetrics {
    /// Swing-relation score in `[-1, +1]`.
    pub score: Option<f64>,
    /// Compact swing code in `-2..=+2`.
    pub state_code: Option<i8>,
    /// Scale the swing fields were computed at: 1 minor, 2 major.
    pub scale_code: u8,
    /// Relation of the latest two confirmed swing highs.
    pub high_relation_code: Option<i8>,
    /// Relation of the latest two confirmed swing lows.
    pub low_relation_code: Option<i8>,
    /// `(close - ema20) / atr14`.
    pub ema_distance_atr_20: Option<f64>,
    /// `(close - ema50) / atr14`.
    pub ema_distance_atr_50: Option<f64>,
    /// `(close - ema200) / atr14`.
    pub ema_distance_atr_200: Option<f64>,
    /// Close/EMA 20 cross code.
    pub ema_cross_code_20: Option<i8>,
    /// Close/EMA 50 cross code.
    pub ema_cross_code_50: Option<i8>,
    /// Close/EMA 200 cross code.
    pub ema_cross_code_200: Option<i8>,
    /// EMA 20 zone retest code.
    pub ema_retest_code_20: Option<i8>,
    /// EMA 50 zone retest code.
    pub ema_retest_code_50: Option<i8>,
    /// EMA 200 zone retest code.
    pub ema_retest_code_200: Option<i8>,
    /// Empirical percentile of the absolute score, in `0..=100`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Why the score is null; null when it is present.
    pub unavailable_reason_code: Option<u8>,
    /// Closed bars the compact state has held.
    pub state_age_bars: u64,
    /// Compact-state changes in the last 50 code-bearing bars.
    pub state_changes_50: u64,
}

/// Progression's published per-bar metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressionMetrics {
    /// Raw progress of the published leg, unclamped.
    pub progress: Option<f64>,
    /// Ordinal stage in `0..=5`.
    pub stage_code: Option<i8>,
    /// Direction of the published leg: +1 or -1.
    pub leg_direction: Option<i8>,
    /// Scale of the published leg: 1 minor, 2 major.
    pub leg_scale_code: Option<u8>,
    /// Empirical percentile of the absolute displacement, in `0..=100`.
    pub size_percentile: Option<f64>,
    /// Readiness code: 0 unavailable, 1 provisional, 2 ready.
    pub readiness_code: u8,
    /// Why progress is null; null when it is present.
    pub unavailable_reason_code: Option<u8>,
    /// Closed bars the published stage has held.
    pub state_age_bars: u64,
    /// Published-stage changes in the last 50 code-bearing bars.
    pub state_changes_50: u64,
}

/// The context diagnostics, which never vote on an axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMetrics {
    /// `atr14 / SMA20(atr14)`.
    pub atr_ratio: Option<f64>,
    /// Relative volume.
    pub rvol_ratio: Option<f64>,
    /// 1 expanding, 2 coiling, 3 settling, 4 high-range/low-participation.
    pub context_code: Option<u8>,
}

/// The published metrics of one bar, grouped by axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Direction's metrics.
    pub direction: DirectionMetrics,
    /// Momentum's metrics.
    pub momentum: MomentumMetrics,
    /// Structure's metrics.
    pub structure: StructureMetrics,
    /// Progression's metrics.
    pub progression: ProgressionMetrics,
    /// Context diagnostics.
    pub context: ContextMetrics,
}

impl Metrics {
    /// Converts the model's published bars into the research record.
    ///
    /// The conversion copies values and codes; it computes nothing. Every
    /// missing value keeps the model's own unavailable-reason code.
    #[must_use]
    pub fn from_bars(
        direction: &DirectionBar,
        momentum: &MomentumBar,
        structure: &StructureBar,
        progression: &ProgressionBar,
        context: &ContextBar,
    ) -> Self {
        Self {
            direction: DirectionMetrics {
                direction: direction.direction,
                state_code: direction.state_code,
                size_percentile: direction.size_percentile,
                readiness_code: direction.readiness.code(),
                unavailable_reason_code: reason_code(direction.unavailable_reason),
                state_age_bars: direction.state_age_bars,
                state_changes_50: direction.state_changes_50,
            },
            momentum: MomentumMetrics {
                force: momentum.force,
                acceleration: momentum.acceleration,
                intensification: momentum.intensification,
                state_code: momentum.state_code,
                size_percentile: momentum.size_percentile,
                readiness_code: momentum.readiness.code(),
                unavailable_reason_code: reason_code(momentum.unavailable_reason),
                state_age_bars: momentum.state_age_bars,
                state_changes_50: momentum.state_changes_50,
            },
            structure: StructureMetrics {
                score: structure.score,
                state_code: structure.state_code,
                scale_code: structure.scale_code,
                high_relation_code: structure.high_relation_code,
                low_relation_code: structure.low_relation_code,
                ema_distance_atr_20: structure.ema_distance_atr_20,
                ema_distance_atr_50: structure.ema_distance_atr_50,
                ema_distance_atr_200: structure.ema_distance_atr_200,
                ema_cross_code_20: structure.ema_cross_code_20,
                ema_cross_code_50: structure.ema_cross_code_50,
                ema_cross_code_200: structure.ema_cross_code_200,
                ema_retest_code_20: structure.ema_retest_code_20,
                ema_retest_code_50: structure.ema_retest_code_50,
                ema_retest_code_200: structure.ema_retest_code_200,
                size_percentile: structure.size_percentile,
                readiness_code: structure.readiness.code(),
                unavailable_reason_code: reason_code(structure.unavailable_reason),
                state_age_bars: structure.state_age_bars,
                state_changes_50: structure.state_changes_50,
            },
            progression: ProgressionMetrics {
                progress: progression.progress,
                stage_code: progression.stage_code,
                leg_direction: progression.leg_direction,
                leg_scale_code: progression.leg_scale_code,
                size_percentile: progression.size_percentile,
                readiness_code: progression.readiness.code(),
                unavailable_reason_code: reason_code(progression.unavailable_reason),
                state_age_bars: progression.state_age_bars,
                state_changes_50: progression.state_changes_50,
            },
            context: ContextMetrics {
                atr_ratio: context.atr_ratio,
                rvol_ratio: context.rvol_ratio,
                context_code: context.context_code,
            },
        }
    }
}

fn reason_code(reason: Option<UnavailableReason>) -> Option<u8> {
    reason.map(UnavailableReason::code)
}
