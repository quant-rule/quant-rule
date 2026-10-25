//! SQL statements and parameter bindings for the dataset writer.
//!
//! The column lists are explicit so a schema change is a round-trip test
//! failure rather than a silent default. Values are copied from the analytics
//! rows; this module never computes one.

use quantrule_analytics::layers::{ConsensusRow, Layer2Row, Layer3Row, LevelRow, PivotRow};
use rusqlite::types::Value;

pub(super) fn bar_params(dataset_id: i64, row: &Layer2Row) -> [Value; 74] {
    let resolution = &row.resolution;
    let row = &row.layer1;
    let metrics = &row.metrics;
    let readings = &row.readings;
    [
        Value::from(dataset_id),
        Value::from(row.timeframe_secs as i64),
        Value::from(row.bar_index as i64),
        Value::from(row.candle.timestamp()),
        Value::from(row.candle.open()),
        Value::from(row.candle.high()),
        Value::from(row.candle.low()),
        Value::from(row.candle.close()),
        Value::from(row.candle.volume()),
        Value::from(readings.ema20),
        Value::from(readings.ema50),
        Value::from(readings.ema200),
        Value::from(readings.rsi14),
        Value::from(readings.atr14),
        Value::from(readings.rvol20),
        Value::from(metrics.direction.direction),
        Value::from(metrics.direction.state_code.map(i64::from)),
        Value::from(metrics.direction.size_percentile),
        Value::from(i64::from(metrics.direction.readiness_code)),
        Value::from(metrics.direction.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.direction.state_age_bars as i64),
        Value::from(metrics.direction.state_changes_50 as i64),
        Value::from(metrics.momentum.force),
        Value::from(metrics.momentum.acceleration),
        Value::from(metrics.momentum.intensification),
        Value::from(metrics.momentum.state_code.map(i64::from)),
        Value::from(metrics.momentum.size_percentile),
        Value::from(i64::from(metrics.momentum.readiness_code)),
        Value::from(metrics.momentum.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.momentum.state_age_bars as i64),
        Value::from(metrics.momentum.state_changes_50 as i64),
        Value::from(metrics.structure.score),
        Value::from(metrics.structure.state_code.map(i64::from)),
        Value::from(i64::from(metrics.structure.scale_code)),
        Value::from(metrics.structure.high_relation_code.map(i64::from)),
        Value::from(metrics.structure.low_relation_code.map(i64::from)),
        Value::from(metrics.structure.ema_distance_atr_20),
        Value::from(metrics.structure.ema_distance_atr_50),
        Value::from(metrics.structure.ema_distance_atr_200),
        Value::from(metrics.structure.ema_cross_code_20.map(i64::from)),
        Value::from(metrics.structure.ema_cross_code_50.map(i64::from)),
        Value::from(metrics.structure.ema_cross_code_200.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_20.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_50.map(i64::from)),
        Value::from(metrics.structure.ema_retest_code_200.map(i64::from)),
        Value::from(metrics.structure.size_percentile),
        Value::from(i64::from(metrics.structure.readiness_code)),
        Value::from(metrics.structure.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.structure.state_age_bars as i64),
        Value::from(metrics.structure.state_changes_50 as i64),
        Value::from(metrics.progression.progress),
        Value::from(metrics.progression.stage_code.map(i64::from)),
        Value::from(metrics.progression.leg_direction.map(i64::from)),
        Value::from(metrics.progression.leg_scale_code.map(i64::from)),
        Value::from(metrics.progression.size_percentile),
        Value::from(i64::from(metrics.progression.readiness_code)),
        Value::from(metrics.progression.unavailable_reason_code.map(i64::from)),
        Value::from(metrics.progression.state_age_bars as i64),
        Value::from(metrics.progression.state_changes_50 as i64),
        Value::from(metrics.context.atr_ratio),
        Value::from(metrics.context.rvol_ratio),
        Value::from(metrics.context.context_code.map(i64::from)),
        Value::from(resolution.direction.continuous),
        Value::from(resolution.direction.ordinal.map(i64::from)),
        Value::from(resolution.direction.reason.map(i64::from)),
        Value::from(resolution.momentum.continuous),
        Value::from(resolution.momentum.ordinal.map(i64::from)),
        Value::from(resolution.momentum.reason.map(i64::from)),
        Value::from(resolution.structure.continuous),
        Value::from(resolution.structure.ordinal.map(i64::from)),
        Value::from(resolution.structure.reason.map(i64::from)),
        Value::from(resolution.progression.continuous),
        Value::from(resolution.progression.ordinal.map(i64::from)),
        Value::from(resolution.progression.reason.map(i64::from)),
    ]
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

pub(super) fn layer3_params(dataset_id: i64, row: &Layer3Row) -> [Value; 84] {
    let aggregates = &row.aggregates;
    let direction = &aggregates.direction;
    let momentum = &aggregates.momentum;
    let structure = &aggregates.structure;
    let progression = &aggregates.progression;
    let events = &aggregates.events;
    let minor = &progression.minor;
    let major = &progression.major;
    let histogram = structure.structure_code_histogram;
    [
        Value::from(dataset_id),
        Value::from(row.base.layer1.bar_index as i64),
        Value::from(list_text(&aggregates.ladder)),
        Value::from(list_text(&direction.timeframes)),
        Value::from(direction.direction_timeframe_count as i64),
        Value::from(direction.direction_mean),
        Value::from(direction.direction_bullish_share),
        Value::from(direction.direction_bearish_share),
        Value::from(direction.direction_neutral_share),
        Value::from(direction.direction_gradient),
        Value::from(direction.direction_ready_count as i64),
        Value::from(list_text(&momentum.timeframes)),
        Value::from(momentum.momentum_timeframe_count as i64),
        Value::from(momentum.momentum_force_mean),
        Value::from(momentum.momentum_acceleration_mean),
        Value::from(momentum.momentum_positive_share),
        Value::from(momentum.momentum_negative_share),
        Value::from(momentum.momentum_force_fast_slow_gap),
        Value::from(momentum.momentum_acceleration_fast_slow_gap),
        Value::from(momentum.momentum_gradient),
        Value::from(momentum.momentum_ready_count as i64),
        Value::from(list_text(&structure.timeframes)),
        Value::from(structure.structure_timeframe_count as i64),
        Value::from(i64::from(histogram[0])),
        Value::from(i64::from(histogram[1])),
        Value::from(i64::from(histogram[2])),
        Value::from(i64::from(histogram[3])),
        Value::from(i64::from(histogram[4])),
        Value::from(structure.structure_bullish_share),
        Value::from(structure.structure_bearish_share),
        Value::from(structure.structure_neutral_share),
        Value::from(structure.structure_state_span.map(i64::from)),
        Value::from(structure.structure_gradient),
        Value::from(structure.structure_ready_share),
        Value::from(structure.structure_conflict_timeframe_count as i64),
        Value::from(i64::from(structure.level_cluster_count)),
        Value::from(i64::from(structure.largest_level_cluster_timeframe_count)),
        Value::from(structure.largest_level_cluster_span_atr),
        Value::from(list_text(&progression.timeframes)),
        Value::from(progression.progression_timeframe_count as i64),
        Value::from(i64::from(minor.stage_histogram[0])),
        Value::from(i64::from(minor.stage_histogram[1])),
        Value::from(i64::from(minor.stage_histogram[2])),
        Value::from(i64::from(minor.stage_histogram[3])),
        Value::from(i64::from(minor.stage_histogram[4])),
        Value::from(i64::from(minor.stage_histogram[5])),
        Value::from(minor.stage_mode.map(i64::from)),
        Value::from(minor.stage_min.map(i64::from)),
        Value::from(minor.stage_max.map(i64::from)),
        Value::from(minor.stage_span.map(i64::from)),
        Value::from(minor.modal_stage_share),
        Value::from(i64::from(major.stage_histogram[0])),
        Value::from(i64::from(major.stage_histogram[1])),
        Value::from(i64::from(major.stage_histogram[2])),
        Value::from(i64::from(major.stage_histogram[3])),
        Value::from(i64::from(major.stage_histogram[4])),
        Value::from(i64::from(major.stage_histogram[5])),
        Value::from(major.stage_mode.map(i64::from)),
        Value::from(major.stage_min.map(i64::from)),
        Value::from(major.stage_max.map(i64::from)),
        Value::from(major.stage_span.map(i64::from)),
        Value::from(major.modal_stage_share),
        Value::from(i64::from(progression.active_minor_leg_count)),
        Value::from(i64::from(progression.active_major_leg_count)),
        Value::from(progression.progression_ready_share),
        Value::from(progression.leg_direction_agreement_share),
        Value::from(i64::from(progression.cross_timeframe_leg_match_count)),
        Value::from(i64::from(events.event_count_by_type[0])),
        Value::from(i64::from(events.event_count_by_type[1])),
        Value::from(i64::from(events.event_count_by_type[2])),
        Value::from(i64::from(events.event_count_by_subtype.level[0])),
        Value::from(i64::from(events.event_count_by_subtype.level[1])),
        Value::from(i64::from(events.event_count_by_subtype.level[2])),
        Value::from(i64::from(events.event_count_by_subtype.divergence[0])),
        Value::from(i64::from(events.event_count_by_subtype.divergence[1])),
        Value::from(i64::from(events.event_count_by_subtype.divergence[2])),
        Value::from(i64::from(events.event_count_by_subtype.divergence[3])),
        Value::from(i64::from(events.event_count_by_subtype.signal[0])),
        Value::from(i64::from(events.event_count_by_subtype.signal[1])),
        Value::from(i64::from(events.event_count_by_subtype.signal[2])),
        Value::from(i64::from(events.divergence_timeframe_count)),
        Value::from(events.divergence_direction_agreement_share),
        Value::from(i64::from(events.level_event_timeframe_count)),
        Value::from(i64::from(events.signal_event_timeframe_count)),
    ]
}

pub(super) const LAYER3_AGGREGATE_INSERT: &str = "\
INSERT INTO layer3 (\
    dataset_id, bar_index, agg_ladder, agg_direction_timeframes, \
    agg_direction_timeframe_count, agg_direction_mean, agg_direction_bullish_share, \
    agg_direction_bearish_share, agg_direction_neutral_share, agg_direction_gradient, \
    agg_direction_ready_count, agg_momentum_timeframes, agg_momentum_timeframe_count, \
    agg_momentum_force_mean, agg_momentum_acceleration_mean, \
    agg_momentum_positive_share, agg_momentum_negative_share, \
    agg_momentum_force_fast_slow_gap, agg_momentum_acceleration_fast_slow_gap, \
    agg_momentum_gradient, agg_momentum_ready_count, agg_structure_timeframes, \
    agg_structure_timeframe_count, agg_structure_code_minus2_count, \
    agg_structure_code_minus1_count, agg_structure_code_zero_count, \
    agg_structure_code_plus1_count, agg_structure_code_plus2_count, \
    agg_structure_bullish_share, agg_structure_bearish_share, \
    agg_structure_neutral_share, agg_structure_state_span, agg_structure_gradient, \
    agg_structure_ready_share, agg_structure_conflict_timeframe_count, \
    agg_level_cluster_count, agg_largest_level_cluster_timeframe_count, \
    agg_largest_level_cluster_span_atr, agg_progression_timeframes, \
    agg_progression_timeframe_count, agg_progression_minor_stage0_count, \
    agg_progression_minor_stage1_count, agg_progression_minor_stage2_count, \
    agg_progression_minor_stage3_count, agg_progression_minor_stage4_count, \
    agg_progression_minor_stage5_count, agg_progression_minor_stage_mode, \
    agg_progression_minor_stage_min, agg_progression_minor_stage_max, \
    agg_progression_minor_stage_span, agg_progression_minor_modal_stage_share, \
    agg_progression_major_stage0_count, agg_progression_major_stage1_count, \
    agg_progression_major_stage2_count, agg_progression_major_stage3_count, \
    agg_progression_major_stage4_count, agg_progression_major_stage5_count, \
    agg_progression_major_stage_mode, agg_progression_major_stage_min, \
    agg_progression_major_stage_max, agg_progression_major_stage_span, \
    agg_progression_major_modal_stage_share, agg_progression_active_minor_leg_count, \
    agg_progression_active_major_leg_count, agg_progression_ready_share, \
    agg_progression_leg_direction_agreement_share, \
    agg_progression_cross_timeframe_leg_match_count, agg_event_count_level, \
    agg_event_count_divergence, agg_event_count_signal, \
    agg_event_count_level_test_retest, agg_event_count_level_cross, \
    agg_event_count_level_break, agg_event_count_divergence_regular_bullish, \
    agg_event_count_divergence_regular_bearish, \
    agg_event_count_divergence_hidden_bullish, \
    agg_event_count_divergence_hidden_bearish, \
    agg_event_count_signal_threshold_crossing, \
    agg_event_count_signal_series_crossover, agg_event_count_signal_state_change, \
    agg_divergence_timeframe_count, agg_divergence_direction_agreement_share, \
    agg_level_event_timeframe_count, agg_signal_event_timeframe_count\
) VALUES (\
    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, \
    ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, \
    ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, \
    ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, \
    ?65, ?66, ?67, ?68, ?69, ?70, ?71, ?72, ?73, ?74, ?75, ?76, ?77, ?78, ?79, ?80, \
    ?81, ?82, ?83, ?84\
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
