//! Tests for the wide `bar` and record writers.

use super::MAX_M1_BARS;
use super::fixtures::{
    COLUMNS, count, event_row, ingest_bars, layer2, leg_row, level_row, pivot_row, record,
    sparse_row, stored_row, stored_rows, unique_row,
};
use super::sql::{BAR_INSERT, LEVEL_INSERT, PIVOT_INSERT};
use crate::store::{Store, StoreError};
use quantrule_analytics::layers::{Layer2Row, LevelRow, PivotRow};
use quantrule_analytics::resolution::AxisResolution;
use rusqlite::types::Value;

#[test]
fn every_written_column_round_trips() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 0)]);
    let report = store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 1);
    assert_eq!(report.timeframes, 1);

    let expected = vec![
        Value::from(report.dataset_id),
        Value::from(60_i64),
        Value::from(0_i64),
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
    let bars = layer2([sparse_row()]);
    let report = store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
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
            &ingest_bars(&bars).rows(&bars, &[], &[]),
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
        .ingest(
            &record("demo", "source:1", bars.len()),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
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
            .ingest(
                &record("demo", "source:1", 1),
                &ingest_bars(&bars).rows(&bars, &[], &[]),
            )
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
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
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
            &ingest_bars(&bars).rows(&bars, &pivots, &levels),
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
                &ingest_bars(&bars).rows(&bars, pivots, levels),
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
    let bars = layer2([unique_row(60, 0)]);
    store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
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
    assert_eq!(materialized, 1);
}

#[test]
fn the_same_identity_is_a_no_op() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let dataset = record("demo", "source:1", 1);
    let rows = layer2([unique_row(60, 0)]);
    let pivots = [pivot_row(60, 1, 1, 10.0, 0, 0, 0)];
    let levels = [level_row(60, 1001, 1, 1, 10.0, 0, 0)];
    let events = [event_row(60, 1, 1, 1, 0, 0, 1)];
    let legs = [leg_row(60, 1, 1, 10.0, 0, Some(0), 1)];
    let fixture = ingest_bars(&rows);
    let records = fixture.rows_with_records(&rows, &pivots, &levels, &events, &legs);
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
    assert_eq!(count(&store, "event"), 1);
    assert_eq!(count(&store, "leg"), 1);
}

#[test]
fn a_different_identity_is_a_conflict_and_changes_nothing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 0)]);
    store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
        .expect("the first ingest");

    let error = store
        .ingest(
            &record("demo", "source:2", 1),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
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
    store
        .connection()
        .execute_batch(
            "CREATE TRIGGER fail_level BEFORE INSERT ON level \
             BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
        )
        .expect("the trigger is created");
    let bars = layer2([unique_row(60, 0)]);
    let pivots = [pivot_row(60, 1, 1, 11.0, 0, 0, 0)];
    let levels = [level_row(60, 1001, 1, 1, 11.0, 0, 0)];

    let error = store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&bars).rows(&bars, &pivots, &levels),
        )
        .expect_err("the forced failure rolls back the transaction");
    assert!(matches!(error, StoreError::Write { .. }));
    assert_eq!(count(&store, "dataset"), 0);
    assert_eq!(count(&store, "bar"), 0);
    assert_eq!(count(&store, "consensus"), 0);
    assert_eq!(count(&store, "layer3"), 0);
    assert_eq!(count(&store, "pivot"), 0);
    assert_eq!(count(&store, "level"), 0);
}

#[test]
fn the_m1_cap_is_enforced_before_writing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let error = store
        .ingest(
            &record("demo", "source:1", MAX_M1_BARS + 1),
            &ingest_bars(&[]).rows(&[], &[], &[]),
        )
        .expect_err("the cap is enforced");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
}

#[test]
fn the_m1_count_must_match_the_rows() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let error = store
        .ingest(
            &record("demo", "source:1", 1),
            &ingest_bars(&[]).rows(&[], &[], &[]),
        )
        .expect_err("a missing M1 row is rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));

    let bars = layer2([unique_row(60, 0)]);
    let error = store
        .ingest(
            &record("demo", "source:1", 0),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
        .expect_err("an undeclared M1 row is rejected");
    assert!(matches!(error, StoreError::InvalidDataset { .. }));
    assert_eq!(count(&store, "dataset"), 0);
}

#[test]
fn an_empty_dataset_is_accepted() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let report = store
        .ingest(
            &record("empty", "source:0", 0),
            &ingest_bars(&[]).rows(&[], &[], &[]),
        )
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
    let reordered = [rows[2], rows[0], rows[1]];
    first
        .ingest(
            &record("demo", "source:1", 2),
            &ingest_bars(&rows).rows(&rows, &[], &[]),
        )
        .expect("the first ingest");
    second
        .ingest(
            &record("demo", "source:1", 2),
            &ingest_bars(&reordered).rows(&reordered, &[], &[]),
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
        .ingest(
            &record("demo", "source:1", 2),
            &ingest_bars(&rows).rows(&rows, &[], &[]),
        )
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
            .ingest(&dataset, &ingest_bars(&[]).rows(&[], &[], &[]))
            .expect_err("an empty identity is rejected");
        assert!(matches!(error, StoreError::InvalidDataset { .. }));
    }
    assert_eq!(count(&store, "dataset"), 0);
}
