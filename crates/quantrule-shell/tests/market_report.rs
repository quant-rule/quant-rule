//! Binary-level tests for the `market-report` command.
//!
//! The tests run the real binary against the checked-in M1 fixture and assert
//! the artifact set, the exact text/JSON/PNG goldens, determinism, interactive
//! parity, date filtering, and the failure behavior that keeps the report
//! descriptive.

use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use quantrule_data::format_utc_timestamp;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const DATA_VERSION: &str = "test:v1";
const ARTIFACTS: [&str; 4] = ["summary.txt", "report.json", "price-axes.png", "events.png"];

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_str()
        .expect("fixture path is valid UTF-8")
        .to_owned()
}

fn report_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/report-tests")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    path
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

fn png_info(bytes: &[u8]) -> (u32, u32) {
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let reader = decoder.read_info().expect("the PNG header is valid");
    (reader.info().width, reader.info().height)
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("the test path is UTF-8")
}

fn artifact_paths(out: &Path) -> String {
    ARTIFACTS
        .iter()
        .map(|name| format!("{}\n", out.join(name).display()))
        .collect()
}

fn directory_names(out: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(out)
        .expect("the output directory is readable")
        .map(|entry| {
            entry
                .expect("the directory entry is readable")
                .file_name()
                .to_str()
                .expect("the artifact name is UTF-8")
                .to_owned()
        })
        .collect();
    names.sort();
    names
}

/// The UTC open time of `2025-01-01 00:00:00`, shared by the generated series.
const GENERATED_EPOCH: i64 = 1_735_689_600;

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

/// Writes a deterministic M1 series that spans more than one UTC day.
fn synthetic_csv(name: &str, total: usize) -> String {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/report-tests/source");
    fs::create_dir_all(&directory).expect("the source directory is created");
    let path = directory.join(format!("{name}.csv"));
    let mut text = String::from("Date,Open,High,Low,Close,Volume\n");
    for index in 0..total {
        text.push_str(&generated_row(index));
        text.push('\n');
    }
    fs::write(&path, text).expect("the generated source is written");
    path_text(&path).to_owned()
}

#[test]
fn market_report_writes_the_four_artifacts_in_order() {
    let path = fixture("state_m1.csv");
    let out = report_dir("artifacts");
    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);

    assert_success(&output);
    assert_eq!(stdout_text(&output), artifact_paths(&out));
    assert_eq!(stderr_text(&output), "");

    let mut expected: Vec<String> = ARTIFACTS.iter().map(|name| (*name).to_owned()).collect();
    expected.sort();
    assert_eq!(directory_names(&out), expected);

    let summary = fs::read_to_string(out.join("summary.txt")).expect("the summary is readable");
    assert!(summary.starts_with("QuantRule market report\n"));
    assert!(summary.contains("data_version: test:v1\n"));
    assert!(summary.contains("requested_period: full file\n"));
    assert!(
        summary.ends_with("[artifacts]\nsummary.txt\nreport.json\nprice-axes.png\nevents.png\n")
    );

    let report = fs::read(out.join("report.json")).expect("the report is readable");
    let value: serde_json::Value = serde_json::from_slice(&report).expect("the report is JSON");
    assert_eq!(value["view_contract"], "2026-10-27");
    assert_eq!(value["data_version"], DATA_VERSION);

    let price_axes = fs::read(out.join("price-axes.png")).expect("the price chart is readable");
    let events = fs::read(out.join("events.png")).expect("the events chart is readable");
    assert_eq!(png_info(&price_axes), (1_280, 960));
    assert_eq!(png_info(&events), (1_280, 720));
}

#[test]
fn market_report_summary_json_and_charts_are_exact_goldens() {
    let path = fixture("state_m1.csv");
    let out = report_dir("goldens");
    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);
    assert_success(&output);

    assert_eq!(
        fs::read(out.join("summary.txt")).expect("the summary is readable"),
        include_bytes!("fixtures/market-report.txt")
    );
    assert_eq!(
        fs::read(out.join("report.json")).expect("the report is readable"),
        include_bytes!("fixtures/market-report.json")
    );
    assert_eq!(
        fs::read(out.join("price-axes.png")).expect("the price chart is readable"),
        include_bytes!("fixtures/market-report-price-axes.png")
    );
    assert_eq!(
        fs::read(out.join("events.png")).expect("the events chart is readable"),
        include_bytes!("fixtures/market-report-events.png")
    );
}

#[test]
fn market_report_json_prints_the_exact_report_file_bytes() {
    let path = fixture("state_m1.csv");
    let out = report_dir("json-stdout");
    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
        "--json",
    ]);
    assert_success(&output);
    assert_eq!(stderr_text(&output), "");

    let file = fs::read(out.join("report.json")).expect("the report is readable");
    assert_eq!(output.stdout, file);

    let value: serde_json::Value = serde_json::from_slice(&file).expect("the report is JSON");
    assert_eq!(value["model_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["view_contract"], "2026-10-27");
    assert_eq!(value["data_version"], DATA_VERSION);
    assert!(value["requested_period"]["from"].is_null());
    assert!(value["requested_period"]["to"].is_null());
    assert_eq!(
        value["market_report"]["period"]["first_open_time"],
        GENERATED_EPOCH
    );
    assert_eq!(
        value["market_report"]["period"]["last_open_time"],
        GENERATED_EPOCH + 4 * 3_600 + 19 * 60
    );
    assert_eq!(
        value["market_report"]["ladder"],
        serde_json::json!([60, 300, 900, 3_600])
    );
    let timeframes = value["market_report"]["timeframes"]
        .as_array()
        .expect("the timeframes are a list");
    assert_eq!(timeframes.len(), 4);
    for timeframe in timeframes {
        assert_eq!(
            timeframe["axes"].as_array().map(Vec::len),
            Some(4),
            "each timeframe publishes four axes"
        );
    }
    assert_eq!(value["market_report"]["charts"]["timeframe_secs"], 3_600);
    assert_eq!(value["market_report"]["charts"]["source_bars"], 4);
    assert_eq!(value["market_report"]["charts"]["bucket_size"], 1);
    assert_eq!(
        value["market_report"]["charts"]["bars"]
            .as_array()
            .map(Vec::len),
        Some(4)
    );
    assert_eq!(
        value["market_report"]["events"]["total"],
        value["market_report"]["events"]["by_type"]["level"]
            .as_u64()
            .expect("the level count is a number")
            + value["market_report"]["events"]["by_type"]["divergence"]
                .as_u64()
                .expect("the divergence count is a number")
            + value["market_report"]["events"]["by_type"]["signal"]
                .as_u64()
                .expect("the signal count is a number")
    );
}

#[test]
fn market_report_is_deterministic_and_interactive_matches() {
    let path = fixture("state_m1.csv");
    let first = report_dir("determinism-a");
    let second = report_dir("determinism-b");
    let interactive_out = report_dir("determinism-interactive");

    let first_run = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&first),
        "--json",
    ]);
    let second_run = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&second),
        "--json",
    ]);
    assert_success(&first_run);
    assert_success(&second_run);
    assert_eq!(stdout_text(&first_run), stdout_text(&second_run));

    for name in ARTIFACTS {
        assert_eq!(
            fs::read(first.join(name)).expect("the first artifact is readable"),
            fs::read(second.join(name)).expect("the second artifact is readable"),
            "{name} is not deterministic"
        );
    }

    let interactive = run_interactive(&format!(
        "market-report {path} --data-version {DATA_VERSION} --out {} --json\nquit\n",
        path_text(&interactive_out)
    ));
    assert_success(&interactive);
    assert_eq!(stdout_text(&interactive), stdout_text(&first_run));
    assert!(stderr_text(&interactive).contains("quantrule> "));
}

#[test]
fn market_report_filters_the_source_before_the_ladder_build() {
    let path = synthetic_csv("range", 1_500);
    let full = report_dir("range-full");
    let first_day = report_dir("range-first-day");
    let day = report_dir("range-day");

    let full_run = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&full),
        "--json",
    ]);
    let first_day_run = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--to",
        "2025-01-01",
        "--out",
        path_text(&first_day),
        "--json",
    ]);
    let day_run = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2025-01-02",
        "--to",
        "2025-01-02",
        "--out",
        path_text(&day),
        "--json",
    ]);
    assert_success(&full_run);
    assert_success(&first_day_run);
    assert_success(&day_run);

    let full_value: serde_json::Value =
        serde_json::from_slice(&full_run.stdout).expect("the full report is JSON");
    let first_day_value: serde_json::Value =
        serde_json::from_slice(&first_day_run.stdout).expect("the first-day report is JSON");
    let day_value: serde_json::Value =
        serde_json::from_slice(&day_run.stdout).expect("the day report is JSON");

    assert_eq!(full_value["market_report"]["timeframes"][0]["bars"], 1_500);
    assert_eq!(full_value["market_report"]["timeframes"][3]["bars"], 25);
    assert_eq!(
        first_day_value["market_report"]["timeframes"][0]["bars"],
        1_440
    );
    assert_eq!(
        first_day_value["market_report"]["timeframes"][3]["bars"],
        24
    );
    assert_eq!(day_value["market_report"]["timeframes"][0]["bars"], 60);
    assert_eq!(day_value["market_report"]["timeframes"][1]["bars"], 12);
    assert_eq!(day_value["market_report"]["timeframes"][2]["bars"], 4);
    assert_eq!(day_value["market_report"]["timeframes"][3]["bars"], 1);
    assert_eq!(
        day_value["market_report"]["period"]["first_open_time"],
        GENERATED_EPOCH + 86_400
    );
    assert_eq!(
        day_value["market_report"]["period"]["last_open_time"],
        GENERATED_EPOCH + 86_400 + 59 * 60
    );
    assert_eq!(day_value["requested_period"]["from"], "2025-01-02");
    assert_eq!(day_value["requested_period"]["to"], "2025-01-02");
    assert!(first_day_value["requested_period"]["from"].is_null());
    assert_eq!(first_day_value["requested_period"]["to"], "2025-01-01");
    assert!(full_value["requested_period"]["from"].is_null());
    assert!(full_value["requested_period"]["to"].is_null());
}

#[test]
fn market_report_accepts_an_existing_empty_directory() {
    let path = fixture("state_m1.csv");
    let out = report_dir("empty-directory");
    fs::create_dir_all(&out).expect("the empty directory is created");

    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);
    assert_success(&output);
    assert_eq!(directory_names(&out).len(), 4);
}

#[test]
fn market_report_rejects_a_non_empty_output_directory() {
    let path = fixture("state_m1.csv");
    let out = report_dir("non-empty");
    fs::create_dir_all(&out).expect("the output directory is created");
    fs::write(out.join("keep.txt"), b"keep").expect("the blocker file is written");

    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("must be absent or empty"));
    assert_eq!(
        fs::read(out.join("keep.txt")).expect("the blocker is readable"),
        b"keep"
    );
    assert!(!out.join("summary.txt").exists());
}

#[test]
fn market_report_rejects_an_output_path_that_is_a_file() {
    let path = fixture("state_m1.csv");
    let out = report_dir("file-output");
    fs::create_dir_all(out.parent().expect("the output has a parent"))
        .expect("the parent directory is created");
    fs::write(&out, b"not a directory").expect("the blocker file is written");

    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("exists and is not a directory"));
    assert_eq!(
        fs::read(&out).expect("the blocker is readable"),
        b"not a directory"
    );
}

#[test]
fn market_report_empty_range_writes_no_artifacts() {
    let path = fixture("state_m1.csv");
    let out = report_dir("empty-range");

    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2025-02-01",
        "--to",
        "2025-02-02",
        "--out",
        path_text(&out),
    ]);
    assert_failure(&output);
    assert!(
        stderr_text(&output)
            .contains("no M1 candles in the requested period '2025-02-01..2025-02-02'")
    );
    assert!(!out.exists(), "an empty range created the output directory");
}

#[test]
fn market_report_has_no_execution_vocabulary() {
    let path = fixture("state_m1.csv");
    let out = report_dir("vocabulary");
    let output = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--out",
        path_text(&out),
    ]);
    assert_success(&output);

    let summary = fs::read_to_string(out.join("summary.txt"))
        .expect("the summary is readable")
        .to_lowercase();
    let report = fs::read_to_string(out.join("report.json"))
        .expect("the report is readable")
        .to_lowercase();
    for forbidden in [
        "order", "fill", "position", "trade", "equity", "pnl", "drawdown", "strategy",
    ] {
        assert!(!summary.contains(forbidden), "summary contains {forbidden}");
        assert!(!report.contains(forbidden), "report contains {forbidden}");
    }
}

#[test]
fn market_report_refuses_bad_arguments() {
    let path = fixture("state_m1.csv");
    let out = report_dir("bad-arguments");
    let out_text = path_text(&out);

    let missing_version = run(&["market-report", &path, "--out", out_text]);
    assert_failure(&missing_version);
    assert!(stderr_text(&missing_version).contains("--data-version is required"));

    let missing_out = run(&["market-report", &path, "--data-version", DATA_VERSION]);
    assert_failure(&missing_out);
    assert!(stderr_text(&missing_out).contains("--out is required"));

    let invalid_date = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2025-02-30",
        "--out",
        out_text,
    ]);
    assert_failure(&invalid_date);
    assert!(stderr_text(&invalid_date).contains("invalid UTC date"));

    let reversed = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2025-01-02",
        "--to",
        "2025-01-01",
        "--out",
        out_text,
    ]);
    assert_failure(&reversed);
    assert!(stderr_text(&reversed).contains("is after"));

    let unknown = run(&[
        "market-report",
        &path,
        "--data-version",
        DATA_VERSION,
        "--bogus",
        "--out",
        out_text,
    ]);
    assert_failure(&unknown);
    assert!(stderr_text(&unknown).contains("unknown option '--bogus'"));
}

#[test]
fn help_lists_the_market_report_command() {
    let output = run(&["help"]);
    assert_success(&output);
    let text = stdout_text(&output);
    for token in [
        "market-report",
        "--data-version",
        "--from",
        "--to",
        "--out",
        "--json",
    ] {
        assert!(text.contains(token), "help omits {token}");
    }
}
