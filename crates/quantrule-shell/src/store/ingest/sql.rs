//! SQL statements and parameter bindings for the dataset writer.
//!
//! The column lists are explicit so a schema change is a round-trip test
//! failure rather than a silent default. Values are copied from the analytics
//! rows; this module never computes one.

use quantrule_analytics::layers::{
    ConsensusRow, EventRow, HigherBlock, Layer2Row, Layer3Row, LegRow, LevelRow, Metrics, PivotRow,
    ReadingsSnapshot,
};
use quantrule_analytics::resolution::AxisResolutions;
use quantrule_data::Candle;
use quantrule_model::aggregate::CrossTimeframeAggregates;
use rusqlite::types::Value;

use super::LAYER3_HIGHER_TIMEFRAMES;

pub(super) fn bar_params(dataset_id: i64, row: &Layer2Row) -> Vec<Value> {
    let mut values = Vec::with_capacity(74);
    values.push(Value::from(dataset_id));
    values.push(Value::from(row.layer1.timeframe_secs as i64));
    values.push(Value::from(row.layer1.bar_index as i64));
    push_candle(&mut values, &row.layer1.candle);
    push_readings(&mut values, &row.layer1.readings);
    push_metrics(&mut values, &row.layer1.metrics);
    push_resolutions(&mut values, &row.resolution);
    values
}

/// Appends a candle's timestamp, OHLC, and volume.
fn push_candle(values: &mut Vec<Value>, candle: &Candle) {
    values.push(Value::from(candle.timestamp()));
    values.push(Value::from(candle.open()));
    values.push(Value::from(candle.high()));
    values.push(Value::from(candle.low()));
    values.push(Value::from(candle.close()));
    values.push(Value::from(candle.volume()));
}

/// Appends the six scalar readings.
fn push_readings(values: &mut Vec<Value>, readings: &ReadingsSnapshot) {
    values.push(Value::from(readings.ema20));
    values.push(Value::from(readings.ema50));
    values.push(Value::from(readings.ema200));
    values.push(Value::from(readings.rsi14));
    values.push(Value::from(readings.atr14));
    values.push(Value::from(readings.rvol20));
}

/// Appends the 47 metric columns, in the schema's order.
fn push_metrics(values: &mut Vec<Value>, metrics: &Metrics) {
    values.push(Value::from(metrics.direction.direction));
    values.push(Value::from(metrics.direction.state_code.map(i64::from)));
    values.push(Value::from(metrics.direction.size_percentile));
    values.push(Value::from(i64::from(metrics.direction.readiness_code)));
    values.push(Value::from(
        metrics.direction.unavailable_reason_code.map(i64::from),
    ));
    values.push(Value::from(metrics.direction.state_age_bars as i64));
    values.push(Value::from(metrics.direction.state_changes_50 as i64));
    values.push(Value::from(metrics.momentum.force));
    values.push(Value::from(metrics.momentum.acceleration));
    values.push(Value::from(metrics.momentum.intensification));
    values.push(Value::from(metrics.momentum.state_code.map(i64::from)));
    values.push(Value::from(metrics.momentum.size_percentile));
    values.push(Value::from(i64::from(metrics.momentum.readiness_code)));
    values.push(Value::from(
        metrics.momentum.unavailable_reason_code.map(i64::from),
    ));
    values.push(Value::from(metrics.momentum.state_age_bars as i64));
    values.push(Value::from(metrics.momentum.state_changes_50 as i64));
    values.push(Value::from(metrics.structure.score));
    values.push(Value::from(metrics.structure.state_code.map(i64::from)));
    values.push(Value::from(i64::from(metrics.structure.scale_code)));
    values.push(Value::from(
        metrics.structure.high_relation_code.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.low_relation_code.map(i64::from),
    ));
    values.push(Value::from(metrics.structure.ema_distance_atr_20));
    values.push(Value::from(metrics.structure.ema_distance_atr_50));
    values.push(Value::from(metrics.structure.ema_distance_atr_200));
    values.push(Value::from(
        metrics.structure.ema_cross_code_20.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.ema_cross_code_50.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.ema_cross_code_200.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.ema_retest_code_20.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.ema_retest_code_50.map(i64::from),
    ));
    values.push(Value::from(
        metrics.structure.ema_retest_code_200.map(i64::from),
    ));
    values.push(Value::from(metrics.structure.size_percentile));
    values.push(Value::from(i64::from(metrics.structure.readiness_code)));
    values.push(Value::from(
        metrics.structure.unavailable_reason_code.map(i64::from),
    ));
    values.push(Value::from(metrics.structure.state_age_bars as i64));
    values.push(Value::from(metrics.structure.state_changes_50 as i64));
    values.push(Value::from(metrics.progression.progress));
    values.push(Value::from(metrics.progression.stage_code.map(i64::from)));
    values.push(Value::from(
        metrics.progression.leg_direction.map(i64::from),
    ));
    values.push(Value::from(
        metrics.progression.leg_scale_code.map(i64::from),
    ));
    values.push(Value::from(metrics.progression.size_percentile));
    values.push(Value::from(i64::from(metrics.progression.readiness_code)));
    values.push(Value::from(
        metrics.progression.unavailable_reason_code.map(i64::from),
    ));
    values.push(Value::from(metrics.progression.state_age_bars as i64));
    values.push(Value::from(metrics.progression.state_changes_50 as i64));
    values.push(Value::from(metrics.context.atr_ratio));
    values.push(Value::from(metrics.context.rvol_ratio));
    values.push(Value::from(metrics.context.context_code.map(i64::from)));
}

/// Appends the 12 resolution columns, in the schema's order.
fn push_resolutions(values: &mut Vec<Value>, resolution: &AxisResolutions) {
    values.push(Value::from(resolution.direction.continuous));
    values.push(Value::from(resolution.direction.ordinal.map(i64::from)));
    values.push(Value::from(resolution.direction.reason.map(i64::from)));
    values.push(Value::from(resolution.momentum.continuous));
    values.push(Value::from(resolution.momentum.ordinal.map(i64::from)));
    values.push(Value::from(resolution.momentum.reason.map(i64::from)));
    values.push(Value::from(resolution.structure.continuous));
    values.push(Value::from(resolution.structure.ordinal.map(i64::from)));
    values.push(Value::from(resolution.structure.reason.map(i64::from)));
    values.push(Value::from(resolution.progression.continuous));
    values.push(Value::from(resolution.progression.ordinal.map(i64::from)));
    values.push(Value::from(resolution.progression.reason.map(i64::from)));
}

pub(super) const BAR_INSERT: &str = "\
INSERT INTO bar (\
    dataset_id, timeframe_secs, bar_index, open_time, open, high, low, close, volume, \
    ema20, ema50, ema200, rsi14, atr14, rvol20, \
    direction, direction_state_code, direction_size_percentile, direction_readiness_code, \
    direction_unavailable_reason_code, direction_state_age_bars, direction_state_changes_50, \
    momentum_force, momentum_acceleration, momentum_intensification, momentum_state_code, \
    momentum_size_percentile, momentum_readiness_code, momentum_unavailable_reason_code, \
    momentum_state_age_bars, momentum_state_changes_50, \
    structure_score, structure_code, structure_scale_code, high_relation_code, low_relation_code, \
    ema_distance_atr_20, ema_distance_atr_50, ema_distance_atr_200, \
    ema_cross_code_20, ema_cross_code_50, ema_cross_code_200, \
    ema_retest_code_20, ema_retest_code_50, ema_retest_code_200, \
    structure_size_percentile, structure_readiness_code, structure_unavailable_reason_code, \
    structure_state_age_bars, structure_state_changes_50, \
    progress, progression_stage_code, progression_leg_direction, progression_leg_scale_code, \
    progression_size_percentile, progression_readiness_code, progression_unavailable_reason_code, \
    progression_state_age_bars, progression_state_changes_50, \
    atr_ratio, rvol_ratio, context_code, \
    direction_continuous, direction_ordinal, direction_resolution_reason_code, \
    momentum_continuous, momentum_ordinal, momentum_resolution_reason_code, \
    structure_continuous, structure_ordinal, structure_resolution_reason_code, \
    progression_continuous, progression_ordinal, progression_resolution_reason_code\
) VALUES (\
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, \
    ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, \
    ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, \
    ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70, \
    ?71, ?72, ?73, ?74\
)";

pub(super) fn list_text(values: &[u64]) -> String {
    let mut text = String::new();
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            text.push(';');
        }
        text.push_str(&value.to_string());
    }
    text
}

pub(super) fn consensus_params(dataset_id: i64, row: &ConsensusRow) -> [Value; 7] {
    [
        Value::from(dataset_id),
        Value::from(row.consensus.timeframe_secs as i64),
        Value::from(row.bar_index as i64),
        Value::from(i64::from(row.consensus.bullish_axis_count)),
        Value::from(i64::from(row.consensus.bearish_axis_count)),
        Value::from(i64::from(row.consensus.neutral_axis_count)),
        Value::from(row.consensus.weakest_directional_axis_code.map(i64::from)),
    ]
}

pub(super) const CONSENSUS_INSERT: &str = "\
INSERT INTO consensus (\
    dataset_id, timeframe_secs, bar_index, bullish_axis_count, bearish_axis_count, \
    neutral_axis_count, weakest_directional_axis_code\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

pub(super) fn layer3_params(dataset_id: i64, row: &Layer3Row) -> Vec<Value> {
    let mut values = Vec::with_capacity(285);
    values.push(Value::from(dataset_id));
    values.push(Value::from(row.base.layer1.bar_index as i64));
    for timeframe_secs in LAYER3_HIGHER_TIMEFRAMES {
        let block = row
            .higher
            .iter()
            .find(|block| block.timeframe_secs == timeframe_secs);
        push_block(&mut values, block);
    }
    push_aggregates(&mut values, &row.aggregates);
    debug_assert_eq!(values.len(), 285);
    values
}

/// Appends one 67-column higher block, or 67 nulls when it is absent.
fn push_block(values: &mut Vec<Value>, block: Option<&HigherBlock>) {
    let Some(block) = block else {
        for _ in 0..67 {
            values.push(Value::Null);
        }
        return;
    };
    values.push(Value::from(block.bar_index as i64));
    push_candle(values, &block.candle);
    push_metrics(values, &block.metrics);
    push_resolutions(values, &block.resolution);
    values.push(Value::from(block.staleness_secs as i64));
}

/// Appends the 82 cross-timeframe aggregate columns.
fn push_aggregates(values: &mut Vec<Value>, aggregates: &CrossTimeframeAggregates) {
    let direction = &aggregates.direction;
    let momentum = &aggregates.momentum;
    let structure = &aggregates.structure;
    let progression = &aggregates.progression;
    let events = &aggregates.events;
    let minor = &progression.minor;
    let major = &progression.major;
    let histogram = structure.structure_code_histogram;
    values.push(Value::from(list_text(&aggregates.ladder)));
    values.push(Value::from(list_text(&direction.timeframes)));
    values.push(Value::from(direction.direction_timeframe_count as i64));
    values.push(Value::from(direction.direction_mean));
    values.push(Value::from(direction.direction_bullish_share));
    values.push(Value::from(direction.direction_bearish_share));
    values.push(Value::from(direction.direction_neutral_share));
    values.push(Value::from(direction.direction_gradient));
    values.push(Value::from(direction.direction_ready_count as i64));
    values.push(Value::from(list_text(&momentum.timeframes)));
    values.push(Value::from(momentum.momentum_timeframe_count as i64));
    values.push(Value::from(momentum.momentum_force_mean));
    values.push(Value::from(momentum.momentum_acceleration_mean));
    values.push(Value::from(momentum.momentum_positive_share));
    values.push(Value::from(momentum.momentum_negative_share));
    values.push(Value::from(momentum.momentum_force_fast_slow_gap));
    values.push(Value::from(momentum.momentum_acceleration_fast_slow_gap));
    values.push(Value::from(momentum.momentum_gradient));
    values.push(Value::from(momentum.momentum_ready_count as i64));
    values.push(Value::from(list_text(&structure.timeframes)));
    values.push(Value::from(structure.structure_timeframe_count as i64));
    values.push(Value::from(i64::from(histogram[0])));
    values.push(Value::from(i64::from(histogram[1])));
    values.push(Value::from(i64::from(histogram[2])));
    values.push(Value::from(i64::from(histogram[3])));
    values.push(Value::from(i64::from(histogram[4])));
    values.push(Value::from(structure.structure_bullish_share));
    values.push(Value::from(structure.structure_bearish_share));
    values.push(Value::from(structure.structure_neutral_share));
    values.push(Value::from(structure.structure_state_span.map(i64::from)));
    values.push(Value::from(structure.structure_gradient));
    values.push(Value::from(structure.structure_ready_share));
    values.push(Value::from(
        structure.structure_conflict_timeframe_count as i64,
    ));
    values.push(Value::from(i64::from(structure.level_cluster_count)));
    values.push(Value::from(i64::from(
        structure.largest_level_cluster_timeframe_count,
    )));
    values.push(Value::from(structure.largest_level_cluster_span_atr));
    values.push(Value::from(list_text(&progression.timeframes)));
    values.push(Value::from(progression.progression_timeframe_count as i64));
    values.push(Value::from(i64::from(minor.stage_histogram[0])));
    values.push(Value::from(i64::from(minor.stage_histogram[1])));
    values.push(Value::from(i64::from(minor.stage_histogram[2])));
    values.push(Value::from(i64::from(minor.stage_histogram[3])));
    values.push(Value::from(i64::from(minor.stage_histogram[4])));
    values.push(Value::from(i64::from(minor.stage_histogram[5])));
    values.push(Value::from(minor.stage_mode.map(i64::from)));
    values.push(Value::from(minor.stage_min.map(i64::from)));
    values.push(Value::from(minor.stage_max.map(i64::from)));
    values.push(Value::from(minor.stage_span.map(i64::from)));
    values.push(Value::from(minor.modal_stage_share));
    values.push(Value::from(i64::from(major.stage_histogram[0])));
    values.push(Value::from(i64::from(major.stage_histogram[1])));
    values.push(Value::from(i64::from(major.stage_histogram[2])));
    values.push(Value::from(i64::from(major.stage_histogram[3])));
    values.push(Value::from(i64::from(major.stage_histogram[4])));
    values.push(Value::from(i64::from(major.stage_histogram[5])));
    values.push(Value::from(major.stage_mode.map(i64::from)));
    values.push(Value::from(major.stage_min.map(i64::from)));
    values.push(Value::from(major.stage_max.map(i64::from)));
    values.push(Value::from(major.stage_span.map(i64::from)));
    values.push(Value::from(major.modal_stage_share));
    values.push(Value::from(i64::from(progression.active_minor_leg_count)));
    values.push(Value::from(i64::from(progression.active_major_leg_count)));
    values.push(Value::from(progression.progression_ready_share));
    values.push(Value::from(progression.leg_direction_agreement_share));
    values.push(Value::from(i64::from(
        progression.cross_timeframe_leg_match_count,
    )));
    values.push(Value::from(i64::from(events.event_count_by_type[0])));
    values.push(Value::from(i64::from(events.event_count_by_type[1])));
    values.push(Value::from(i64::from(events.event_count_by_type[2])));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.level[0],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.level[1],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.level[2],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.divergence[0],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.divergence[1],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.divergence[2],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.divergence[3],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.signal[0],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.signal[1],
    )));
    values.push(Value::from(i64::from(
        events.event_count_by_subtype.signal[2],
    )));
    values.push(Value::from(i64::from(events.divergence_timeframe_count)));
    values.push(Value::from(events.divergence_direction_agreement_share));
    values.push(Value::from(i64::from(events.level_event_timeframe_count)));
    values.push(Value::from(i64::from(events.signal_event_timeframe_count)));
}

pub(super) const LAYER3_INSERT: &str = "\
INSERT INTO layer3 (\
    dataset_id, bar_index, m5_bar_index, m5_open_time, m5_open, m5_high, m5_low, m5_close, \
    m5_volume, m5_direction, m5_direction_state_code, m5_direction_size_percentile, m5_direction_readiness_code, m5_direction_unavailable_reason_code, m5_direction_state_age_bars, m5_direction_state_changes_50, \
    m5_momentum_force, m5_momentum_acceleration, m5_momentum_intensification, m5_momentum_state_code, m5_momentum_size_percentile, m5_momentum_readiness_code, m5_momentum_unavailable_reason_code, m5_momentum_state_age_bars, \
    m5_momentum_state_changes_50, m5_structure_score, m5_structure_code, m5_structure_scale_code, m5_high_relation_code, m5_low_relation_code, m5_ema_distance_atr_20, m5_ema_distance_atr_50, \
    m5_ema_distance_atr_200, m5_ema_cross_code_20, m5_ema_cross_code_50, m5_ema_cross_code_200, m5_ema_retest_code_20, m5_ema_retest_code_50, m5_ema_retest_code_200, m5_structure_size_percentile, \
    m5_structure_readiness_code, m5_structure_unavailable_reason_code, m5_structure_state_age_bars, m5_structure_state_changes_50, m5_progress, m5_progression_stage_code, m5_progression_leg_direction, m5_progression_leg_scale_code, \
    m5_progression_size_percentile, m5_progression_readiness_code, m5_progression_unavailable_reason_code, m5_progression_state_age_bars, m5_progression_state_changes_50, m5_atr_ratio, m5_rvol_ratio, m5_context_code, \
    m5_direction_continuous, m5_direction_ordinal, m5_direction_resolution_reason_code, m5_momentum_continuous, m5_momentum_ordinal, m5_momentum_resolution_reason_code, m5_structure_continuous, m5_structure_ordinal, \
    m5_structure_resolution_reason_code, m5_progression_continuous, m5_progression_ordinal, m5_progression_resolution_reason_code, m5_staleness_secs, m15_bar_index, m15_open_time, m15_open, \
    m15_high, m15_low, m15_close, m15_volume, m15_direction, m15_direction_state_code, m15_direction_size_percentile, m15_direction_readiness_code, \
    m15_direction_unavailable_reason_code, m15_direction_state_age_bars, m15_direction_state_changes_50, m15_momentum_force, m15_momentum_acceleration, m15_momentum_intensification, m15_momentum_state_code, m15_momentum_size_percentile, \
    m15_momentum_readiness_code, m15_momentum_unavailable_reason_code, m15_momentum_state_age_bars, m15_momentum_state_changes_50, m15_structure_score, m15_structure_code, m15_structure_scale_code, m15_high_relation_code, \
    m15_low_relation_code, m15_ema_distance_atr_20, m15_ema_distance_atr_50, m15_ema_distance_atr_200, m15_ema_cross_code_20, m15_ema_cross_code_50, m15_ema_cross_code_200, m15_ema_retest_code_20, \
    m15_ema_retest_code_50, m15_ema_retest_code_200, m15_structure_size_percentile, m15_structure_readiness_code, m15_structure_unavailable_reason_code, m15_structure_state_age_bars, m15_structure_state_changes_50, m15_progress, \
    m15_progression_stage_code, m15_progression_leg_direction, m15_progression_leg_scale_code, m15_progression_size_percentile, m15_progression_readiness_code, m15_progression_unavailable_reason_code, m15_progression_state_age_bars, m15_progression_state_changes_50, \
    m15_atr_ratio, m15_rvol_ratio, m15_context_code, m15_direction_continuous, m15_direction_ordinal, m15_direction_resolution_reason_code, m15_momentum_continuous, m15_momentum_ordinal, \
    m15_momentum_resolution_reason_code, m15_structure_continuous, m15_structure_ordinal, m15_structure_resolution_reason_code, m15_progression_continuous, m15_progression_ordinal, m15_progression_resolution_reason_code, m15_staleness_secs, \
    h1_bar_index, h1_open_time, h1_open, h1_high, h1_low, h1_close, h1_volume, h1_direction, \
    h1_direction_state_code, h1_direction_size_percentile, h1_direction_readiness_code, h1_direction_unavailable_reason_code, h1_direction_state_age_bars, h1_direction_state_changes_50, h1_momentum_force, h1_momentum_acceleration, \
    h1_momentum_intensification, h1_momentum_state_code, h1_momentum_size_percentile, h1_momentum_readiness_code, h1_momentum_unavailable_reason_code, h1_momentum_state_age_bars, h1_momentum_state_changes_50, h1_structure_score, \
    h1_structure_code, h1_structure_scale_code, h1_high_relation_code, h1_low_relation_code, h1_ema_distance_atr_20, h1_ema_distance_atr_50, h1_ema_distance_atr_200, h1_ema_cross_code_20, \
    h1_ema_cross_code_50, h1_ema_cross_code_200, h1_ema_retest_code_20, h1_ema_retest_code_50, h1_ema_retest_code_200, h1_structure_size_percentile, h1_structure_readiness_code, h1_structure_unavailable_reason_code, \
    h1_structure_state_age_bars, h1_structure_state_changes_50, h1_progress, h1_progression_stage_code, h1_progression_leg_direction, h1_progression_leg_scale_code, h1_progression_size_percentile, h1_progression_readiness_code, \
    h1_progression_unavailable_reason_code, h1_progression_state_age_bars, h1_progression_state_changes_50, h1_atr_ratio, h1_rvol_ratio, h1_context_code, h1_direction_continuous, h1_direction_ordinal, \
    h1_direction_resolution_reason_code, h1_momentum_continuous, h1_momentum_ordinal, h1_momentum_resolution_reason_code, h1_structure_continuous, h1_structure_ordinal, h1_structure_resolution_reason_code, h1_progression_continuous, \
    h1_progression_ordinal, h1_progression_resolution_reason_code, h1_staleness_secs, agg_ladder, agg_direction_timeframes, agg_direction_timeframe_count, agg_direction_mean, agg_direction_bullish_share, \
    agg_direction_bearish_share, agg_direction_neutral_share, agg_direction_gradient, agg_direction_ready_count, agg_momentum_timeframes, agg_momentum_timeframe_count, agg_momentum_force_mean, agg_momentum_acceleration_mean, \
    agg_momentum_positive_share, agg_momentum_negative_share, agg_momentum_force_fast_slow_gap, agg_momentum_acceleration_fast_slow_gap, agg_momentum_gradient, agg_momentum_ready_count, agg_structure_timeframes, agg_structure_timeframe_count, \
    agg_structure_code_minus2_count, agg_structure_code_minus1_count, agg_structure_code_zero_count, agg_structure_code_plus1_count, agg_structure_code_plus2_count, agg_structure_bullish_share, agg_structure_bearish_share, agg_structure_neutral_share, \
    agg_structure_state_span, agg_structure_gradient, agg_structure_ready_share, agg_structure_conflict_timeframe_count, agg_level_cluster_count, agg_largest_level_cluster_timeframe_count, agg_largest_level_cluster_span_atr, agg_progression_timeframes, \
    agg_progression_timeframe_count, agg_progression_minor_stage0_count, agg_progression_minor_stage1_count, agg_progression_minor_stage2_count, agg_progression_minor_stage3_count, agg_progression_minor_stage4_count, agg_progression_minor_stage5_count, agg_progression_minor_stage_mode, \
    agg_progression_minor_stage_min, agg_progression_minor_stage_max, agg_progression_minor_stage_span, agg_progression_minor_modal_stage_share, agg_progression_major_stage0_count, agg_progression_major_stage1_count, agg_progression_major_stage2_count, agg_progression_major_stage3_count, \
    agg_progression_major_stage4_count, agg_progression_major_stage5_count, agg_progression_major_stage_mode, agg_progression_major_stage_min, agg_progression_major_stage_max, agg_progression_major_stage_span, agg_progression_major_modal_stage_share, agg_progression_active_minor_leg_count, \
    agg_progression_active_major_leg_count, agg_progression_ready_share, agg_progression_leg_direction_agreement_share, agg_progression_cross_timeframe_leg_match_count, agg_event_count_level, agg_event_count_divergence, agg_event_count_signal, agg_event_count_level_test_retest, \
    agg_event_count_level_cross, agg_event_count_level_break, agg_event_count_divergence_regular_bullish, agg_event_count_divergence_regular_bearish, agg_event_count_divergence_hidden_bullish, agg_event_count_divergence_hidden_bearish, agg_event_count_signal_threshold_crossing, agg_event_count_signal_series_crossover, \
    agg_event_count_signal_state_change, agg_divergence_timeframe_count, agg_divergence_direction_agreement_share, agg_level_event_timeframe_count, agg_signal_event_timeframe_count\
) VALUES (\
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, \
    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, \
    ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, \
    ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, \
    ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, \
    ?81, ?82, ?83, ?84, ?85, ?86, ?87, ?88, ?89, ?90, ?91, ?92, ?93, ?94, ?95, ?96, \
    ?97, ?98, ?99, ?100, ?101, ?102, ?103, ?104, ?105, ?106, ?107, ?108, ?109, ?110, ?111, ?112, \
    ?113, ?114, ?115, ?116, ?117, ?118, ?119, ?120, ?121, ?122, ?123, ?124, ?125, ?126, ?127, ?128, \
    ?129, ?130, ?131, ?132, ?133, ?134, ?135, ?136, ?137, ?138, ?139, ?140, ?141, ?142, ?143, ?144, \
    ?145, ?146, ?147, ?148, ?149, ?150, ?151, ?152, ?153, ?154, ?155, ?156, ?157, ?158, ?159, ?160, \
    ?161, ?162, ?163, ?164, ?165, ?166, ?167, ?168, ?169, ?170, ?171, ?172, ?173, ?174, ?175, ?176, \
    ?177, ?178, ?179, ?180, ?181, ?182, ?183, ?184, ?185, ?186, ?187, ?188, ?189, ?190, ?191, ?192, \
    ?193, ?194, ?195, ?196, ?197, ?198, ?199, ?200, ?201, ?202, ?203, ?204, ?205, ?206, ?207, ?208, \
    ?209, ?210, ?211, ?212, ?213, ?214, ?215, ?216, ?217, ?218, ?219, ?220, ?221, ?222, ?223, ?224, \
    ?225, ?226, ?227, ?228, ?229, ?230, ?231, ?232, ?233, ?234, ?235, ?236, ?237, ?238, ?239, ?240, \
    ?241, ?242, ?243, ?244, ?245, ?246, ?247, ?248, ?249, ?250, ?251, ?252, ?253, ?254, ?255, ?256, \
    ?257, ?258, ?259, ?260, ?261, ?262, ?263, ?264, ?265, ?266, ?267, ?268, ?269, ?270, ?271, ?272, \
    ?273, ?274, ?275, ?276, ?277, ?278, ?279, ?280, ?281, ?282, ?283, ?284, ?285\
)";

pub(super) fn pivot_params(dataset_id: i64, row: &PivotRow) -> [Value; 8] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(i64::from(row.scale_code)),
        Value::from(i64::from(row.kind_code)),
        Value::from(row.price),
        Value::from(row.occurred_bar_index as i64),
        Value::from(row.available_bar_index as i64),
        Value::from(i64::from(row.confirmed_code)),
    ]
}

pub(super) const PIVOT_INSERT: &str = "\
INSERT INTO pivot (\
    dataset_id, timeframe_secs, scale_code, kind_code, price, occurred_bar_index, \
    available_bar_index, confirmed_code\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

pub(super) fn level_params(dataset_id: i64, row: &LevelRow) -> [Value; 8] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(i64::from(row.reference_code)),
        Value::from(i64::from(row.scale_code)),
        Value::from(i64::from(row.kind_code)),
        Value::from(row.price),
        Value::from(row.occurred_bar_index as i64),
        Value::from(row.available_bar_index as i64),
    ]
}

pub(super) const LEVEL_INSERT: &str = "\
INSERT INTO level (\
    dataset_id, timeframe_secs, reference_code, scale_code, kind_code, price, \
    occurred_bar_index, available_bar_index\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

pub(super) fn event_params(dataset_id: i64, row: &EventRow) -> [Value; 11] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(i64::from(row.event_type_code)),
        Value::from(i64::from(row.event_subtype_code)),
        Value::from(i64::from(row.source_code)),
        Value::from(row.occurred_bar_index as i64),
        Value::from(row.available_bar_index as i64),
        Value::from(i64::from(row.direction_code)),
        Value::from(i64::from(row.reference_code.unwrap_or(0))),
        Value::from(row.primary_value),
        Value::from(row.reference_value),
    ]
}

pub(super) const EVENT_INSERT: &str = "\
INSERT INTO event (\
    dataset_id, timeframe_secs, event_type_code, event_subtype_code, source_code, \
    occurred_bar_index, available_bar_index, direction_code, reference_code, \
    primary_value, reference_value\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)";

pub(super) fn leg_params(dataset_id: i64, row: &LegRow) -> [Value; 19] {
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(row.leg_id as i64),
        Value::from(i64::from(row.scale_code)),
        Value::from(row.origin_price),
        Value::from(row.origin_occurred_bar_index as i64),
        Value::from(row.origin_available_bar_index.map(|bar| bar as i64)),
        Value::from(i64::from(row.origin_confirmed_code)),
        Value::from(i64::from(row.leg_direction_code)),
        Value::from(row.target_price),
        Value::from(row.target_basis_code.map(i64::from)),
        Value::from(i64::from(row.target_version)),
        Value::from(i64::from(row.target_stable_code)),
        Value::from(row.progress),
        Value::from(row.displacement_price),
        Value::from(row.displacement_atr),
        Value::from(i64::from(row.lifecycle_code)),
        Value::from(row.stage_code.map(i64::from)),
        Value::from(row.target_reached_code.map(i64::from)),
    ]
}

pub(super) const LEG_INSERT: &str = "\
INSERT INTO leg (\
    dataset_id, timeframe_secs, leg_id, scale_code, origin_price, \
    origin_occurred_bar_index, origin_available_bar_index, origin_confirmed_code, \
    leg_direction_code, target_price, target_basis_code, target_version, \
    target_stable_code, progress, displacement_price, displacement_atr, \
    lifecycle_code, stage_code, target_reached_code\
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)";
