//! Binary-level tests for the surviving shell command surface.
//!
//! Every test runs the real `quantrule` binary against a checked-in fixture and
//! asserts on stdout, stderr, and exit status. No test reads `data/`, the
//! network, or the wall clock.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fixture_text(name: &str) -> String {
    fs::read_to_string(fixture(name)).unwrap_or_else(|error| panic!("read {name}: {error}"))
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-shell-tests")
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

fn bundle_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in
        fs::read_dir(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
    {
        let entry =
            entry.unwrap_or_else(|error| panic!("read entry in {}: {error}", path.display()));
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            let bytes =
                fs::read(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            files.insert(name, bytes);
        }
    }
    files
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
    assert!(
        !output.status.success(),
        "expected failure; stdout: {}",
        stdout_text(output)
    );
}

#[test]
fn candles_prints_the_retained_window() {
    let output = run(&["candles", fixture("three.csv").to_str().expect("UTF-8")]);
    assert_success(&output);
    assert_eq!(stdout_text(&output), fixture_text("three.ndjson"));
}

#[test]
fn interactive_and_non_interactive_output_are_byte_identical() {
    let path = fixture("three.csv");
    let path = path.to_str().expect("the fixture path is UTF-8");
    let non_interactive = run(&["candles", path]);
    let interactive = run_interactive(&format!("candles {path}\n"));
    assert_success(&non_interactive);
    assert_success(&interactive);
    assert_eq!(non_interactive.stdout, interactive.stdout);
}

#[test]
fn candles_tolerant_prints_the_quality_report() {
    let output = run(&[
        "candles-tolerant",
        fixture("tolerant_mixed.csv").to_str().expect("UTF-8"),
    ]);
    assert_success(&output);
    let text = stderr_text(&output);
    assert!(text.contains("reason="));
}

#[test]
fn timeframes_prints_every_series_and_the_aggregation_report() {
    let output = run(&["timeframes", fixture("three.csv").to_str().expect("UTF-8")]);
    assert_success(&output);
    assert!(stdout_text(&output).contains("\"timeframe\":\"1m\""));
    assert!(stderr_text(&output).contains("aggregation timeframe=15m"));
}

#[test]
fn events_prints_the_flat_event_batch() {
    let output = run(&["events", fixture("state_m1.csv").to_str().expect("UTF-8")]);
    assert_success(&output);
    let text = stdout_text(&output);
    assert!(!text.is_empty(), "the fixture must publish events");
    for line in text.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
        assert!(value.get("timeframe").is_some());
        assert!(value.get("type").is_some());
        assert!(value.get("available_bar").is_some());
        assert!(value.get("primary_value").is_some());
    }
}

#[test]
fn events_is_deterministic() {
    let path = fixture("state_m1.csv");
    let path = path.to_str().expect("the fixture path is UTF-8");
    let first = run(&["events", path, "--window", "latest"]);
    let second = run(&["events", path, "--window", "latest"]);
    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn events_are_in_availability_order() {
    let output = run(&["events", fixture("state_m1.csv").to_str().expect("UTF-8")]);
    assert_success(&output);

    let mut previous: BTreeMap<String, u64> = BTreeMap::new();
    for line in stdout_text(&output).lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
        let timeframe = value["timeframe"].as_str().expect("a timeframe label");
        let available = value["available_bar"].as_u64().expect("an available bar");
        if let Some(previous) = previous.insert(timeframe.to_owned(), available) {
            assert!(
                previous <= available,
                "the {timeframe} batch is out of availability order: {previous} then {available}"
            );
        }
    }
}

#[test]
fn help_lists_only_the_surviving_commands() {
    let output = run(&["help"]);
    assert_success(&output);
    let text = stdout_text(&output);
    for command in [
        "candles",
        "candles-tolerant",
        "timeframes",
        "events",
        "report model",
        "policy check",
        "strategy explain",
        "strategy run",
        "execution run",
        "portfolio replay",
        "backtest run",
        "fetch",
        "help",
    ] {
        assert!(text.contains(command), "help must list {command}");
    }
    for removed in [
        "market-state",
        "market-dashboard",
        "market-chart",
        "market-report",
        "axis",
        "legs",
        "model-series",
        "strategy-run",
        "store",
        "db",
        "research",
        "lab",
    ] {
        assert!(!text.contains(removed), "help must not list {removed}");
    }
    assert_eq!(text, fixture_text("help.txt"));
}

#[test]
fn help_all_and_glossary_are_stable_topics() {
    let all = run(&["help", "all"]);
    assert_success(&all);
    assert_eq!(stdout_text(&all), fixture_text("help-all.txt"));

    let glossary = run(&["help", "glossary"]);
    assert_success(&glossary);
    assert_eq!(stdout_text(&glossary), fixture_text("help-glossary.txt"));
}

#[test]
fn a_removed_command_fails_cleanly() {
    let output = run(&["market-state", "x.csv"]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("unknown command"));
}

#[test]
fn policy_check_validates_the_current_example() {
    let config =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/examples/btcusd-2024/run.json");
    let output = run(&[
        "policy",
        "check",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
    ]);
    assert_success(&output);
    let text = stdout_text(&output);
    assert!(text.starts_with("policy ok run=btcusd-2024"));
    assert!(text.contains("model=M1.0"));
    assert!(text.contains("model_ladder=1m,5m,15m,1h"));
    assert!(text.contains("timeframes=5m,15m"));
}

#[test]
fn a_retired_model_schema_fails_cleanly() {
    let config = fixture("config-run/run-old-model.json");
    let output = run(&[
        "policy",
        "check",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
    ]);
    assert_failure(&output);
    let error = stderr_text(&output);
    assert!(error.contains("model-v3.json"), "unexpected error: {error}");
    assert!(error.contains("$.schema"), "unexpected error: {error}");
    assert!(
        error.contains("expected quantrule.model/v4"),
        "unexpected error: {error}"
    );
}

#[test]
fn report_model_writes_the_flat_bundle() {
    let out = output_dir("report-model");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = out.join("model");
    for name in [
        "summary.txt",
        "report.json",
        "result.json",
        "readings.csv",
        "events.csv",
        "levels.csv",
        "legs.csv",
        "provenance.json",
    ] {
        assert!(bundle.join(name).exists(), "{name} is missing");
    }
    assert!(!bundle.join("price-axes.png").exists());
    assert!(!bundle.join("events.png").exists());

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("report.json")).expect("read report"))
            .expect("report is JSON");
    assert_eq!(report["view_contract"], "2026-11-05");
    assert!(report.get("flat_report").is_some());
    assert!(report.get("market_report").is_none());

    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(bundle.join("result.json")).expect("read result"))
            .expect("result is JSON");
    assert_eq!(result["schema"], "quantrule.model-result/v4");
    assert!(result.get("charts").is_none());
    assert!(result.get("timeframes").is_some());

    let summary = fs::read_to_string(bundle.join("summary.txt")).expect("read summary");
    assert!(summary.contains("QuantRule model report"));
    assert!(summary.contains("[coverage]"));
    assert!(summary.contains("[events]"));
    assert!(summary.contains("[latest]"));
    assert!(summary.contains("[progression]"));
    assert!(!summary.contains("[axes]"));
}

#[test]
fn report_model_is_deterministic() {
    let config = fixture("config-run/run.json");
    let config = config.to_str().expect("the config path is UTF-8");

    let first = output_dir("report-model-first");
    let second = output_dir("report-model-second");
    for out in [&first, &second] {
        let output = run(&[
            "report",
            "model",
            "--config",
            config,
            "--out",
            out.to_str().expect("the output path is UTF-8"),
        ]);
        assert_success(&output);
    }

    let first_files = bundle_files(&first.join("model"));
    let second_files = bundle_files(&second.join("model"));
    assert!(!first_files.is_empty(), "the bundle must contain artifacts");
    assert_eq!(
        first_files.keys().collect::<Vec<_>>(),
        second_files.keys().collect::<Vec<_>>(),
        "the bundles contain different artifact names"
    );
    for (name, bytes) in &first_files {
        assert_eq!(
            bytes,
            second_files
                .get(name)
                .expect("the second bundle has the artifact"),
            "{name} differs across runs"
        );
    }
}

#[test]
fn report_model_refuses_a_removed_engine_and_a_missing_config() {
    let output = run(&["report", "strategy", "--config", "config-run/run.json"]);
    assert_failure(&output);

    let output = run(&["report", "model"]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("--config is required"));
}

#[test]
fn report_model_refuses_a_non_empty_output() {
    let out = output_dir("report-model-non-empty");
    fs::create_dir_all(out.join("model")).expect("the output directory is created");
    fs::write(out.join("model").join("keep.txt"), b"keep").expect("the marker is written");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr_text(&output).contains("must be absent or empty"));
}

#[test]
fn quit_exits_the_interactive_shell() {
    let output = run_interactive("quit\n");
    assert_success(&output);
    assert!(stdout_text(&output).is_empty());
}
