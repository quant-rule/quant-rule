//! Binary-level tests for `strategy explain` and `strategy run`.
//!
//! Every test generates the model bundle with the real `report model` command,
//! then runs the strategy command against it. The tests assert on stdout, the
//! written bundle, determinism, and the strategy/execution boundary.

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
        .join("../../target/p3-strategy-tests")
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

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn model_bundle(name: &str) -> PathBuf {
    let root = output_dir(name);
    let config = fixture("config-run/run.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]));
    root.join("model")
}

fn strategy(name: &str) -> PathBuf {
    fixture(&format!("strategy/{name}"))
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
            files.insert(name, fs::read(&path).expect("the artifact is readable"));
        }
    }
    files
}

fn assert_no_execution_or_account_fields(value: &serde_json::Value) {
    for forbidden in ["fill", "quantity", "fee", "cash", "equity", "pnl"] {
        assert!(
            value.get(forbidden).is_none(),
            "the decision record carries '{forbidden}'"
        );
    }
}

#[test]
fn explain_prints_one_decision_and_evidence() {
    let model = model_bundle("explain");
    let config = strategy("event-composite.json");
    let output = run(&[
        "strategy",
        "explain",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--bar",
        "21",
    ]);
    assert_success(&output);
    let text = stdout(&output);
    let value: serde_json::Value = serde_json::from_str(text.trim()).expect("one JSON object");
    assert_eq!(value["schema"], "quantrule.strategy-decision/v1");
    assert_eq!(value["decision"], "enter");
    assert_eq!(value["side"], "long");
    assert_eq!(value["position_before"], "flat");
    assert_eq!(value["position_after"], "long");
    assert!(value["atr14"].as_f64().is_some());
    assert!(
        value["evidence"]
            .as_array()
            .is_some_and(|items| !items.is_empty())
    );
    assert_no_execution_or_account_fields(&value);
}

#[test]
fn explain_is_deterministic() {
    let model = model_bundle("explain-deterministic");
    let config = strategy("event-composite.json");
    let args = [
        "strategy",
        "explain",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--bar",
        "21",
    ];
    let first = run(&args);
    let second = run(&args);
    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn run_writes_the_decisions_bundle() {
    let model = model_bundle("run-bundle");
    let config = strategy("event-composite.json");
    let out = output_dir("run-bundle-output");
    let output = run(&[
        "strategy",
        "run",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = out.join("strategy");
    let files = bundle_files(&bundle);
    assert!(files.contains_key("decisions.ndjson"));
    assert!(files.contains_key("summary.txt"));
    assert!(files.contains_key("provenance.json"));

    let decisions = String::from_utf8(files["decisions.ndjson"].clone()).expect("NDJSON is UTF-8");
    let lines: Vec<&str> = decisions.lines().collect();
    assert!(!lines.is_empty());
    for line in &lines {
        let value: serde_json::Value = serde_json::from_str(line).expect("each line is JSON");
        assert_eq!(value["schema"], "quantrule.strategy-decision/v1");
        assert_no_execution_or_account_fields(&value);
    }

    let summary = String::from_utf8(files["summary.txt"].clone()).expect("summary is UTF-8");
    assert!(summary.contains("engine: event_composite"));
    assert!(summary.contains("decisions:"));
    assert!(summary.contains("data_version: test:v1"));

    let provenance: serde_json::Value =
        serde_json::from_slice(&files["provenance.json"]).expect("provenance is JSON");
    assert_eq!(provenance["schema"], "quantrule.stage-provenance/v1");
    assert_eq!(provenance["stage"], "strategy");
    assert_eq!(provenance["artifact"], "decisions");
    assert_eq!(provenance["source_artifacts"][0]["stage"], "model");
}

#[test]
fn run_json_prints_the_artifact_envelope() {
    let model = model_bundle("run-json");
    let config = strategy("event-composite.json");
    let out = output_dir("run-json-output");
    let output = run(&[
        "strategy",
        "run",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert!(value["bundle"].as_str().is_some());
    assert_eq!(value["artifacts"][0], "decisions.ndjson");
}

#[test]
fn run_is_byte_deterministic() {
    let model = model_bundle("run-deterministic");
    let config = strategy("event-composite.json");
    let first_out = output_dir("run-deterministic-first");
    let second_out = output_dir("run-deterministic-second");
    for out in [&first_out, &second_out] {
        assert_success(&run(&[
            "strategy",
            "run",
            "--model",
            model.to_str().expect("the model path is UTF-8"),
            "--strategy",
            config.to_str().expect("the strategy path is UTF-8"),
            "--out",
            out.to_str().expect("the output path is UTF-8"),
        ]));
    }
    assert_eq!(
        bundle_files(&first_out.join("strategy")),
        bundle_files(&second_out.join("strategy"))
    );
}

#[test]
fn run_refuses_a_non_empty_target() {
    let model = model_bundle("run-non-empty");
    let config = strategy("event-composite.json");
    let out = output_dir("run-non-empty-output");
    assert_success(&run(&[
        "strategy",
        "run",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]));
    let second = run(&[
        "strategy",
        "run",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&second);
    assert!(stderr(&second).contains("must be absent or empty"));
}

#[test]
fn run_refuses_the_compression_bracket_engine() {
    let model = model_bundle("run-bracket");
    let config = strategy("compression-bracket.json");
    let out = output_dir("run-bracket-output");
    let output = run(&[
        "strategy",
        "run",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("cannot run standalone"));
}

#[test]
fn explain_supports_the_compression_bracket_engine() {
    let model = model_bundle("explain-bracket");
    let config = strategy("compression-bracket.json");
    let output = run(&[
        "strategy",
        "explain",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--bar",
        "10",
    ]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert_eq!(value["schema"], "quantrule.strategy-decision/v1");
}

#[test]
fn explain_rejects_a_condition_timeframe_outside_the_context() {
    let model = model_bundle("explain-invalid-context");
    let config = strategy("invalid-context.json");
    let output = run(&[
        "strategy",
        "explain",
        "--model",
        model.to_str().expect("the model path is UTF-8"),
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--bar",
        "0",
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("configured context timeframe"));
}

#[test]
fn explain_rejects_a_missing_model_bundle() {
    let config = strategy("event-composite.json");
    let output = run(&[
        "strategy",
        "explain",
        "--model",
        "target/no-such-model-bundle",
        "--strategy",
        config.to_str().expect("the strategy path is UTF-8"),
        "--bar",
        "0",
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("cannot read"));
}
