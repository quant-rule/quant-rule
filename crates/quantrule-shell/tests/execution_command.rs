//! Binary-level tests for `execution run`.
//!
//! Each test builds the model and strategy artifacts with the real commands,
//! then runs execution against them. The tests assert the bundle shape,
//! determinism, the non-empty-target refusal, the data-version handoff, and the
//! bracket-decision refusal.

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
        .join("../../target/p3-execution-tests")
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

fn build_decisions(name: &str) -> (PathBuf, PathBuf) {
    let root = output_dir(name);
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]));
    assert_success(&run(&[
        "strategy",
        "run",
        "--model",
        root.join("model")
            .to_str()
            .expect("the model path is UTF-8"),
        "--strategy",
        strategy.to_str().expect("the strategy path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]));
    (root.join("strategy"), root)
}

fn copy_file(source: &Path, target: &Path) {
    fs::copy(source, target).unwrap_or_else(|error| {
        panic!("copy {} to {}: {error}", source.display(), target.display())
    });
}

fn write_bundle_file(bundle: &Path, name: &str, text: &str) {
    fs::write(bundle.join(name), text)
        .unwrap_or_else(|error| panic!("write {}: {error}", bundle.join(name).display()));
}

#[test]
fn run_writes_the_execution_bundle() {
    let (decisions, root) = build_decisions("execution-bundle");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "execution",
        "run",
        "--decisions",
        decisions.to_str().expect("the decisions path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = root.join("execution");
    let files = bundle_files(&bundle);
    for name in [
        "orders.csv",
        "fills.csv",
        "rejections.csv",
        "summary.txt",
        "provenance.json",
    ] {
        assert!(files.contains_key(name), "{name} is missing");
    }

    let summary = String::from_utf8(files["summary.txt"].clone()).expect("summary is UTF-8");
    assert!(summary.contains("execution_timeframe: 5m"));
    assert!(summary.contains("decision_bars:"));
    assert!(summary.contains("fills: count="));

    let fills = String::from_utf8(files["fills.csv"].clone()).expect("fills are UTF-8");
    assert!(fills.lines().count() > 1, "the fixture must produce a fill");

    let provenance: serde_json::Value =
        serde_json::from_slice(&files["provenance.json"]).expect("provenance is JSON");
    assert_eq!(provenance["schema"], "quantrule.stage-provenance/v1");
    assert_eq!(provenance["stage"], "execution");
    assert_eq!(provenance["artifact"], "fills");
    assert_eq!(provenance["source_artifacts"][0]["stage"], "strategy");
}

#[test]
fn run_is_byte_deterministic() {
    let (decisions_a, root_a) = build_decisions("execution-deterministic-a");
    let (decisions_b, root_b) = build_decisions("execution-deterministic-b");
    let config = fixture("config-run/run.json");
    for (decisions, root) in [(&decisions_a, &root_a), (&decisions_b, &root_b)] {
        assert_success(&run(&[
            "execution",
            "run",
            "--decisions",
            decisions.to_str().expect("the decisions path is UTF-8"),
            "--config",
            config.to_str().expect("the config path is UTF-8"),
            "--out",
            root.to_str().expect("the output path is UTF-8"),
        ]));
    }
    assert_eq!(
        bundle_files(&root_a.join("execution")),
        bundle_files(&root_b.join("execution"))
    );
}

#[test]
fn run_refuses_a_non_empty_target() {
    let (decisions, root) = build_decisions("execution-non-empty");
    let config = fixture("config-run/run.json");
    let args = [
        "execution",
        "run",
        "--decisions",
        decisions.to_str().expect("the decisions path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ];
    assert_success(&run(&args));
    let second = run(&args);
    assert_failure(&second);
    assert!(stderr(&second).contains("must be absent or empty"));
}

#[test]
fn run_json_prints_the_artifact_envelope() {
    let (decisions, root) = build_decisions("execution-json");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "execution",
        "run",
        "--decisions",
        decisions.to_str().expect("the decisions path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert!(value["bundle"].as_str().is_some());
    assert_eq!(value["artifacts"][0], "orders.csv");
}

#[test]
fn run_rejects_a_missing_decisions_bundle() {
    let config = fixture("config-run/run.json");
    let out = output_dir("execution-missing-output");
    let output = run(&[
        "execution",
        "run",
        "--decisions",
        "target/no-such-strategy-bundle",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("cannot read"));
}

#[test]
fn run_rejects_a_data_version_mismatch() {
    let (decisions, _) = build_decisions("execution-version-source");
    let bundle = output_dir("execution-version-bundle");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    copy_file(
        &decisions.join("decisions.ndjson"),
        &bundle.join("decisions.ndjson"),
    );
    let mut provenance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(decisions.join("provenance.json")).expect("the provenance is readable"),
    )
    .expect("the provenance is JSON");
    provenance["data_version"] = serde_json::json!("other:v1");
    write_bundle_file(
        &bundle,
        "provenance.json",
        &format!(
            "{}\n",
            serde_json::to_string(&provenance).expect("the value is JSON")
        ),
    );

    let config = fixture("config-run/run.json");
    let out = output_dir("execution-version-output");
    let output = run(&[
        "execution",
        "run",
        "--decisions",
        bundle.to_str().expect("the bundle path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("does not match the run"));
}

#[test]
fn run_rejects_a_bracket_decision() {
    let (decisions, _) = build_decisions("execution-bracket-source");
    let bundle = output_dir("execution-bracket-bundle");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    copy_file(
        &decisions.join("provenance.json"),
        &bundle.join("provenance.json"),
    );

    let text =
        fs::read_to_string(decisions.join("decisions.ndjson")).expect("the decisions are readable");
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut first: serde_json::Value =
        serde_json::from_str(&lines[0]).expect("the first decision is JSON");
    first["decision"] = serde_json::json!("enter_bracket");
    first["side"] = serde_json::Value::Null;
    first["reason"] = serde_json::json!("bracket_entry");
    first["position_after"] = serde_json::json!("flat");
    first["intent"] = serde_json::json!({
        "kind": "bracket",
        "upper_price": 101.0,
        "lower_price": 99.0,
        "stop": { "kind": "atr_multiple", "multiple": 2.0 },
        "target": { "kind": "reward_to_risk", "multiple": 1.5 },
        "valid_for_bars": 1,
        "max_slippage_bps": null
    });
    lines[0] = serde_json::to_string(&first).expect("the decision is JSON");
    write_bundle_file(&bundle, "decisions.ndjson", &(lines.join("\n") + "\n"));

    let config = fixture("config-run/run.json");
    let out = output_dir("execution-bracket-output");
    let output = run(&[
        "execution",
        "run",
        "--decisions",
        bundle.to_str().expect("the bundle path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("side-aware sizing"));
}
