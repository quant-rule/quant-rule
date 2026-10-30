//! Binary-level tests for the `strategy-run` command.
//!
//! The tests run the real binary against a deterministic generated M1 series
//! and assert the exact NDJSON entries and exits, the absence of fill and P&L
//! fields, determinism, interactive parity, date filtering, and the failure
//! behavior that keeps the command on the strategy-only path.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

use quantrule_data::format_utc_timestamp;
use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const DATA_VERSION: &str = "test:strategy:v1";
const STRATEGY: &str = "axis-agreement";
const GOLDEN: &str = include_str!("fixtures/strategy-run.ndjson");

/// The UTC open time of `2025-01-01 00:00:00`, shared by the generated series.
const GENERATED_EPOCH: i64 = 1_735_689_600;

fn fixture(name: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
        .to_str()
        .expect("fixture path is valid UTF-8")
        .to_owned()
}

fn evidence_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/strategy-run-tests")
        .join(name)
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

/// One deterministic M1 row with a slow multi-scale wave.
fn generated_row(index: usize) -> String {
    let timestamp = GENERATED_EPOCH + index as i64 * 60;
    let stamp = format_utc_timestamp(timestamp);
    let phase = index as f64;
    let open = 100.0
        + 6.0 * (phase / 700.0).sin()
        + 2.0 * (phase / 130.0).sin()
        + 0.5 * (phase / 17.0).sin();
    let close = 100.0
        + 6.0 * ((phase + 1.0) / 700.0).sin()
        + 2.0 * ((phase + 1.0) / 130.0).sin()
        + 0.5 * ((phase + 1.0) / 17.0).sin();
    let high = open.max(close) + 0.3 + 0.1 * (phase / 31.0).sin().abs();
    let low = open.min(close) - 0.3 - 0.1 * (phase / 29.0).cos().abs();
    let volume = 1_000 + (index * 37) % 900;
    format!(
        "{} {}+00:00,{open:.5},{high:.5},{low:.5},{close:.5},{volume}",
        &stamp[..10],
        &stamp[11..19]
    )
}

/// Writes a deterministic M1 series under `target/` and returns its path.
fn synthetic_csv(name: &str, total: usize) -> String {
    let directory = evidence_dir("source");
    fs::create_dir_all(&directory).expect("the evidence directory is created");
    let path = directory.join(format!("{name}.csv"));
    let mut text = String::from("Date,Open,High,Low,Close,Volume\n");
    for index in 0..total {
        text.push_str(&generated_row(index));
        text.push('\n');
    }
    fs::write(&path, text).expect("the generated source is written");
    path.to_str()
        .expect("the generated path is valid UTF-8")
        .to_owned()
}

fn long_series() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| synthetic_csv("strategy_m1", 8_000))
}

fn short_series() -> String {
    synthetic_csv("strategy_short", 5)
}

fn parse_ndjson(text: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str(line).expect("the line is JSON"))
        .collect()
}

#[test]
fn strategy_run_prints_the_golden_ndjson() {
    let path = long_series();
    let output = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);

    assert_success(&output);
    assert_eq!(stderr_text(&output), "");
    assert_eq!(stdout_text(&output), GOLDEN);
}

#[test]
fn strategy_run_output_has_no_fill_or_pnl_fields() {
    let path = long_series();
    let output = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);
    assert_success(&output);

    let records = parse_ndjson(&stdout_text(&output));
    assert!(!records.is_empty(), "the fixture must produce decisions");

    let forbidden = [
        "fill",
        "fill_price",
        "quantity",
        "position",
        "equity",
        "pnl",
        "cash",
        "fee",
        "fees",
        "slippage",
        "order",
        "trade",
    ];
    for record in &records {
        let object = record.as_object().expect("a record is an object");
        for key in forbidden {
            assert!(
                !object.contains_key(key),
                "record carries the forbidden field {key}: {record}"
            );
        }
        assert!(matches!(record["type"].as_str(), Some("entry" | "exit")));
        assert!(matches!(record["side"].as_i64(), Some(1 | -1)));
        assert_eq!(record["timeframe"], "15m");
        assert_eq!(record["strategy"], STRATEGY);
        assert!(record["bar"].is_u64());
        assert!(record["datetime"].is_string());
    }
}

#[test]
fn repeated_runs_are_byte_identical() {
    let path = long_series();
    let first = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);
    let second = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);

    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(first.stderr, second.stderr);
}

#[test]
fn interactive_output_matches_non_interactive() {
    let path = long_series();
    let command =
        format!("strategy-run {path} --strategy {STRATEGY} --data-version {DATA_VERSION}\nquit\n");
    let interactive = run_interactive(&command);
    let non_interactive = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);

    assert_success(&interactive);
    assert_success(&non_interactive);
    assert_eq!(stdout_text(&interactive), stdout_text(&non_interactive));
    assert!(
        stderr_text(&interactive).contains("quantrule> "),
        "prompt is missing from stderr: {}",
        stderr_text(&interactive)
    );
}

#[test]
fn json_envelope_carries_provenance_and_the_same_records() {
    let path = long_series();
    let plain = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);
    let envelope = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
        "--json",
    ]);

    assert_success(&plain);
    assert_success(&envelope);

    let value: Value = serde_json::from_str(&stdout_text(&envelope)).expect("the envelope is JSON");
    assert_eq!(value["model_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(value["strategy"], STRATEGY);
    assert_eq!(value["data_version"], DATA_VERSION);
    assert_eq!(value["timeframe_secs"], 900);
    assert_eq!(value["strategy_config"]["timeframe_secs"], 900);
    assert_eq!(value["strategy_config"]["long_threshold"], 0.1);
    assert_eq!(value["strategy_config"]["short_threshold"], -0.1);
    assert_eq!(value["requested_period"]["from"], Value::Null);
    assert_eq!(value["requested_period"]["to"], Value::Null);
    assert_eq!(
        value["records"],
        Value::Array(parse_ndjson(&stdout_text(&plain)))
    );
}

#[test]
fn json_output_is_identical_interactively_and_non_interactively() {
    let path = long_series();
    let command = format!(
        "strategy-run {path} --strategy {STRATEGY} --data-version {DATA_VERSION} --json\nquit\n"
    );
    let interactive = run_interactive(&command);
    let non_interactive = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
        "--json",
    ]);

    assert_success(&interactive);
    assert_success(&non_interactive);
    assert_eq!(stdout_text(&interactive), stdout_text(&non_interactive));
}

#[test]
fn date_filter_keeps_records_inside_the_range() {
    let path = long_series();
    let output = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2025-01-03",
        "--to",
        "2025-01-06",
    ]);
    assert_success(&output);

    let records = parse_ndjson(&stdout_text(&output));
    assert!(!records.is_empty(), "the range must produce decisions");
    for record in &records {
        let datetime = record["datetime"]
            .as_str()
            .expect("the record has a datetime");
        let date = &datetime[..10];
        assert!(
            ("2025-01-03"..="2025-01-06").contains(&date),
            "record outside the requested range: {datetime}"
        );
    }
}

#[test]
fn an_empty_period_is_rejected() {
    let path = long_series();
    let output = run(&[
        "strategy-run",
        path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
        "--from",
        "2026-01-01",
        "--to",
        "2026-01-02",
    ]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("no M1 candles in the requested period"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn a_short_series_has_no_timeframe_bars() {
    let path = short_series();
    let output = run(&[
        "strategy-run",
        &path,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
    ]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("produced no 900-second bars"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn an_unknown_strategy_is_rejected() {
    let output = run(&[
        "strategy-run",
        &fixture("three.csv"),
        "--strategy",
        "ema-cross",
        "--data-version",
        DATA_VERSION,
    ]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("unknown strategy 'ema-cross'"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

#[test]
fn a_missing_data_version_is_rejected() {
    let output = run(&[
        "strategy-run",
        &fixture("three.csv"),
        "--strategy",
        STRATEGY,
    ]);

    assert_failure(&output);
    assert!(
        stderr_text(&output).contains("--data-version is required"),
        "unexpected stderr: {}",
        stderr_text(&output)
    );
}

/// Copies the example run configuration and points it at `source`.
fn run_config_with(source: &str, structure_scale: &str) -> PathBuf {
    let target = evidence_dir(&format!("config-{structure_scale}"));
    if target.exists() {
        fs::remove_dir_all(&target).expect("the old config copy is removed");
    }
    fs::create_dir_all(&target).expect("the config copy directory is created");
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/examples/solusd-2025");
    for entry in fs::read_dir(&example).expect("the example config is readable") {
        let entry = entry.expect("the example entry is readable");
        if entry.path().is_file() {
            fs::copy(entry.path(), target.join(entry.file_name()))
                .expect("the config file is copied");
        }
    }

    let data_path = target.join("data.json");
    let mut data: Value =
        serde_json::from_str(&fs::read_to_string(&data_path).expect("data.json is read"))
            .expect("data.json is JSON");
    data["source"]["path"] = Value::String(source.to_owned());
    fs::write(
        &data_path,
        serde_json::to_vec_pretty(&data).expect("data.json is written"),
    )
    .expect("data.json is written");

    let model_path = target.join("model.json");
    let mut model: Value =
        serde_json::from_str(&fs::read_to_string(&model_path).expect("model.json is read"))
            .expect("model.json is JSON");
    model["structure_scale"] = Value::String(structure_scale.to_owned());
    fs::write(
        &model_path,
        serde_json::to_vec_pretty(&model).expect("model.json is written"),
    )
    .expect("model.json is written");

    let analytics_path = target.join("analytics.json");
    let mut analytics: Value =
        serde_json::from_str(&fs::read_to_string(&analytics_path).expect("analytics.json is read"))
            .expect("analytics.json is JSON");
    analytics["structure_scale"] = Value::String(structure_scale.to_owned());
    fs::write(
        &analytics_path,
        serde_json::to_vec_pretty(&analytics).expect("analytics.json is written"),
    )
    .expect("analytics.json is written");

    target.join("run.json")
}

#[test]
fn strategy_run_uses_the_run_model_config() {
    let source = long_series();
    let run_config = run_config_with(source, "minor");

    let strategy = run(&[
        "strategy-run",
        source,
        "--strategy",
        STRATEGY,
        "--data-version",
        DATA_VERSION,
        "--config",
        run_config.to_str().expect("the run path is UTF-8"),
        "--json",
    ]);
    assert_success(&strategy);
    let strategy: Value =
        serde_json::from_slice(&strategy.stdout).expect("strategy-run prints JSON");
    let records = strategy["records"]
        .as_array()
        .expect("strategy-run records are an array");

    let out = evidence_dir("report-minor");
    if out.exists() {
        fs::remove_dir_all(&out).expect("the old report is removed");
    }
    let report = run(&[
        "report",
        "strategy",
        "--config",
        run_config.to_str().expect("the run path is UTF-8"),
        "--out",
        out.to_str().expect("the report path is UTF-8"),
        "--json",
    ]);
    assert_success(&report);
    let report: Value = serde_json::from_slice(&report.stdout).expect("report prints JSON");
    let decisions = report["records"]["decisions"]
        .as_array()
        .expect("report decisions are an array");

    assert_eq!(records.len(), decisions.len());
    for (record, decision) in records.iter().zip(decisions) {
        for key in ["type", "side", "bar"] {
            assert_eq!(record[key], decision[key], "decision field {key} differs");
        }
        for key in ["direction", "momentum_force", "structure_score"] {
            assert_eq!(record[key], decision[key], "model field {key} differs");
        }
    }
}
