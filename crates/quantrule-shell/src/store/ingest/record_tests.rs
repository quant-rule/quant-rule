//! Tests for the event and leg record writers.
//!
//! The tests exercise the writer's own boundary: it copies the engine's records
//! into the schema, validates their shape and contract ranges before the
//! transaction opens, and never lets a record change a metric value.

use super::fixtures::{
    count, event_row, ingest_bars, insert_columns, layer2, leg_row, level_row, record, stored_rows,
    unique_row,
};
use super::sql::{EVENT_INSERT, LEG_INSERT};
use crate::store::{Store, StoreError};
use quantrule_analytics::layers::{EventRow, LegRow};
use rusqlite::types::Value;

/// A complete event whose reference and measured values are all present.
fn full_event() -> EventRow {
    EventRow {
        timeframe_secs: 60,
        event_type_code: 2,
        event_subtype_code: 3,
        source_code: 7,
        occurred_bar_index: 1,
        available_bar_index: 3,
        direction_code: -1,
        reference_code: Some(1001),
        primary_value: Some(41.25),
        reference_value: Some(38.5),
    }
}

/// An event with no reference and no measured values.
fn bare_event() -> EventRow {
    EventRow {
        timeframe_secs: 60,
        event_type_code: 1,
        event_subtype_code: 1,
        source_code: 1,
        occurred_bar_index: 2,
        available_bar_index: 2,
        direction_code: 0,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    }
}

/// A confirmed leg whose every optional value is present.
fn full_leg() -> LegRow {
    LegRow {
        timeframe_secs: 60,
        leg_id: 1,
        scale_code: 1,
        origin_price: 9.75,
        origin_occurred_bar_index: 1,
        origin_available_bar_index: Some(3),
        origin_confirmed_code: 1,
        leg_direction_code: 1,
        target_price: Some(12.5),
        target_basis_code: Some(1),
        target_version: 2,
        target_stable_code: 1,
        progress: Some(0.75),
        displacement_price: Some(1.5),
        displacement_atr: Some(1.25),
        lifecycle_code: 1,
        stage_code: Some(3),
        target_reached_code: Some(0),
    }
}

/// A forming leg whose origin and target are not yet knowable.
fn forming_leg() -> LegRow {
    LegRow {
        timeframe_secs: 60,
        leg_id: 2,
        scale_code: 2,
        origin_price: 11.0,
        origin_occurred_bar_index: 2,
        origin_available_bar_index: None,
        origin_confirmed_code: 0,
        leg_direction_code: -1,
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

/// The base event every invalid event case mutates.
fn base_event() -> EventRow {
    event_row(60, 1, 1, 1, 1, 1, 0)
}

/// A Signal threshold crossing at `threshold`.
fn threshold_event(threshold: f64) -> EventRow {
    EventRow {
        timeframe_secs: 60,
        event_type_code: 3,
        event_subtype_code: 1,
        source_code: 4,
        occurred_bar_index: 1,
        available_bar_index: 1,
        direction_code: 1,
        reference_code: None,
        primary_value: Some(62.0),
        reference_value: Some(threshold),
    }
}

/// The base leg every invalid leg case mutates.
fn base_leg() -> LegRow {
    LegRow {
        timeframe_secs: 60,
        leg_id: 1,
        scale_code: 1,
        origin_price: 10.0,
        origin_occurred_bar_index: 0,
        origin_available_bar_index: Some(1),
        origin_confirmed_code: 1,
        leg_direction_code: 1,
        target_price: Some(12.0),
        target_basis_code: Some(1),
        target_version: 0,
        target_stable_code: 1,
        progress: Some(0.5),
        displacement_price: Some(1.0),
        displacement_atr: Some(0.5),
        lifecycle_code: 1,
        stage_code: Some(2),
        target_reached_code: Some(0),
    }
}

#[test]
fn every_written_event_and_leg_round_trips() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2((0..8).map(|bar| unique_row(60, bar)));
    let events = [full_event(), bare_event()];
    let legs = [full_leg(), forming_leg()];
    let levels = [level_row(60, 1001, 1, 1, 11.25, 2, 4)];
    let report = store
        .ingest(
            &record("demo", "source:1", 8),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &levels, &events, &legs),
        )
        .expect("the dataset is ingested");
    assert_eq!(report.bars, 8);

    let stored_events = stored_rows(
        &store,
        "SELECT dataset_id, timeframe_secs, event_type_code, event_subtype_code, \
             source_code, occurred_bar_index, available_bar_index, direction_code, \
             reference_code, primary_value, reference_value \
             FROM event ORDER BY available_bar_index, event_type_code, event_subtype_code",
    );
    assert_eq!(
        stored_events,
        vec![
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(2_i64),
                Value::from(2_i64),
                Value::from(0_i64),
                Value::from(0_i64),
                Value::Null,
                Value::Null,
            ],
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(2_i64),
                Value::from(3_i64),
                Value::from(7_i64),
                Value::from(1_i64),
                Value::from(3_i64),
                Value::from(-1_i64),
                Value::from(1001_i64),
                Value::from(41.25),
                Value::from(38.5),
            ],
        ]
    );

    let stored_legs = stored_rows(
        &store,
        "SELECT dataset_id, timeframe_secs, leg_id, scale_code, origin_price, \
             origin_occurred_bar_index, origin_available_bar_index, origin_confirmed_code, \
             leg_direction_code, target_price, target_basis_code, target_version, \
             target_stable_code, progress, displacement_price, displacement_atr, \
             lifecycle_code, stage_code, target_reached_code \
             FROM leg ORDER BY leg_id",
    );
    assert_eq!(
        stored_legs,
        vec![
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(9.75),
                Value::from(1_i64),
                Value::from(3_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(12.5),
                Value::from(1_i64),
                Value::from(2_i64),
                Value::from(1_i64),
                Value::from(0.75),
                Value::from(1.5),
                Value::from(1.25),
                Value::from(1_i64),
                Value::from(3_i64),
                Value::from(0_i64),
            ],
            vec![
                Value::from(report.dataset_id),
                Value::from(60_i64),
                Value::from(2_i64),
                Value::from(2_i64),
                Value::from(11.0),
                Value::from(2_i64),
                Value::Null,
                Value::from(0_i64),
                Value::from(-1_i64),
                Value::Null,
                Value::Null,
                Value::from(0_i64),
                Value::from(0_i64),
                Value::Null,
                Value::Null,
                Value::Null,
                Value::from(0_i64),
                Value::Null,
                Value::Null,
            ],
        ]
    );

    let stored_levels = stored_rows(
        &store,
        "SELECT dataset_id, timeframe_secs, reference_code, scale_code, kind_code, price, \
             occurred_bar_index, available_bar_index FROM level ORDER BY reference_code",
    );
    assert_eq!(
        stored_levels,
        vec![vec![
            Value::from(report.dataset_id),
            Value::from(60_i64),
            Value::from(1001_i64),
            Value::from(1_i64),
            Value::from(1_i64),
            Value::from(11.25),
            Value::from(2_i64),
            Value::from(4_i64),
        ]]
    );
}

#[test]
fn coincident_threshold_events_are_distinct_rows() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2((0..4).map(|bar| unique_row(60, bar)));
    let events = [
        threshold_event(70.0),
        threshold_event(30.0),
        threshold_event(50.0),
    ];
    store
        .ingest(
            &record("thresholds", "source:1", 4),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &[], &events, &[]),
        )
        .expect("the coincident crossings are ingested");
    assert_eq!(count(&store, "event"), 3);

    let stored = stored_rows(
        &store,
        "SELECT event_type_code, event_subtype_code, source_code, occurred_bar_index, \
             available_bar_index, reference_code, reference_value \
             FROM event ORDER BY reference_value",
    );
    assert_eq!(
        stored,
        vec![
            vec![
                Value::from(3_i64),
                Value::from(1_i64),
                Value::from(4_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(0_i64),
                Value::from(30.0),
            ],
            vec![
                Value::from(3_i64),
                Value::from(1_i64),
                Value::from(4_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(0_i64),
                Value::from(50.0),
            ],
            vec![
                Value::from(3_i64),
                Value::from(1_i64),
                Value::from(4_i64),
                Value::from(1_i64),
                Value::from(1_i64),
                Value::from(0_i64),
                Value::from(70.0),
            ],
        ]
    );
}

#[test]
fn invalid_event_and_leg_records_are_rejected_before_writing() {
    let mut store = Store::open_in_memory().expect("the store opens");
    let bars = layer2([unique_row(60, 0), unique_row(60, 1), unique_row(60, 2)]);
    let cases: Vec<(Vec<EventRow>, Vec<LegRow>)> = vec![
        (
            vec![EventRow {
                event_type_code: 4,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                event_subtype_code: 0,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                event_type_code: 2,
                event_subtype_code: 5,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                source_code: 0,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                source_code: 13,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                occurred_bar_index: 3,
                available_bar_index: 3,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                available_bar_index: 3,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                occurred_bar_index: 2,
                available_bar_index: 1,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                direction_code: 2,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                reference_code: Some(0),
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                reference_code: Some(4),
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                reference_code: Some(1000),
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                primary_value: Some(f64::NAN),
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                reference_value: Some(f64::INFINITY),
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![EventRow {
                event_type_code: 3,
                event_subtype_code: 1,
                ..base_event()
            }],
            Vec::new(),
        ),
        (
            vec![threshold_event(30.0), threshold_event(30.0)],
            Vec::new(),
        ),
        (vec![base_event(), base_event()], Vec::new()),
        (vec![event_row(300, 1, 1, 1, 0, 0, 0)], Vec::new()),
        (
            Vec::new(),
            vec![LegRow {
                leg_id: 0,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                scale_code: 3,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_price: f64::NAN,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_occurred_bar_index: 3,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_confirmed_code: 2,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_available_bar_index: None,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_confirmed_code: 0,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_occurred_bar_index: 2,
                origin_available_bar_index: Some(1),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                origin_available_bar_index: Some(3),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                leg_direction_code: 0,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_price: Some(f64::NAN),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_basis_code: Some(3),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_price: None,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_basis_code: None,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_stable_code: 2,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_price: None,
                target_basis_code: None,
                target_stable_code: 1,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                progress: Some(f64::INFINITY),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                displacement_price: Some(f64::NAN),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                displacement_atr: Some(f64::NEG_INFINITY),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                lifecycle_code: 5,
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                stage_code: Some(6),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                stage_code: Some(-1),
                ..base_leg()
            }],
        ),
        (
            Vec::new(),
            vec![LegRow {
                target_reached_code: Some(2),
                ..base_leg()
            }],
        ),
        (Vec::new(), vec![base_leg(), base_leg()]),
        (Vec::new(), vec![leg_row(300, 1, 1, 10.0, 0, Some(1), 1)]),
    ];

    for (index, (events, legs)) in cases.iter().enumerate() {
        let error = match store.ingest(
            &record("demo", "source:1", 3),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &[], events, legs),
        ) {
            Ok(report) => panic!("case {index} was accepted: {report:?}"),
            Err(error) => error,
        };
        assert!(
            matches!(error, StoreError::InvalidDataset { .. }),
            "case {index}"
        );
        assert_eq!(count(&store, "dataset"), 0, "case {index}");
        assert_eq!(count(&store, "bar"), 0, "case {index}");
        assert_eq!(count(&store, "event"), 0, "case {index}");
        assert_eq!(count(&store, "leg"), 0, "case {index}");
    }
}

#[test]
fn a_failed_event_insert_rolls_back_every_record() {
    let mut store = Store::open_in_memory().expect("the store opens");
    store
        .connection()
        .execute_batch(
            "CREATE TRIGGER fail_event BEFORE INSERT ON event \
             BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
        )
        .expect("the trigger is created");
    let bars = layer2((0..3).map(|bar| unique_row(60, bar)));
    let events = [base_event()];
    let legs = [base_leg()];

    let error = store
        .ingest(
            &record("demo", "source:1", 3),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &[], &events, &legs),
        )
        .expect_err("the forced failure rolls back the transaction");
    assert!(matches!(error, StoreError::Write { .. }));
    assert_empty(&store);
}

#[test]
fn a_failed_leg_insert_rolls_back_every_record() {
    let mut store = Store::open_in_memory().expect("the store opens");
    store
        .connection()
        .execute_batch(
            "CREATE TRIGGER fail_leg BEFORE INSERT ON leg \
             BEGIN SELECT RAISE(ABORT, 'forced failure'); END;",
        )
        .expect("the trigger is created");
    let bars = layer2((0..3).map(|bar| unique_row(60, bar)));
    let events = [base_event()];
    let legs = [base_leg()];

    let error = store
        .ingest(
            &record("demo", "source:1", 3),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &[], &events, &legs),
        )
        .expect_err("the forced failure rolls back the transaction");
    assert!(matches!(error, StoreError::Write { .. }));
    assert_empty(&store);
}

fn assert_empty(store: &Store) {
    for table in [
        "dataset",
        "bar",
        "consensus",
        "layer3",
        "pivot",
        "level",
        "event",
        "leg",
    ] {
        assert_eq!(count(store, table), 0, "table {table}");
    }
}

#[test]
fn events_and_legs_do_not_change_any_metric_value() {
    let bars = layer2((0..8).map(|bar| unique_row(60, bar)));
    let events = [full_event(), bare_event()];
    let legs = [full_leg(), forming_leg()];

    let mut plain = Store::open_in_memory().expect("the plain store opens");
    plain
        .ingest(
            &record("demo", "source:1", 8),
            &ingest_bars(&bars).rows(&bars, &[], &[]),
        )
        .expect("the plain dataset is ingested");

    let mut with_records = Store::open_in_memory().expect("the record store opens");
    with_records
        .ingest(
            &record("demo", "source:1", 8),
            &ingest_bars(&bars).rows_with_records(&bars, &[], &[], &events, &legs),
        )
        .expect("the dataset with records is ingested");

    let plain_bars = stored_rows(
        &plain,
        "SELECT * FROM bar ORDER BY timeframe_secs, bar_index",
    );
    let record_bars = stored_rows(
        &with_records,
        "SELECT * FROM bar ORDER BY timeframe_secs, bar_index",
    );
    assert_eq!(plain_bars, record_bars);

    let plain_layer3 = stored_rows(&plain, "SELECT * FROM layer3 ORDER BY bar_index");
    let record_layer3 = stored_rows(&with_records, "SELECT * FROM layer3 ORDER BY bar_index");
    assert_eq!(plain_layer3, record_layer3);
}

#[test]
fn the_event_insert_binds_eleven_parameters() {
    assert_eq!(EVENT_INSERT.matches('?').count(), 11);
}

#[test]
fn the_leg_insert_binds_nineteen_parameters() {
    assert_eq!(LEG_INSERT.matches('?').count(), 19);
}

#[test]
fn the_event_and_leg_insert_columns_match_the_ddl_order() {
    let event_columns = table_columns("CREATE TABLE event (");
    let expected_event: Vec<String> = event_columns
        .into_iter()
        .filter(|column| column != "id")
        .collect();
    assert_eq!(insert_columns(EVENT_INSERT), expected_event);

    let leg_columns = table_columns("CREATE TABLE leg (");
    assert_eq!(insert_columns(LEG_INSERT), leg_columns);
}

/// Reads a table's column names from the checked-in DDL, in declaration order.
fn table_columns(header: &str) -> Vec<String> {
    let schema = include_str!("../schema.rs");
    let start = schema.find(header).expect("the DDL") + header.len();
    let end = start + schema[start..].find("\n);").expect("the DDL end");
    schema[start..end]
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with("PRIMARY KEY") || line.starts_with("FOREIGN KEY")
            {
                return None;
            }
            let name = line.split_whitespace().next()?;
            Some(name.trim_end_matches(',').to_string())
        })
        .collect()
}
