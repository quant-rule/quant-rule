//! The pure projection from a model run to the dashboard view.
//!
//! This module maps the per-bar model values in a [`ModelRun`] into
//! [`DashboardState`]. It owns the axis positions, sizes, readiness, typed
//! absence, the per-timeframe envelope, and the cross-timeframe envelope. The
//! Direction tenure, Structure activity, Structure reference prices, and
//! Progression recent pace are view-only measurements derived here from
//! published values; they never feed back into an axis. Typed readings,
//! agreement, and churn are filled by their own work items.
//!
//! The module is pure: it reads the run, the explicit ladder, and the
//! configuration it is given. It performs no I/O, reads no clock, and uses no
//! randomness.

mod direction;
mod progression;
mod structure;

use crate::kernel::{Readiness, UnavailableReason};
use crate::readings::PivotScale;
use crate::readings::roles::AxisCode;
use crate::run::{ModelBar, ModelRun, TimeframeModel};

use self::direction::direction_tenure;
use self::progression::progression_axis;
use self::structure::{level_references, structure_activity};

use super::{
    AxisDetail, AxisReadiness, AxisState, AxisValue, BiasContinuity, BiasState, ChurnClass,
    Coverage, CrossTfAgreement, CrossTfChurn, CrossTfState, DashboardConfig, DashboardState,
    DiagnosticsView, DirectionDetail, Domain, LegDirectionReading, MomentumDetail,
    ProgressionDetail, RECENT_PACE_WINDOW_BARS, StructureDetail, TenureOriginKind,
    TimeframeConsensusView, TimeframeDerived, TimeframeState,
};

/// The contract's fixed churn window in closed bars.
const CHURN_WINDOW_BARS: u32 = 50;

impl DashboardState {
    /// Builds the dashboard view from a completed model run.
    ///
    /// `ladder` is the configured timeframe ladder. It is sorted and
    /// deduplicated, so neither the caller's order nor a repeated member changes
    /// the result. A ladder member without a closed model bar becomes an absent
    /// [`TimeframeState`] instead of being omitted.
    ///
    /// The projection reads only `run`, `ladder`, and `config`. It does not
    /// append, fill, or synthesize a measurement for a missing bar.
    ///
    /// The cross-timeframe agreement and churn summaries are not yet derived
    /// from the final per-axis view. Their envelope carries the configured
    /// thresholds and the real coverage, publishes no per-axis majority, and
    /// reports the incomplete-history churn class. A later work item fills
    /// both summaries; no consumer should read the placeholder as a verdict.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_model::dashboard::{DashboardConfig, DashboardState};
    /// use quantrule_model::run::ModelRun;
    /// use quantrule_model::state::MarketStateConfig;
    ///
    /// let run = ModelRun::compute(&[], &MarketStateConfig::default());
    /// let view = DashboardState::from_run(&run, &[300, 60], &DashboardConfig::default());
    ///
    /// assert_eq!(view.ladder, vec![60, 300]);
    /// assert!(view.timeframes.iter().all(|timeframe| !timeframe.present));
    /// ```
    #[must_use]
    pub fn from_run(run: &ModelRun, ladder: &[u64], config: &DashboardConfig) -> Self {
        let ladder = normalized_ladder(ladder);
        let absent_scale_code = absent_structure_scale_code(run);
        let mut present_timeframes = Vec::new();
        let mut timeframes = Vec::with_capacity(ladder.len());

        for &timeframe_secs in &ladder {
            let state = run.timeframe(timeframe_secs).and_then(|timeframe| {
                timeframe
                    .len()
                    .checked_sub(1)
                    .and_then(|bar| TimeframeState::at(timeframe, bar))
            });
            match state {
                Some(state) => {
                    present_timeframes.push(timeframe_secs);
                    timeframes.push(state);
                }
                None => {
                    timeframes.push(TimeframeState::absent(timeframe_secs, absent_scale_code));
                }
            }
        }

        let coverage = Coverage {
            present: present_timeframes.len() as u32,
            configured: ladder.len() as u32,
        };
        let progression_leg_direction = timeframes.iter().map(leg_direction_reading).collect();

        let cross_tf_state = CrossTfState {
            configured_timeframes: ladder.clone(),
            present_timeframes,
            aggregates: run.aggregates().clone(),
            agreement: pending_agreement(config, coverage),
            churn: pending_churn(config, coverage),
            progression_leg_direction,
        };

        Self {
            ladder,
            timeframes,
            cross_tf_state,
        }
    }
}

impl TimeframeState {
    /// Builds one timeframe's state at a closed bar.
    ///
    /// `bar` is a zero-based position in `timeframe`. The loaded window is
    /// reported as `0..=bar`, so the value at a bar does not depend on a later
    /// bar of the same model. This is the per-timeframe entry point used by the
    /// assembler and by causality tests.
    ///
    /// Returns `None` when `bar` is not a closed bar of `timeframe`.
    ///
    /// # Examples
    ///
    /// ```
    /// use quantrule_data::Candle;
    /// use quantrule_model::dashboard::TimeframeState;
    /// use quantrule_model::run::TimeframeModel;
    /// use quantrule_model::state::{MarketStateConfig, TimeframeInput};
    ///
    /// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
    ///     .expect("the example candle is valid");
    /// let model = TimeframeModel::compute(
    ///     &TimeframeInput {
    ///         timeframe_secs: 60,
    ///         candles: &[candle],
    ///     },
    ///     &MarketStateConfig::default(),
    /// )
    /// .expect("the example model is non-empty");
    /// let state = TimeframeState::at(&model, 0).expect("the bar exists");
    ///
    /// assert!(state.present);
    /// assert_eq!(state.latest_bar, Some(0));
    /// assert_eq!(state.window_len, 1);
    /// ```
    #[must_use]
    pub fn at(timeframe: &TimeframeModel, bar: usize) -> Option<Self> {
        let model_bar = timeframe.bar(bar)?;
        let axes = vec![
            direction_axis(timeframe, &model_bar),
            momentum_axis(&model_bar),
            structure_axis(timeframe, &model_bar),
            progression_axis(timeframe, &model_bar),
        ];

        Some(Self {
            timeframe_secs: model_bar.timeframe_secs,
            present: true,
            latest_bar: Some(bar),
            latest_open_time: Some(model_bar.candle.timestamp()),
            window_start: 0,
            window_len: bar.saturating_add(1),
            axes,
            diagnostics: DiagnosticsView {
                atr_ratio: model_bar.context.atr_ratio,
                rvol_ratio: model_bar.context.rvol_ratio,
                context_code: model_bar.context.context_code,
            },
            derived: TimeframeDerived {
                timeframe_consensus: TimeframeConsensusView {
                    bullish_axis_count: model_bar.consensus.bullish_axis_count,
                    bearish_axis_count: model_bar.consensus.bearish_axis_count,
                    neutral_axis_count: model_bar.consensus.neutral_axis_count,
                    weakest_directional_axis_code: model_bar
                        .consensus
                        .weakest_directional_axis_code,
                },
            },
        })
    }

    /// Builds an absent row for a ladder member without a closed bar.
    ///
    /// The four axes are typed `Undefined` with the insufficient-bars reason.
    /// The structure scale is the run's published scale when one exists; the
    /// model default is used only when the whole run is empty, where no
    /// structure measurement exists to describe.
    fn absent(timeframe_secs: u64, structure_scale_code: u8) -> Self {
        let axes = AxisCode::ALL
            .into_iter()
            .map(|axis| absent_axis(axis, structure_scale_code))
            .collect();

        Self {
            timeframe_secs,
            present: false,
            latest_bar: None,
            latest_open_time: None,
            window_start: 0,
            window_len: 0,
            axes,
            diagnostics: DiagnosticsView {
                atr_ratio: None,
                rvol_ratio: None,
                context_code: None,
            },
            derived: TimeframeDerived {
                timeframe_consensus: TimeframeConsensusView {
                    bullish_axis_count: 0,
                    bearish_axis_count: 0,
                    neutral_axis_count: 0,
                    weakest_directional_axis_code: None,
                },
            },
        }
    }
}

/// Sorts and removes repeated ladder members before assembly.
///
/// The contract publishes an ascending ladder. Normalizing here means a
/// reordered or accidentally repeated caller list cannot change the row order
/// or the coverage denominator.
fn normalized_ladder(ladder: &[u64]) -> Vec<u64> {
    let mut ladder = ladder.to_vec();
    ladder.sort_unstable();
    ladder.dedup();
    ladder
}

/// The configured swing scale, read from a present timeframe.
///
/// A missing row still carries the scale its run used. When no timeframe has a
/// closed bar, no structure detail exists; the model's default major scale is
/// then the only non-invented code available to the required field.
fn absent_structure_scale_code(run: &ModelRun) -> u8 {
    run.timeframes()
        .iter()
        .find_map(|timeframe| timeframe.latest_bar().map(|bar| bar.structure.scale_code))
        .unwrap_or(PivotScale::Major.code())
}

/// Builds the Direction axis from the model bar and its tenure evidence.
///
/// The radius is the bias-tenure percentile; the model's magnitude percentile
/// moves to `magnitude_pct` and stays visible as detail. A tenure that is too
/// short to rank keeps the detail but has no size, and carries the Direction
/// reason when the Direction itself is unavailable.
fn direction_axis(timeframe: &TimeframeModel, model_bar: &ModelBar) -> AxisState {
    let direction = model_bar.direction;
    let tenure = direction_tenure(timeframe.direction().bars(), model_bar.bar_index);
    axis_state(
        AxisCode::Direction,
        signed_position(
            direction.direction,
            Domain::DirectionalBias,
            direction.unavailable_reason,
        ),
        rank_size_or_reason(tenure.tenure_percentile, direction.unavailable_reason),
        direction.readiness,
        AxisDetail::Direction(DirectionDetail {
            bias_state: tenure.bias_state,
            bias_continuity: tenure.bias_continuity,
            bias_tenure_bars: tenure.tenure_bars,
            tenure_origin_kind: tenure.origin_kind,
            tenure_origin_bar: tenure.origin_bar,
            magnitude_pct: direction.size_percentile,
            compact_state_age_bars: direction.state_age_bars,
            compact_state_changes_50: direction.state_changes_50,
        }),
    )
}

/// Builds the Momentum axis from the model bar.
fn momentum_axis(model_bar: &ModelBar) -> AxisState {
    let momentum = model_bar.momentum;
    axis_state(
        AxisCode::Momentum,
        // Acceleration is the position. When the force exists but its
        // three-bar change does not, the position is absent while the force
        // and the incumbent readiness stay visible.
        signed_position(
            momentum.acceleration,
            Domain::Acceleration,
            momentum.unavailable_reason,
        ),
        rank_size(momentum.size_percentile),
        momentum.readiness,
        AxisDetail::Momentum(MomentumDetail {
            force: momentum.force,
            acceleration: momentum.acceleration,
            intensification: momentum.intensification,
            compact_state_age_bars: momentum.state_age_bars,
            compact_state_changes_50: momentum.state_changes_50,
        }),
    )
}

/// Builds the Structure axis from the model bar and its view-only evidence.
///
/// Activity is counted from Structure-mapped events in the recent window and
/// reference prices come from the published readings and confirmed levels.
/// Neither changes the score, the size percentile, or the readiness: the radius
/// remains the swing-alignment magnitude percentile.
fn structure_axis(timeframe: &TimeframeModel, model_bar: &ModelBar) -> AxisState {
    let structure = model_bar.structure;
    let confirmed_level_count = timeframe
        .levels()
        .partition_point(|level| level.available_at <= model_bar.bar_index);
    let activity = structure_activity(timeframe, model_bar.bar_index);

    axis_state(
        AxisCode::Structure,
        signed_position(
            structure.score,
            Domain::StructureRelation,
            structure.unavailable_reason,
        ),
        rank_size(structure.size_percentile),
        structure.readiness,
        AxisDetail::Structure(StructureDetail {
            score: structure.score,
            high_relation_code: structure.high_relation_code,
            low_relation_code: structure.low_relation_code,
            scale_code: structure.scale_code,
            ema_distance_atr_20: structure.ema_distance_atr_20,
            ema_distance_atr_50: structure.ema_distance_atr_50,
            ema_distance_atr_200: structure.ema_distance_atr_200,
            ema_cross_code_20: structure.ema_cross_code_20,
            ema_cross_code_50: structure.ema_cross_code_50,
            ema_cross_code_200: structure.ema_cross_code_200,
            ema_retest_code_20: structure.ema_retest_code_20,
            ema_retest_code_50: structure.ema_retest_code_50,
            ema_retest_code_200: structure.ema_retest_code_200,
            level_references: level_references(timeframe, model_bar),
            confirmed_level_count,
            activity_count: activity.count,
            activity_window: activity.window,
            activity_per_bar: activity.per_bar,
            compact_state_age_bars: structure.state_age_bars,
            compact_state_changes_50: structure.state_changes_50,
        }),
    )
}

/// Builds a signed position, preserving a measured zero.
fn signed_position(
    value: Option<f64>,
    domain: Domain,
    reason: Option<UnavailableReason>,
) -> AxisValue {
    match value {
        Some(value) => AxisValue::signed(value, domain),
        None => AxisValue::undefined(reason.unwrap_or(UnavailableReason::InsufficientBars)),
    }
}

/// Builds a percentile size, preserving a measured zero.
fn rank_size(percentile: Option<f64>) -> AxisValue {
    match percentile {
        Some(pct) => AxisValue::rank(pct),
        None => AxisValue::undefined(UnavailableReason::InsufficientBars),
    }
}

/// Builds a percentile size, using the axis reason when the axis itself is
/// unavailable.
///
/// A view-only percentile can be missing for two different reasons: too few
/// prior samples while the axis is present, or no axis value at all. The
/// dashboard contract keeps those distinct.
fn rank_size_or_reason(percentile: Option<f64>, reason: Option<UnavailableReason>) -> AxisValue {
    match percentile {
        Some(pct) => AxisValue::rank(pct),
        None => AxisValue::undefined(reason.unwrap_or(UnavailableReason::InsufficientBars)),
    }
}

/// Builds an axis state with the mechanical readiness mapping.
fn axis_state(
    axis: AxisCode,
    position: AxisValue,
    size: AxisValue,
    readiness: Readiness,
    detail: AxisDetail,
) -> AxisState {
    AxisState {
        axis,
        position,
        size,
        readiness: AxisReadiness::from(readiness),
        readiness_code: readiness.code(),
        readings: Vec::new(),
        detail,
    }
}

/// Builds an axis state for a missing timeframe.
fn absent_axis(axis: AxisCode, structure_scale_code: u8) -> AxisState {
    axis_state(
        axis,
        AxisValue::undefined(UnavailableReason::InsufficientBars),
        AxisValue::undefined(UnavailableReason::InsufficientBars),
        Readiness::Unavailable,
        absent_detail(axis, structure_scale_code),
    )
}

/// Builds the typed absence detail for a missing timeframe.
fn absent_detail(axis: AxisCode, structure_scale_code: u8) -> AxisDetail {
    match axis {
        AxisCode::Direction => AxisDetail::Direction(DirectionDetail {
            bias_state: BiasState::Undefined,
            bias_continuity: BiasContinuity::Undefined,
            bias_tenure_bars: None,
            tenure_origin_kind: TenureOriginKind::None,
            tenure_origin_bar: None,
            magnitude_pct: None,
            compact_state_age_bars: 0,
            compact_state_changes_50: 0,
        }),
        AxisCode::Momentum => AxisDetail::Momentum(MomentumDetail {
            force: None,
            acceleration: None,
            intensification: None,
            compact_state_age_bars: 0,
            compact_state_changes_50: 0,
        }),
        AxisCode::Structure => AxisDetail::Structure(StructureDetail {
            score: None,
            high_relation_code: None,
            low_relation_code: None,
            scale_code: structure_scale_code,
            ema_distance_atr_20: None,
            ema_distance_atr_50: None,
            ema_distance_atr_200: None,
            ema_cross_code_20: None,
            ema_cross_code_50: None,
            ema_cross_code_200: None,
            ema_retest_code_20: None,
            ema_retest_code_50: None,
            ema_retest_code_200: None,
            level_references: Vec::new(),
            confirmed_level_count: 0,
            activity_count: 0,
            activity_window: 0,
            activity_per_bar: None,
            compact_state_age_bars: 0,
            compact_state_changes_50: 0,
        }),
        AxisCode::Progression => AxisDetail::Progression(ProgressionDetail {
            progress: None,
            stage_code: None,
            leg_direction: None,
            leg_scale_code: None,
            lifecycle_code: None,
            target_version: None,
            target_stable: None,
            origin_bar: None,
            origin_available_bar: None,
            displacement_pct: None,
            recent_pace_signed_atr_per_bar: None,
            recent_pace_direction: None,
            recent_pace_window_bars: RECENT_PACE_WINDOW_BARS,
            average_net_displacement_atr_per_bar: None,
            recent_pace_frozen_terminal: false,
        }),
    }
}

/// Reads the published leg direction out of the Progression axis.
fn leg_direction_reading(state: &TimeframeState) -> LegDirectionReading {
    let direction = state
        .axes
        .iter()
        .find(|axis| axis.axis == AxisCode::Progression)
        .and_then(|axis| match &axis.detail {
            AxisDetail::Progression(detail) => detail.leg_direction,
            _ => None,
        });

    LegDirectionReading {
        timeframe_secs: state.timeframe_secs,
        direction,
    }
}

/// The not-yet-derived agreement envelope.
///
/// Agreement is computed from the final per-axis view in a later work item.
/// Until then the envelope publishes the configured thresholds and the real
/// coverage, with no per-axis entries and no claimed majority.
fn pending_agreement(config: &DashboardConfig, coverage: Coverage) -> CrossTfAgreement {
    CrossTfAgreement {
        of: 3,
        agreeing_axes: 0,
        axes: Vec::new(),
        coverage,
        min_valid_timeframes: config.min_cross_tf_members,
    }
}

/// The not-yet-derived churn envelope.
///
/// Churn is computed from the final per-axis view in a later work item. The
/// incomplete-history class is the only class this envelope can claim.
fn pending_churn(config: &DashboardConfig, coverage: Coverage) -> CrossTfChurn {
    CrossTfChurn {
        max_changes_50: 0,
        sources: Vec::new(),
        threshold: config.churn_threshold,
        window: CHURN_WINDOW_BARS,
        history_complete: false,
        churn_class: ChurnClass::Building,
        coverage,
    }
}

#[cfg(test)]
mod tests;
