//! Binary-level tests for the config-driven `report model` command.
//!
//! The bundle is checked for its flat artifact set, the spec artifacts, the
//! provenance boundary, and event ordering. No test reads `data/` or the
//! network.

use std::collections::BTreeMap;
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
        let states = &timeframe["latest"]["states"];
        assert!(states.get("vol_regime").is_some());
        assert!(states.get("structure_state").is_some());
    }
}

#[test]
fn the_events_csv_is_in_availability_order() {
    let out = output_dir("events-order");
    let config = fixture("config-run/run.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]));

    let csv = String::from_utf8(read(&out.join("model").join("events.csv")))
        .expect("events.csv is UTF-8");
    let mut lines = csv.lines();
    let header = lines.next().expect("the header");
    let columns: Vec<&str> = header.split(',').collect();
    let timeframe = columns
        .iter()
        .position(|column| *column == "timeframe_secs")
        .expect("a timeframe column");
    let available = columns
        .iter()
        .position(|column| *column == "available_bar_index")
        .expect("an availability column");

    let mut previous: BTreeMap<String, u64> = BTreeMap::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        let timeframe = fields[timeframe].to_owned();
        let available = fields[available].parse::<u64>().expect("a numeric bar");
        if let Some(previous) = previous.insert(timeframe.clone(), available) {
            assert!(
                previous <= available,
                "the {timeframe} rows are out of availability order: {previous} then {available}"
            );
        }
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
    assert_eq!(
        session
            .lines()
            .nth(1)
            .and_then(|line| line.split(',').nth(2)),
        Some("0"),
        "outside-session rows must publish session_code 0"
    );

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"id\":\"M2.1\""));
    assert!(provenance.contains("\"session_events\""));
    assert!(provenance.contains("\"panel_events\""));
    let provenance_json: serde_json::Value =
        serde_json::from_str(&provenance).expect("provenance is JSON");
    let panel_sources = provenance_json["spec"]["panel_sources"]
        .as_array()
        .expect("panel sources");
    assert!(panel_sources.iter().any(|source| {
        source["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("state_m1.csv"))
            && source["content_hash"]
                .as_str()
                .is_some_and(|hash| !hash.is_empty())
    }));
}

#[test]
fn a_direction_prediction_is_recorded_as_direction() {
    let out = output_dir("spec-v2-direction");
    let config = fixture("config-run/run.json");
    let spec = fixture("config-run/spec-v2-direction.json");
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

    let provenance: serde_json::Value =
        serde_json::from_slice(&read(&out.join("model").join("provenance.json")))
            .expect("provenance is JSON");
    let prediction = &provenance["spec"]["prediction"];
    assert_eq!(prediction["kind"], "direction");
    assert_eq!(prediction["events"][0], "signal.ema_pair_cross");
    assert!(prediction["long"].as_array().is_some_and(Vec::is_empty));
    assert!(prediction["short"].as_array().is_some_and(Vec::is_empty));
}

#[test]
fn two_model_specs_with_disjoint_active_events_stay_disjoint() {
    let config = fixture("config-run/run.json");
    let mut codes = Vec::new();
    for (name, spec_name) in [("rsi", "spec-v2-rsi.json"), ("ema", "spec-v2-ema.json")] {
        let out = output_dir(&format!("isolation-{name}"));
        let spec = fixture(&format!("config-run/{spec_name}"));
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
        let csv = String::from_utf8(read(&out.join("model").join("events.csv")))
            .expect("events.csv is UTF-8");
        let mut lines = csv.lines();
        lines.next();
        codes.push(
            lines
                .map(|line| {
                    line.split(',')
                        .nth(2)
                        .expect("an event_code column")
                        .parse::<u16>()
                        .expect("a numeric event code")
                })
                .collect::<std::collections::BTreeSet<_>>(),
        );
    }
    assert!(codes[0].contains(&8), "the RSI threshold event is missing");
    assert!(codes[1].contains(&9), "the EMA pair event is missing");
    assert!(
        codes[0].is_disjoint(&codes[1]),
        "the two active-event selections overlap"
    );
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
