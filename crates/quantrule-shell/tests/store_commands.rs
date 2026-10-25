//! Binary-level tests for the store query commands.
//!
//! The tests ingest the checked-in M1 fixture into a target database, then run
//! the read-only commands and assert on their headers, rows, and errors.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const DATA_VERSION: &str = "test:demo:60:0-260";

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_str()
        .expect("fixture path is valid UTF-8")
        .to_owned()
}

fn artifact_dir(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/store-command-tests")
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

fn run_interactive(input: &str) -> Output {
    let mut child = Command::new(BIN)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the quantrule binary starts");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(input.as_bytes())
        .expect("interactive input is written");
    child
        .wait_with_output()
        .expect("the quantrule binary exits")
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is valid UTF-8")
}

fn stderr_text(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is valid UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        stderr_text(output)
    );
}

fn assert_failure(output: &Output) {
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(output), "");
}

fn ingest(root: &str) -> String {
    let db = format!("{root}/store.db");
    let path = fixture("state_m1.csv");
    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        DATA_VERSION,
        "--db",
        &db,
    ]);
    assert_success(&output);
    db
}

fn csv_lines(text: &str) -> Vec<Vec<&str>> {
    text.lines().map(|line| line.split(',').collect()).collect()
}

fn column(header: &[&str], name: &str) -> usize {
    header
        .iter()
        .position(|column| *column == name)
        .expect("the column exists")
}

#[test]
fn store_list_prints_the_dataset_and_counts() {
    let root = artifact_dir("store-list");
    let db = ingest(&root);
    let output = run(&["store", "list", "--db", &db]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines = csv_lines(&text);
    assert_eq!(lines.len(), 2);
    let header = &lines[0];
    let row = &lines[1];
    assert_eq!(row[column(header, "name")], "demo");
    assert_eq!(row[column(header, "data_version")], DATA_VERSION);
    assert_eq!(row[column(header, "structure_scale_code")], "2");
    assert_eq!(row[column(header, "m1_bars")], "260");
    assert_eq!(row[column(header, "layer3_materialized")], "1");
    assert_eq!(row[column(header, "bar_rows")], "333");
    assert_eq!(row[column(header, "consensus_rows")], "333");
    assert_eq!(row[column(header, "layer3_rows")], "260");
    assert_eq!(row[column(header, "pivot_rows")], "354");
    assert_eq!(row[column(header, "level_rows")], "94");
    assert_eq!(row[column(header, "event_rows")], "624");
    assert_eq!(row[column(header, "leg_rows")], "217");
}

#[test]
fn store_show_prints_one_row_and_the_full_width() {
    let root = artifact_dir("store-show");
    let db = ingest(&root);
    let output = run(&[
        "store",
        "show",
        "demo",
        "bar",
        "--timeframe",
        "1m",
        "--bar",
        "0",
        "--limit",
        "1",
        "--db",
        &db,
    ]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines = csv_lines(&text);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].len(), 74);
    assert_eq!(lines[0][0], "dataset_id");
    assert_eq!(lines[0][1], "timeframe_secs");
    assert_eq!(lines[0][2], "bar_index");
    assert_eq!(lines[1][1], "60");
    assert_eq!(lines[1][2], "0");
}

#[test]
fn store_show_reaches_every_table() {
    let root = artifact_dir("store-show-tables");
    let db = ingest(&root);
    for (table, width) in [
        ("bar", 74),
        ("consensus", 7),
        ("layer3", 285),
        ("pivot", 8),
        ("level", 8),
        ("event", 12),
        ("leg", 19),
    ] {
        let output = run(&["store", "show", "demo", table, "--limit", "1", "--db", &db]);
        assert_success(&output);
        let text = stdout_text(&output);
        let header = text.lines().next().expect("the header exists");
        assert_eq!(header.split(',').count(), width, "the {table} header width");
    }
}

#[test]
fn db_views_print_the_expected_widths() {
    let root = artifact_dir("db-views");
    let db = ingest(&root);

    let series = run(&[
        "db",
        "series",
        "demo",
        "--timeframe",
        "1m",
        "--limit",
        "2",
        "--db",
        &db,
    ]);
    assert_success(&series);
    let series_text = stdout_text(&series);
    let series_lines: Vec<&str> = series_text.lines().collect();
    assert_eq!(series_lines.len(), 2);
    assert!(
        series_lines[0].starts_with("{\"dataset\":\"demo\",\"timeframe_secs\":60,\"bar_index\":0,")
    );
    assert!(series_lines[1].contains("\"bar_index\":1,"));

    let bars = run(&["db", "bars", "demo", "--limit", "1", "--db", &db]);
    assert_success(&bars);
    assert_eq!(csv_lines(&stdout_text(&bars))[0].len(), 74);

    let metrics = run(&[
        "db",
        "metrics",
        "demo",
        "--timeframe",
        "1m",
        "--limit",
        "1",
        "--db",
        &db,
    ]);
    assert_success(&metrics);
    assert_eq!(csv_lines(&stdout_text(&metrics))[0].len(), 61);

    let layer3 = run(&["db", "layer3", "demo", "--limit", "1", "--db", &db]);
    assert_success(&layer3);
    assert_eq!(csv_lines(&stdout_text(&layer3))[0].len(), 285);
}

#[test]
fn a_missing_database_is_not_created() {
    let root = artifact_dir("missing-database");
    let db = format!("{root}/missing.db");
    let output = run(&["store", "list", "--db", &db]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("store not found"));
    assert!(!Path::new(&db).exists(), "a query created a database");
}

#[test]
fn a_missing_dataset_is_an_error() {
    let root = artifact_dir("missing-dataset");
    let db = ingest(&root);
    let output = run(&["store", "show", "missing", "bar", "--db", &db]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("dataset 'missing' was not found"));
}

#[test]
fn interactive_store_list_matches_non_interactive() {
    let root = artifact_dir("interactive-list");
    let db = ingest(&root);
    let non_interactive = run(&["store", "list", "--db", &db]);
    assert_success(&non_interactive);
    let interactive = run_interactive(&format!("store list --db {db}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(
        stdout_text(&non_interactive),
        stdout_text(&interactive),
        "interactive and non-interactive output differ"
    );
    assert!(stderr_text(&interactive).contains("quantrule> "));
}
