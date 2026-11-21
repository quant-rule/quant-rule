//! Tests for the model condition preparer.

use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn root(name: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "qr-model-condition-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("tree");
    path
}

fn write_readings(root: &Path, rows: &[(i64, f64, i64, i64)]) {
    let mut text = String::from(
        "timeframe_secs,bar,timestamp,open,high,low,close,volume,atr,level_touches,level_age,last_pivot_high,last_pivot_low\n",
    );
    for (bar, close, touches, age) in rows {
        text.push_str(&format!(
            "900,{bar},{},{},{},{},{},1.0,1.0,{touches},{age},{},{}\n",
            1_700_000_000 + bar * 900,
            close,
            close + 0.5,
            close - 0.5,
            close,
            close,
            close
        ));
    }
    fs::write(root.join("readings.csv"), text).expect("readings");
}

fn write_events(root: &Path, rows: &[(i64, i64, i64)]) {
    let mut text = String::from(
        "timeframe_secs,event_index,event_code,event_type_code,event_subtype_code,source_code,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value,data_completeness\n",
    );
    for (index, (code, bar, direction)) in rows.iter().enumerate() {
        text.push_str(&format!(
            "900,{index},{code},1,1,1,{bar},{bar},{direction},L1,,100.0,1\n"
        ));
    }
    fs::write(root.join("events.csv"), text).expect("events");
}

fn write_condition(root: &Path, document: Value) -> PathBuf {
    let path = root.join("condition.json");
    fs::write(&path, serde_json::to_string(&document).expect("json")).expect("condition");
    path
}

fn run_condition(bundle: &Path, condition: &Path, out: &Path) -> Result<(), String> {
    run(&[
        "--bundle".to_owned(),
        bundle.display().to_string(),
        "--condition".to_owned(),
        condition.display().to_string(),
        "--registry".to_owned(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/registry/indicators.json")
            .display()
            .to_string(),
        "--out".to_owned(),
        out.display().to_string(),
    ])
}

#[test]
fn min_gap_excludes_the_same_bar() {
    let root = root("gap");
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle).expect("bundle");
    write_readings(
        &bundle,
        &[(0, 100.0, 0, 0), (1, 100.1, 0, 0), (2, 100.2, 0, 0)],
    );
    write_events(&bundle, &[(1, 0, 1), (9, 0, 1), (9, 2, 1)]);
    let condition = write_condition(
        &root,
        json!({
            "schema": SCHEMA,
            "kind": "sequence",
            "first_event": "level.test",
            "second_event": "signal.ema_pair_cross",
            "within_bars": 5,
            "min_gap_bars": 1
        }),
    );
    let out = root.join("out");
    run_condition(&bundle, &condition, &out).expect("run");
    let document: Value =
        serde_json::from_str(&fs::read_to_string(out.join("condition.json")).expect("json"))
            .expect("parse");
    assert_eq!(document["condition_bars"], 1);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn confluence_counts_independent_references() {
    let root = root("confluence");
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle).expect("bundle");
    write_readings(
        &bundle,
        &[(0, 100.0, 0, 0), (1, 100.1, 0, 0), (2, 100.2, 0, 0)],
    );
    let condition = write_condition(
        &root,
        json!({
            "schema": SCHEMA,
            "kind": "confluence",
            "references": ["last_pivot_high", "last_pivot_low"],
            "min_count": 2,
            "within_atr": 1.0
        }),
    );
    let out = root.join("out");
    run_condition(&bundle, &condition, &out).expect("run");
    let document: Value =
        serde_json::from_str(&fs::read_to_string(out.join("condition.json")).expect("json"))
            .expect("parse");
    assert_eq!(document["condition_bars"], 3);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn column_freshness_filters_bars() {
    let root = root("freshness");
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle).expect("bundle");
    write_readings(
        &bundle,
        &[
            (0, 100.0, 1, 0),
            (1, 100.1, 1, 1),
            (2, 100.2, 1, 2),
            (3, 100.3, 1, 3),
        ],
    );
    let condition = write_condition(
        &root,
        json!({
            "schema": SCHEMA,
            "kind": "column",
            "column": "level_touches",
            "values": [1],
            "level_age_max": 2
        }),
    );
    let out = root.join("out");
    run_condition(&bundle, &condition, &out).expect("run");
    let document: Value =
        serde_json::from_str(&fs::read_to_string(out.join("condition.json")).expect("json"))
            .expect("parse");
    assert_eq!(document["condition_bars"], 3);
    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn the_output_is_deterministic() {
    let root = root("deterministic");
    let bundle = root.join("bundle");
    fs::create_dir_all(&bundle).expect("bundle");
    write_readings(&bundle, &[(0, 100.0, 1, 0), (1, 100.1, 1, 1)]);
    let condition = write_condition(
        &root,
        json!({
            "schema": SCHEMA,
            "kind": "column",
            "column": "level_touches",
            "values": [1]
        }),
    );
    let first = root.join("first");
    let second = root.join("second");
    run_condition(&bundle, &condition, &first).expect("first");
    run_condition(&bundle, &condition, &second).expect("second");
    assert_eq!(
        fs::read(first.join("readings.csv")).expect("first"),
        fs::read(second.join("readings.csv")).expect("second")
    );
    fs::remove_dir_all(root).expect("cleanup");
}
