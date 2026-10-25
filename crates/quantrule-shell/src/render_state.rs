//! Rendering for the model's market state.
//!
//! The shell calls the model's [`MarketState`] and maps its numeric fields to
//! the command's output schema. It does not compute an axis, aggregate a
//! timeframe, or detect an event; it only formats the numbers and the fixed
//! presentation labels.

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use serde_json::{Value, json};

use quantrule_model::aggregate::{CrossTimeframeAggregates, TimeframeEventSummary};
use quantrule_model::axes::progression::ProgressionLeg;
use quantrule_model::events::EventRecord;
use quantrule_model::run::{ModelRun, TimeframeModel};
use quantrule_model::state::{MarketState, TimeframeConsensus, TimeframeMarketState};

use crate::command::Axis;
use crate::labels;

/// Writes the assembled market state as one JSON object.
pub(crate) fn write_market_state(state: &MarketState, writer: &mut impl Write) -> io::Result<()> {
    let value = json!({
        "ladder": state.timeframes.iter().map(|timeframe| labels::timeframe_label(timeframe.snapshot.timeframe_secs)).collect::<Vec<_>>(),
        "timeframes": state.timeframes.iter().enumerate().map(|(index, timeframe)| timeframe_value(index, timeframe, &state.aggregates, state.consensus.get(index))).collect::<Vec<_>>(),
        "aggregates": aggregates_value(&state.aggregates),
        "consensus": state.consensus.iter().map(consensus_value).collect::<Vec<_>>(),
    });
    write_line(&value, writer)
}

/// Writes one JSON line per timeframe for the selected axis.
pub(crate) fn write_axis(
    state: &MarketState,
    axis: Axis,
    writer: &mut impl Write,
) -> io::Result<()> {
    for timeframe in &state.timeframes {
        let value = axis_value(timeframe, axis);
        write_line(&value, writer)?;
    }
    Ok(())
}

/// Writes the full event batch, grouped by timeframe and availability order.
pub(crate) fn write_event_batch(run: &ModelRun, writer: &mut impl Write) -> io::Result<()> {
    for timeframe in run.timeframes() {
        for event in timeframe.events() {
            write_line(&event_value(timeframe, event), writer)?;
        }
    }
    Ok(())
}

/// Writes one JSON line per timeframe and scale, with the published leg or null.
pub(crate) fn write_legs(state: &MarketState, writer: &mut impl Write) -> io::Result<()> {
    for timeframe in &state.timeframes {
        for scale in [1_u8, 2] {
            let leg = match scale {
                1 => timeframe.snapshot.progression.minor_leg,
                _ => timeframe.snapshot.progression.major_leg,
            };
            let value = json!({
                "timeframe": labels::timeframe_label(timeframe.snapshot.timeframe_secs),
                "scale": labels::scale_label(scale),
                "leg": leg.map(|leg| leg_value(leg, scale)),
            });
            write_line(&value, writer)?;
        }
    }
    Ok(())
}

fn write_line(value: &Value, writer: &mut impl Write) -> io::Result<()> {
    let text = serde_json::to_string(value).map_err(io::Error::other)?;
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

fn timeframe_value(
    index: usize,
    timeframe: &TimeframeMarketState,
    aggregates: &CrossTimeframeAggregates,
    consensus: Option<&TimeframeConsensus>,
) -> Value {
    let snapshot = &timeframe.snapshot;
    let event_summary = aggregates.events.per_timeframe.get(index);
    json!({
        "timeframe": labels::timeframe_label(snapshot.timeframe_secs),
        "bar": snapshot.latest_bar_index,
        "series_index": timeframe.latest_series_index,
        "datetime": format_utc_timestamp(timeframe.latest_open_time),
        "window_start": timeframe.window_start,
        "window_len": timeframe.window_len,
        "direction": {
            "value": snapshot.direction.direction,
            "state": snapshot.direction.state_code,
            "state_label": snapshot.direction.state_code.map(labels::direction_state_label),
            "percentile": snapshot.direction.size_percentile,
            "readiness": snapshot.direction.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.direction.readiness.code()),
            "reason": snapshot.direction.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.direction.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.direction.state_age_bars,
            "state_changes_50": snapshot.direction.state_changes_50,
        },
        "momentum": {
            "force": snapshot.momentum.force,
            "acceleration": snapshot.momentum.acceleration,
            "intensification": snapshot.momentum.intensification,
            "state": snapshot.momentum.state_code,
            "state_label": snapshot.momentum.state_code.map(labels::momentum_state_label),
            "percentile": snapshot.momentum.size_percentile,
            "readiness": snapshot.momentum.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.momentum.readiness.code()),
            "reason": snapshot.momentum.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.momentum.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.momentum.state_age_bars,
            "state_changes_50": snapshot.momentum.state_changes_50,
        },
        "structure": {
            "score": snapshot.structure.score,
            "code": snapshot.structure.state_code,
            "code_label": snapshot.structure.state_code.map(labels::structure_label),
            "scale": labels::scale_label(snapshot.structure.scale_code),
            "high_relation": snapshot.structure.high_relation_code,
            "low_relation": snapshot.structure.low_relation_code,
            "percentile": snapshot.structure.size_percentile,
            "readiness": snapshot.structure.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.structure.readiness.code()),
            "reason": snapshot.structure.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.structure.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.structure.state_age_bars,
            "state_changes_50": snapshot.structure.state_changes_50,
        },
        "progression": {
            "progress": snapshot.progression.progress,
            "stage": snapshot.progression.stage_code,
            "stage_label": snapshot.progression.stage_code.map(labels::stage_label),
            "leg_direction": snapshot.progression.leg_direction,
            "leg_direction_label": snapshot.progression.leg_direction.map(labels::leg_direction_label),
            "leg_scale": snapshot.progression.leg_scale_code.map(labels::scale_label),
            "percentile": snapshot.progression.size_percentile,
            "readiness": snapshot.progression.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.progression.readiness.code()),
            "reason": snapshot.progression.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.progression.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.progression.state_age_bars,
            "state_changes_50": snapshot.progression.state_changes_50,
        },
        "context": {
            "atr_ratio": snapshot.context.atr_ratio,
            "rvol_ratio": snapshot.context.rvol_ratio,
            "code": snapshot.context.context_code,
            "code_label": snapshot.context.context_code.map(labels::context_label),
        },
        "consensus": consensus.map(consensus_value),
        "events": event_summary.map(summary_value),
    })
}

fn consensus_value(consensus: &TimeframeConsensus) -> Value {
    json!({
        "timeframe": labels::timeframe_label(consensus.timeframe_secs),
        "bullish": consensus.bullish_axis_count,
        "bearish": consensus.bearish_axis_count,
        "neutral": consensus.neutral_axis_count,
        "weakest_axis": consensus.weakest_directional_axis_code,
        "weakest_axis_label": consensus.weakest_directional_axis_code.map(labels::axis_label),
    })
}

fn summary_value(summary: &TimeframeEventSummary) -> Value {
    json!({
        "timeframe": labels::timeframe_label(summary.timeframe_secs),
        "counts_by_type": summary.counts_by_type,
        "counts_by_subtype": {
            "level": summary.counts_by_subtype.level,
            "divergence": summary.counts_by_subtype.divergence,
            "signal": summary.counts_by_subtype.signal,
        },
        "latest_type": summary.latest_event_type_code,
        "latest_type_label": summary.latest_event_type_code.map(labels::event_type_label),
        "latest_subtype": summary.latest_event_subtype_code,
        "latest_subtype_label": summary.latest_event_type_code.zip(summary.latest_event_subtype_code).map(|(type_code, subtype_code)| labels::event_subtype_label(type_code, subtype_code)),
        "latest_occurred_bar": summary.latest_occurred_bar_index,
        "latest_available_bar": summary.latest_available_bar_index,
        "latest_age_bars": summary.latest_event_age_bars,
    })
}

fn aggregates_value(aggregates: &CrossTimeframeAggregates) -> Value {
    json!({
        "ladder": aggregates.ladder.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
        "direction": {
            "timeframes": aggregates.direction.timeframes.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
            "mean": aggregates.direction.direction_mean,
            "bullish_share": aggregates.direction.direction_bullish_share,
            "bearish_share": aggregates.direction.direction_bearish_share,
            "neutral_share": aggregates.direction.direction_neutral_share,
            "gradient": aggregates.direction.direction_gradient,
            "ready_count": aggregates.direction.direction_ready_count,
            "timeframe_count": aggregates.direction.direction_timeframe_count,
        },
        "momentum": {
            "timeframes": aggregates.momentum.timeframes.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
            "force_mean": aggregates.momentum.momentum_force_mean,
            "acceleration_mean": aggregates.momentum.momentum_acceleration_mean,
            "positive_share": aggregates.momentum.momentum_positive_share,
            "negative_share": aggregates.momentum.momentum_negative_share,
            "force_fast_slow_gap": aggregates.momentum.momentum_force_fast_slow_gap,
            "acceleration_fast_slow_gap": aggregates.momentum.momentum_acceleration_fast_slow_gap,
            "gradient": aggregates.momentum.momentum_gradient,
            "ready_count": aggregates.momentum.momentum_ready_count,
            "timeframe_count": aggregates.momentum.momentum_timeframe_count,
        },
        "structure": {
            "timeframes": aggregates.structure.timeframes.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
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
            "timeframes": aggregates.progression.timeframes.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
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
            "timeframes": aggregates.events.timeframes.iter().map(|seconds| labels::timeframe_label(*seconds)).collect::<Vec<_>>(),
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
            "per_timeframe": aggregates.events.per_timeframe.iter().map(summary_value).collect::<Vec<_>>(),
        },
    })
}

fn scale_stage_value(stage: &quantrule_model::aggregate::ScaleStageAggregates) -> Value {
    json!({
        "histogram": stage.stage_histogram,
        "mode": stage.stage_mode,
        "mode_label": stage.stage_mode.map(|code| labels::stage_label(code as i8)),
        "min": stage.stage_min,
        "max": stage.stage_max,
        "span": stage.stage_span,
        "modal_share": stage.modal_stage_share,
    })
}

fn axis_value(timeframe: &TimeframeMarketState, axis: Axis) -> Value {
    let snapshot = &timeframe.snapshot;
    let label = labels::timeframe_label(snapshot.timeframe_secs);
    match axis {
        Axis::Direction => json!({
            "timeframe": label,
            "bar": snapshot.latest_bar_index,
            "axis": "direction",
            "value": snapshot.direction.direction,
            "state": snapshot.direction.state_code,
            "state_label": snapshot.direction.state_code.map(labels::direction_state_label),
            "percentile": snapshot.direction.size_percentile,
            "readiness": snapshot.direction.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.direction.readiness.code()),
            "reason": snapshot.direction.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.direction.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.direction.state_age_bars,
            "state_changes_50": snapshot.direction.state_changes_50,
        }),
        Axis::Momentum => json!({
            "timeframe": label,
            "bar": snapshot.latest_bar_index,
            "axis": "momentum",
            "force": snapshot.momentum.force,
            "acceleration": snapshot.momentum.acceleration,
            "intensification": snapshot.momentum.intensification,
            "state": snapshot.momentum.state_code,
            "state_label": snapshot.momentum.state_code.map(labels::momentum_state_label),
            "percentile": snapshot.momentum.size_percentile,
            "readiness": snapshot.momentum.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.momentum.readiness.code()),
            "reason": snapshot.momentum.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.momentum.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.momentum.state_age_bars,
            "state_changes_50": snapshot.momentum.state_changes_50,
        }),
        Axis::Structure => json!({
            "timeframe": label,
            "bar": snapshot.latest_bar_index,
            "axis": "structure",
            "score": snapshot.structure.score,
            "code": snapshot.structure.state_code,
            "code_label": snapshot.structure.state_code.map(labels::structure_label),
            "scale": labels::scale_label(snapshot.structure.scale_code),
            "high_relation": snapshot.structure.high_relation_code,
            "low_relation": snapshot.structure.low_relation_code,
            "ema_distance_atr_20": snapshot.structure.ema_distance_atr_20,
            "ema_distance_atr_50": snapshot.structure.ema_distance_atr_50,
            "ema_distance_atr_200": snapshot.structure.ema_distance_atr_200,
            "ema_cross_20": snapshot.structure.ema_cross_code_20,
            "ema_cross_50": snapshot.structure.ema_cross_code_50,
            "ema_cross_200": snapshot.structure.ema_cross_code_200,
            "ema_retest_20": snapshot.structure.ema_retest_code_20,
            "ema_retest_50": snapshot.structure.ema_retest_code_50,
            "ema_retest_200": snapshot.structure.ema_retest_code_200,
            "percentile": snapshot.structure.size_percentile,
            "readiness": snapshot.structure.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.structure.readiness.code()),
            "reason": snapshot.structure.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.structure.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.structure.state_age_bars,
            "state_changes_50": snapshot.structure.state_changes_50,
        }),
        Axis::Progression => json!({
            "timeframe": label,
            "bar": snapshot.latest_bar_index,
            "axis": "progression",
            "progress": snapshot.progression.progress,
            "stage": snapshot.progression.stage_code,
            "stage_label": snapshot.progression.stage_code.map(labels::stage_label),
            "leg_direction": snapshot.progression.leg_direction,
            "leg_direction_label": snapshot.progression.leg_direction.map(labels::leg_direction_label),
            "leg_scale": snapshot.progression.leg_scale_code.map(labels::scale_label),
            "percentile": snapshot.progression.size_percentile,
            "readiness": snapshot.progression.readiness.code(),
            "readiness_label": labels::readiness_label(snapshot.progression.readiness.code()),
            "reason": snapshot.progression.unavailable_reason.map(|reason| reason.code()),
            "reason_label": snapshot.progression.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": snapshot.progression.state_age_bars,
            "state_changes_50": snapshot.progression.state_changes_50,
        }),
    }
}

fn event_value(timeframe: &TimeframeModel, event: &EventRecord) -> Value {
    let occurred_time = timeframe
        .candle(event.occurred_bar_index)
        .map(|candle| format_utc_timestamp(candle.timestamp()));
    let available_time = timeframe
        .candle(event.available_bar_index)
        .map(|candle| format_utc_timestamp(candle.timestamp()));
    json!({
        "timeframe": labels::timeframe_label(timeframe.timeframe_secs()),
        "type": event.event_type.code(),
        "type_label": labels::event_type_label(event.event_type.code()),
        "subtype": event.event_subtype_code,
        "subtype_label": labels::event_subtype_label(event.event_type.code(), event.event_subtype_code),
        "source": event.source_code,
        "source_label": labels::source_label(event.source_code),
        "occurred_bar": event.occurred_bar_index,
        "occurred_time": occurred_time,
        "available_bar": event.available_bar_index,
        "available_time": available_time,
        "direction": event.direction_code,
        "reference_code": event.reference_code,
        "primary_value": event.primary_value,
        "reference_value": event.reference_value,
    })
}

fn leg_value(leg: ProgressionLeg, scale: u8) -> Value {
    json!({
        "leg_id": leg.leg_id,
        "scale": labels::scale_label(scale),
        "origin_price": leg.origin_price,
        "origin_occurred_at": leg.origin_occurred_at,
        "origin_available_at": leg.origin_available_at,
        "origin_confirmed": leg.origin_confirmed_code,
        "direction": leg.direction_code,
        "direction_label": labels::leg_direction_label(leg.direction_code),
        "target_price": leg.target_price,
        "target_basis": leg.target_basis_code,
        "target_basis_label": leg.target_basis_code.map(labels::target_basis_label),
        "target_version": leg.target_version,
        "target_stable": leg.target_stable_code,
        "progress": leg.progress,
        "displacement_price": leg.displacement_price,
        "displacement_atr": leg.displacement_atr,
        "lifecycle": leg.lifecycle_code,
        "lifecycle_label": labels::lifecycle_label(leg.lifecycle_code),
        "stage": leg.stage_code,
        "stage_label": leg.stage_code.map(labels::stage_label),
        "target_reached": leg.target_reached_code,
    })
}
