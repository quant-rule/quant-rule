//! Binary-level tests for the shell command surface.
//!
//! Every test runs the real `quantrule` binary against a checked-in fixture and
//! asserts on stdout, stderr, and exit status. No test reads `data/`.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use quantrule_data::format_utc_timestamp;
use rusqlite::Connection;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_str()
        .expect("fixture path is valid UTF-8")
        .to_owned()
}

/// The UTC open time of `2025-01-01 00:00:00`, shared by the generated series.
const GENERATED_EPOCH: i64 = 1_735_689_600;

/// Writes a generated M1 series under `target/` and returns its path.
///
/// The generator is deterministic and keeps the fixture out of Git. It is used
/// only for boundaries larger than the checked-in shell fixtures.
fn generated_csv(name: &str, header: &str, rows: impl IntoIterator<Item = String>) -> String {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/evidence-tests");
    fs::create_dir_all(&directory).expect("the evidence fixture directory is created");
    let path = directory.join(format!("{name}.csv"));
    let mut text = String::from(header);
    text.push('\n');
    for row in rows {
        text.push_str(&row);
        text.push('\n');
    }
    fs::write(&path, text).expect("the evidence fixture is written");
    path.to_str()
        .expect("the evidence fixture path is valid UTF-8")
        .to_owned()
}

/// One deterministic M1 row with enough movement for the model to publish events.
fn generated_row(index: usize) -> String {
    let timestamp = GENERATED_EPOCH + index as i64 * 60;
    let stamp = format_utc_timestamp(timestamp);
    let phase = index as f64;
    let open = 100.0 + 4.0 * (phase / 5.0).sin() + 0.8 * (phase / 2.0).sin();
    let close = 100.0 + 4.0 * ((phase + 1.0) / 5.0).sin() + 0.8 * ((phase + 1.0) / 2.0).sin();
    let high = open.max(close) + 0.4 + 0.2 * (phase / 3.0).sin().abs();
    let low = open.min(close) - 0.4 - 0.2 * (phase / 3.0).cos().abs();
    let volume = 1_000 + (index * 37) % 900;
    format!(
        "{} {}+00:00,{open:.5},{high:.5},{low:.5},{close:.5},{volume}",
        &stamp[..10],
        &stamp[11..19]
    )
}

/// Extends the checked-in 260-bar state fixture to `total` M1 bars.
fn extended_state_csv(name: &str, total: usize) -> String {
    let source =
        fs::read_to_string(fixture("state_m1.csv")).expect("the state fixture is readable");
    let mut lines = source.lines();
    let header = lines.next().expect("the state fixture has a header");
    let mut rows: Vec<String> = lines.map(str::to_owned).collect();
    assert_eq!(rows.len(), 260, "the state fixture is the expected width");
    assert!(
        total >= rows.len(),
        "the extension cannot shorten the fixture"
    );
    rows.extend((rows.len()..total).map(generated_row));
    generated_csv(name, header, rows)
}

/// Writes a fully synthetic M1 series with `total` bars.
fn synthetic_csv(name: &str, total: usize) -> String {
    generated_csv(
        name,
        "Date,Open,High,Low,Close,Volume",
        (0..total).map(generated_row),
    )
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
fn candles_reject_a_non_positive_price() {
    let path = fixture("zero_price.csv");
    let output = run(&["candles", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 2: open must be positive\n")
    );
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
fn help_detail_prints_one_command_contract() {
    let output = run(&["help", "lab", "summary"]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-lab-summary.txt")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn help_two_word_topic_prints_the_store_ingest_contract() {
    let output = run(&["help", "store", "ingest"]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-store-ingest.txt")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn help_all_prints_the_complete_reference() {
    let output = run(&["help", "all"]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help-all.txt"));
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn help_glossary_prints_the_definitions() {
    let output = run(&["help", "glossary"]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-glossary.txt")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn help_tutorial_points_at_the_tutorial() {
    let output = run(&["help", "tutorial"]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-tutorial.txt")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn help_roles_points_at_the_manuals() {
    let output = run(&["help", "roles"]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-roles.txt")
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn interactive_help_topic_matches_non_interactive() {
    let output = run_interactive("help store ingest\nquit\n");

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/help-store-ingest.txt")
    );
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn help_unknown_topic_points_to_all() {
    let output = run(&["help", "bogus"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: unknown help topic 'bogus'; type 'help all' for every command and reference topic\n"
    );
}

#[test]
fn interactive_unknown_help_topic_keeps_the_session_alive() {
    let output = run_interactive("help bogus\nhelp\nquit\n");

    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/help.txt"));
    assert!(
        stderr_text(&output).contains("unknown help topic 'bogus'"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn help_reserved_topic_rejects_extra_words() {
    let output = run(&["help", "all", "extra"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: help [<command>|all|glossary|tutorial|roles]\n"
    );
}

#[test]
fn help_three_words_are_rejected() {
    let output = run(&["help", "store", "ingest", "extra"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: help [<command>|all|glossary|tutorial|roles]\n"
    );
}

#[test]
fn help_output_has_no_ansi_escape() {
    let output = run(&["help", "all"]);

    assert_success(&output);
    assert!(!stdout_text(&output).contains('\u{1b}'));
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
fn timeframes_complete_m1_prints_all_five_series_and_exact_report() {
    let path = fixture("timeframes.csv");
    let output = run(&["timeframes", &path]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/timeframes.ndjson")
    );
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/timeframes.report.txt")
    );
}

#[test]
fn timeframes_reports_partial_buckets_and_gaps() {
    let path = fixture("timeframes-gaps.csv");
    let output = run(&["timeframes", &path]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/timeframes-gaps.ndjson")
    );
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/timeframes-gaps.report.txt")
    );
}

#[test]
fn timeframes_empty_input_reports_five_empty_series() {
    let path = fixture("empty.csv");
    let output = run(&["timeframes", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), "");
    assert_eq!(
        stderr_text(&output),
        "aggregation timeframe=1m retained=0 partial=0 gaps=0\n\
         aggregation timeframe=5m retained=0 partial=0 gaps=0\n\
         aggregation timeframe=15m retained=0 partial=0 gaps=0\n\
         aggregation timeframe=1h retained=0 partial=0 gaps=0\n\
         aggregation timeframe=4h retained=0 partial=0 gaps=0\n"
    );
}

#[test]
fn timeframes_repeated_runs_are_byte_identical() {
    let path = fixture("timeframes-gaps.csv");
    let first = run(&["timeframes", &path]);
    let second = run(&["timeframes", &path]);

    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
    assert_eq!(first.status.code(), second.status.code());
}

#[test]
fn timeframes_interactive_and_non_interactive_stdout_are_byte_identical() {
    let path = fixture("timeframes-gaps.csv");
    let expected = include_str!("fixtures/timeframes-gaps.ndjson");

    let non_interactive = run(&["timeframes", &path]);
    assert_success(&non_interactive);
    assert_eq!(stdout_text(&non_interactive), expected);
    assert_eq!(
        stderr_text(&non_interactive),
        include_str!("fixtures/timeframes-gaps.report.txt")
    );

    let interactive = run_interactive(&format!("timeframes {path}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), expected);
    let stderr = stderr_text(&interactive);
    assert!(
        stderr.contains("aggregation timeframe=5m retained=3 partial=1 gaps=1"),
        "stderr: {stderr}"
    );
    assert!(stderr.contains("quantrule> "), "stderr: {stderr}");
}

#[test]
fn timeframes_native_h1_fails_with_the_source_cadence_error() {
    let path = fixture("timeframes-h1.csv");
    let first = run(&["timeframes", &path]);
    let second = run(&["timeframes", &path]);

    assert_failure(&first);
    assert_eq!(
        stderr_text(&first),
        format!(
            "error: {path}: source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder\n"
        )
    );
    assert_eq!(first.stderr, second.stderr);
}

#[test]
fn timeframes_a_single_row_is_not_rejected_on_cadence() {
    let path = fixture("single.csv");
    let output = run(&["timeframes", &path]);

    assert_success(&output);
    assert_eq!(
        stderr_text(&output),
        "aggregation timeframe=1m retained=1 partial=0 gaps=0\n\
         aggregation timeframe=5m retained=0 partial=1 gaps=0\n\
         partial timeframe=5m bucket=2025-01-01T00:00:00Z present=1 expected=5\n\
         aggregation timeframe=15m retained=0 partial=1 gaps=0\n\
         partial timeframe=15m bucket=2025-01-01T00:00:00Z present=1 expected=15\n\
         aggregation timeframe=1h retained=0 partial=1 gaps=0\n\
         partial timeframe=1h bucket=2025-01-01T00:00:00Z present=1 expected=60\n\
         aggregation timeframe=4h retained=0 partial=1 gaps=0\n\
         partial timeframe=4h bucket=2025-01-01T00:00:00Z present=1 expected=240\n"
    );
}

#[test]
fn timeframes_misaligned_m1_fails_without_stdout() {
    let path = fixture("timeframes-misaligned.csv");
    let output = run(&["timeframes", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: {path}: line 2: M1 timestamp 2025-01-01T00:00:30Z is not aligned to a 60-second boundary\n"
        )
    );
}

#[test]
fn timeframes_invalid_candle_fails_without_stdout() {
    let path = fixture("invalid_candle.csv");
    let output = run(&["timeframes", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 2: high is below low\n")
    );
}

#[test]
fn timeframes_reject_a_non_positive_price() {
    let path = fixture("zero_price.csv");
    let output = run(&["timeframes", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 2: open must be positive\n")
    );
}

#[test]
fn timeframes_duplicate_timestamp_fails_without_stdout() {
    let path = fixture("duplicate.csv");
    let output = run(&["timeframes", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!("error: {path}: line 3: duplicate timestamp at 2025-01-01T00:00:00Z\n")
    );
}

#[test]
fn timeframes_without_a_path_fails_with_usage() {
    let output = run(&["timeframes"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: timeframes <csv-path>\n"
    );
}

#[test]
fn timeframes_with_extra_arguments_fails_with_usage() {
    let path = fixture("timeframes.csv");
    let output = run(&["timeframes", &path, "extra"]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        "error: usage: timeframes <csv-path>\n"
    );
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
fn tolerant_duplicate_ordering_wins_over_candle_validity() {
    let path = fixture("tolerant_invalid_duplicate.csv");
    let output = run(&["candles-tolerant", &path]);

    assert_success(&output);
    assert_eq!(stdout_text(&output).lines().count(), 1);
    assert_eq!(
        stderr_text(&output),
        include_str!("fixtures/tolerant_invalid_duplicate.report")
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

#[test]
fn market_state_prints_the_four_timeframe_ladder() {
    let path = fixture("state_m1.csv");
    let output = run(&["market-state", &path]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout_text(&output).trim()).expect("the market state is JSON");

    assert_eq!(value["ladder"].as_array().expect("ladder").len(), 4);
    assert_eq!(value["timeframes"].as_array().expect("timeframes").len(), 4);
    assert_eq!(value["consensus"].as_array().expect("consensus").len(), 4);
    assert_eq!(value["timeframes"][0]["timeframe"], "1m");
    assert!(value["timeframes"][0]["series_index"].is_number());
    assert!(value["timeframes"][0]["datetime"].is_string());
    assert!(value["timeframes"][0]["window_start"].is_number());
    assert!(value["timeframes"][0]["window_len"].is_number());
    assert!(value["aggregates"]["direction"].is_object());
    assert!(value["aggregates"]["events"]["per_timeframe"].is_array());
    assert!(value["timeframes"][0]["consensus"]["bullish"].is_number());
}

#[test]
fn market_state_rejects_a_native_h1_source() {
    let path = fixture("timeframes-h1.csv");
    let output = run(&["market-state", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: {path}: source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder\n"
        )
    );
}

#[test]
fn market_state_uses_the_full_series_by_default_and_the_tail_on_request() {
    let path = fixture("over_window.csv");

    let full = run(&["market-state", &path]);
    assert_success(&full);
    let full_value: serde_json::Value =
        serde_json::from_str(stdout_text(&full).trim()).expect("the market state is JSON");
    assert_eq!(full_value["timeframes"][0]["bar"], 1000);

    let latest = run(&["market-state", &path, "--window", "latest"]);
    assert_success(&latest);
    let latest_value: serde_json::Value =
        serde_json::from_str(stdout_text(&latest).trim()).expect("the market state is JSON");
    assert_eq!(latest_value["timeframes"][0]["bar"], 999);

    let interactive = run_interactive(&format!("market-state {path} --window latest\nquit\n"));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), stdout_text(&latest));
}

#[test]
fn market_state_output_is_deterministic() {
    let path = fixture("state_m1.csv");
    let first = run(&["market-state", &path]);
    let second = run(&["market-state", &path]);
    assert_success(&first);
    assert_success(&second);
    assert_eq!(stdout_text(&first), stdout_text(&second));
}

#[test]
fn interactive_market_state_matches_non_interactive() {
    let path = fixture("state_m1.csv");
    let non_interactive = run(&["market-state", &path]);
    let interactive = run_interactive(&format!("market-state {path}\nquit\n"));
    assert_success(&non_interactive);
    assert_success(&interactive);
    assert_eq!(stdout_text(&non_interactive), stdout_text(&interactive));
}

#[test]
fn market_dashboard_prints_the_exact_text_golden() {
    let path = fixture("state_m1.csv");
    let output = run(&["market-dashboard", &path]);
    assert_success(&output);
    assert_eq!(stdout_text(&output), include_str!("fixtures/dashboard.txt"));
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn market_dashboard_json_is_the_exact_numeric_golden() {
    let path = fixture("state_m1.csv");
    let output = run(&["market-dashboard", &path, "--json"]);
    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        include_str!("fixtures/dashboard.json")
    );

    let value: serde_json::Value =
        serde_json::from_str(stdout_text(&output).trim()).expect("the dashboard is JSON");
    assert_eq!(value["view_contract"], "2026-10-25");
    assert_eq!(
        value["dashboard_state"]["ladder"],
        serde_json::json!([60, 300, 900, 3600])
    );
    let timeframes = value["dashboard_state"]["timeframes"]
        .as_array()
        .expect("timeframes is a list");
    assert_eq!(timeframes.len(), 4);
    let axes = timeframes[0]["axes"].as_array().expect("axes is a list");
    assert_eq!(axes.len(), 4);
    assert_eq!(axes[0]["axis"], "direction");
    assert_eq!(axes[0]["position"]["kind"], "signed");
    assert_eq!(axes[0]["size"]["kind"], "rank");
    assert!(axes[0]["detail"]["bias_state"].is_string());
    assert!(value["dashboard_state"]["cross_tf_state"]["aggregates"]["direction"].is_object());
    assert!(value["dashboard_state"]["cross_tf_state"]["agreement"]["axes"].is_array());
    assert!(value["dashboard_state"]["cross_tf_state"]["churn"]["churn_class"].is_string());
}

#[test]
fn market_dashboard_is_deterministic_and_interactive_matches() {
    let path = fixture("state_m1.csv");

    let text = run(&["market-dashboard", &path]);
    let text_again = run(&["market-dashboard", &path]);
    let json = run(&["market-dashboard", &path, "--json"]);
    let json_again = run(&["market-dashboard", &path, "--json"]);
    assert_success(&text);
    assert_success(&text_again);
    assert_success(&json);
    assert_success(&json_again);
    assert_eq!(stdout_text(&text), stdout_text(&text_again));
    assert_eq!(stdout_text(&json), stdout_text(&json_again));

    let interactive_text = run_interactive(&format!("market-dashboard {path}\nquit\n"));
    let interactive_json = run_interactive(&format!("market-dashboard {path} --json\nquit\n"));
    assert_success(&interactive_text);
    assert_success(&interactive_json);
    assert_eq!(stdout_text(&text), stdout_text(&interactive_text));
    assert_eq!(stdout_text(&json), stdout_text(&interactive_json));
}

#[test]
fn market_dashboard_prints_absent_timeframes_without_a_zero() {
    let path = fixture("single.csv");
    let output = run(&["market-dashboard", &path]);
    assert_success(&output);
    let text = stdout_text(&output);
    assert_eq!(text.matches("— no closed bar").count(), 6);
    assert!(!text.contains(" 0%"));
    assert!(text.contains("legend"));

    let empty = run(&["market-dashboard", &fixture("empty.csv")]);
    assert_success(&empty);
    let empty_text = stdout_text(&empty);
    assert_eq!(empty_text.matches("— no closed bar").count(), 8);
    assert!(!empty_text.contains('─'), "an empty input drew a track");

    let empty_json = run(&["market-dashboard", &fixture("empty.csv"), "--json"]);
    assert_success(&empty_json);
    let value: serde_json::Value =
        serde_json::from_str(stdout_text(&empty_json).trim()).expect("the empty dashboard is JSON");
    let timeframes = value["dashboard_state"]["timeframes"]
        .as_array()
        .expect("timeframes is a list");
    assert_eq!(timeframes.len(), 4);
    for timeframe in timeframes {
        assert_eq!(timeframe["present"], false);
        let axes = timeframe["axes"].as_array().expect("axes is a list");
        assert_eq!(axes.len(), 4);
        for axis in axes {
            assert_eq!(axis["position"]["kind"], "undefined");
            assert!(axis["position"]["reason_code"].is_u64());
            assert_eq!(axis["size"]["kind"], "undefined");
        }
    }
}

#[test]
fn market_dashboard_window_option_selects_the_input() {
    let path = extended_state_csv("dashboard-window", 1_100);

    let full = run(&["market-dashboard", &path, "--json"]);
    let latest = run(&["market-dashboard", &path, "--json", "--window", "latest"]);
    assert_success(&full);
    assert_success(&latest);
    let full_value: serde_json::Value =
        serde_json::from_str(stdout_text(&full).trim()).expect("the full dashboard is JSON");
    let latest_value: serde_json::Value =
        serde_json::from_str(stdout_text(&latest).trim()).expect("the latest dashboard is JSON");
    assert_eq!(
        full_value["dashboard_state"]["timeframes"][0]["window_len"],
        1_100
    );
    assert_eq!(
        latest_value["dashboard_state"]["timeframes"][0]["window_len"],
        1_000
    );

    let full_text = run(&["market-dashboard", &path]);
    let latest_text = run(&["market-dashboard", &path, "--window", "latest"]);
    assert_success(&full_text);
    assert_success(&latest_text);
    assert!(stdout_text(&full_text).contains("window=full"));
    assert!(stdout_text(&latest_text).contains("window=latest"));
}

#[test]
fn market_dashboard_refuses_bad_arguments() {
    let path = fixture("state_m1.csv");

    let missing = run(&["market-dashboard"]);
    assert_failure(&missing);
    assert!(stderr_text(&missing).contains("missing CSV path"));

    let bad_window = run(&["market-dashboard", &path, "--window", "bogus"]);
    assert_failure(&bad_window);
    assert!(stderr_text(&bad_window).contains("--window expects full or latest"));

    let repeated_json = run(&["market-dashboard", &path, "--json", "--json"]);
    assert_failure(&repeated_json);
    assert!(stderr_text(&repeated_json).contains("--json was supplied twice"));

    let unknown = run(&["market-dashboard", &path, "--bogus"]);
    assert_failure(&unknown);
    assert!(stderr_text(&unknown).contains("unknown option '--bogus'"));

    let second_path = run(&["market-dashboard", &path, &path]);
    assert_failure(&second_path);
    assert!(stderr_text(&second_path).contains("unexpected argument"));
}

#[test]
fn market_dashboard_has_no_strategy_vocabulary() {
    let path = fixture("state_m1.csv");
    let text = stdout_text(&run(&["market-dashboard", &path])).to_lowercase();
    let json = stdout_text(&run(&["market-dashboard", &path, "--json"])).to_lowercase();
    for forbidden in [
        "coiling",
        "ignition",
        "exhaust",
        "confidence",
        "probability",
        "recommend",
        "strategy",
    ] {
        assert!(!text.contains(forbidden), "text contains {forbidden}");
        assert!(!json.contains(forbidden), "JSON contains {forbidden}");
    }
}

#[test]
fn axis_prints_one_labeled_line_per_timeframe() {
    let path = fixture("state_m1.csv");
    let output = run(&["axis", &path, "structure"]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4);
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line).expect("axis JSON");
        assert_eq!(value["axis"], "structure");
        assert!(value["timeframe"].is_string());
        assert!(value["readiness_label"].is_string());
    }
}

#[test]
fn axis_rejects_an_unknown_axis() {
    let path = fixture("state_m1.csv");
    let output = run(&["axis", &path, "bogus"]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("unknown axis"));
}

#[test]
fn events_print_labeled_records() {
    let path = fixture("state_m1.csv");
    let output = run(&["events", &path]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert!(!lines.is_empty());
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line).expect("event JSON");
        assert!(value["type_label"].is_string());
        assert!(value["subtype_label"].is_string());
        assert!(value["source_label"].is_string());
        assert!(value["timeframe"].is_string());
        assert!(value["occurred_time"].is_string());
        assert!(value["available_time"].is_string());
    }
}

#[test]
fn a_long_series_keeps_an_early_event_and_reports_the_true_tail() {
    let path = extended_state_csv("state-1500", 1_500);

    let full = run(&["market-state", &path]);
    assert_success(&full);
    let full_value: serde_json::Value =
        serde_json::from_str(stdout_text(&full).trim()).expect("the full market state is JSON");
    assert_eq!(full_value["timeframes"][0]["bar"], 1499);

    let latest = run(&["market-state", &path, "--window", "latest"]);
    assert_success(&latest);
    let latest_value: serde_json::Value =
        serde_json::from_str(stdout_text(&latest).trim()).expect("the latest market state is JSON");
    assert_eq!(latest_value["timeframes"][0]["bar"], 999);

    let events = run(&["events", &path]);
    assert_success(&events);
    let early = "2025-01-01T00:14:00Z";
    let has_early_event = |text: &str| {
        text.lines().any(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("event JSON");
            value["timeframe"] == "1m" && value["occurred_time"] == early
        })
    };
    assert!(
        has_early_event(&stdout_text(&events)),
        "the full run lost an event from before the last 1,000 bars"
    );

    let tail_events = run(&["events", &path, "--window", "latest"]);
    assert_success(&tail_events);
    assert!(
        !has_early_event(&stdout_text(&tail_events)),
        "--window latest unexpectedly saw an event outside its input"
    );
}

#[test]
fn model_series_returns_the_last_1000_bars_of_a_1200_bar_run() {
    let path = extended_state_csv("state-1200", 1_200);
    let output = run(&["model-series", &path, "--timeframe", "1m", "--bars", "1000"]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1_000);

    let first: serde_json::Value = serde_json::from_str(lines[0]).expect("first model bar");
    let last: serde_json::Value = serde_json::from_str(lines[999]).expect("last model bar");
    assert_eq!(first["bar"], 200);
    assert_eq!(first["window_start"], 200);
    assert_eq!(first["window_len"], 1_200);
    assert_eq!(last["bar"], 1_199);
    assert_eq!(last["series_index"], 1_199);
}

#[test]
fn h1_reaches_ema200_with_enough_history() {
    let path = synthetic_csv("h1-12000", 12_000);
    let output = run(&["model-series", &path, "--timeframe", "1h", "--bars", "1"]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    let value: serde_json::Value = serde_json::from_str(lines[0]).expect("the H1 model bar");
    assert_eq!(value["timeframe"], "1h");
    assert_eq!(value["bar"], 199);
    assert_eq!(value["window_start"], 199);
    assert_eq!(value["window_len"], 200);
    assert!(
        value["readings"]["ema200"].is_number(),
        "H1 did not reach EMA 200 with 12,000 M1 bars"
    );
}

#[test]
fn model_series_prints_h4_from_a_short_source() {
    let path = fixture("state_m1.csv");
    let output = run(&["model-series", &path, "--timeframe", "4h", "--bars", "5"]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    let value: serde_json::Value = serde_json::from_str(lines[0]).expect("the H4 model bar");
    assert_eq!(value["timeframe"], "4h");
    assert_eq!(value["bar"], 0);
    assert_eq!(value["window_len"], 1);
}

#[test]
fn model_series_prints_the_requested_tail() {
    let path = fixture("state_m1.csv");
    let output = run(&["model-series", &path, "--timeframe", "1m", "--bars", "3"]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3);

    let first: serde_json::Value = serde_json::from_str(lines[0]).expect("model-series JSON");
    let last: serde_json::Value = serde_json::from_str(lines[2]).expect("model-series JSON");
    assert_eq!(first["timeframe"], "1m");
    assert_eq!(first["bar"], 257);
    assert_eq!(last["bar"], 259);
    assert_eq!(last["series_index"], 259);
    assert_eq!(first["window_start"], 257);
    assert_eq!(first["window_len"], 260);
    assert!(last["readings"]["ema20"].is_number());
    assert!(last["direction"]["value"].is_number());
    assert!(last["consensus"]["bullish"].is_number());

    let interactive = run_interactive(&format!(
        "model-series {path} --timeframe 1m --bars 3\nquit\n"
    ));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), stdout_text(&output));
}

#[test]
fn legs_print_both_scales_per_timeframe() {
    let path = fixture("state_m1.csv");
    let output = run(&["legs", &path]);
    assert_success(&output);
    let text = stdout_text(&output);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 8);
    for line in lines {
        let value: serde_json::Value = serde_json::from_str(line).expect("leg JSON");
        assert!(value["timeframe"].is_string());
        assert!(value["scale"].is_string());
        assert!(value.get("leg").is_some());
    }
}

#[test]
fn help_lists_the_market_state_commands() {
    let output = run(&["help"]);
    assert_success(&output);
    let text = stdout_text(&output);
    for command in ["market-state", "axis", "events", "legs"] {
        assert!(text.contains(command), "help omits {command}");
    }
}

fn artifact_dir(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/research-tests")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the artifact directory is created");
    path.to_str()
        .expect("the artifact path is UTF-8")
        .to_string()
}

#[test]
fn research_export_writes_the_dataset_and_repeats_byte_for_byte() {
    let path = fixture("state_m1.csv");
    let first = artifact_dir("export-a");
    let second = artifact_dir("export-b");

    let output = run(&[
        "research",
        "export",
        &path,
        "demo",
        "--data-version",
        "test:demo:60:0-260",
        "--root",
        &first,
    ]);
    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "exported demo\n  data version: test:demo:60:0-260\n  raw rows: 260\n  layer1 rows: 333\n  layer2 rows: 333\n  layer3 rows: 260\n  files: 9\n"
    );

    let output = run(&[
        "research",
        "export",
        &path,
        "demo",
        "--data-version",
        "test:demo:60:0-260",
        "--root",
        &second,
    ]);
    assert_success(&output);

    let read = |root: &str, name: &str| {
        fs::read(Path::new(root).join("processed/demo").join(name)).expect("the file is written")
    };
    for name in [
        "layer1.csv",
        "layer2.csv",
        "layer3.csv",
        "data_dictionary.csv",
        "data_dictionary.md",
        "codes.csv",
        "README.md",
    ] {
        assert_eq!(read(&first, name), read(&second, name), "{name} differs");
    }
    assert_eq!(
        fs::read(Path::new(&first).join("raw/demo/m1.csv")).expect("the raw file is written"),
        fs::read(Path::new(&second).join("raw/demo/m1.csv")).expect("the raw file is written"),
    );

    let layer3 = fs::read_to_string(Path::new(&first).join("processed/demo/layer3.csv"))
        .expect("the layer 3 file");
    let mut lines = layer3.lines();
    let header = lines.next().expect("a header");
    assert_eq!(header.split(',').count(), 356);
    assert_eq!(lines.count(), 260);

    let manifest = fs::read_to_string(Path::new(&first).join("processed/demo/manifest.json"))
        .expect("the manifest");
    assert!(manifest.contains("\"data_version\": \"test:demo:60:0-260\""));
    assert!(manifest.contains("\"model_version\": \"0.3.0\""));
    assert!(
        manifest.contains(&format!(
            "\"model_version\": \"{}\"",
            env!("CARGO_PKG_VERSION")
        )),
        "the manifest version must match the crate version"
    );
    assert!(manifest.contains("\"layer3\": 260"));
}

#[test]
fn research_export_requires_a_data_version() {
    let path = fixture("state_m1.csv");
    let output = run(&["research", "export", &path, "demo"]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("missing --data-version"));
}

#[test]
fn research_export_rejects_an_unsafe_dataset_name() {
    let path = fixture("state_m1.csv");
    let root = artifact_dir("export-invalid");
    let output = run(&[
        "research",
        "export",
        &path,
        "..",
        "--data-version",
        "test:demo:60:0-260",
        "--root",
        &root,
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("dataset name"));
}

#[test]
fn research_summary_rejects_a_native_h1_source() {
    let path = fixture("timeframes-h1.csv");
    let output = run(&["research", "summary", &path]);

    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: {path}: source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder\n"
        )
    );
}

#[test]
fn research_summary_is_deterministic_and_counts_every_axis() {
    let path = fixture("state_m1.csv");
    let first = run(&["research", "summary", &path]);
    let second = run(&["research", "summary", &path]);
    assert_success(&first);
    assert_success(&second);
    let text = stdout_text(&first);
    assert_eq!(text, stdout_text(&second));
    assert!(text.starts_with("summary layer1=333 layer2=333 layer3=260\n"));
    assert_eq!(text.lines().count(), 41);
    assert!(!text.contains("composite"));

    let direction = text
        .lines()
        .find(|line| line.starts_with("axis timeframe=60 axis=direction "))
        .expect("the M1 direction line");
    let value = |name: &str| {
        direction
            .split_whitespace()
            .find_map(|field| field.strip_prefix(name))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("the field")
    };
    assert_eq!(value("n=") + value("null="), 260);

    assert!(text.contains("ordinals="));
    assert!(text.contains("relation timeframe=60 left=direction right=momentum "));
    let relation = text
        .lines()
        .find(|line| line.starts_with("relation timeframe=60 left=direction right=momentum "))
        .expect("the pair line");
    assert!(relation.contains("r="));

    let output = run(&["research", "summary"]);
    assert_failure(&output);
}

#[test]
fn store_init_creates_and_reopens_the_store() {
    let root = artifact_dir("store-init");
    let db = format!("{root}/store.db");
    let expected = format!("store {db} schema=2\n");

    let output = run(&["store", "init", "--db", &db]);
    assert_success(&output);
    assert_eq!(stdout_text(&output), expected);
    assert!(Path::new(&db).exists());

    let output = run(&["store", "init", "--db", &db]);
    assert_success(&output);
    assert_eq!(stdout_text(&output), expected);
}

#[test]
fn store_init_rejects_a_missing_db_value() {
    let output = run(&["store", "init", "--db"]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("--db needs a value"));
}

fn scalar(connection: &Connection, sql: &str) -> i64 {
    connection
        .query_row(sql, [], |row| row.get(0))
        .expect("the scalar query succeeds")
}

fn assert_record_integrity(connection: &Connection) {
    let pivots = scalar(connection, "SELECT count(*) FROM pivot");
    let confirmed = scalar(
        connection,
        "SELECT count(*) FROM pivot WHERE confirmed_code = 1",
    );
    let candidates = scalar(
        connection,
        "SELECT count(*) FROM pivot WHERE confirmed_code = 0",
    );
    let levels = scalar(connection, "SELECT count(*) FROM level");
    assert!(pivots > 0, "the fixture stores no pivots");
    assert!(confirmed > 0, "the fixture stores no confirmed pivots");
    assert!(candidates > 0, "the fixture stores no candidates");
    assert!(levels > 0, "the fixture stores no levels");
    assert_eq!(confirmed, levels, "one level per confirmed pivot");

    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM pivot \
             WHERE available_bar_index < occurred_bar_index \
             OR (confirmed_code = 0 AND available_bar_index != occurred_bar_index) \
             OR (confirmed_code = 1 AND available_bar_index <= occurred_bar_index)"
        ),
        0,
        "a pivot availability violation"
    );
    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM level WHERE available_bar_index < occurred_bar_index"
        ),
        0,
        "a level availability violation"
    );
    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM pivot \
             WHERE scale_code NOT IN (1, 2) OR kind_code NOT IN (1, 2) \
             OR confirmed_code NOT IN (0, 1)"
        ),
        0,
        "an invalid pivot code"
    );
    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM level \
             WHERE reference_code < 1001 OR scale_code NOT IN (1, 2) OR kind_code NOT IN (1, 2)"
        ),
        0,
        "an invalid level code"
    );
    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM (\
                 SELECT timeframe_secs, min(reference_code) AS first, \
                        max(reference_code) AS last, count(*) AS n \
                 FROM level GROUP BY timeframe_secs\
             ) WHERE first != 1001 OR last != 1000 + n"
        ),
        0,
        "a level reference gap"
    );
}

fn assert_resolution_integrity(connection: &Connection) {
    for axis in ["direction", "momentum", "structure", "progression"] {
        let continuous = format!("{axis}_continuous");
        let ordinal = format!("{axis}_ordinal");
        let reason = format!("{axis}_resolution_reason_code");
        assert_eq!(
            scalar(
                connection,
                &format!(
                    "SELECT count(*) FROM bar WHERE \
                     ({continuous} IS NULL AND ({ordinal} IS NOT NULL OR {reason} IS NULL \
                       OR {reason} NOT BETWEEN 1 AND 6)) \
                     OR ({continuous} IS NOT NULL AND ({continuous} < -1.0 \
                       OR {continuous} > 1.0 OR {ordinal} IS NULL \
                       OR {ordinal} NOT BETWEEN 0 AND 9 OR {reason} IS NOT NULL \
                       OR {ordinal} != (({continuous} >= -0.8) + ({continuous} >= -0.6) \
                           + ({continuous} >= -0.4) + ({continuous} >= -0.2) \
                           + ({continuous} >= 0.0) + ({continuous} >= 0.2) \
                           + ({continuous} >= 0.4) + ({continuous} >= 0.6) \
                           + ({continuous} >= 0.8))))"
                ),
            ),
            0,
            "a {axis} resolution violation"
        );
        assert!(
            scalar(
                connection,
                &format!("SELECT count(*) FROM bar WHERE {continuous} IS NOT NULL")
            ) > 0,
            "the fixture stores no {axis} resolution"
        );
    }

    assert_eq!(
        scalar(
            connection,
            "SELECT count(*) FROM bar WHERE \
             (progress IS NULL AND progression_continuous IS NOT NULL) \
             OR (progress IS NOT NULL AND progression_continuous IS NULL) \
             OR (progress IS NOT NULL AND progression_continuous \
                 != max(-1.0, min(1.0, progress)))"
        ),
        0,
        "a progression clamp violation"
    );
}

#[test]
fn store_ingest_writes_every_ladder_bar_and_repeats_as_a_no_op() {
    let root = artifact_dir("store-ingest");
    let db = format!("{root}/store.db");
    let path = fixture("state_m1.csv");

    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        "test:demo:60:0-260",
        "--db",
        &db,
    ]);
    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "ingested demo bars=333 timeframes=4\n"
    );

    let connection = Connection::open(&db).expect("the store opens");
    assert_eq!(scalar(&connection, "SELECT count(*) FROM dataset"), 1);
    assert_eq!(scalar(&connection, "SELECT count(*) FROM bar"), 333);
    assert_record_integrity(&connection);
    let record_counts = || {
        (
            scalar(&connection, "SELECT count(*) FROM pivot"),
            scalar(&connection, "SELECT count(*) FROM level"),
        )
    };
    let before = record_counts();
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
            "SELECT count(DISTINCT timeframe_secs) FROM bar"
        ),
        4
    );
    assert_eq!(
        scalar(
            &connection,
            "SELECT layer3_materialized FROM dataset WHERE name = 'demo'"
        ),
        1
    );
    assert!(
        scalar(
            &connection,
            "SELECT count(*) FROM layer3 WHERE m5_bar_index IS NOT NULL"
        ) > 0,
        "the M5 block is not materialized"
    );
    assert_resolution_integrity(&connection);

    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        "test:demo:60:0-260",
        "--db",
        &db,
    ]);
    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "unchanged demo bars=333 timeframes=4\n"
    );
    assert_eq!(scalar(&connection, "SELECT count(*) FROM dataset"), 1);
    assert_eq!(scalar(&connection, "SELECT count(*) FROM bar"), 333);
    assert_eq!(record_counts(), before, "the no-op changed the records");
    assert_record_integrity(&connection);
}

#[test]
fn event_batch_and_store_keep_more_than_the_bounded_history() {
    let root = artifact_dir("event-retention");
    let db = format!("{root}/store.db");
    let path = synthetic_csv("event-retention-3000", 3_000);

    let events = run(&["events", &path]);
    assert_success(&events);
    let m1_events = stdout_text(&events)
        .lines()
        .filter(|line| {
            let value: serde_json::Value = serde_json::from_str(line).expect("event JSON");
            value["timeframe"] == "1m"
        })
        .count();
    assert!(
        m1_events > 1_024,
        "the fixture produced only {m1_events} M1 events"
    );

    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "events",
        "--data-version",
        "test:events:60:0-3000",
        "--db",
        &db,
    ]);
    assert_success(&output);

    let connection = Connection::open(&db).expect("the store opens");
    assert_eq!(
        scalar(
            &connection,
            "SELECT count(*) FROM event WHERE timeframe_secs = 60"
        ),
        m1_events as i64,
        "the store did not retain the full M1 event batch"
    );
}

#[test]
fn store_ingest_rejects_a_conflicting_identity_and_changes_nothing() {
    let root = artifact_dir("store-ingest-conflict");
    let db = format!("{root}/store.db");
    let path = fixture("state_m1.csv");
    let ingest = |version: &str| {
        run(&[
            "store",
            "ingest",
            &path,
            "--dataset",
            "demo",
            "--data-version",
            version,
            "--db",
            &db,
        ])
    };

    let output = ingest("test:demo:60:0-260");
    assert_success(&output);

    let connection = Connection::open(&db).expect("the store opens");
    assert_record_integrity(&connection);
    let record_counts = || {
        (
            scalar(&connection, "SELECT count(*) FROM pivot"),
            scalar(&connection, "SELECT count(*) FROM level"),
        )
    };
    let before = record_counts();

    let output = ingest("test:demo:60:1-260");
    assert_failure(&output);
    assert!(stderr_text(&output).contains("different identity"));

    assert_eq!(scalar(&connection, "SELECT count(*) FROM dataset"), 1);
    assert_eq!(scalar(&connection, "SELECT count(*) FROM bar"), 333);
    assert_eq!(record_counts(), before, "the conflict changed the records");
    assert_record_integrity(&connection);
    let stored: String = connection
        .query_row(
            "SELECT data_version FROM dataset WHERE name = 'demo'",
            [],
            |row| row.get(0),
        )
        .expect("the identity is read");
    assert_eq!(stored, "test:demo:60:0-260");
}

#[test]
fn store_ingest_rejects_an_invalid_source_without_creating_the_database() {
    let root = artifact_dir("store-ingest-invalid");
    let db = format!("{root}/store.db");
    let path = fixture("invalid_candle.csv");

    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        "test:demo:60:0-1",
        "--db",
        &db,
    ]);
    assert_failure(&output);
    assert!(!Path::new(&db).exists(), "a failed source wrote a database");
}

#[test]
fn store_ingest_rejects_a_native_h1_source_without_creating_the_database() {
    let root = artifact_dir("store-ingest-h1");
    let db = format!("{root}/store.db");
    let path = fixture("timeframes-h1.csv");

    let output = run(&[
        "store",
        "ingest",
        &path,
        "--dataset",
        "demo",
        "--data-version",
        "test:demo:60:0-6",
        "--db",
        &db,
    ]);
    assert_failure(&output);
    assert_eq!(
        stderr_text(&output),
        format!(
            "error: {path}: source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder\n"
        )
    );
    assert!(
        !Path::new(&db).exists(),
        "a rejected source wrote a database"
    );
}

#[test]
fn interactive_store_ingest_matches_non_interactive_output() {
    let root = artifact_dir("store-ingest-interactive");
    let db = format!("{root}/store.db");
    let path = fixture("state_m1.csv");
    let input = format!(
        "store ingest {path} --dataset demo --data-version test:demo:60:0-260 --db {db}\nquit\n"
    );

    let output = run_interactive(&input);
    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "ingested demo bars=333 timeframes=4\n"
    );
    assert!(stderr_text(&output).contains("quantrule> "));
}

#[test]
fn policy_check_prints_the_validated_envelope() {
    let output = run(&[
        "policy",
        "check",
        "--config",
        &fixture("config-run/run.json"),
    ]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "policy ok run=fixture account=USD 10000 target=0.1 365d max_drawdown=0.2 risk=1 leverage=1 positions=1 tickets=EURUSD timeframes=5m,15m strategy=ladder-context execution=5m ladder=5m,15m stop_atr=2 reward_to_risk=1.5\n"
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn strategy_explain_prints_the_trader_rule() {
    let output = run(&[
        "strategy",
        "explain",
        "--config",
        &fixture("config-run/run.json"),
    ]);

    assert_success(&output);
    assert_eq!(
        stdout_text(&output),
        "strategy ladder-context\nexecution 5m\ncontext 15m\nthesis Trade with the higher-timeframe regime after a lower-timeframe structure shift.\nentry-long direction > 0.1, momentum > 0.1, structure >= 0, shift <= 0 -> >= 1\nentry-short direction < -0.1, momentum < -0.1, structure <= 0, shift >= 0 -> <= -1\ncontext-long direction share >= 0.5, momentum bearish share <= 0.5, conflicts <= 1, progression bullish, late/terminal <= 1\ncontext-short direction share >= 0.5, momentum bullish share <= 0.5, conflicts <= 1, progression bearish, late/terminal <= 1\nexit context_lost,structure_lost\nrisk stop=2 ATR target=3 ATR reward_to_risk=1.5\n"
    );
    assert_eq!(stderr_text(&output), "");
}

#[test]
fn policy_check_rejects_a_cross_config_contradiction() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/policy-contradiction");
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("the test directory is created");
    for name in [
        "policy.json",
        "data.json",
        "model.json",
        "strategy.json",
        "execution.json",
        "portfolio.json",
        "analytics.json",
    ] {
        fs::copy(fixture(&format!("config-run/{name}")), directory.join(name))
            .expect("the fixture is copied");
    }
    let mut portfolio = fs::read_to_string(fixture("config-run/portfolio.json"))
        .expect("the portfolio fixture is readable");
    portfolio = portfolio.replace("\"notional\": 10000.0", "\"notional\": 20000.0");
    fs::write(directory.join("portfolio.json"), portfolio).expect("the contradiction is written");
    fs::write(
        directory.join("run.json"),
        "{\n  \"schema\": \"quantrule.run/v2\",\n  \"name\": \"contradiction\",\n  \"policy\": \"policy.json\",\n  \"data\": \"data.json\",\n  \"model\": \"model.json\",\n  \"strategy\": \"strategy.json\",\n  \"execution\": \"execution.json\",\n  \"portfolio\": \"portfolio.json\",\n  \"analytics\": \"analytics.json\",\n  \"output_root\": \"out\"\n}\n",
    )
    .expect("the run file is written");

    let output = run(&[
        "policy",
        "check",
        "--config",
        directory
            .join("run.json")
            .to_str()
            .expect("the path is UTF-8"),
    ]);

    assert_failure(&output);
    assert!(stderr_text(&output).contains("$.sizing.notional"));
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn strategy_run_uses_the_config_strategy() {
    let output = run(&[
        "strategy-run",
        &fixture("state_m1.csv"),
        "--strategy",
        "ladder-context",
        "--data-version",
        "test:v1",
        "--config",
        &fixture("config-run/run.json"),
    ]);

    assert_success(&output);
    assert_eq!(stderr_text(&output), "");
}
