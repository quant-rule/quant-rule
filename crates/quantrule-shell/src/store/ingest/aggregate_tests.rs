//! Writer tests for the consensus and cross-timeframe aggregate rows.
//!
//! These tests sit beside `tests.rs` because the bar-row coverage there is
//! already long. They use the same in-memory store and the same private writer
//! entry point, and they assert on the stored rows rather than on the values the
//! layers happen to carry.

use super::fixtures::{assert_params_match, count, insert_columns, stored_rows_with_columns};
use super::sql::{CONSENSUS_INSERT, LAYER3_INSERT, consensus_params, layer3_params};
use super::{DatasetRecord, IngestReport, IngestRows};
use crate::store::{Store, StoreError};
use quantrule_analytics::layers::{
    ConsensusRow, ContextMetrics, DirectionMetrics, HigherBlock, Layer1Row, Layer2Row, Layer3Row,
    Metrics, MomentumMetrics, ProgressionMetrics, ReadingsSnapshot, StructureMetrics,
};
use quantrule_data::Candle;
use quantrule_model::aggregate::{
    CrossTimeframeAggregates, DirectionAggregates, EventAggregates, EventSubtypeCounts,
    MomentumAggregates, ProgressionAggregates, ScaleStageAggregates, StructureAggregates,
};
use quantrule_model::state::TimeframeConsensus;

const M1: u64 = 60;
const M5: u64 = 300;

fn record(m1_bars: usize) -> DatasetRecord {
    DatasetRecord {
        name: "demo".to_string(),
        data_version: "source:1".to_string(),
        model_version: "0.1.0".to_string(),
        structure_scale_code: 2,
        m1_bars,
    }
}

/// A Layer 2 row whose optional metrics are all null.
fn row(timeframe_secs: u64, bar_index: usize) -> Layer2Row {
    layer2_at(
        timeframe_secs,
        bar_index,
        1_700_000_000 + bar_index as i64 * 60,
    )
}

/// A Layer 2 row at an explicit open time.
fn layer2_at(timeframe_secs: u64, bar_index: usize, timestamp: i64) -> Layer2Row {
    let candle = Candle::new(
        bar_index as u64,
        timestamp,
        10.0,
        11.0,
        9.0,
        10.5,
        Some(1.0),
    )
    .expect("the test candle is valid");
    Layer2Row::from_layer1(Layer1Row {
        timeframe_secs,
        bar_index,
        candle,
        readings: ReadingsSnapshot {
            ema20: None,
            ema50: None,
            ema200: None,
            rsi14: None,
            atr14: None,
            rvol20: None,
        },
        metrics: Metrics {
            direction: DirectionMetrics {
                direction: None,
                state_code: None,
                size_percentile: None,
                readiness_code: 0,
                unavailable_reason_code: Some(1),
                state_age_bars: 0,
                state_changes_50: 0,
            },
            momentum: MomentumMetrics {
                force: None,
                acceleration: None,
                intensification: None,
                state_code: None,
                size_percentile: None,
                readiness_code: 0,
                unavailable_reason_code: Some(1),
                state_age_bars: 0,
                state_changes_50: 0,
            },
            structure: StructureMetrics {
                score: None,
                state_code: None,
                scale_code: 2,
                high_relation_code: None,
                low_relation_code: None,
                ema_distance_atr_20: None,
                ema_distance_atr_50: None,
                ema_distance_atr_200: None,
                ema_cross_code_20: None,
                ema_cross_code_50: None,
                ema_cross_code_200: None,
                ema_retest_code_20: None,
                ema_retest_code_50: None,
                ema_retest_code_200: None,
                size_percentile: None,
                readiness_code: 0,
                unavailable_reason_code: Some(1),
                state_age_bars: 0,
                state_changes_50: 0,
            },
            progression: ProgressionMetrics {
                progress: Some(0.0),
                stage_code: None,
                leg_direction: None,
                leg_scale_code: None,
                size_percentile: None,
                readiness_code: 0,
                unavailable_reason_code: None,
                state_age_bars: 0,
                state_changes_50: 0,
            },
            context: ContextMetrics {
                atr_ratio: None,
                rvol_ratio: None,
                context_code: None,
            },
        },
    })
}

fn consensus(
    timeframe_secs: u64,
    bar_index: usize,
    counts: (u8, u8, u8),
    weakest: Option<u8>,
) -> ConsensusRow {
    ConsensusRow {
        bar_index,
        consensus: TimeframeConsensus {
            timeframe_secs,
            bullish_axis_count: counts.0,
            bearish_axis_count: counts.1,
            neutral_axis_count: counts.2,
            weakest_directional_axis_code: weakest,
        },
    }
}

fn layer3(base: Layer2Row, aggregates: CrossTimeframeAggregates) -> Layer3Row {
    Layer3Row {
        base,
        higher: Vec::new(),
        aggregates,
    }
}

/// The as-of block a Layer 2 row would contribute.
fn block(row: &Layer2Row, staleness_secs: u64) -> HigherBlock {
    HigherBlock {
        timeframe_secs: row.layer1.timeframe_secs,
        bar_index: row.layer1.bar_index,
        candle: row.layer1.candle,
        metrics: row.layer1.metrics,
        resolution: row.resolution,
        staleness_secs,
    }
}

fn sample_stage(offset: u8) -> ScaleStageAggregates {
    ScaleStageAggregates {
        stage_histogram: [
            u32::from(offset),
            u32::from(offset) + 1,
            u32::from(offset) + 2,
            u32::from(offset) + 3,
            u32::from(offset) + 4,
            u32::from(offset) + 5,
        ],
        stage_mode: Some(offset),
        stage_min: Some(0),
        stage_max: Some(5),
        stage_span: Some(5),
        modal_stage_share: Some(0.4 + f64::from(offset) / 100.0),
    }
}

/// An aggregate record whose 82 values are distinct enough to spot a swap.
fn sample_aggregates(ladder: &[u64]) -> CrossTimeframeAggregates {
    let count = ladder.len();
    CrossTimeframeAggregates {
        ladder: ladder.to_vec(),
        direction: DirectionAggregates {
            timeframes: ladder.to_vec(),
            direction_timeframe_count: count,
            direction_mean: Some(0.11),
            direction_bullish_share: Some(0.12),
            direction_bearish_share: Some(0.13),
            direction_neutral_share: Some(0.14),
            direction_gradient: Some(0.15),
            direction_ready_count: 1,
        },
        momentum: MomentumAggregates {
            timeframes: ladder.to_vec(),
            momentum_timeframe_count: count,
            momentum_force_mean: Some(0.21),
            momentum_acceleration_mean: Some(0.22),
            momentum_positive_share: Some(0.23),
            momentum_negative_share: Some(0.24),
            momentum_force_fast_slow_gap: Some(0.25),
            momentum_acceleration_fast_slow_gap: Some(0.26),
            momentum_gradient: Some(0.27),
            momentum_ready_count: 1,
        },
        structure: StructureAggregates {
            timeframes: ladder.to_vec(),
            structure_timeframe_count: count,
            structure_code_histogram: [1, 2, 3, 4, 5],
            structure_bullish_share: Some(0.31),
            structure_bearish_share: Some(0.32),
            structure_neutral_share: Some(0.33),
            structure_state_span: Some(4),
            structure_gradient: Some(0.34),
            structure_ready_share: Some(0.35),
            structure_conflict_timeframe_count: 6,
            level_cluster_count: 7,
            largest_level_cluster_timeframe_count: 8,
            largest_level_cluster_span_atr: Some(0.39),
        },
        progression: ProgressionAggregates {
            timeframes: ladder.to_vec(),
            progression_timeframe_count: count,
            minor: sample_stage(1),
            major: sample_stage(2),
            active_major_leg_count: 7,
            active_minor_leg_count: 8,
            progression_ready_share: Some(0.43),
            leg_direction_agreement_share: Some(0.44),
            cross_timeframe_leg_match_count: 9,
        },
        events: EventAggregates {
            timeframes: ladder.to_vec(),
            per_timeframe: Vec::new(),
            event_count_by_type: [1, 2, 3],
            event_count_by_subtype: EventSubtypeCounts {
                level: [4, 5, 6],
                divergence: [7, 8, 9, 10],
                signal: [11, 12, 13],
            },
            divergence_count_by_subtype: [7, 8, 9, 10],
            divergence_timeframe_count: 14,
            divergence_direction_agreement_share: Some(0.45),
            level_event_timeframe_count: 15,
            signal_event_timeframe_count: 16,
        },
    }
}

fn ingest(
    store: &mut Store,
    bars: &[Layer2Row],
    consensus: &[ConsensusRow],
    layer3: &[Layer3Row],
    m1_bars: usize,
) -> Result<IngestReport, StoreError> {
    store.ingest(
        &record(m1_bars),
        &IngestRows::new(bars, consensus, layer3, &[], &[], &[], &[]),
    )
}

fn table_layer3_columns() -> Vec<String> {
    let schema = include_str!("../schema.rs");
    let header = "CREATE TABLE layer3 (";
    let start = schema.find(header).expect("the layer3 DDL") + header.len();
    let end = start + schema[start..].find("\n);").expect("the DDL end");
    schema[start..end]
        .lines()
        .filter_map(|line| {
            let name = line.split_whitespace().next()?;
            (!name.starts_with("PRIMARY")).then(|| name.trim_end_matches(',').to_string())
        })
        .collect()
}

#[test]
fn the_layer3_insert_columns_match_the_ddl_order() {
    let columns = insert_columns(LAYER3_INSERT);
    assert_eq!(columns.len(), 285);
    assert_eq!(columns, table_layer3_columns());
    assert_eq!(LAYER3_INSERT.matches('?').count(), 285);

    let consensus = insert_columns(CONSENSUS_INSERT);
    assert_eq!(
        consensus,
        vec![
            "dataset_id".to_string(),
            "timeframe_secs".to_string(),
            "bar_index".to_string(),
            "bullish_axis_count".to_string(),
            "bearish_axis_count".to_string(),
            "neutral_axis_count".to_string(),
            "weakest_directional_axis_code".to_string(),
        ]
    );
}

#[test]
fn layer3_blocks_round_trip() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let base = layer2_at(M1, 0, 1_700_000_300);
    let m5 = layer2_at(M5, 0, 1_700_000_000);
    let m15 = layer2_at(900, 0, 1_699_999_200);
    let h1 = layer2_at(3_600, 0, 1_699_996_400);
    let bars = [base, m5, m15, h1];
    let consensus_rows = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M5, 0, (1, 1, 1), Some(1)),
        consensus(900, 0, (1, 1, 1), Some(1)),
        consensus(3_600, 0, (1, 1, 1), Some(1)),
    ];
    let aggregates = sample_aggregates(&[M1, M5, 900, 3_600]);
    let rows = [Layer3Row {
        base,
        higher: vec![block(&m5, 60), block(&m15, 260), block(&h1, 360)],
        aggregates: aggregates.clone(),
    }];

    let report =
        ingest(&mut store, &bars, &consensus_rows, &rows, 1).expect("the dataset is ingested");

    let (columns, stored) =
        stored_rows_with_columns(&store, "SELECT * FROM layer3 ORDER BY bar_index");
    assert_eq!(stored.len(), 1);
    assert_params_match(
        &columns,
        &insert_columns(LAYER3_INSERT),
        &stored[0],
        &layer3_params(report.dataset_id, &rows[0]),
    );

    let materialized: i64 = store
        .connection()
        .query_row(
            "SELECT layer3_materialized FROM dataset WHERE id = ?1",
            [report.dataset_id],
            |row| row.get(0),
        )
        .expect("the flag is read");
    assert_eq!(materialized, 1);
}

#[test]
fn consensus_and_aggregates_round_trip() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = [row(M1, 0), row(M1, 1)];
    let consensus_rows = [
        consensus(M1, 0, (3, 0, 0), Some(1)),
        consensus(M1, 1, (1, 1, 1), Some(3)),
    ];
    let mut first = sample_aggregates(&[M1]);
    first.direction.direction_mean = Some(0.99);
    let mut second = sample_aggregates(&[M1]);
    second.structure.structure_state_span = None;
    second.events.divergence_direction_agreement_share = None;
    let layer3_rows = [layer3(bars[0], first), layer3(bars[1], second)];

    let report = ingest(&mut store, &bars, &consensus_rows, &layer3_rows, 2)
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 2);

    let (columns, rows) =
        stored_rows_with_columns(&store, "SELECT * FROM consensus ORDER BY bar_index");
    assert_eq!(rows.len(), 2);
    let consensus_columns = insert_columns(CONSENSUS_INSERT);
    for (index, expected) in consensus_rows.iter().enumerate() {
        assert_params_match(
            &columns,
            &consensus_columns,
            &rows[index],
            &consensus_params(report.dataset_id, expected),
        );
    }

    let (columns, rows) =
        stored_rows_with_columns(&store, "SELECT * FROM layer3 ORDER BY bar_index");
    assert_eq!(rows.len(), 2);
    let aggregate_columns = insert_columns(LAYER3_INSERT);
    for (index, expected) in layer3_rows.iter().enumerate() {
        assert_params_match(
            &columns,
            &aggregate_columns,
            &rows[index],
            &layer3_params(report.dataset_id, expected),
        );
    }

    let populated_blocks: i64 = store
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM layer3 WHERE m5_bar_index IS NOT NULL \
             OR m15_bar_index IS NOT NULL OR h1_bar_index IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("the block count is read");
    assert_eq!(populated_blocks, 0, "an absent block column is populated");

    let materialized: i64 = store
        .connection()
        .query_row(
            "SELECT layer3_materialized FROM dataset WHERE id = ?1",
            [report.dataset_id],
            |row| row.get(0),
        )
        .expect("the flag is read");
    assert_eq!(materialized, 1);
}

#[test]
fn lists_are_semicolon_separated_and_never_null() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = [row(M1, 0), row(M5, 0)];
    let consensus_rows = [
        consensus(M1, 0, (1, 1, 1), Some(2)),
        consensus(M5, 0, (1, 1, 1), Some(2)),
    ];
    let mut aggregates = sample_aggregates(&[M1, M5]);
    aggregates.momentum.timeframes.clear();
    aggregates.momentum.momentum_force_mean = None;
    let layer3_rows = [layer3(bars[0], aggregates)];

    ingest(&mut store, &bars, &consensus_rows, &layer3_rows, 1).expect("the dataset is ingested");

    let (ladder, direction, momentum): (Option<String>, Option<String>, Option<String>) = store
        .connection()
        .query_row(
            "SELECT agg_ladder, agg_direction_timeframes, agg_momentum_timeframes FROM layer3",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("the lists are read");
    assert_eq!(ladder.as_deref(), Some("60;300"));
    assert_eq!(direction.as_deref(), Some("60;300"));
    assert_eq!(momentum.as_deref(), Some(""));
}

#[test]
fn null_aggregates_stay_null_and_zero_counts_stay_zero() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = [row(M1, 0)];
    let consensus_rows = [consensus(M1, 0, (0, 0, 0), None)];
    let mut aggregates = sample_aggregates(&[M1]);
    aggregates.direction.direction_mean = None;
    aggregates.structure.structure_state_span = None;
    aggregates.events.event_count_by_type = [0, 0, 0];
    aggregates.events.event_count_by_subtype = EventSubtypeCounts::default();
    aggregates.events.divergence_count_by_subtype = [0, 0, 0, 0];
    let layer3_rows = [layer3(bars[0], aggregates)];

    ingest(&mut store, &bars, &consensus_rows, &layer3_rows, 1).expect("the dataset is ingested");

    let (mean, span, level, divergence): (Option<f64>, Option<i64>, i64, i64) = store
        .connection()
        .query_row(
            "SELECT agg_direction_mean, agg_structure_state_span, agg_event_count_level, \
             agg_event_count_divergence_regular_bullish FROM layer3",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("the values are read");
    assert_eq!(mean, None);
    assert_eq!(span, None);
    assert_eq!(level, 0);
    assert_eq!(divergence, 0);
}

fn assert_rejected(
    bars: &[Layer2Row],
    consensus: &[ConsensusRow],
    layer3: &[Layer3Row],
    m1_bars: usize,
) {
    let mut store = Store::open_in_memory().expect("the store opens");
    let error = ingest(&mut store, bars, consensus, layer3, m1_bars)
        .expect_err("the invalid records are rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
    assert_eq!(count(&store, "bar"), 0);
    assert_eq!(count(&store, "consensus"), 0);
    assert_eq!(count(&store, "layer3"), 0);
}

#[test]
fn invalid_consensus_is_rejected() {
    let bars = [row(M1, 0), row(M1, 1)];
    let valid = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M1, 1, (1, 1, 1), Some(1)),
    ];
    let aggregates = sample_aggregates(&[M1]);
    let layer3_rows = [
        layer3(bars[0], aggregates.clone()),
        layer3(bars[1], aggregates),
    ];

    let mut duplicate = valid.to_vec();
    duplicate[1] = consensus(M1, 0, (1, 1, 1), Some(1));
    assert_rejected(&bars, &duplicate, &layer3_rows, 2);

    assert_rejected(&bars, &valid[..1], &layer3_rows, 2);

    let mut out_of_range = valid.to_vec();
    out_of_range[1] = consensus(M1, 9, (1, 1, 1), Some(1));
    assert_rejected(&bars, &out_of_range, &layer3_rows, 2);

    for counts in [(4, 0, 0), (2, 2, 0), (0, 4, 0), (0, 0, 4)] {
        let mut invalid = valid.to_vec();
        invalid[1] = consensus(M1, 1, counts, Some(1));
        assert_rejected(&bars, &invalid, &layer3_rows, 2);
    }

    for weakest in [Some(0), Some(4)] {
        let mut invalid = valid.to_vec();
        invalid[1] = consensus(M1, 1, (1, 1, 1), weakest);
        assert_rejected(&bars, &invalid, &layer3_rows, 2);
    }
}

#[test]
fn invalid_layer3_is_rejected() {
    let bars = [row(M1, 0), row(M1, 1)];
    let consensus_rows = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M1, 1, (1, 1, 1), Some(1)),
    ];
    let aggregates = sample_aggregates(&[M1]);
    let valid = [
        layer3(bars[0], aggregates.clone()),
        layer3(bars[1], aggregates.clone()),
    ];

    let duplicate = [valid[0].clone(), valid[0].clone()];
    assert_rejected(&bars, &consensus_rows, &duplicate, 2);

    assert_rejected(&bars, &consensus_rows, &valid[..1], 2);

    let out_of_range = [layer3(row(M1, 9), aggregates.clone())];
    assert_rejected(&bars, &consensus_rows, &out_of_range, 2);

    let non_m1 = [layer3(row(M5, 0), aggregates.clone())];
    assert_rejected(&bars, &consensus_rows, &non_m1, 2);

    let mut empty_ladder = aggregates.clone();
    empty_ladder.ladder.clear();
    let empty = [layer3(bars[0], empty_ladder)];
    assert_rejected(&bars, &consensus_rows, &empty, 2);

    let mut non_member = aggregates.clone();
    non_member.ladder = vec![M1, M5];
    non_member.direction.timeframes = vec![M1, M5];
    let non_member_rows = [layer3(bars[0], non_member)];
    assert_rejected(&bars, &consensus_rows, &non_member_rows, 2);

    let multi_bars = [row(M1, 0), row(M5, 0)];
    let multi_consensus = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M5, 0, (1, 1, 1), Some(1)),
    ];
    let mut unsorted = sample_aggregates(&[M1, M5]);
    unsorted.direction.timeframes = vec![M5, M1];
    let unsorted_rows = [layer3(multi_bars[0], unsorted)];
    assert_rejected(&multi_bars, &multi_consensus, &unsorted_rows, 1);

    let mut duplicated = sample_aggregates(&[M1, M5]);
    duplicated.direction.timeframes = vec![M1, M1];
    let duplicated_rows = [layer3(multi_bars[0], duplicated)];
    assert_rejected(&multi_bars, &multi_consensus, &duplicated_rows, 1);

    let mut non_finite = aggregates;
    non_finite.direction.direction_mean = Some(f64::NAN);
    let non_finite_rows = [layer3(bars[0], non_finite)];
    assert_rejected(&bars, &consensus_rows, &non_finite_rows, 2);

    let block_base = layer2_at(M1, 0, 1_700_000_300);
    let block_m5 = layer2_at(M5, 0, 1_700_000_000);
    let block_bars = [block_base, block_m5];
    let block_consensus = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M5, 0, (1, 1, 1), Some(1)),
    ];
    let block_aggregates = sample_aggregates(&[M1, M5]);
    let valid_block = block(&block_m5, 60);
    let valid_block_row = Layer3Row {
        base: block_base,
        higher: vec![valid_block],
        aggregates: block_aggregates.clone(),
    };

    let duplicate_block = Layer3Row {
        higher: vec![valid_block, valid_block],
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[duplicate_block], 1);

    let mut unsupported = valid_block;
    unsupported.timeframe_secs = 600;
    let unsupported_row = Layer3Row {
        higher: vec![unsupported],
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[unsupported_row], 1);

    let mut missing = valid_block;
    missing.bar_index = 9;
    let missing_row = Layer3Row {
        higher: vec![missing],
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[missing_row], 1);

    let mut stale = valid_block;
    stale.staleness_secs = 61;
    let stale_row = Layer3Row {
        higher: vec![stale],
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[stale_row], 1);

    let mut mismatched = valid_block;
    mismatched.candle = Candle::new(0, 1_700_000_000, 10.0, 12.0, 9.0, 10.5, Some(1.0))
        .expect("the mismatched candle is valid");
    let mismatched_row = Layer3Row {
        higher: vec![mismatched],
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[mismatched_row], 1);

    let mut wrong_base = block_base;
    wrong_base.layer1.metrics.direction.direction = Some(0.5);
    let wrong_base_row = Layer3Row {
        base: wrong_base,
        ..valid_block_row.clone()
    };
    assert_rejected(&block_bars, &block_consensus, &[wrong_base_row], 1);

    let future_base = layer2_at(M1, 0, 1_700_000_300);
    let future_m5 = layer2_at(M5, 0, 1_700_000_300);
    let future_bars = [future_base, future_m5];
    let future_consensus = [
        consensus(M1, 0, (1, 1, 1), Some(1)),
        consensus(M5, 0, (1, 1, 1), Some(1)),
    ];
    let future_row = Layer3Row {
        base: future_base,
        higher: vec![block(&future_m5, 0)],
        aggregates: block_aggregates,
    };
    assert_rejected(&future_bars, &future_consensus, &[future_row], 1);
}
