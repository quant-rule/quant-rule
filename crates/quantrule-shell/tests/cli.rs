//! Binary-level tests for the shell command surface.
//!
//! Every test runs the real `quantrule` binary against a checked-in fixture and
//! asserts on stdout, stderr, and exit status. No test reads `data/`.

use std::io::Write;
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
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1; stdout: {}; stderr: {}",
        stdout_text(output),
        stderr_text(output)
    );
    assert_eq!(stdout_text(output), "");
}

#[test]
fn single_candle_prints_exact_ndjson() {
    let path = fixture("single.csv");
    let output = run(&["candles", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/single.ndjson"));
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn three_candles_print_exact_ndjson_with_null_and_zero_volume() {
    let path = fixture("three.csv");
    let output = run(&["candles", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/three.ndjson"));
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn header_only_file_prints_nothing_and_succeeds() {
    let path = fixture("empty.csv");
    let output = run(&["candles", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), "");
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn over_window_prints_the_last_1000_candles() {
    let path = fixture("over_window.csv");
    let output = run(&["candles", &path]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/over_window.ndjson")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn interactive_and_non_interactive_output_are_byte_identical() {
    let path = fixture("three.csv");
    let expected = include_str!("fixtures/three.ndjson");

    let non_interactive = run(&["candles", &path]);
    assert_success(&non_interactive);
    assert_eq!(stdout_text(&non_interactive), expected);
    assert_eq!(stderr_text(&non_interactive), "");

    let interactive = run_interactive(&format!("candles {path}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), expected);
    assert!(
        stderr_text(&interactive).contains("quantrule> "),
        "prompt is missing from stderr: {}",
        stderr_text(&interactive)
    );
}

#[test]
fn repeated_runs_are_byte_identical() {
    let path = fixture("three.csv");
    let first = run(&["candles", &path]);
    let second = run(&["candles", &path]);

    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(first.status.code(), second.status.code());
}

#[test]
fn help_prints_the_command_reference() {
    let output = run(&["help"]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help.txt"));
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn interactive_help_matches_non_interactive_help() {
    let output = run_interactive("help\nquit\n");

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help.txt"));
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn fetch_without_arguments_fails_with_usage() {
    let output = run(&["fetch"]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("usage: fetch <symbol> <start> <end>"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn fetch_with_an_invalid_date_fails_before_any_network_access() {
    let output = run(&["fetch", "gold", "2025-02-30", "2025-12-31"]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("invalid date range"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn fetch_with_an_unknown_option_fails_without_stdout() {
    let output = run(&["fetch", "gold", "2025-01-01", "2025-12-31", "--bogus", "x"]);

    assert_failure(&output);
    assert_eq!(stderr_text(&output), "error: unknown option '--bogus'\n");
}

#[test]
fn fetch_with_an_unknown_timeframe_fails_before_any_network_access() {
    let output = run(&[
        "fetch",
        "gold",
        "2025-01-01",
        "2025-12-31",
        "--timeframe",
        "30m",
    ]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: --timeframe expects 1m, 5m, 15m, or 1h, found '30m'\n"
    );
    assert_eq!(stdout_text(&output), "");
}

#[test]
fn interactive_fetch_argument_error_keeps_the_session_alive() {
    let output = run_interactive("fetch\nhelp\nquit\n");

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help.txt"));
    assert!(
        stderr_text(&output).contains("usage: fetch <symbol> <start> <end>"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn quit_exits_without_output() {
    let output = run_interactive("quit\n");

    assert_success(&output);
    assert_eq!(stdout_text(&output), "");
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn missing_file_fails_without_stdout() {
    let path = fixture("missing.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).starts_with(&format!("error: cannot open {path}: ")),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn bad_header_fails_without_stdout() {
    let path = fixture("bad_header.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: invalid CSV header; expected Date,Open,High,Low,Close,Volume\n")
    );
}

#[test]
fn missing_header_fails_without_stdout() {
    let path = fixture("no_header.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: missing CSV header\n")
    );
}

#[test]
fn malformed_row_fails_without_partial_stdout() {
    let path = fixture("malformed.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 3: expected 6 fields, found 5\n")
    );
}

#[test]
fn duplicate_timestamp_fails_without_stdout() {
    let path = fixture("duplicate.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 3: duplicate timestamp at 2025-01-01T00:00:00Z\n")
    );
}

#[test]
fn invalid_candle_fails_without_stdout() {
    let path = fixture("invalid_candle.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 2: high is below low\n")
    );
}

#[test]
fn out_of_order_timestamp_fails_without_stdout() {
    let path = fixture("out_of_order.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: {path}: line 3: timestamp 2025-01-01T00:00:00Z is earlier than 2025-01-01T00:01:00Z\n"
        )
    );
}

#[test]
fn interactive_error_keeps_the_session_alive() {
    let missing = fixture("missing.csv");
    let output = run_interactive(&format!("candles {missing}\nhelp\nquit\n"));

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help.txt"));
    let stderr = stderr_text(&output);
    assert!(stderr.contains("error: cannot open"), "stderr: {stderr}");
    assert!(stderr.contains("quantrule> "), "stderr: {stderr}");
}

#[test]
fn unknown_command_fails_without_stdout() {
    let output = run(&["bogus"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: unknown command 'bogus'; type 'help'\n"
    );
}

#[test]
fn candles_without_a_path_fails_with_usage() {
    let output = run(&["candles"]);

    assert_failure(&output);
    assert_eq!(stderr_text(&output), "error: usage: candles <csv-path>\n");
}

#[test]
fn candles_with_extra_arguments_fails_with_usage() {
    let path = fixture("single.csv");
    let output = run(&["candles", &path, "extra"]);

    assert_failure(&output);
    assert_eq!(stderr_text(&output), "error: usage: candles <csv-path>\n");
}

#[test]
fn tolerant_mixed_prints_valid_candles_and_exact_report() {
    let path = fixture("tolerant_mixed.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/tolerant_mixed.ndjson")
    );
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/tolerant_mixed.report")
    );
}

#[test]
fn tolerant_all_invalid_succeeds_without_stdout() {
    let path = fixture("tolerant_all_invalid.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), "");
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/tolerant_all_invalid.report")
    );
}

#[test]
fn tolerant_header_only_reports_zero_counts() {
    let path = fixture("empty.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), "");
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/tolerant_empty.report")
    );
}

#[test]
fn tolerant_over_window_reports_eviction() {
    let path = fixture("over_window.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/over_window.ndjson")
    );
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/tolerant_evicted.report")
    );
}

#[test]
fn tolerant_repeated_runs_are_byte_identical() {
    let path = fixture("tolerant_mixed.csv");
    let first = run(&["candles-tolerant", &path]);
    let second = run(&["candles-tolerant", &path]);

    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(first.status.code(), second.status.code());
}

#[test]
fn tolerant_interactive_and_non_interactive_stdout_are_byte_identical() {
    let path = fixture("tolerant_mixed.csv");
    let expected = include_str!("fixtures/tolerant_mixed.ndjson");

    let non_interactive = run(&["candles-tolerant", &path]);
    assert_success(&non_interactive);
    assert_eq!(stdout_text(&non_interactive), expected);
    assert_eq!(
        stderr_text(&non_interactive),
        include_str!("fixtures/tolerant_mixed.report")
    );

    let interactive = run_interactive(&format!("candles-tolerant {path}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), expected);
    let stderr = stderr_text(&interactive);
    assert!(stderr.contains("quality rows_read=8"), "stderr: {stderr}");
    assert!(stderr.contains("skipped line=3"), "stderr: {stderr}");
    assert!(stderr.contains("quantrule> "), "stderr: {stderr}");
}

#[test]
fn strict_candles_still_fails_on_tolerant_mixed_file() {
    let path = fixture("tolerant_mixed.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 3: high is below low\n")
    );
}

#[test]
fn tolerant_bad_header_fails_without_stdout() {
    let path = fixture("bad_header.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: invalid CSV header; expected Date,Open,High,Low,Close,Volume\n")
    );
}

#[test]
fn tolerant_missing_header_fails_without_stdout() {
    let path = fixture("no_header.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: missing CSV header\n")
    );
}

#[test]
fn tolerant_missing_file_fails_without_stdout() {
    let path = fixture("missing.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).starts_with(&format!("error: cannot open {path}: ")),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn tolerant_candles_without_a_path_fails_with_usage() {
    let output = run(&["candles-tolerant"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: candles-tolerant <csv-path>\n"
    );
}

#[test]
fn tolerant_candles_with_extra_arguments_fails_with_usage() {
    let path = fixture("single.csv");
    let output = run(&["candles-tolerant", &path, "extra"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: candles-tolerant <csv-path>\n"
    );
}
