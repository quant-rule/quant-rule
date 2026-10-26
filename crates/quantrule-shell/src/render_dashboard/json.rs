//! The numeric JSON envelope for the dashboard.
//!
//! The shell copies every published number, null, list, and stable enum code
//! from [`super::DashboardState`]; it does not compute an axis, a reading, an
//! aggregate, or a view-only measurement.

use std::io::{self, Write};

use quantrule_model::aggregate::{
    CrossTimeframeAggregates, ScaleStageAggregates, TimeframeEventSummary,
};
use quantrule_model::dashboard::{
    AxisAgreement, AxisDetail, AxisState, AxisValue, CrossTfAgreement, CrossTfChurn,
    DashboardState, DatedReading, DirectionDetail, EventReading, LevelReading, MomentumDetail,
    ProgressionDetail, StructureDetail, TimeframeState, TransitionReading,
};
use serde_json::{Value, json};

use super::VIEW_CONTRACT;
use crate::labels;

/// Writes the numeric dashboard as one JSON object.
///
/// The envelope carries the model version and the view-contract version. Every
/// numeric field, null, list, and stable enum code of [`DashboardState`] is
/// present; no label is added.
pub(crate) fn write_json(state: &DashboardState, writer: &mut impl Write) -> io::Result<()> {
    let value = json!({
        "model_version": env!("CARGO_PKG_VERSION"),
        "view_contract": VIEW_CONTRACT,
        "dashboard_state": dashboard_value(state),
    });
    let text = serde_json::to_string(&value).map_err(io::Error::other)?;
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

fn dashboard_value(state: &DashboardState) -> Value {
    json!({
        "ladder": state.ladder.to_vec(),
        "timeframes": state.timeframes.iter().map(timeframe_value).collect::<Vec<_>>(),
        "cross_tf_state": cross_tf_value(state),
    })
}

fn timeframe_value(timeframe: &TimeframeState) -> Value {
    json!({
        "timeframe_secs": timeframe.timeframe_secs,
        "present": timeframe.present,
        "latest_bar": timeframe.latest_bar,
        "latest_open_time": timeframe.latest_open_time,
        "window_start": timeframe.window_start,
        "window_len": timeframe.window_len,
        "axes": timeframe.axes.iter().map(axis_value).collect::<Vec<_>>(),
        "diagnostics": {
            "atr_ratio": timeframe.diagnostics.atr_ratio,
            "rvol_ratio": timeframe.diagnostics.rvol_ratio,
            "context_code": timeframe.diagnostics.context_code,
        },
        "derived": {
            "timeframe_consensus": {
                "bullish_axis_count": timeframe.derived.timeframe_consensus.bullish_axis_count,
                "bearish_axis_count": timeframe.derived.timeframe_consensus.bearish_axis_count,
                "neutral_axis_count": timeframe.derived.timeframe_consensus.neutral_axis_count,
                "weakest_directional_axis_code": timeframe.derived.timeframe_consensus.weakest_directional_axis_code,
            },
        },
    })
}

fn axis_value(axis: &AxisState) -> Value {
    json!({
        "axis": labels::axis_label(axis.axis.code()),
        "position": axis_value_json(&axis.position),
        "size": axis_value_json(&axis.size),
        "readiness": labels::axis_readiness_code(axis.readiness),
        "readiness_code": axis.readiness_code,
        "readings": axis.readings.iter().map(reading_value).collect::<Vec<_>>(),
        "detail": detail_value(&axis.detail),
    })
}

fn axis_value_json(value: &AxisValue) -> Value {
    match value {
        AxisValue::Signed { value, domain } => json!({
            "kind": "signed",
            "value": value,
            "domain": labels::domain_code(*domain),
        }),
        AxisValue::Rank { pct } => json!({
            "kind": "rank",
            "pct": pct,
        }),
        AxisValue::Ordinal { stage } => json!({
            "kind": "ordinal",
            "stage": labels::dashboard_stage_code(*stage),
        }),
        AxisValue::Undefined { reason } => json!({
            "kind": "undefined",
            "reason_code": reason.code(),
        }),
    }
}

fn detail_value(detail: &AxisDetail) -> Value {
    match detail {
        AxisDetail::Direction(detail) => direction_detail_value(detail),
        AxisDetail::Momentum(detail) => momentum_detail_value(detail),
        AxisDetail::Structure(detail) => structure_detail_value(detail),
        AxisDetail::Progression(detail) => progression_detail_value(detail),
    }
}

fn direction_detail_value(detail: &DirectionDetail) -> Value {
    json!({
        "kind": "direction",
        "bias_state": labels::bias_state_label(detail.bias_state),
        "bias_continuity": labels::bias_continuity_label(detail.bias_continuity),
        "bias_tenure_bars": detail.bias_tenure_bars,
        "tenure_origin_kind": labels::tenure_origin_label(detail.tenure_origin_kind),
        "tenure_origin_bar": detail.tenure_origin_bar,
        "magnitude_pct": detail.magnitude_pct,
        "compact_state_age_bars": detail.compact_state_age_bars,
        "compact_state_changes_50": detail.compact_state_changes_50,
    })
}

fn momentum_detail_value(detail: &MomentumDetail) -> Value {
    json!({
        "kind": "momentum",
        "force": detail.force,
        "acceleration": detail.acceleration,
        "intensification": detail.intensification,
        "compact_state_age_bars": detail.compact_state_age_bars,
        "compact_state_changes_50": detail.compact_state_changes_50,
    })
}

fn structure_detail_value(detail: &StructureDetail) -> Value {
    json!({
        "kind": "structure",
        "score": detail.score,
        "high_relation_code": detail.high_relation_code,
        "low_relation_code": detail.low_relation_code,
        "scale_code": detail.scale_code,
        "ema_distance_atr_20": detail.ema_distance_atr_20,
        "ema_distance_atr_50": detail.ema_distance_atr_50,
        "ema_distance_atr_200": detail.ema_distance_atr_200,
        "ema_cross_code_20": detail.ema_cross_code_20,
        "ema_cross_code_50": detail.ema_cross_code_50,
        "ema_cross_code_200": detail.ema_cross_code_200,
        "ema_retest_code_20": detail.ema_retest_code_20,
        "ema_retest_code_50": detail.ema_retest_code_50,
        "ema_retest_code_200": detail.ema_retest_code_200,
        "level_references": detail.level_references.iter().map(level_reference_value).collect::<Vec<_>>(),
        "confirmed_level_count": detail.confirmed_level_count,
        "activity_count": detail.activity_count,
        "activity_window": detail.activity_window,
        "activity_per_bar": detail.activity_per_bar,
        "compact_state_age_bars": detail.compact_state_age_bars,
        "compact_state_changes_50": detail.compact_state_changes_50,
    })
}

fn level_reference_value(reference: &LevelReading) -> Value {
    json!({
        "reference_kind_code": reference.reference_kind_code,
        "price": reference.price,
        "available_bar": reference.available_bar,
    })
}

fn progression_detail_value(detail: &ProgressionDetail) -> Value {
    json!({
        "kind": "progression",
        "progress": detail.progress,
        "stage_code": detail.stage_code,
        "leg_direction": detail.leg_direction,
        "leg_scale_code": detail.leg_scale_code,
        "lifecycle_code": detail.lifecycle_code,
        "target_version": detail.target_version,
        "target_stable": detail.target_stable,
        "origin_bar": detail.origin_bar,
        "origin_available_bar": detail.origin_available_bar,
        "displacement_pct": detail.displacement_pct,
        "recent_pace_signed_atr_per_bar": detail.recent_pace_signed_atr_per_bar,
        "recent_pace_direction": detail.recent_pace_direction.map(labels::pace_direction_label),
        "recent_pace_window_bars": detail.recent_pace_window_bars,
        "average_net_displacement_atr_per_bar": detail.average_net_displacement_atr_per_bar,
        "recent_pace_frozen_terminal": detail.recent_pace_frozen_terminal,
    })
}

fn reading_value(reading: &DatedReading) -> Value {
    match reading {
        DatedReading::Event(event) => event_reading_value(event),
        DatedReading::Transition(transition) => transition_reading_value(transition),
    }
}

fn event_reading_value(event: &EventReading) -> Value {
    json!({
        "kind": "event",
        "event_type_code": event.event_type_code,
        "event_subtype_code": event.event_subtype_code,
        "source_code": event.source_code,
        "occurred_bar": event.occurred_bar,
        "available_bar": event.available_bar,
        "direction_code": event.direction_code,
        "reference_code": event.reference_code,
        "primary_value": event.primary_value,
        "reference_value": event.reference_value,
    })
}

fn transition_reading_value(transition: &TransitionReading) -> Value {
    json!({
        "kind": "transition",
        "axis": labels::axis_label(transition.axis.code()),
        "transition_kind": labels::transition_kind_code(transition.kind),
        "from_code": transition.from_code,
        "to_code": transition.to_code,
        "occurred_bar": transition.occurred_bar,
        "available_bar": transition.available_bar,
    })
}

fn cross_tf_value(state: &DashboardState) -> Value {
    let cross = &state.cross_tf_state;
    json!({
        "configured_timeframes": cross.configured_timeframes.to_vec(),
        "present_timeframes": cross.present_timeframes.to_vec(),
        "aggregates": aggregates_value(&cross.aggregates),
        "agreement": agreement_value(&cross.agreement),
        "churn": churn_value(&cross.churn),
        "progression_leg_direction": cross.progression_leg_direction.iter().map(|reading| json!({
            "timeframe_secs": reading.timeframe_secs,
            "direction": reading.direction,
        })).collect::<Vec<_>>(),
    })
}

fn agreement_value(agreement: &CrossTfAgreement) -> Value {
    json!({
        "of": agreement.of,
        "agreeing_axes": agreement.agreeing_axes,
        "axes": agreement.axes.iter().map(axis_agreement_value).collect::<Vec<_>>(),
        "coverage": {
            "present": agreement.coverage.present,
            "configured": agreement.coverage.configured,
        },
        "min_valid_timeframes": agreement.min_valid_timeframes,
    })
}

fn axis_agreement_value(axis: &AxisAgreement) -> Value {
    json!({
        "axis": labels::axis_label(axis.axis.code()),
        "valid_timeframes": axis.valid_timeframes,
        "bullish_count": axis.bullish_count,
        "bearish_count": axis.bearish_count,
        "neutral_count": axis.neutral_count,
        "majority": axis.majority.map(labels::majority_label),
        "majority_share": axis.majority_share,
        "agrees": axis.agrees,
        "insufficient_coverage": axis.insufficient_coverage,
    })
}

fn churn_value(churn: &CrossTfChurn) -> Value {
    json!({
        "max_changes_50": churn.max_changes_50,
        "sources": churn.sources.iter().map(|source| json!({
            "timeframe_secs": source.timeframe_secs,
            "axis": labels::axis_label(source.axis.code()),
        })).collect::<Vec<_>>(),
        "threshold": churn.threshold,
        "window": churn.window,
        "history_complete": churn.history_complete,
        "churn_class": labels::churn_class_label(churn.churn_class),
        "coverage": {
            "present": churn.coverage.present,
            "configured": churn.coverage.configured,
        },
    })
}

fn aggregates_value(aggregates: &CrossTimeframeAggregates) -> Value {
    json!({
        "ladder": aggregates.ladder.to_vec(),
        "direction": {
            "timeframes": aggregates.direction.timeframes.to_vec(),
            "timeframe_count": aggregates.direction.direction_timeframe_count,
            "mean": aggregates.direction.direction_mean,
            "bullish_share": aggregates.direction.direction_bullish_share,
            "bearish_share": aggregates.direction.direction_bearish_share,
            "neutral_share": aggregates.direction.direction_neutral_share,
            "gradient": aggregates.direction.direction_gradient,
            "ready_count": aggregates.direction.direction_ready_count,
        },
        "momentum": {
            "timeframes": aggregates.momentum.timeframes.to_vec(),
            "timeframe_count": aggregates.momentum.momentum_timeframe_count,
            "force_mean": aggregates.momentum.momentum_force_mean,
            "acceleration_mean": aggregates.momentum.momentum_acceleration_mean,
            "positive_share": aggregates.momentum.momentum_positive_share,
            "negative_share": aggregates.momentum.momentum_negative_share,
            "force_fast_slow_gap": aggregates.momentum.momentum_force_fast_slow_gap,
            "acceleration_fast_slow_gap": aggregates.momentum.momentum_acceleration_fast_slow_gap,
            "gradient": aggregates.momentum.momentum_gradient,
            "ready_count": aggregates.momentum.momentum_ready_count,
        },
        "structure": {
            "timeframes": aggregates.structure.timeframes.to_vec(),
            "timeframe_count": aggregates.structure.structure_timeframe_count,
            "code_histogram": aggregates.structure.structure_code_histogram,
            "bullish_share": aggregates.structure.structure_bullish_share,
            "bearish_share": aggregates.structure.structure_bearish_share,
            "neutral_share": aggregates.structure.structure_neutral_share,
            "state_span": aggregates.structure.structure_state_span,
            "gradient": aggregates.structure.structure_gradient,
            "ready_share": aggregates.structure.structure_ready_share,
            "conflict_timeframe_count": aggregates.structure.structure_conflict_timeframe_count,
            "level_cluster_count": aggregates.structure.level_cluster_count,
            "largest_level_cluster_timeframe_count": aggregates.structure.largest_level_cluster_timeframe_count,
            "largest_level_cluster_span_atr": aggregates.structure.largest_level_cluster_span_atr,
        },
        "progression": {
            "timeframes": aggregates.progression.timeframes.to_vec(),
            "timeframe_count": aggregates.progression.progression_timeframe_count,
            "minor": scale_stage_value(&aggregates.progression.minor),
            "major": scale_stage_value(&aggregates.progression.major),
            "active_major_leg_count": aggregates.progression.active_major_leg_count,
            "active_minor_leg_count": aggregates.progression.active_minor_leg_count,
            "ready_share": aggregates.progression.progression_ready_share,
            "leg_direction_agreement_share": aggregates.progression.leg_direction_agreement_share,
            "cross_timeframe_leg_match_count": aggregates.progression.cross_timeframe_leg_match_count,
        },
        "events": {
            "timeframes": aggregates.events.timeframes.to_vec(),
            "per_timeframe": aggregates.events.per_timeframe.iter().map(event_summary_value).collect::<Vec<_>>(),
            "count_by_type": aggregates.events.event_count_by_type,
            "count_by_subtype": {
                "level": aggregates.events.event_count_by_subtype.level,
                "divergence": aggregates.events.event_count_by_subtype.divergence,
                "signal": aggregates.events.event_count_by_subtype.signal,
            },
            "divergence_count_by_subtype": aggregates.events.divergence_count_by_subtype,
            "divergence_timeframe_count": aggregates.events.divergence_timeframe_count,
            "divergence_direction_agreement_share": aggregates.events.divergence_direction_agreement_share,
            "level_event_timeframe_count": aggregates.events.level_event_timeframe_count,
            "signal_event_timeframe_count": aggregates.events.signal_event_timeframe_count,
        },
    })
}

fn scale_stage_value(stage: &ScaleStageAggregates) -> Value {
    json!({
        "stage_histogram": stage.stage_histogram,
        "stage_mode": stage.stage_mode,
        "stage_min": stage.stage_min,
        "stage_max": stage.stage_max,
        "stage_span": stage.stage_span,
        "modal_stage_share": stage.modal_stage_share,
    })
}

fn event_summary_value(summary: &TimeframeEventSummary) -> Value {
    json!({
        "timeframe_secs": summary.timeframe_secs,
        "counts_by_type": summary.counts_by_type,
        "counts_by_subtype": {
            "level": summary.counts_by_subtype.level,
            "divergence": summary.counts_by_subtype.divergence,
            "signal": summary.counts_by_subtype.signal,
        },
        "latest_event_type_code": summary.latest_event_type_code,
        "latest_event_subtype_code": summary.latest_event_subtype_code,
        "latest_occurred_bar": summary.latest_occurred_bar_index,
        "latest_available_bar": summary.latest_available_bar_index,
        "latest_event_age_bars": summary.latest_event_age_bars,
    })
}
