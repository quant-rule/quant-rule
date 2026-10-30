//! Shared fixtures for the dataset-writer tests.
//!
//! The fixtures build shape-valid Layer 2 rows together with the matching
//! consensus and Layer 3 rows, so each test can focus on the behavior it
//! covers. They are test data only; the writer never reads this module.

use super::{DatasetRecord, IngestRows};
use crate::store::Store;
use quantrule_analytics::layers::{
    ConsensusRow, ContextMetrics, DirectionMetrics, EventRow, Layer1Row, Layer2Row, Layer3Row,
    LegRow, LevelRow, M1_SECONDS, Metrics, MomentumMetrics, PivotRow, ProgressionMetrics,
    ReadingsSnapshot,
};
use quantrule_data::Candle;
use quantrule_model::aggregate::{
    CrossTimeframeAggregates, DirectionAggregates, EventAggregates, EventSubtypeCounts,
    MomentumAggregates, ProgressionAggregates, ScaleStageAggregates, StructureAggregates,
};
use quantrule_model::consensus::TimeframeConsensus;
use rusqlite::types::Value;

/// The 74 columns the writer binds, in `BAR_INSERT` order.
pub(super) const COLUMNS: [&str; 74] = [
    "dataset_id",
    "timeframe_secs",
    "bar_index",
    "open_time",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "ema20",
    "ema50",
    "ema200",
    "rsi14",
    "atr14",
    "rvol20",
    "direction",
    "direction_state_code",
    "direction_size_percentile",
    "direction_readiness_code",
    "direction_unavailable_reason_code",
    "direction_state_age_bars",
    "direction_state_changes_50",
    "momentum_force",
    "momentum_acceleration",
    "momentum_intensification",
    "momentum_state_code",
    "momentum_size_percentile",
    "momentum_readiness_code",
    "momentum_unavailable_reason_code",
    "momentum_state_age_bars",
    "momentum_state_changes_50",
    "structure_score",
    "structure_code",
    "structure_scale_code",
    "high_relation_code",
    "low_relation_code",
    "ema_distance_atr_20",
    "ema_distance_atr_50",
    "ema_distance_atr_200",
    "ema_cross_code_20",
    "ema_cross_code_50",
    "ema_cross_code_200",
    "ema_retest_code_20",
    "ema_retest_code_50",
    "ema_retest_code_200",
    "structure_size_percentile",
    "structure_readiness_code",
    "structure_unavailable_reason_code",
    "structure_state_age_bars",
    "structure_state_changes_50",
    "progress",
    "progression_stage_code",
    "progression_leg_direction",
    "progression_leg_scale_code",
    "progression_size_percentile",
    "progression_readiness_code",
    "progression_unavailable_reason_code",
    "progression_state_age_bars",
    "progression_state_changes_50",
    "atr_ratio",
    "rvol_ratio",
    "context_code",
    "direction_continuous",
    "direction_ordinal",
    "direction_resolution_reason_code",
    "momentum_continuous",
    "momentum_ordinal",
    "momentum_resolution_reason_code",
    "structure_continuous",
    "structure_ordinal",
    "structure_resolution_reason_code",
    "progression_continuous",
    "progression_ordinal",
    "progression_resolution_reason_code",
];

fn candle(timestamp: i64, volume: Option<f64>) -> Candle {
    Candle::new(0, timestamp, 10.0, 11.0, 9.0, 10.5, volume).expect("the test candle is valid")
}

pub(super) fn record(name: &str, data_version: &str, m1_bars: usize) -> DatasetRecord {
    DatasetRecord {
        name: name.to_string(),
        data_version: data_version.to_string(),
        model_version: "0.1.0".to_string(),
        structure_scale_code: 2,
        m1_bars,
    }
}

/// Derives Layer 2 rows from Layer 1 rows.
pub(super) fn layer2(bars: impl IntoIterator<Item = Layer1Row>) -> Vec<Layer2Row> {
    bars.into_iter().map(Layer2Row::from_layer1).collect()
}

/// The consensus and Layer 3 rows that accompany a set of Layer 2 bars.
pub(super) struct FixtureRows {
    consensus: Vec<ConsensusRow>,
    layer3: Vec<Layer3Row>,
}

impl FixtureRows {
    /// Builds shape-valid rows for every bar and every M1 bar.
    fn for_bars(bars: &[Layer2Row]) -> Self {
        let consensus = bars
            .iter()
            .map(|row| ConsensusRow {
                bar_index: row.layer1.bar_index,
                consensus: TimeframeConsensus {
                    timeframe_secs: row.layer1.timeframe_secs,
                    bullish_axis_count: 1,
                    bearish_axis_count: 1,
                    neutral_axis_count: 1,
                    weakest_directional_axis_code: Some(2),
                },
            })
            .collect();
        let ladder: Vec<u64> = bars
            .iter()
            .map(|row| row.layer1.timeframe_secs)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        let layer3 = bars
            .iter()
            .filter(|row| row.layer1.timeframe_secs == M1_SECONDS)
            .map(|row| Layer3Row {
                base: *row,
                higher: Vec::new(),
                aggregates: empty_aggregates(&ladder),
            })
            .collect();
        Self { consensus, layer3 }
    }

    pub(super) fn rows<'a>(
        &'a self,
        bars: &'a [Layer2Row],
        pivots: &'a [PivotRow],
        levels: &'a [LevelRow],
    ) -> IngestRows<'a> {
        self.rows_with_records(bars, pivots, levels, &[], &[])
    }

    pub(super) fn rows_with_records<'a>(
        &'a self,
        bars: &'a [Layer2Row],
        pivots: &'a [PivotRow],
        levels: &'a [LevelRow],
        events: &'a [EventRow],
        legs: &'a [LegRow],
    ) -> IngestRows<'a> {
        IngestRows::new(
            bars,
            &self.consensus,
            &self.layer3,
            pivots,
            levels,
            events,
            legs,
        )
    }
}

/// Bundles Layer 2 bars with matching consensus and Layer 3 rows.
pub(super) fn ingest_bars(bars: &[Layer2Row]) -> FixtureRows {
    FixtureRows::for_bars(bars)
}

/// An aggregate record with a valid ladder and no measurements.
fn empty_aggregates(ladder: &[u64]) -> CrossTimeframeAggregates {
    let count = ladder.len();
    let stage = ScaleStageAggregates {
        stage_histogram: [0; 6],
        stage_mode: None,
        stage_min: None,
        stage_max: None,
        stage_span: None,
        modal_stage_share: None,
    };
    CrossTimeframeAggregates {
        ladder: ladder.to_vec(),
        direction: DirectionAggregates {
            timeframes: Vec::new(),
            direction_timeframe_count: count,
            direction_mean: None,
            direction_bullish_share: None,
            direction_bearish_share: None,
            direction_neutral_share: None,
            direction_gradient: None,
            direction_ready_count: 0,
        },
        momentum: MomentumAggregates {
            timeframes: Vec::new(),
            momentum_timeframe_count: count,
            momentum_force_mean: None,
            momentum_acceleration_mean: None,
            momentum_positive_share: None,
            momentum_negative_share: None,
            momentum_force_fast_slow_gap: None,
            momentum_acceleration_fast_slow_gap: None,
            momentum_gradient: None,
            momentum_ready_count: 0,
        },
        structure: StructureAggregates {
            timeframes: Vec::new(),
            structure_timeframe_count: count,
            structure_code_histogram: [0; 5],
            structure_bullish_share: None,
            structure_bearish_share: None,
            structure_neutral_share: None,
            structure_state_span: None,
            structure_gradient: None,
            structure_ready_share: None,
            structure_conflict_timeframe_count: 0,
            level_cluster_count: 0,
            largest_level_cluster_timeframe_count: 0,
            largest_level_cluster_span_atr: None,
        },
        progression: ProgressionAggregates {
            timeframes: Vec::new(),
            progression_timeframe_count: count,
            minor: stage,
            major: stage,
            active_major_leg_count: 0,
            active_minor_leg_count: 0,
            progression_ready_share: None,
            leg_direction_agreement_share: None,
            cross_timeframe_leg_match_count: 0,
        },
        events: EventAggregates {
            timeframes: ladder.to_vec(),
            per_timeframe: Vec::new(),
            event_count_by_type: [0; 3],
            event_count_by_subtype: EventSubtypeCounts::default(),
            divergence_count_by_subtype: [0; 4],
            divergence_timeframe_count: 0,
            divergence_direction_agreement_share: None,
            level_event_timeframe_count: 0,
            signal_event_timeframe_count: 0,
        },
    }
}

/// Every field carries a distinct value, so a swapped column is visible.
pub(super) fn unique_row(timeframe_secs: u64, bar_index: usize) -> Layer1Row {
    Layer1Row {
        timeframe_secs,
        bar_index,
        candle: candle(1_700_000_000, Some(123.0)),
        readings: ReadingsSnapshot {
            ema20: Some(20.1),
            ema50: Some(50.2),
            ema200: Some(200.3),
            rsi14: Some(55.4),
            atr14: Some(1.5),
            rvol20: Some(0.6),
        },
        metrics: Metrics {
            direction: DirectionMetrics {
                direction: Some(0.11),
                state_code: Some(1),
                size_percentile: Some(12.0),
                readiness_code: 2,
                unavailable_reason_code: None,
                state_age_bars: 3,
                state_changes_50: 4,
            },
            momentum: MomentumMetrics {
                force: Some(-0.21),
                acceleration: Some(0.22),
                intensification: Some(-0.23),
                state_code: Some(-1),
                size_percentile: Some(24.0),
                readiness_code: 1,
                unavailable_reason_code: Some(7),
                state_age_bars: 5,
                state_changes_50: 6,
            },
            structure: StructureFields::values(),
            progression: ProgressionMetrics {
                progress: Some(0.41),
                stage_code: Some(3),
                leg_direction: Some(-1),
                leg_scale_code: Some(1),
                size_percentile: Some(42.0),
                readiness_code: 0,
                unavailable_reason_code: Some(9),
                state_age_bars: 9,
                state_changes_50: 10,
            },
            context: ContextMetrics {
                atr_ratio: Some(0.51),
                rvol_ratio: Some(0.52),
                context_code: Some(4),
            },
        },
    }
}

/// A row whose optional values are all null and whose zeros are valid.
pub(super) fn sparse_row() -> Layer1Row {
    Layer1Row {
        timeframe_secs: 60,
        bar_index: 0,
        candle: candle(1_700_000_000, Some(0.0)),
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
            structure: StructureFields::sparse(),
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
    }
}

struct StructureFields;

impl StructureFields {
    fn values() -> quantrule_analytics::layers::StructureMetrics {
        use quantrule_analytics::layers::StructureMetrics;
        StructureMetrics {
            score: Some(0.31),
            state_code: Some(2),
            scale_code: 2,
            high_relation_code: Some(1),
            low_relation_code: Some(-1),
            ema_distance_atr_20: Some(0.32),
            ema_distance_atr_50: Some(0.33),
            ema_distance_atr_200: Some(0.34),
            ema_cross_code_20: Some(1),
            ema_cross_code_50: Some(2),
            ema_cross_code_200: Some(3),
            ema_retest_code_20: Some(4),
            ema_retest_code_50: Some(5),
            ema_retest_code_200: Some(6),
            size_percentile: Some(36.0),
            readiness_code: 2,
            unavailable_reason_code: None,
            state_age_bars: 7,
            state_changes_50: 8,
        }
    }

    fn sparse() -> quantrule_analytics::layers::StructureMetrics {
        use quantrule_analytics::layers::StructureMetrics;
        StructureMetrics {
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
        }
    }
}

pub(super) fn stored_row(store: &Store, dataset_id: i64) -> Vec<Value> {
    let columns = COLUMNS.join(", ");
    let sql = format!(
        "SELECT {columns} FROM bar WHERE dataset_id = ?1 ORDER BY timeframe_secs, bar_index"
    );
    store
        .connection()
        .query_row(&sql, [dataset_id], |row| {
            (0..COLUMNS.len())
                .map(|index| row.get(index))
                .collect::<Result<Vec<Value>, _>>()
        })
        .expect("the stored row is read")
}

pub(super) fn count(store: &Store, table: &str) -> i64 {
    store
        .connection()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("the count is read")
}

pub(super) fn pivot_row(
    timeframe_secs: u64,
    scale_code: u8,
    kind_code: u8,
    price: f64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    confirmed_code: u8,
) -> PivotRow {
    PivotRow {
        timeframe_secs,
        scale_code,
        kind_code,
        price,
        occurred_bar_index,
        available_bar_index,
        confirmed_code,
    }
}

pub(super) fn level_row(
    timeframe_secs: u64,
    reference_code: u32,
    scale_code: u8,
    kind_code: u8,
    price: f64,
    occurred_bar_index: usize,
    available_bar_index: usize,
) -> LevelRow {
    LevelRow {
        timeframe_secs,
        reference_code,
        scale_code,
        kind_code,
        price,
        occurred_bar_index,
        available_bar_index,
    }
}

pub(super) fn event_row(
    timeframe_secs: u64,
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
) -> EventRow {
    EventRow {
        timeframe_secs,
        event_type_code,
        event_subtype_code,
        source_code,
        occurred_bar_index,
        available_bar_index,
        direction_code,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }
}

pub(super) fn leg_row(
    timeframe_secs: u64,
    leg_id: u64,
    scale_code: u8,
    origin_price: f64,
    origin_occurred_bar_index: usize,
    origin_available_bar_index: Option<usize>,
    origin_confirmed_code: u8,
) -> LegRow {
    LegRow {
        timeframe_secs,
        leg_id,
        scale_code,
        origin_price,
        origin_occurred_bar_index,
        origin_available_bar_index,
        origin_confirmed_code,
        leg_direction_code: 1,
        target_price: None,
        target_basis_code: None,
        target_version: 0,
        target_stable_code: 0,
        progress: None,
        displacement_price: None,
        displacement_atr: None,
        lifecycle_code: 0,
        stage_code: None,
        target_reached_code: None,
    }
}

pub(super) fn stored_rows(store: &Store, sql: &str) -> Vec<Vec<Value>> {
    let mut statement = store.connection().prepare(sql).expect("the query prepares");
    let width = statement.column_count();
    statement
        .query_map([], |row| {
            (0..width)
                .map(|index| row.get(index))
                .collect::<Result<Vec<Value>, _>>()
        })
        .expect("the rows are read")
        .collect::<Result<_, _>>()
        .expect("the rows are valid")
}

/// Reads a query's column names and rows, preserving the SQL column order.
pub(super) fn stored_rows_with_columns(store: &Store, sql: &str) -> (Vec<String>, Vec<Vec<Value>>) {
    let mut statement = store.connection().prepare(sql).expect("the query prepares");
    let columns = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let width = statement.column_count();
    let rows = statement
        .query_map([], |row| {
            (0..width)
                .map(|index| row.get(index))
                .collect::<Result<Vec<Value>, _>>()
        })
        .expect("the rows are read")
        .collect::<Result<_, _>>()
        .expect("the rows are valid");
    (columns, rows)
}

/// Asserts that each bound parameter matches the named column in a stored row.
pub(super) fn assert_params_match(
    table_columns: &[String],
    insert_columns: &[String],
    row: &[Value],
    params: &[Value],
) {
    assert_eq!(insert_columns.len(), params.len());
    for (column, expected) in insert_columns.iter().zip(params) {
        let index = table_columns
            .iter()
            .position(|name| name == column)
            .expect("the column exists");
        assert_eq!(&row[index], expected, "column {column}");
    }
}

/// Parses the column list of an `INSERT INTO ... (...) VALUES` statement.
pub(super) fn insert_columns(sql: &str) -> Vec<String> {
    let start = sql.find('(').expect("an open parenthesis") + 1;
    let end = sql.find(") VALUES").expect("a values clause");
    sql[start..end]
        .split(',')
        .map(|column| column.trim().to_string())
        .filter(|column| !column.is_empty())
        .collect()
}
