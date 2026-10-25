//! Binary-level tests for `store verify`.
//!
//! The tests ingest the checked-in M1 fixture, verify it, corrupt selected
//! values or flags, and assert the error path. No test writes outside `target/`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

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
        .join("../../target/store-verify-tests")
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
    let path = fixture();
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

fn verify(db: &str) -> Output {
    let path = fixture();
    run(&[
        "store",
        "verify",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        DATA_VERSION,
        "--db",
        db,
    ])
}

#[test]
fn store_verify_reports_zero_mismatches_and_stays_a_no_op() {
    let root = artifact_dir("verify-ok");
    let db = ingest(&root);

    let output = verify(&db);
    assert_success(&output);
    assert_eq!(stdout_text(&output), "verified demo bars=333 tables=7\n");

    let path = fixture();
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
    assert_eq!(
        stdout_text(&output),
        "unchanged demo bars=333 timeframes=4\n"
    );

    let output = verify(&db);
    assert_success(&output);
    assert_eq!(stdout_text(&output), "verified demo bars=333 tables=7\n");
}

#[test]
fn store_verify_reports_a_changed_value() {
    let root = artifact_dir("verify-changed");
    let db = ingest(&root);
    let connection = Connection::open(&db).expect("the store opens");
    connection
        .execute(
            "UPDATE bar SET close = close + 1 WHERE timeframe_secs = 60 AND bar_index = 0",
            [],
        )
        .expect("the bar is changed");
    drop(connection);

    let output = verify(&db);
    assert_failure(&output);
    let error = stderr_text(&output);
    assert!(error.contains("verification failed"));
    assert!(error.contains("bar row 0 column close"));
}

#[test]
fn store_verify_rejects_an_incomplete_dataset() {
    let root = artifact_dir("verify-incomplete");
    let db = ingest(&root);
    let connection = Connection::open(&db).expect("the store opens");
    connection
        .execute("UPDATE dataset SET layer3_materialized = 0", [])
        .expect("the flag is changed");
    drop(connection);

    let output = verify(&db);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("has no materialized Layer 3"));
}

#[test]
fn store_verify_rejects_a_wrong_data_version() {
    let root = artifact_dir("verify-version");
    let db = ingest(&root);
    let path = fixture();
    let output = run(&[
        "store",
        "verify",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        "test:demo:60:1-260",
        "--db",
        &db,
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("dataset data_version"));
}

#[test]
fn store_verify_rejects_a_missing_dataset() {
    let root = artifact_dir("verify-missing-dataset");
    let db = ingest(&root);
    let path = fixture();
    let output = run(&[
        "store",
        "verify",
        &path,
        "--dataset",
        "missing",
        "--data-version",
        DATA_VERSION,
        "--db",
        &db,
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("dataset 'missing' was not found"));
}

#[test]
fn store_verify_rejects_a_missing_database_without_creating_it() {
    let root = artifact_dir("verify-missing-db");
    let db = format!("{root}/missing.db");
    let path = fixture();
    let output = run(&[
        "store",
        "verify",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        DATA_VERSION,
        "--db",
        &db,
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("store not found"));
    assert!(!Path::new(&db).exists(), "verify created a database");
}

#[test]
fn store_verify_reports_an_orphan_row() {
    let root = artifact_dir("verify-orphan");
    let db = ingest(&root);
    let connection = Connection::open(&db).expect("the store opens");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = OFF;\
             INSERT INTO level \
             (dataset_id, timeframe_secs, reference_code, scale_code, kind_code, price, \
              occurred_bar_index, available_bar_index) \
             VALUES (999, 60, 9999, 1, 1, 10.0, 0, 0);",
        )
        .expect("the orphan is inserted");
    drop(connection);

    let output = verify(&db);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("foreign_key_check"));
}

#[test]
fn interactive_store_verify_matches_non_interactive() {
    let root = artifact_dir("verify-interactive");
    let db = ingest(&root);
    let path = fixture();
    let non_interactive = verify(&db);
    assert_success(&non_interactive);
    let interactive = run_interactive(&format!(
        "store verify {path} --dataset demo --data-version {DATA_VERSION} --db {db}\nquit\n"
    ));
    assert_success(&interactive);
    assert_eq!(
        stdout_text(&non_interactive),
        stdout_text(&interactive),
        "interactive and non-interactive verify output differ"
    );
    assert!(stderr_text(&interactive).contains("quantrule> "));
}
