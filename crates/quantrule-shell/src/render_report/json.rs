//! The numeric JSON envelope for the full-period market report.
//!
//! The shell copies every published number, null, list, and stable code from
//! [`MarketReport`]; it computes no statistic and adds no label. The envelope
//! carries the provenance the contract requires: the compiled model version,
//! the view-contract version, the caller's data version, and the requested
//! period.

use std::io;

use quantrule_model::aggregate::{
    CrossTimeframeAggregates, EventSubtypeCounts, ScaleStageAggregates, TimeframeEventSummary,
};
use quantrule_model::report::{
    AxisReport, ChartReport, EventReport, EventTypeCounts, LatestEvent, LatestState, MarketReport,
    ReportChartBar, ReportChartEvent, ReportConsensus, ReportContext, ReportDirection, ReportLeg,
    ReportMomentum, ReportProgression, ReportReadings, ReportStructure, TimeframeEventReport,
    TimeframeReport,
};
use serde_json::{Value, json};

use super::VIEW_CONTRACT;

/// Renders the report envelope as compact JSON bytes with a trailing newline.
///
/// # Errors
///
/// Returns the serializer's I/O error when the value cannot be encoded.
pub(crate) fn json_bytes(
    report: &MarketReport,
    data_version: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> io::Result<Vec<u8>> {
    let value = json!({
        "model_version": env!("CARGO_PKG_VERSION"),
        "view_contract": VIEW_CONTRACT,
        "data_version": data_version,
        "requested_period": {
            "from": from,
            "to": to,
        },
        "market_report": report_value(report),
    });
    let text = serde_json::to_string(&value).map_err(io::Error::other)?;
    Ok(format!("{text}\n").into_bytes())
}

fn report_value(report: &MarketReport) -> Value {
    json!({
        "period": {
            "first_open_time": report.period.first_open_time,
            "last_open_time": report.period.last_open_time,
        },
        "ladder": report.ladder.to_vec(),
        "timeframes": report
            .timeframes
            .iter()
            .map(timeframe_value)
            .collect::<Vec<_>>(),
        "cross_timeframe": cross_timeframe_value(&report.cross_timeframe),
        "events": event_report_value(&report.events),
        "charts": chart_value(&report.charts),
    })
}

fn timeframe_value(timeframe: &TimeframeReport) -> Value {
    json!({
        "timeframe_secs": timeframe.timeframe_secs,
        "bars": timeframe.bars,
        "expected_bars": timeframe.expected_bars,
        "missing_bars": timeframe.missing_bars,
        "coverage_share": timeframe.coverage_share,
        "first_open_time": timeframe.first_open_time,
        "last_open_time": timeframe.last_open_time,
        "axes": timeframe.axes.iter().map(axis_value).collect::<Vec<_>>(),
        "latest": timeframe.latest.as_ref().map(latest_value),
    })
}

fn axis_value(axis: &AxisReport) -> Value {
    json!({
        "axis": axis.axis.code(),
        "present_count": axis.present_count,
        "null_count": axis.null_count,
        "min": axis.min,
        "max": axis.max,
        "mean": axis.mean,
        "code_counts": axis.code_counts,
        "code_null_count": axis.code_null_count,
    })
}

fn latest_value(latest: &LatestState) -> Value {
    json!({
        "bar_index": latest.bar_index,
        "open_time": latest.open_time,
        "readings": readings_value(&latest.readings),
        "direction": direction_value(&latest.direction),
        "momentum": momentum_value(&latest.momentum),
        "structure": structure_value(&latest.structure),
        "progression": progression_value(&latest.progression),
        "context": context_value(&latest.context),
        "consensus": consensus_value(&latest.consensus),
    })
}

fn readings_value(readings: &ReportReadings) -> Value {
    json!({
        "ema20": readings.ema20,
        "ema50": readings.ema50,
        "ema200": readings.ema200,
        "rsi14": readings.rsi14,
        "atr14": readings.atr14,
        "rvol20": readings.rvol20,
    })
}

fn direction_value(direction: &ReportDirection) -> Value {
    json!({
        "direction": direction.direction,
        "state_code": direction.state_code,
        "size_percentile": direction.size_percentile,
        "readiness_code": direction.readiness_code,
        "reason_code": direction.reason_code,
        "state_age_bars": direction.state_age_bars,
        "state_changes_50": direction.state_changes_50,
    })
}

fn momentum_value(momentum: &ReportMomentum) -> Value {
    json!({
        "force": momentum.force,
        "acceleration": momentum.acceleration,
        "intensification": momentum.intensification,
        "state_code": momentum.state_code,
        "size_percentile": momentum.size_percentile,
        "readiness_code": momentum.readiness_code,
        "reason_code": momentum.reason_code,
        "state_age_bars": momentum.state_age_bars,
        "state_changes_50": momentum.state_changes_50,
    })
}

fn structure_value(structure: &ReportStructure) -> Value {
    json!({
        "high_relation_code": structure.high_relation_code,
        "low_relation_code": structure.low_relation_code,
        "score": structure.score,
        "state_code": structure.state_code,
        "scale_code": structure.scale_code,
        "ema_distance_atr_20": structure.ema_distance_atr_20,
        "ema_distance_atr_50": structure.ema_distance_atr_50,
        "ema_distance_atr_200": structure.ema_distance_atr_200,
        "ema_cross_code_20": structure.ema_cross_code_20,
        "ema_cross_code_50": structure.ema_cross_code_50,
        "ema_cross_code_200": structure.ema_cross_code_200,
        "ema_retest_code_20": structure.ema_retest_code_20,
        "ema_retest_code_50": structure.ema_retest_code_50,
        "ema_retest_code_200": structure.ema_retest_code_200,
        "size_percentile": structure.size_percentile,
        "readiness_code": structure.readiness_code,
        "reason_code": structure.reason_code,
        "state_age_bars": structure.state_age_bars,
        "state_changes_50": structure.state_changes_50,
    })
}

fn progression_value(progression: &ReportProgression) -> Value {
    json!({
        "progress": progression.progress,
        "stage_code": progression.stage_code,
        "leg_direction": progression.leg_direction,
        "leg_scale_code": progression.leg_scale_code,
        "size_percentile": progression.size_percentile,
        "readiness_code": progression.readiness_code,
        "reason_code": progression.reason_code,
        "state_age_bars": progression.state_age_bars,
        "state_changes_50": progression.state_changes_50,
        "leg": progression.leg.as_ref().map(leg_value),
    })
}

fn leg_value(leg: &ReportLeg) -> Value {
    json!({
        "leg_id": leg.leg_id,
        "timeframe_secs": leg.timeframe_secs,
        "scale_code": leg.scale_code,
        "origin_price": leg.origin_price,
        "origin_occurred_at": leg.origin_occurred_at,
        "origin_available_at": leg.origin_available_at,
        "origin_confirmed_code": leg.origin_confirmed_code,
        "direction_code": leg.direction_code,
        "target_price": leg.target_price,
        "target_basis_code": leg.target_basis_code,
        "target_version": leg.target_version,
        "target_stable_code": leg.target_stable_code,
        "progress": leg.progress,
        "displacement_price": leg.displacement_price,
        "displacement_atr": leg.displacement_atr,
        "lifecycle_code": leg.lifecycle_code,
        "stage_code": leg.stage_code,
        "target_reached_code": leg.target_reached_code,
    })
}

fn context_value(context: &ReportContext) -> Value {
    json!({
        "atr_ratio": context.atr_ratio,
        "rvol_ratio": context.rvol_ratio,
        "context_code": context.context_code,
    })
}

fn consensus_value(consensus: &ReportConsensus) -> Value {
    json!({
        "bullish_axis_count": consensus.bullish_axis_count,
        "bearish_axis_count": consensus.bearish_axis_count,
        "neutral_axis_count": consensus.neutral_axis_count,
        "weakest_directional_axis_code": consensus.weakest_directional_axis_code,
    })
}

fn event_report_value(events: &EventReport) -> Value {
    json!({
        "total": events.total,
        "by_type": type_counts_value(events.by_type),
        "by_subtype": subtype_counts_value(&events.by_subtype),
        "timeframes": events
            .timeframes
            .iter()
            .map(timeframe_event_value)
            .collect::<Vec<_>>(),
    })
}

fn timeframe_event_value(events: &TimeframeEventReport) -> Value {
    json!({
        "timeframe_secs": events.timeframe_secs,
        "total": events.total,
        "by_type": type_counts_value(events.by_type),
        "by_subtype": subtype_counts_value(&events.by_subtype),
        "latest": events.latest.as_ref().map(latest_event_value),
    })
}

fn type_counts_value(counts: EventTypeCounts) -> Value {
    json!({
        "level": counts.level,
        "divergence": counts.divergence,
        "signal": counts.signal,
    })
}

fn subtype_counts_value(counts: &EventSubtypeCounts) -> Value {
    json!({
        "level": counts.level,
        "divergence": counts.divergence,
        "signal": counts.signal,
    })
}

fn latest_event_value(event: &LatestEvent) -> Value {
    json!({
        "event_type_code": event.event_type_code,
        "event_subtype_code": event.event_subtype_code,
        "source_code": event.source_code,
        "occurred_bar_index": event.occurred_bar_index,
        "available_bar_index": event.available_bar_index,
        "direction_code": event.direction_code,
        "reference_code": event.reference_code,
        "primary_value": event.primary_value,
        "reference_value": event.reference_value,
    })
}

fn chart_value(chart: &ChartReport) -> Value {
    json!({
        "timeframe_secs": chart.timeframe_secs,
        "source_bars": chart.source_bars,
        "bucket_size": chart.bucket_size,
        "bars": chart.bars.iter().map(chart_bar_value).collect::<Vec<_>>(),
        "events": chart.events.iter().map(chart_event_value).collect::<Vec<_>>(),
    })
}

fn chart_bar_value(bar: &ReportChartBar) -> Value {
    json!({
        "first_bar_index": bar.first_bar_index,
        "last_bar_index": bar.last_bar_index,
        "open_time": bar.open_time,
        "last_open_time": bar.last_open_time,
        "open": bar.open,
        "high": bar.high,
        "low": bar.low,
        "close": bar.close,
        "volume": bar.volume,
        "direction": bar.direction,
        "momentum": bar.momentum,
        "structure": bar.structure,
        "progression_stage_code": bar.progression_stage_code,
        "progression_progress": bar.progression_progress,
    })
}

fn chart_event_value(event: &ReportChartEvent) -> Value {
    json!({
        "bucket_index": event.bucket_index,
        "available_bar_index": event.available_bar_index,
        "occurred_bar_index": event.occurred_bar_index,
        "event_type_code": event.event_type_code,
        "event_subtype_code": event.event_subtype_code,
        "direction_code": event.direction_code,
    })
}

fn cross_timeframe_value(aggregates: &CrossTimeframeAggregates) -> Value {
    json!({
        "ladder": aggregates.ladder.to_vec(),
        "direction": {
            "timeframes": aggregates.direction.timeframes.to_vec(),
            "direction_timeframe_count": aggregates.direction.direction_timeframe_count,
            "direction_mean": aggregates.direction.direction_mean,
            "direction_bullish_share": aggregates.direction.direction_bullish_share,
            "direction_bearish_share": aggregates.direction.direction_bearish_share,
            "direction_neutral_share": aggregates.direction.direction_neutral_share,
            "direction_gradient": aggregates.direction.direction_gradient,
            "direction_ready_count": aggregates.direction.direction_ready_count,
        },
        "momentum": {
            "timeframes": aggregates.momentum.timeframes.to_vec(),
            "momentum_timeframe_count": aggregates.momentum.momentum_timeframe_count,
            "momentum_force_mean": aggregates.momentum.momentum_force_mean,
            "momentum_acceleration_mean": aggregates.momentum.momentum_acceleration_mean,
            "momentum_positive_share": aggregates.momentum.momentum_positive_share,
            "momentum_negative_share": aggregates.momentum.momentum_negative_share,
            "momentum_force_fast_slow_gap": aggregates.momentum.momentum_force_fast_slow_gap,
            "momentum_acceleration_fast_slow_gap": aggregates.momentum.momentum_acceleration_fast_slow_gap,
            "momentum_gradient": aggregates.momentum.momentum_gradient,
            "momentum_ready_count": aggregates.momentum.momentum_ready_count,
        },
        "structure": {
            "timeframes": aggregates.structure.timeframes.to_vec(),
            "structure_timeframe_count": aggregates.structure.structure_timeframe_count,
            "structure_code_histogram": aggregates.structure.structure_code_histogram,
            "structure_bullish_share": aggregates.structure.structure_bullish_share,
            "structure_bearish_share": aggregates.structure.structure_bearish_share,
            "structure_neutral_share": aggregates.structure.structure_neutral_share,
            "structure_state_span": aggregates.structure.structure_state_span,
            "structure_gradient": aggregates.structure.structure_gradient,
            "structure_ready_share": aggregates.structure.structure_ready_share,
            "structure_conflict_timeframe_count": aggregates.structure.structure_conflict_timeframe_count,
            "level_cluster_count": aggregates.structure.level_cluster_count,
            "largest_level_cluster_timeframe_count": aggregates.structure.largest_level_cluster_timeframe_count,
            "largest_level_cluster_span_atr": aggregates.structure.largest_level_cluster_span_atr,
        },
        "progression": {
            "timeframes": aggregates.progression.timeframes.to_vec(),
            "progression_timeframe_count": aggregates.progression.progression_timeframe_count,
            "minor": scale_stage_value(&aggregates.progression.minor),
            "major": scale_stage_value(&aggregates.progression.major),
            "active_major_leg_count": aggregates.progression.active_major_leg_count,
            "active_minor_leg_count": aggregates.progression.active_minor_leg_count,
            "progression_ready_share": aggregates.progression.progression_ready_share,
            "leg_direction_agreement_share": aggregates.progression.leg_direction_agreement_share,
            "cross_timeframe_leg_match_count": aggregates.progression.cross_timeframe_leg_match_count,
        },
        "events": {
            "timeframes": aggregates.events.timeframes.to_vec(),
            "per_timeframe": aggregates.events.per_timeframe.iter().map(event_summary_value).collect::<Vec<_>>(),
            "event_count_by_type": aggregates.events.event_count_by_type,
            "event_count_by_subtype": subtype_counts_value(&aggregates.events.event_count_by_subtype),
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
        "counts_by_subtype": subtype_counts_value(&summary.counts_by_subtype),
        "latest_event_type_code": summary.latest_event_type_code,
        "latest_event_subtype_code": summary.latest_event_subtype_code,
        "latest_occurred_bar_index": summary.latest_occurred_bar_index,
        "latest_available_bar_index": summary.latest_available_bar_index,
        "latest_event_age_bars": summary.latest_event_age_bars,
    })
}

#[cfg(test)]
mod tests {
    use quantrule_model::report::MarketReport;
    use quantrule_model::run::ModelRun;
    use quantrule_model::state::MarketStateConfig;
    use serde_json::Value;

    use super::json_bytes;

    #[test]
    fn an_empty_report_renders_the_full_envelope() {
        let run = ModelRun::compute(&[], &MarketStateConfig::default());
        let report = MarketReport::compute(&run, &[60, 300, 900, 3_600]);

        let bytes = json_bytes(&report, "v1", None, None).expect("the report serializes");
        assert!(bytes.ends_with(b"\n"));
        let value: Value = serde_json::from_slice(&bytes).expect("the bytes are JSON");

        assert_eq!(value["view_contract"], "2026-10-27");
        assert_eq!(value["data_version"], "v1");
        assert!(value["requested_period"]["from"].is_null());
        assert!(value["requested_period"]["to"].is_null());
        assert_eq!(
            value["market_report"]["period"]["first_open_time"],
            Value::Null
        );
        assert_eq!(
            value["market_report"]["ladder"].as_array().map(Vec::len),
            Some(4)
        );
        assert_eq!(
            value["market_report"]["timeframes"]
                .as_array()
                .map(Vec::len),
            Some(4)
        );
        assert_eq!(
            value["market_report"]["charts"]["bars"],
            Value::Array(Vec::new())
        );
        assert!(value["market_report"]["events"]["total"].is_number());
    }

    #[test]
    fn the_envelope_keeps_the_requested_period() {
        let run = ModelRun::compute(&[], &MarketStateConfig::default());
        let report = MarketReport::compute(&run, &[]);

        let bytes = json_bytes(&report, "sol:2025", Some("2025-01-01"), Some("2025-12-31"))
            .expect("the report serializes");
        let value: Value = serde_json::from_slice(&bytes).expect("the bytes are JSON");

        assert_eq!(value["requested_period"]["from"], "2025-01-01");
        assert_eq!(value["requested_period"]["to"], "2025-12-31");
        assert_eq!(value["market_report"]["ladder"], Value::Array(Vec::new()));
    }

    #[test]
    fn the_envelope_has_no_execution_fields() {
        let run = ModelRun::compute(&[], &MarketStateConfig::default());
        let report = MarketReport::compute(&run, &[60, 300, 900, 3_600]);

        let bytes = json_bytes(&report, "v1", None, None).expect("the report serializes");
        let text = String::from_utf8(bytes).expect("the report is UTF-8");

        for forbidden in [
            "\"order",
            "\"fill",
            "\"position",
            "\"trade",
            "\"equity",
            "\"pnl",
            "\"drawdown",
            "\"strategy",
        ] {
            assert!(
                !text.contains(forbidden),
                "the report contains the execution field {forbidden}"
            );
        }
    }
}
