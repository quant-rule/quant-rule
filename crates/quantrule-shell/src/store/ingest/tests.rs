use super::{BAR_INSERT, DatasetRecord, IngestRows, LEVEL_INSERT, MAX_M1_BARS, PIVOT_INSERT};
use crate::store::{Store, StoreError};
use quantrule_analytics::layers::{
    ContextMetrics, DirectionMetrics, Layer1Row, Layer2Row, LevelRow, Metrics, MomentumMetrics,
    PivotRow, ProgressionMetrics, ReadingsSnapshot,
};
use quantrule_analytics::resolution::AxisResolution;
use quantrule_data::Candle;
use rusqlite::types::Value;

/// The 74 columns the writer binds, in `BAR_INSERT` order.
const COLUMNS: [&str; 74] = [
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

fn record(name: &str, data_version: &str, m1_bars: usize) -> DatasetRecord {
    DatasetRecord {
        name: name.to_string(),
        data_version: data_version.to_string(),
        model_version: "0.1.0".to_string(),
        structure_scale_code: 2,
        m1_bars,
    }
}

/// Derives Layer 2 rows from Layer 1 rows.
fn layer2(bars: impl IntoIterator<Item = Layer1Row>) -> Vec<Layer2Row> {
    bars.into_iter().map(Layer2Row::from_layer1).collect()
}

/// Bundles Layer 2 bars with no record rows.
fn ingest_bars(bars: &[Layer2Row]) -> IngestRows<'_> {
    IngestRows::new(bars, &[], &[])
}

/// Every field carries a distinct value, so a swapped column is visible.
fn unique_row(timeframe_secs: u64, bar_index: usize) -> Layer1Row {
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
fn sparse_row() -> Layer1Row {
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

fn stored_row(store: &Store, dataset_id: i64) -> Vec<Value> {
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

fn count(store: &Store, table: &str) -> i64 {
    store
        .connection()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .expect("the count is read")
}

fn pivot_row(
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

fn level_row(
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

fn stored_rows(store: &Store, sql: &str) -> Vec<Vec<Value>> {
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

#[test]
fn every_written_column_round_trips() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 7)]);
    let report = store
        .ingest(&record("demo", "source:1", 1), &ingest_bars(&bars))
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 1);
    assert_eq!(report.timeframes, 1);

    let expected = vec![
        Value::from(report.dataset_id),
        Value::from(60_i64),
        Value::from(7_i64),
        Value::from(1_700_000_000_i64),
        Value::from(10.0),
        Value::from(11.0),
        Value::from(9.0),
        Value::from(10.5),
        Value::from(Some(123.0)),
        Value::from(Some(20.1)),
        Value::from(Some(50.2)),
        Value::from(Some(200.3)),
        Value::from(Some(55.4)),
        Value::from(Some(1.5)),
        Value::from(Some(0.6)),
        Value::from(Some(0.11)),
        Value::from(Some(1_i64)),
        Value::from(Some(12.0)),
        Value::from(2_i64),
        Value::from(None::<i64>),
        Value::from(3_i64),
        Value::from(4_i64),
        Value::from(Some(-0.21)),
        Value::from(Some(0.22)),
        Value::from(Some(-0.23)),
        Value::from(Some(-1_i64)),
        Value::from(Some(24.0)),
        Value::from(1_i64),
        Value::from(Some(7_i64)),
        Value::from(5_i64),
        Value::from(6_i64),
        Value::from(Some(0.31)),
        Value::from(Some(2_i64)),
        Value::from(2_i64),
        Value::from(Some(1_i64)),
        Value::from(Some(-1_i64)),
        Value::from(Some(0.32)),
        Value::from(Some(0.33)),
        Value::from(Some(0.34)),
        Value::from(Some(1_i64)),
        Value::from(Some(2_i64)),
        Value::from(Some(3_i64)),
        Value::from(Some(4_i64)),
        Value::from(Some(5_i64)),
        Value::from(Some(6_i64)),
        Value::from(Some(36.0)),
        Value::from(2_i64),
        Value::from(None::<i64>),
        Value::from(7_i64),
        Value::from(8_i64),
        Value::from(Some(0.41)),
        Value::from(Some(3_i64)),
        Value::from(Some(-1_i64)),
        Value::from(Some(1_i64)),
        Value::from(Some(42.0)),
        Value::from(0_i64),
        Value::from(Some(9_i64)),
        Value::from(9_i64),
        Value::from(10_i64),
        Value::from(Some(0.51)),
        Value::from(Some(0.52)),
        Value::from(Some(4_i64)),
        Value::from(Some(0.11)),
        Value::from(Some(5_i64)),
        Value::from(None::<i64>),
        Value::from(Some(-0.21)),
        Value::from(Some(3_i64)),
        Value::from(None::<i64>),
        Value::from(Some(0.31)),
        Value::from(Some(6_i64)),
        Value::from(None::<i64>),
        Value::from(Some(0.41)),
        Value::from(Some(7_i64)),
        Value::from(None::<i64>),
    ];
    assert_eq!(expected.len(), COLUMNS.len());

    let stored = stored_row(&store, report.dataset_id);
    assert_eq!(stored.len(), COLUMNS.len());
    for ((name, stored), expected) in COLUMNS.iter().zip(&stored).zip(&expected) {
        assert_eq!(stored, expected, "column {name}");
    }
}

#[test]
fn nulls_stay_null_and_zeros_stay_zero() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let report = store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&layer2([sparse_row()])),
        )
        .expect("the dataset is ingested");

    let stored = stored_row(&store, report.dataset_id);
    let by_name = |name: &str| {
        let index = COLUMNS
            .iter()
            .position(|column| *column == name)
            .expect("the column exists");
        stored[index].clone()
    };

    assert_eq!(by_name("volume"), Value::Real(0.0));
    assert_eq!(by_name("ema20"), Value::Null);
    assert_eq!(by_name("direction"), Value::Null);
    assert_eq!(by_name("direction_readiness_code"), Value::Integer(0));
    assert_eq!(
        by_name("direction_unavailable_reason_code"),
        Value::Integer(1)
    );
    assert_eq!(by_name("progress"), Value::Real(0.0));
    assert_eq!(by_name("structure_code"), Value::Null);
    assert_eq!(by_name("context_code"), Value::Null);
    assert_eq!(by_name("structure_scale_code"), Value::Integer(2));

    assert_eq!(by_name("direction_continuous"), Value::Null);
    assert_eq!(by_name("direction_ordinal"), Value::Null);
    assert_eq!(
        by_name("direction_resolution_reason_code"),
        Value::Integer(1)
    );
    assert_eq!(by_name("momentum_continuous"), Value::Null);
    assert_eq!(by_name("momentum_ordinal"), Value::Null);
    assert_eq!(
        by_name("momentum_resolution_reason_code"),
        Value::Integer(1)
    );
    assert_eq!(by_name("structure_continuous"), Value::Null);
    assert_eq!(by_name("structure_ordinal"), Value::Null);
    assert_eq!(
        by_name("structure_resolution_reason_code"),
        Value::Integer(1)
    );
    assert_eq!(by_name("progression_continuous"), Value::Real(0.0));
    assert_eq!(by_name("progression_ordinal"), Value::Integer(5));
    assert_eq!(by_name("progression_resolution_reason_code"), Value::Null);
}

#[test]
fn resolution_boundaries_are_stored_exactly() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let values = [-1.0, -0.8, 0.0, 0.8, 1.0];
    let bars: Vec<Layer2Row> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let mut row = unique_row(60, index);
            row.metrics.direction.direction = Some(*value);
            row.metrics.direction.unavailable_reason_code = None;
            Layer2Row::from_layer1(row)
        })
        .collect();
    store
        .ingest(
            &record("demo", "source:1", values.len()),
            &ingest_bars(&bars),
        )
        .expect("the dataset is ingested");

    let stored = stored_rows(
        &store,
        "SELECT direction_continuous, direction_ordinal, direction_resolution_reason_code \
         FROM bar ORDER BY bar_index",
    );
    assert_eq!(
        stored,
        vec![
            vec![
                Value::from(Some(-1.0)),
                Value::from(Some(0_i64)),
                Value::Null,
            ],
            vec![
                Value::from(Some(-0.8)),
                Value::from(Some(1_i64)),
                Value::Null,
            ],
            vec![
                Value::from(Some(0.0)),
                Value::from(Some(5_i64)),
                Value::Null,
            ],
            vec![
                Value::from(Some(0.8)),
                Value::from(Some(9_i64)),
                Value::Null,
            ],
            vec![
                Value::from(Some(1.0)),
                Value::from(Some(9_i64)),
                Value::Null,
            ],
        ]
    );
}

#[test]
fn progression_keeps_raw_progress_and_stores_the_clamped_resolution() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars: Vec<Layer2Row> = [(1.5, 1.0, 9), (-0.25, -0.25, 3)]
        .into_iter()
        .enumerate()
        .map(|(index, (raw, _, _))| {
            let mut row = unique_row(60, index);
            row.metrics.progression.progress = Some(raw);
            row.metrics.progression.unavailable_reason_code = None;
            Layer2Row::from_layer1(row)
        })
        .collect();
    store
        .ingest(&record("demo", "source:1", bars.len()), &ingest_bars(&bars))
        .expect("the dataset is ingested");

    let stored = stored_rows(
        &store,
        "SELECT progress, progression_continuous, progression_ordinal, \
             progression_resolution_reason_code FROM bar ORDER BY bar_index",
    );
    assert_eq!(
        stored,
        vec![
            vec![
                Value::from(Some(1.5)),
                Value::from(Some(1.0)),
                Value::from(Some(9_i64)),
                Value::Null,
            ],
            vec![
                Value::from(Some(-0.25)),
                Value::from(Some(-0.25)),
                Value::from(Some(3_i64)),
                Value::Null,
            ],
        ]
    );
}

#[test]
fn invalid_resolutions_are_rejected_before_writing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let cases = [
        AxisResolution {
            continuous: Some(f64::NAN),
            ordinal: Some(0),
            reason: None,
        },
        AxisResolution {
            continuous: Some(f64::INFINITY),
            ordinal: Some(9),
            reason: None,
        },
        AxisResolution {
            continuous: Some(1.5),
            ordinal: Some(9),
            reason: None,
        },
        AxisResolution {
            continuous: Some(0.0),
            ordinal: None,
            reason: None,
        },
        AxisResolution {
            continuous: Some(0.0),
            ordinal: Some(10),
            reason: None,
        },
        AxisResolution {
            continuous: Some(0.0),
            ordinal: Some(4),
            reason: None,
        },
        AxisResolution {
            continuous: Some(0.0),
            ordinal: Some(5),
            reason: Some(1),
        },
        AxisResolution {
            continuous: None,
            ordinal: Some(5),
            reason: Some(1),
        },
        AxisResolution {
            continuous: None,
            ordinal: None,
            reason: None,
        },
        AxisResolution {
            continuous: None,
            ordinal: None,
            reason: Some(7),
        },
    ];

    for (index, resolution) in cases.into_iter().enumerate() {
        let mut row = Layer2Row::from_layer1(unique_row(60, 0));
        row.resolution.direction = resolution;
        let bars = [row];
        let error = store
            .ingest(&record("demo", "source:1", 1), &ingest_bars(&bars))
            .expect_err("the invalid resolution is rejected");
        assert!(
            matches!(error, StoreError::InvalidDataset { .. }),
            "case {index}"
        );
        assert_eq!(count(&store, "dataset"), 0, "case {index}");
        assert_eq!(count(&store, "bar"), 0, "case {index}");
    }

    let mut row = Layer2Row::from_layer1(unique_row(60, 0));
    row.resolution.progression = AxisResolution {
        continuous: None,
        ordinal: None,
        reason: None,
    };
    let bars = [row];
    let error = store
        .ingest(&record("demo", "source:1", 1), &ingest_bars(&bars))
        .expect_err("the invalid progression resolution is rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
    assert_eq!(count(&store, "bar"), 0);
}

#[test]
fn every_written_pivot_and_level_round_trips() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2((0..8).map(|bar| unique_row(60, bar)));
    let pivots = [
        pivot_row(60, 1, 1, 11.25, 2, 2, 0),
        pivot_row(60, 2, 2, 9.75, 2, 7, 1),
    ];
    let levels = [
        level_row(60, 1001, 1, 1, 11.25, 2, 4),
        level_row(60, 1002, 2, 2, 9.75, 2, 7),
    ];
    let report = store
        .ingest(
            &record("demo", "source:1", 8),
            &IngestRows::new(&bars, &pivots, &levels),
        )
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 8);
    assert_eq!(report.timeframes, 1);

    let stored_pivots = stored_rows(
        &store,
        "SELECT dataset_id, timeframe_secs, scale_code, kind_code, price, \
             occurred_bar_index, available_bar_index, confirmed_code \
             FROM pivot ORDER BY scale_code, kind_code",
    );
    assert_eq!(
        stored_pivots,
        vec![
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(11.25),
                Value::from(2_i64),
                Value::from(2_i64),
                Value::from(0_i64),
            ],
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(2_i64),
                Value::from(2_i64),
                Value::from(9.75),
                Value::from(2_i64),
                Value::from(7_i64),
                Value::from(1_i64),
            ],
        ]
    );

    let stored_levels = stored_rows(
        &store,
        "SELECT dataset_id, timeframe_secs, reference_code, scale_code, kind_code, price, \
             occurred_bar_index, available_bar_index \
             FROM level ORDER BY reference_code",
    );
    assert_eq!(
        stored_levels,
        vec![
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(1001_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(11.25),
                Value::from(2_i64),
                Value::from(4_i64),
            ],
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(1002_i64),
                Value::from(2_i64),
                Value::from(2_i64),
                Value::from(9.75),
                Value::from(2_i64),
                Value::from(7_i64),
            ],
        ]
    );

    let candidate: (i64, i64) = store
        .connection()
        .query_row(
            "SELECT occurred_bar_index, available_bar_index FROM pivot \
                 WHERE confirmed_code = 0",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the candidate is read");
    assert_eq!(candidate, (2, 2));
    let confirmed: (i64, i64) = store
        .connection()
        .query_row(
            "SELECT occurred_bar_index, available_bar_index FROM pivot \
                 WHERE confirmed_code = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the confirmed pivot is read");
    assert_eq!(confirmed, (2, 7));
}

#[test]
fn invalid_pivot_and_level_records_are_rejected_before_writing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 0), unique_row(60, 1), unique_row(60, 2)]);
    let cases: Vec<(Vec<PivotRow>, Vec<LevelRow>)> = vec![
        (vec![pivot_row(60, 1, 1, 10.0, 2, 1, 1)], Vec::new()),
        (vec![pivot_row(60, 1, 1, 10.0, 1, 2, 0)], Vec::new()),
        (vec![pivot_row(60, 3, 1, 10.0, 1, 1, 0)], Vec::new()),
        (vec![pivot_row(60, 1, 0, 10.0, 1, 1, 0)], Vec::new()),
        (vec![pivot_row(60, 1, 1, f64::NAN, 1, 1, 0)], Vec::new()),
        (vec![pivot_row(60, 1, 1, 10.0, 99, 99, 0)], Vec::new()),
        (vec![pivot_row(300, 1, 1, 10.0, 1, 1, 0)], Vec::new()),
        (
            vec![
                pivot_row(60, 1, 1, 10.0, 1, 1, 0),
                pivot_row(60, 1, 1, 10.0, 1, 1, 0),
            ],
            Vec::new(),
        ),
        (Vec::new(), vec![level_row(60, 1001, 1, 1, 10.0, 2, 1)]),
        (Vec::new(), vec![level_row(60, 1000, 1, 1, 10.0, 1, 1)]),
        (Vec::new(), vec![level_row(60, 1001, 3, 1, 10.0, 1, 1)]),
        (Vec::new(), vec![level_row(60, 1001, 1, 0, 10.0, 1, 1)]),
        (
            Vec::new(),
            vec![level_row(60, 1001, 1, 1, f64::INFINITY, 1, 1)],
        ),
        (Vec::new(), vec![level_row(60, 1001, 1, 1, 10.0, 99, 99)]),
        (Vec::new(), vec![level_row(300, 1001, 1, 1, 10.0, 1, 1)]),
        (
            Vec::new(),
            vec![
                level_row(60, 1001, 1, 1, 10.0, 1, 1),
                level_row(60, 1001, 1, 1, 10.0, 1, 1),
            ],
        ),
    ];

    for (index, (pivots, levels)) in cases.iter().enumerate() {
        let error = store
            .ingest(
                &record("demo", "source:1", 3),
                &IngestRows::new(&bars, pivots, levels),
            )
            .expect_err("the invalid record is rejected");
        assert!(
            matches!(error, StoreError::InvalidDataset { .. }),
            "case {index}"
        );
        assert_eq!(count(&store, "dataset"), 0, "case {index}");
        assert_eq!(count(&store, "pivot"), 0, "case {index}");
        assert_eq!(count(&store, "level"), 0, "case {index}");
    }
}

#[test]
fn the_dataset_row_records_the_identity_and_contracts() {
    let mut store = Store::open_in_memory().expect("the store opens");
    store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&layer2([unique_row(60, 0)])),
        )
        .expect("the dataset is ingested");

    let (data_version, model_contract, research_contract, scale, m1_bars, materialized) = store
        .connection()
        .query_row(
            "SELECT data_version, model_contract, research_contract, structure_scale_code, \
                 m1_bars, layer3_materialized FROM dataset WHERE name = 'demo'",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .expect("the dataset row is read");
    assert_eq!(data_version, "source:1");
    assert_eq!(model_contract, "2026-10-21");
    assert_eq!(research_contract, "2026-10-24");
    assert_eq!(scale, 2);
    assert_eq!(m1_bars, 1);
    assert_eq!(materialized, 0);
}

#[test]
fn the_same_identity_is_a_no_op() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let dataset = record("demo", "source:1", 1);
    let rows = layer2([unique_row(60, 0)]);
    let pivots = [pivot_row(60, 1, 1, 10.0, 0, 0, 0)];
    let levels = [level_row(60, 1001, 1, 1, 10.0, 0, 0)];
    let records = IngestRows::new(&rows, &pivots, &levels);
    let first = store.ingest(&dataset, &records).expect("the first ingest");
    assert!(!first.unchanged);

    let second = store.ingest(&dataset, &records).expect("the second ingest");
    assert!(second.unchanged);
    assert_eq!(second.dataset_id, first.dataset_id);
    assert_eq!(second.bars, first.bars);
    assert_eq!(second.timeframes, first.timeframes);
    assert_eq!(count(&store, "dataset"), 1);
    assert_eq!(count(&store, "bar"), 1);
    assert_eq!(count(&store, "pivot"), 1);
    assert_eq!(count(&store, "level"), 1);
}

#[test]
fn a_different_identity_is_a_conflict_and_changes_nothing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 0)]);
    store
        .ingest(&record("demo", "source:1", 1), &ingest_bars(&bars))
        .expect("the first ingest");

    let error = store
        .ingest(&record("demo", "source:2", 1), &ingest_bars(&bars))
        .expect_err("a different identity is rejected");
    assert!(matches!(error, StoreError::Conflict { name } if name == "demo"));
    assert_eq!(count(&store, "dataset"), 1);
    assert_eq!(count(&store, "bar"), 1);

    let data_version: String = store
        .connection()
        .query_row(
            "SELECT data_version FROM dataset WHERE name = 'demo'",
            [],
            |row| row.get(0),
        )
        .expect("the identity is read");
    assert_eq!(data_version, "source:1");
}

#[test]
fn a_failed_insert_rolls_back_the_dataset_and_every_record() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let first = unique_row(60, 0);
    let mut duplicate = unique_row(60, 0);
    duplicate.candle = candle(1_700_000_060, Some(1.0));
    let bars = layer2([first, duplicate]);
    let pivots = [pivot_row(60, 1, 1, 11.0, 0, 0, 0)];
    let levels = [level_row(60, 1001, 1, 1, 11.0, 0, 0)];

    let error = store
        .ingest(
            &record("demo", "source:1", 2),
            &IngestRows::new(&bars, &pivots, &levels),
        )
        .expect_err("the duplicate primary key fails the transaction");
    assert!(matches!(error, StoreError::Write { .. }));
    assert_eq!(count(&store, "dataset"), 0);
    assert_eq!(count(&store, "bar"), 0);
    assert_eq!(count(&store, "pivot"), 0);
    assert_eq!(count(&store, "level"), 0);
}

#[test]
fn the_m1_cap_is_enforced_before_writing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let error = store
        .ingest(
            &record("demo", "source:1", MAX_M1_BARS + 1),
            &ingest_bars(&[]),
        )
        .expect_err("the cap is enforced");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
}

#[test]
fn the_m1_count_must_match_the_rows() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let error = store
        .ingest(&record("demo", "source:1", 1), &ingest_bars(&[]))
        .expect_err("a missing M1 row is rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));

    let error = store
        .ingest(
            &record("demo", "source:1", 0),
            &ingest_bars(&layer2([unique_row(60, 0)])),
        )
        .expect_err("an undeclared M1 row is rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
}

#[test]
fn an_empty_dataset_is_accepted() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let report = store
        .ingest(&record("empty", "source:0", 0), &ingest_bars(&[]))
        .expect("an empty dataset is accepted");
    assert!(!report.unchanged);
    assert_eq!(report.bars, 0);
    assert_eq!(report.timeframes, 0);
    assert_eq!(count(&store, "dataset"), 1);
    assert_eq!(count(&store, "bar"), 0);
}

#[test]
fn the_row_order_does_not_change_the_stored_set() {
    let mut first = Store::open_in_memory().expect("the first store opens");
    let mut second = Store::open_in_memory().expect("the second store opens");
    let rows = layer2([unique_row(60, 0), unique_row(300, 0), unique_row(60, 1)]);
    first
        .ingest(&record("demo", "source:1", 2), &ingest_bars(&rows))
        .expect("the first ingest");
    second
        .ingest(
            &record("demo", "source:1", 2),
            &ingest_bars(&[rows[2], rows[0], rows[1]]),
        )
        .expect("the reordered ingest");

    let read = |store: &Store| -> Vec<(i64, i64)> {
        let mut statement = store
            .connection()
            .prepare("SELECT timeframe_secs, bar_index FROM bar ORDER BY timeframe_secs, bar_index")
            .expect("the query prepares");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("the rows are read")
            .collect::<Result<_, _>>()
            .expect("the rows are valid")
    };
    assert_eq!(read(&first), read(&second));
}

#[test]
fn the_report_counts_distinct_timeframes() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let rows = layer2([unique_row(60, 0), unique_row(60, 1), unique_row(300, 0)]);
    let report = store
        .ingest(&record("demo", "source:1", 2), &ingest_bars(&rows))
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 3);
    assert_eq!(report.timeframes, 2);
}

#[test]
fn the_insert_binds_74_parameters() {
    assert_eq!(BAR_INSERT.matches('?').count(), 74);
}

#[test]
fn the_record_inserts_bind_eight_parameters() {
    assert_eq!(PIVOT_INSERT.matches('?').count(), 8);
    assert_eq!(LEVEL_INSERT.matches('?').count(), 8);
}

#[test]
fn an_empty_name_or_data_version_is_rejected() {
    let mut store = Store::open_in_memory().expect("the store opens");
    for dataset in [record("", "source:1", 0), record("demo", "", 0)] {
        let error = store
            .ingest(&dataset, &ingest_bars(&[]))
            .expect_err("an empty identity is rejected");
        assert!(matches!(error, StoreError::InvalidDataset { .. }));
    }
    assert_eq!(count(&store, "dataset"), 0);
}
