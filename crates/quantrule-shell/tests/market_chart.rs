//! Binary-level tests for the `market-chart` command.
//!
//! The tests run the real binary against the checked-in M1 fixture, assert the
//! deterministic summary, and inspect the PNG header. Chart drawing itself is
//! covered by the shell unit tests.

use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

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
        .join("../../target/market-chart-tests")
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

fn png_info(bytes: &[u8]) -> (u32, u32) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().expect("the PNG header is valid");
    (reader.info().width, reader.info().height)
}

fn assert_stdout_prefix(output: &Output, expected: &str) {
    let text = stdout_text(output);
    let line = text
        .strip_suffix('\n')
        .expect("the summary ends with a newline");
    let (prefix, bytes) = line
        .rsplit_once(" bytes=")
        .expect("the summary carries a byte count");
    assert_eq!(prefix, expected);
    assert!(
        bytes.parse::<usize>().expect("the byte count is a number") > 0,
        "the PNG is empty"
    );
}

#[test]
fn market_chart_writes_a_deterministic_png() {
    let root = artifact_dir("market-chart");
    let path = fixture("state_m1.csv");
    let first = format!("{root}/first.png");
    let second = format!("{root}/second.png");

    let output = run(&["market-chart", &path, "--png", &first, "--candles", "3"]);
    assert_success(&output);
    assert_stdout_prefix(&output, &format!("png={first} timeframe=1m candles=3"));
    let bytes = fs::read(&first).expect("the first PNG is readable");
    assert_eq!(png_info(&bytes), (1_280, 960));

    let output = run(&["market-chart", &path, "--png", &second, "--candles", "3"]);
    assert_success(&output);
    let second_bytes = fs::read(&second).expect("the second PNG is readable");
    assert_eq!(
        bytes, second_bytes,
        "the same input produced different PNG bytes"
    );
}

#[test]
fn market_chart_reports_the_selected_timeframe_and_count() {
    let root = artifact_dir("market-chart-timeframe");
    let path = fixture("state_m1.csv");
    let png = format!("{root}/m5.png");

    let output = run(&[
        "market-chart",
        &path,
        "--png",
        &png,
        "--timeframe",
        "5m",
        "--candles",
        "2",
    ]);
    assert_success(&output);
    assert_stdout_prefix(&output, &format!("png={png} timeframe=5m candles=2"));
    let bytes = fs::read(&png).expect("the PNG is readable");
    assert_eq!(png_info(&bytes), (1_280, 960));
}

#[test]
fn market_chart_renders_h4() {
    let root = artifact_dir("market-chart-h4");
    let path = fixture("state_m1.csv");
    let png = format!("{root}/h4.png");

    let output = run(&[
        "market-chart",
        &path,
        "--png",
        &png,
        "--timeframe",
        "4h",
        "--candles",
        "1",
    ]);
    assert_success(&output);
    assert_stdout_prefix(&output, &format!("png={png} timeframe=4h candles=1"));
    let bytes = fs::read(&png).expect("the PNG is readable");
    assert_eq!(png_info(&bytes), (1_280, 960));
}

#[test]
fn interactive_market_chart_matches_non_interactive() {
    let root = artifact_dir("market-chart-interactive");
    let path = fixture("state_m1.csv");
    let png = format!("{root}/chart.png");
    let args = format!("market-chart {path} --png {png} --candles 5");

    let non_interactive = run(&["market-chart", &path, "--png", &png, "--candles", "5"]);
    assert_success(&non_interactive);
    let interactive = run_interactive(&format!("{args}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(
        stdout_text(&non_interactive),
        stdout_text(&interactive),
        "interactive and non-interactive summaries differ"
    );
    assert!(
        stderr_text(&interactive).contains("quantrule> "),
        "prompt is missing from stderr: {}",
        stderr_text(&interactive)
    );
}

#[test]
fn market_chart_rejects_missing_png_without_writing() {
    let root = artifact_dir("market-chart-missing-png");
    let path = fixture("state_m1.csv");
    let output = run(&["market-chart", &path]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    assert!(stderr_text(&output).contains("missing --png"));
    assert_eq!(
        fs::read_dir(&root).expect("the artifact directory").count(),
        0
    );
}

#[test]
fn market_chart_rejects_an_unknown_timeframe_without_writing() {
    let root = artifact_dir("market-chart-bad-timeframe");
    let path = fixture("state_m1.csv");
    let png = format!("{root}/chart.png");
    let output = run(&["market-chart", &path, "--png", &png, "--timeframe", "30m"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    assert!(stderr_text(&output).contains("--timeframe expects"));
    assert!(!Path::new(&png).exists(), "a rejected chart wrote a PNG");
}

#[test]
fn market_chart_rejects_an_empty_source_without_writing() {
    let root = artifact_dir("market-chart-empty");
    let path = fixture("empty.csv");
    let png = format!("{root}/chart.png");
    let output = run(&["market-chart", &path, "--png", &png]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout_text(&output), "");
    assert!(stderr_text(&output).contains("cannot render an empty chart"));
    assert!(!Path::new(&png).exists(), "an empty chart wrote a PNG");
}
