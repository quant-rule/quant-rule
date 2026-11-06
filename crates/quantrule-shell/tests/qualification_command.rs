//! Binary-level tests for `qualify`.
//!
//! Every test first runs the real `backtest run` command on the fixture data,
//! then evaluates the resulting evidence bundle against a checked-in criteria
//! file. The tests assert the bundle layout, the golden record and table, the
//! verdict precedence, the partition and sample gates, determinism, the
//! manifest-integrity refusal, and the absence of a human decision.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-qualification-tests")
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

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, current: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(current)
            .unwrap_or_else(|error| panic!("read {}: {error}", current.display()))
        {
            let entry = entry
                .unwrap_or_else(|error| panic!("read entry in {}: {error}", current.display()));
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, files);
            } else if path.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .expect("the file is under the root")
                    .to_owned();
                files.insert(
                    relative.to_string_lossy().into_owned(),
                    fs::read(&path).expect("the artifact is readable"),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).expect("the target root is created");
    for entry in fs::read_dir(source).expect("the source is readable") {
        let entry = entry.expect("the source entry is readable");
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &destination);
        } else {
            fs::copy(&path, &destination).expect("the file is copied");
        }
    }
}

fn evidence(name: &str) -> PathBuf {
    let root = output_dir(name);
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let output = run(&[
        "backtest",
        "run",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--strategy",
        strategy.to_str().expect("the strategy path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);
    root.join("backtest")
}

fn qualify(evidence: &Path, criteria: &str, out: &Path) -> Output {
    let criteria = fixture(&format!("qualification/{criteria}"));
    run(&[
        "qualify",
        "--evidence",
        evidence.to_str().expect("the evidence path is UTF-8"),
        "--criteria",
        criteria.to_str().expect("the criteria path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ])
}

fn record(out: &Path) -> Value {
    let path = out.join("qualification/qualification.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("the qualification record is JSON")
}

#[test]
fn qualification_writes_the_bundle_and_matches_the_golden_fixture() {
    let evidence = evidence("golden-evidence");
    let out = output_dir("golden-result");
    let output = qualify(&evidence, "pass.json", &out);
    assert_success(&output);
    assert!(stdout(&output).contains("verdict: pass"));
    assert!(stdout(&output).contains("human_decision: pending"));

    let root = out.join("qualification");
    let files = tree(&root);
    for name in [
        "qualification.json",
        "criteria.csv",
        "qualification.md",
        "provenance.json",
    ] {
        assert!(files.contains_key(name), "{name} is missing");
    }
    assert!(
        !root.join("decision.md").exists(),
        "the engine must never write the human decision"
    );

    for name in [
        "qualification.json",
        "criteria.csv",
        "qualification.md",
        "provenance.json",
    ] {
        let expected = fs::read(fixture(&format!("qualification/expected/{name}")))
            .unwrap_or_else(|error| panic!("read expected {name}: {error}"));
        assert_eq!(
            files[name], expected,
            "{name} must match the checked-in golden fixture"
        );
    }

    let value = record(&out);
    assert_eq!(value["schema"], "quantrule.qualification-result/v1");
    assert_eq!(value["stage"], "qualification");
    assert_eq!(value["verdict"], "pass");
    assert_eq!(value["human_decision"], Value::Null);
    assert_eq!(
        value["criteria"].as_array().expect("criteria").len(),
        4,
        "every criterion must be recorded"
    );
    assert_eq!(
        value["reasons"].as_array().expect("reasons").len(),
        1,
        "only the diagnostic failure is a reason"
    );
    assert_eq!(value["evidence"]["observed_trades"], 1);
    assert_eq!(value["evidence"]["partitions"][0], "portfolio_holdout");
}

#[test]
fn qualification_fail_takes_precedence_over_missing_evidence() {
    let evidence = evidence("fail-evidence");
    let out = output_dir("fail-result");
    assert_success(&qualify(&evidence, "fail.json", &out));

    let value = record(&out);
    assert_eq!(value["verdict"], "fail");
    let criteria = value["criteria"].as_array().expect("criteria");
    let drawdown = criteria
        .iter()
        .find(|criterion| criterion["id"] == "drawdown")
        .expect("the drawdown criterion is recorded");
    assert_eq!(drawdown["status"], "fail");
    assert_eq!(drawdown["reason_code"], "operator_not_satisfied");
    let missing = criteria
        .iter()
        .find(|criterion| criterion["id"] == "missing-mandatory")
        .expect("the missing criterion is recorded");
    assert_eq!(missing["status"], "insufficient_evidence");
    assert_eq!(missing["observed"], Value::Null);
    assert_eq!(missing["reason_code"], "missing_measurement");
}

#[test]
fn qualification_missing_partition_is_insufficient() {
    let evidence = evidence("partition-evidence");
    let out = output_dir("partition-result");
    assert_success(&qualify(&evidence, "insufficient.json", &out));

    let value = record(&out);
    assert_eq!(value["verdict"], "insufficient_evidence");
    for criterion in value["criteria"].as_array().expect("criteria") {
        assert_eq!(criterion["status"], "insufficient_evidence");
        assert_eq!(criterion["reason_code"], "required_partition_missing");
    }
    let coverage = value["evidence"]["partition_coverage"]
        .as_array()
        .expect("partition coverage");
    assert_eq!(coverage[0]["partition"], "strategy_validation");
    assert_eq!(coverage[0]["covered"], false);
}

#[test]
fn qualification_missing_measurement_is_insufficient_not_zero() {
    let evidence = evidence("missing-evidence");
    let out = output_dir("missing-result");
    assert_success(&qualify(&evidence, "missing.json", &out));

    let value = record(&out);
    assert_eq!(value["verdict"], "insufficient_evidence");
    let criterion = &value["criteria"][0];
    assert_eq!(criterion["status"], "insufficient_evidence");
    assert_eq!(criterion["observed"], Value::Null);
    assert_eq!(criterion["reason_code"], "missing_measurement");
}

#[test]
fn qualification_resolves_execution_and_model_fields() {
    let evidence = evidence("cross-artifact-evidence");
    let out = output_dir("cross-artifact-result");
    assert_success(&qualify(&evidence, "cross-artifact.json", &out));

    let value = record(&out);
    assert_eq!(value["verdict"], "pass");
    let criteria = value["criteria"].as_array().expect("criteria");
    let slippage = criteria
        .iter()
        .find(|criterion| criterion["id"] == "slippage")
        .expect("the slippage criterion is recorded");
    assert_eq!(slippage["status"], "pass");
    assert_eq!(slippage["observed"], 2.0);
    let bars = criteria
        .iter()
        .find(|criterion| criterion["id"] == "model-bars")
        .expect("the model-bars criterion is recorded");
    assert_eq!(bars["status"], "pass");
    assert!(bars["observed"].as_f64().is_some_and(|bars| bars > 0.0));
}

#[test]
fn qualification_is_byte_deterministic() {
    let evidence = evidence("deterministic-evidence");
    let first = output_dir("deterministic-a");
    let second = output_dir("deterministic-b");
    assert_success(&qualify(&evidence, "pass.json", &first));
    assert_success(&qualify(&evidence, "pass.json", &second));
    assert_eq!(
        tree(&first.join("qualification")),
        tree(&second.join("qualification"))
    );
}

#[test]
fn qualification_json_prints_the_envelope() {
    let evidence = evidence("json-evidence");
    let out = output_dir("json-result");
    let criteria = fixture("qualification/pass.json");
    let output = run(&[
        "qualify",
        "--evidence",
        evidence.to_str().expect("the evidence path is UTF-8"),
        "--criteria",
        criteria.to_str().expect("the criteria path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: Value = serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert_eq!(value["verdict"], "pass");
    assert_eq!(value["human_decision"], Value::Null);
    assert_eq!(value["criteria"]["total"], 4);
    assert_eq!(value["criteria"]["pass"], 3);
    assert_eq!(value["criteria"]["fail"], 1);
}

#[test]
fn qualification_refuses_a_missing_manifest() {
    let out = output_dir("missing-manifest");
    fs::create_dir_all(&out).expect("the output root is created");
    let criteria = fixture("qualification/pass.json");
    let output = run(&[
        "qualify",
        "--evidence",
        out.to_str().expect("the evidence path is UTF-8"),
        "--criteria",
        criteria.to_str().expect("the criteria path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_failure(&output);
    assert!(stderr(&output).contains("manifest.json"));
}

#[test]
fn qualification_refuses_a_tampered_provenance() {
    let evidence = evidence("tampered-evidence");
    let copied = output_dir("tampered-copy");
    copy_tree(&evidence, &copied.join("backtest"));
    let provenance = copied.join("backtest/portfolio/provenance.json");
    let mut bytes = fs::read(&provenance).expect("the provenance is readable");
    bytes.push(b' ');
    fs::write(&provenance, bytes).expect("the provenance is rewritten");

    let out = output_dir("tampered-result");
    let output = qualify(&copied.join("backtest"), "pass.json", &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("provenance hash"));
}

#[test]
fn qualification_rejects_an_invalid_criteria_file() {
    let evidence = evidence("invalid-criteria-evidence");
    let out = output_dir("invalid-criteria-result");
    let output = qualify(&evidence, "bad-operator.json", &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("operator"));
    assert!(!out.join("qualification").exists());
}
