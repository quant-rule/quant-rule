//! Binary-level tests for the config-driven `report model` command.
//!
//! The bundle is checked for its flat artifact set, the spec artifacts, the
//! provenance boundary, and determinism. No test reads `data/` or the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-report-tests")
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

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|source| panic!("cannot read {}: {source}", path.display()))
}

#[test]
fn the_model_bundle_has_the_flat_shape() {
    let out = output_dir("flat-bundle");
    let config = fixture("config-run/run.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]));

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

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"report\":\"model\""));
    assert!(provenance.contains("\"report_version\":\"2026-11-05\""));
    assert!(provenance.contains("\"model_id\":\"M1.0\""));
    assert!(!provenance.contains(env!("CARGO_MANIFEST_DIR")));

    let report: serde_json::Value =
        serde_json::from_slice(&read(&bundle.join("report.json"))).expect("report is JSON");
    assert!(report.get("flat_report").is_some());
    assert!(report.get("market_report").is_none());
    for timeframe in report["flat_report"]["timeframes"]
        .as_array()
        .expect("timeframes")
    {
        assert!(timeframe.get("latest").is_some());
        assert!(timeframe.get("levels").is_some());
        assert!(timeframe.get("events").is_some());
        assert!(timeframe.get("progression").is_some());
        assert!(timeframe.get("axes").is_none());
    }
}

#[test]
fn the_spec_v1_bundle_adds_conditions() {
    let out = output_dir("spec-v1");
    let config = fixture("config-run/run.json");
    let spec = fixture("config-run/spec.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--spec",
        spec.to_str().expect("the spec path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]));

    let bundle = out.join("model");
    assert!(bundle.join("conditions.csv").exists());
    assert!(!bundle.join("prediction.csv").exists());
    assert!(!bundle.join("panel.csv").exists());
    assert!(!bundle.join("session.csv").exists());
    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("fixture-spec"));
    assert!(provenance.contains("fixture-spec"));
}

#[test]
fn the_spec_v2_bundle_adds_conditions_and_prediction() {
    let out = output_dir("spec-v2");
    let config = fixture("config-run/run.json");
    let spec = fixture("config-run/spec-v2.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--spec",
        spec.to_str().expect("the spec path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]));

    let bundle = out.join("model");
    assert!(bundle.join("conditions.csv").exists());
    assert!(bundle.join("prediction.csv").exists());
    assert!(bundle.join("panel.csv").exists());
    assert!(bundle.join("session.csv").exists());

    let panel = String::from_utf8(read(&bundle.join("panel.csv"))).expect("panel CSV");
    assert!(panel.starts_with("timestamp,instrument,is_leader,relative_strength"));
    assert!(panel.lines().count() > 1, "panel.csv must carry rows");

    let session = String::from_utf8(read(&bundle.join("session.csv"))).expect("session CSV");
    assert!(session.starts_with("bar,timestamp,session_code"));
    assert!(session.contains("calendar_bucket"));
    assert!(session.lines().count() > 1, "session.csv must carry rows");

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"id\":\"M2.1\""));
    assert!(provenance.contains("\"session_events\""));
    assert!(provenance.contains("\"panel_events\""));
}

#[test]
fn a_spec_timeframe_outside_the_ladder_is_rejected() {
    let out = output_dir("spec-missing-timeframe");
    let config = fixture("config-run/run.json");
    let spec = fixture("config-run/spec-missing-timeframe.json");
    let output = run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--spec",
        spec.to_str().expect("the spec path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not in the run's data ladder"));
}

#[test]
fn the_json_flag_prints_report_json_to_stdout() {
    let out = output_dir("json-stdout");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout is one JSON value");
    assert!(value.get("flat_report").is_some());
}
