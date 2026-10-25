//! Binary-level audit of the consensus and aggregate rows.
//!
//! The writer unit tests cover binding and validation. This test runs the real
//! binary over `state_m1.csv`, then reads the SQLite file and checks the stored
//! rows against the stored axes. It is the end-to-end check that the model,
//! analytics, and writer agree.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use rusqlite::Connection;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const DATA_VERSION: &str = "test:demo:60:0-260";

fn fixture() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/state_m1.csv")
        .to_str()
        .expect("fixture path is valid UTF-8")
        .to_owned()
}

fn artifact_dir(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/store-aggregate-tests")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the artifact directory is created");
    path.to_str()
        .expect("the artifact path is UTF-8")
        .to_owned()
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the quantrule binary runs")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn scalar(connection: &Connection, sql: &str) -> i64 {
    connection
        .query_row(sql, [], |row| row.get(0))
        .expect("the scalar query succeeds")
}

fn parse_list(text: &str) -> Vec<u64> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split(';')
        .map(|value| value.parse().expect("a timeframe in the list"))
        .collect()
}

#[test]
fn store_ingest_writes_consensus_and_aggregate_rows() {
    let root = artifact_dir("consensus-aggregates");
    let db = format!("{root}/store.db");
    let path = fixture();
    let ingest = || {
        run(&[
            "store",
            "ingest",
            &path,
            "--dataset",
            "demo",
            "--data-version",
            DATA_VERSION,
            "--db",
            &db,
        ])
    };

    let output = ingest();
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "ingested demo bars=333 timeframes=4\n"
    );

    let connection = Connection::open(&db).expect("the store opens");
    assert_eq!(scalar(&connection, "SELECT count(*) FROM consensus"), 333);
    assert_eq!(scalar(&connection, "SELECT count(*) FROM layer3"), 260);
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM bar WHERE timeframe_secs = 60"
        ),
        260
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM bar b LEFT JOIN consensus c \
             ON c.dataset_id = b.dataset_id AND c.timeframe_secs = b.timeframe_secs \
             AND c.bar_index = b.bar_index WHERE c.dataset_id IS NULL"
        ),
        0,
        "a bar has no consensus row"
    );

    let count_mismatches = scalar(
        &connection,
        "SELECT count(*) FROM consensus c JOIN bar b \
         ON b.dataset_id = c.dataset_id AND b.timeframe_secs = c.timeframe_secs \
         AND b.bar_index = c.bar_index \
         WHERE c.bullish_axis_count != \
           (CASE WHEN b.direction > 0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.momentum_force > 0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.structure_score > 0.1 THEN 1 ELSE 0 END) \
         OR c.bearish_axis_count != \
           (CASE WHEN b.direction < -0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.momentum_force < -0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.structure_score < -0.1 THEN 1 ELSE 0 END) \
         OR c.neutral_axis_count != \
           (CASE WHEN b.direction IS NOT NULL AND abs(b.direction) <= 0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.momentum_force IS NOT NULL AND abs(b.momentum_force) <= 0.1 THEN 1 ELSE 0 END) + \
           (CASE WHEN b.structure_score IS NOT NULL AND abs(b.structure_score) <= 0.1 THEN 1 ELSE 0 END)",
    );
    assert_eq!(
        count_mismatches, 0,
        "a consensus count differs from its axes"
    );

    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM consensus WHERE bullish_axis_count > 3 \
             OR bearish_axis_count > 3 OR neutral_axis_count > 3 \
             OR bullish_axis_count + bearish_axis_count + neutral_axis_count > 3"
        ),
        0,
        "a consensus count is out of range"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM consensus WHERE weakest_directional_axis_code IS NOT NULL \
             AND (weakest_directional_axis_code < 1 OR weakest_directional_axis_code > 3)"
        ),
        0,
        "a weakest code is out of range"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM consensus c JOIN bar b \
             ON b.dataset_id = c.dataset_id AND b.timeframe_secs = c.timeframe_secs \
             AND b.bar_index = c.bar_index \
             WHERE (c.weakest_directional_axis_code = 1 AND b.direction_size_percentile IS NULL) \
             OR (c.weakest_directional_axis_code = 2 AND b.momentum_size_percentile IS NULL) \
             OR (c.weakest_directional_axis_code = 3 AND b.structure_size_percentile IS NULL)"
        ),
        0,
        "a weakest code names an axis with no percentile"
    );

    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM layer3 WHERE m5_bar_index IS NOT NULL \
             OR m15_bar_index IS NOT NULL OR h1_bar_index IS NOT NULL"
        ),
        0,
        "a block column is populated"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT layer3_materialized FROM dataset WHERE name = 'demo'"
        ),
        0
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM layer3 l JOIN bar b \
             ON b.dataset_id = l.dataset_id AND b.timeframe_secs = 60 \
             AND b.bar_index = l.bar_index"
        ),
        260,
        "a Layer 3 row has no M1 base"
    );

    let mut statement = connection
        .prepare(
            "SELECT bar_index, agg_ladder, agg_direction_timeframes, \
             agg_momentum_timeframes, agg_structure_timeframes, \
             agg_progression_timeframes FROM layer3 ORDER BY bar_index",
        )
        .expect("the list query prepares");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .expect("the list rows are read");
    let mut expected_bar = 0_i64;
    for row in rows {
        let (bar_index, ladder, direction, momentum, structure, progression) =
            row.expect("the list row is valid");
        assert_eq!(bar_index, expected_bar);
        expected_bar += 1;

        let ladder = parse_list(&ladder);
        assert_eq!(ladder.first(), Some(&60), "the ladder does not start at M1");
        assert!(
            ladder.windows(2).all(|pair| pair[0] < pair[1]),
            "the ladder is unsorted or duplicated"
        );
        for (name, list) in [
            ("direction", direction),
            ("momentum", momentum),
            ("structure", structure),
            ("progression", progression),
        ] {
            let list = parse_list(&list);
            assert!(
                list.windows(2).all(|pair| pair[0] < pair[1]),
                "the {name} list is unsorted or duplicated"
            );
            assert!(
                list.iter().all(|timeframe| ladder.contains(timeframe)),
                "the {name} list names a timeframe outside the ladder"
            );
        }
    }
    assert_eq!(expected_bar, 260);
    drop(statement);

    let mean_mismatches = scalar(
        &connection,
        "SELECT count(*) FROM layer3 l JOIN bar b \
         ON b.dataset_id = l.dataset_id AND b.timeframe_secs = 60 \
         AND b.bar_index = l.bar_index \
         WHERE (l.agg_direction_timeframes = '60' \
                AND l.agg_direction_mean IS NOT b.direction) \
            OR (l.agg_momentum_timeframes = '60' \
                AND l.agg_momentum_force_mean IS NOT b.momentum_force)",
    );
    assert_eq!(
        mean_mismatches, 0,
        "an M1-only mean differs from its base value"
    );
    assert!(
        scalar(
            &connection,
            "SELECT count(*) FROM layer3 WHERE agg_direction_timeframes = '60'"
        ) > 0,
        "the M1-only mean check is not exercised"
    );

    let events_before = scalar(&connection, "SELECT count(*) FROM event");
    let legs_before = scalar(&connection, "SELECT count(*) FROM leg");
    let levels_before = scalar(&connection, "SELECT count(*) FROM level");

    let output = ingest();
    assert_success(&output);
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "unchanged demo bars=333 timeframes=4\n"
    );
    assert_eq!(scalar(&connection, "SELECT count(*) FROM consensus"), 333);
    assert_eq!(scalar(&connection, "SELECT count(*) FROM layer3"), 260);

    let events = scalar(&connection, "SELECT count(*) FROM event");
    let legs = scalar(&connection, "SELECT count(*) FROM leg");
    let levels = scalar(&connection, "SELECT count(*) FROM level");
    assert_eq!(events, events_before, "a re-ingest changed the event count");
    assert_eq!(legs, legs_before, "a re-ingest changed the leg count");
    assert_eq!(levels, levels_before, "a re-ingest changed the level count");
    assert!(events > 0, "the fixture must store events");
    assert!(legs > 0, "the fixture must store legs");
    assert!(levels > 0, "the fixture must store levels");
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM event WHERE available_bar_index < occurred_bar_index"
        ),
        0,
        "an event is available before it occurred"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM level WHERE available_bar_index < occurred_bar_index"
        ),
        0,
        "a level is available before it occurred"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM leg WHERE origin_available_bar_index < origin_occurred_bar_index"
        ),
        0,
        "a leg is available before its origin occurred"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM event e LEFT JOIN bar b \
             ON b.dataset_id = e.dataset_id AND b.timeframe_secs = e.timeframe_secs \
             AND b.bar_index = e.occurred_bar_index WHERE b.dataset_id IS NULL"
        ),
        0,
        "an event names an occurrence bar that is not stored"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM event e LEFT JOIN bar b \
             ON b.dataset_id = e.dataset_id AND b.timeframe_secs = e.timeframe_secs \
             AND b.bar_index = e.available_bar_index WHERE b.dataset_id IS NULL"
        ),
        0,
        "an event names an availability bar that is not stored"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM level l LEFT JOIN bar b \
             ON b.dataset_id = l.dataset_id AND b.timeframe_secs = l.timeframe_secs \
             AND b.bar_index = l.available_bar_index WHERE b.dataset_id IS NULL"
        ),
        0,
        "a level names an availability bar that is not stored"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM leg g LEFT JOIN bar b \
             ON b.dataset_id = g.dataset_id AND b.timeframe_secs = g.timeframe_secs \
             AND b.bar_index = g.origin_occurred_bar_index WHERE b.dataset_id IS NULL"
        ),
        0,
        "a leg names an origin bar that is not stored"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM leg g LEFT JOIN bar b \
             ON b.dataset_id = g.dataset_id AND b.timeframe_secs = g.timeframe_secs \
             AND b.bar_index = g.origin_available_bar_index \
             WHERE g.origin_available_bar_index IS NOT NULL AND b.dataset_id IS NULL"
        ),
        0,
        "a leg names an availability bar that is not stored"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM event WHERE event_type_code = 3 AND event_subtype_code = 1 \
             AND reference_value IS NULL"
        ),
        0,
        "a Signal threshold crossing has no threshold"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM ( \
               SELECT dataset_id, timeframe_secs, leg_id FROM leg \
               GROUP BY dataset_id, timeframe_secs, leg_id HAVING count(*) > 1)"
        ),
        0,
        "a leg identifier is duplicated"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM ( \
               SELECT dataset_id, timeframe_secs, event_type_code, event_subtype_code, \
                      source_code, occurred_bar_index, reference_code \
               FROM event WHERE NOT (event_type_code = 3 AND event_subtype_code = 1) \
               GROUP BY dataset_id, timeframe_secs, event_type_code, event_subtype_code, \
                        source_code, occurred_bar_index, reference_code \
               HAVING count(*) > 1)"
        ),
        0,
        "a non-threshold event occurrence is duplicated"
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM ( \
               SELECT dataset_id, timeframe_secs, event_type_code, event_subtype_code, \
                      source_code, occurred_bar_index, reference_code, reference_value \
               FROM event WHERE event_type_code = 3 AND event_subtype_code = 1 \
               GROUP BY dataset_id, timeframe_secs, event_type_code, event_subtype_code, \
                        source_code, occurred_bar_index, reference_code, reference_value \
               HAVING count(*) > 1)"
        ),
        0,
        "a threshold crossing is duplicated"
    );
}
