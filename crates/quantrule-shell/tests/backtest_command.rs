//! Binary-level tests for `backtest run`.
//!
//! The command is the coupled model, strategy, execution, and portfolio runner.
//! The tests assert the stage bundle layout, the manifest provenance hashes, the
//! accounting invariant, determinism, the non-empty-target refusal, and the
//! missing-strategy error.

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
        .join("../../target/p3-backtest-tests")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    path
}

fn strategy_with(name: &str, edit: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
    let source = fixture("strategy/event-composite.json");
    let text = fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("read {}: {error}", source.display()));
    let mut value: serde_json::Value =
        serde_json::from_str(&text).expect("the strategy fixture is JSON");
    edit(&mut value);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-backtest-tests")
        .join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("the strategy directory is created");
    }
    fs::write(
        &path,
        serde_json::to_vec(&value).expect("the strategy is JSON"),
    )
    .expect("the strategy is written");
    path
}

fn run_config_without_timeframe(name: &str, available: &[&str]) -> PathBuf {
    let source = fixture("config-run");
    let target = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-backtest-tests")
        .join(name);
    fs::create_dir_all(&target).expect("the config directory is created");
    for file in ["run.json", "model.json", "execution.json", "portfolio.json"] {
        fs::copy(source.join(file), target.join(file))
            .unwrap_or_else(|error| panic!("copy {file}: {error}"));
    }
    let mut data: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(source.join("data.json")).expect("data.json is readable"),
    )
    .expect("data.json is JSON");
    data["available_timeframes"] = serde_json::json!(available);
    let source_csv = fixture("state_m1.csv")
        .canonicalize()
        .expect("the fixture CSV is present");
    data["source"]["path"] =
        serde_json::json!(source_csv.to_str().expect("the fixture CSV path is UTF-8"));
    fs::write(
        target.join("data.json"),
        serde_json::to_vec_pretty(&data).expect("data.json is JSON"),
    )
    .expect("data.json is written");
    target.join("run.json")
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

fn backtest_args<'a>(config: &'a str, strategy: &'a str, out: &'a str) -> [&'a str; 8] {
    [
        "backtest",
        "run",
        "--config",
        config,
        "--strategy",
        strategy,
        "--out",
        out,
    ]
}

#[test]
fn backtest_writes_the_stage_bundles() {
    let out = output_dir("backtest-bundle");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_success(&output);

    let root = out.join("backtest");
    let files = tree(&root);
    for name in [
        "manifest.json",
        "provenance.json",
        "model/summary.txt",
        "model/report.json",
        "model/result.json",
        "model/readings.csv",
        "model/events.csv",
        "model/levels.csv",
        "model/legs.csv",
        "model/provenance.json",
        "strategy/decisions.ndjson",
        "strategy/summary.txt",
        "strategy/provenance.json",
        "execution/orders.csv",
        "execution/fills.csv",
        "execution/rejections.csv",
        "execution/summary.txt",
        "execution/provenance.json",
        "portfolio/trades.csv",
        "portfolio/equity.csv",
        "portfolio/funding.csv",
        "portfolio/risk_events.csv",
        "portfolio/metrics.json",
        "portfolio/summary.txt",
        "portfolio/provenance.json",
    ] {
        assert!(files.contains_key(name), "{name} is missing");
    }

    let manifest: serde_json::Value =
        serde_json::from_slice(&files["manifest.json"]).expect("the manifest is JSON");
    assert_eq!(manifest["schema"], "quantrule.backtest-manifest/v1");
    assert_eq!(manifest["stage_order"][0], "model");
    assert_eq!(manifest["stage_order"][3], "portfolio");
    assert_eq!(
        manifest["stages"].as_array().expect("stages").len(),
        4,
        "the manifest must name every stage"
    );
    for stage in manifest["stages"].as_array().expect("stages") {
        assert!(
            stage["provenance_hash"]
                .as_str()
                .is_some_and(|hash| !hash.is_empty()),
            "every stage needs a provenance hash"
        );
    }
    let strategy_provenance: serde_json::Value =
        serde_json::from_slice(&files["strategy/provenance.json"])
            .expect("the strategy provenance is JSON");
    assert_eq!(
        manifest["strategy_config_id"], strategy_provenance["config_id"],
        "the manifest must name the strategy config identity"
    );

    let provenance: serde_json::Value =
        serde_json::from_slice(&files["provenance.json"]).expect("the provenance is JSON");
    assert_eq!(provenance["stage"], "backtest");
    assert_eq!(
        provenance["source_artifacts"]
            .as_array()
            .expect("source artifacts")
            .len(),
        4
    );

    let metrics: serde_json::Value =
        serde_json::from_slice(&files["portfolio/metrics.json"]).expect("the metrics are JSON");
    assert_eq!(metrics["trades"], 1);
    let initial = metrics["initial_cash"].as_f64().expect("initial cash");
    let final_equity = metrics["final_equity"].as_f64().expect("final equity");
    let net_pnl = metrics["net_pnl"].as_f64().expect("net pnl");
    assert!((final_equity - (initial + net_pnl)).abs() < 1e-9);
}

#[test]
fn backtest_is_byte_deterministic() {
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let first = output_dir("backtest-deterministic-a");
    let second = output_dir("backtest-deterministic-b");
    for out in [&first, &second] {
        assert_success(&run(&backtest_args(
            config.to_str().expect("the config path is UTF-8"),
            strategy.to_str().expect("the strategy path is UTF-8"),
            out.to_str().expect("the output path is UTF-8"),
        )));
    }
    assert_eq!(
        tree(&first.join("backtest")),
        tree(&second.join("backtest"))
    );
}

#[test]
fn backtest_refuses_a_non_empty_target() {
    let out = output_dir("backtest-non-empty");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let args = backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    );
    assert_success(&run(&args));
    let second = run(&args);
    assert_failure(&second);
    assert!(stderr(&second).contains("must be absent or empty"));
}

#[test]
fn backtest_rejects_a_strategy_execution_timeframe_mismatch() {
    let strategy = strategy_with("strategy-15m-mismatch.json", |value| {
        value["execution_timeframe"] = serde_json::json!("15m");
        value["context_timeframes"] = serde_json::json!(["1h"]);
        value["entry"][0]["timeframe"] = serde_json::json!("15m");
        value["exit"][0]["timeframe"] = serde_json::json!("15m");
    });
    let out = output_dir("backtest-timeframe-mismatch");
    let config = fixture("config-run/run.json");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(
        stderr(&output).contains("declares execution timeframe 900s"),
        "observed: {}",
        stderr(&output)
    );
    assert!(
        !out.join("backtest/model").exists(),
        "the run must fail before a bundle is written"
    );
}

#[test]
fn backtest_rejects_a_condition_outside_execution_and_context() {
    let strategy = strategy_with("strategy-condition-outside-context.json", |value| {
        value["context_timeframes"] = serde_json::json!([]);
        value["entry"][0]["timeframe"] = serde_json::json!("15m");
        value["exit"][0]["timeframe"] = serde_json::json!("15m");
    });
    let out = output_dir("backtest-condition-outside-context");
    let config = fixture("config-run/run.json");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(
        stderr(&output).contains("a strategy condition names 900s"),
        "observed: {}",
        stderr(&output)
    );
    assert!(
        !out.join("backtest/model").exists(),
        "the run must fail before a bundle is written"
    );
}

#[test]
fn backtest_rejects_a_missing_context_timeframe() {
    let config =
        run_config_without_timeframe("backtest-missing-context-config", &["1m", "5m", "15m"]);
    let strategy = strategy_with("strategy-missing-context.json", |value| {
        value["context_timeframes"] = serde_json::json!(["1h"]);
        value["entry"][0]["timeframe"] = serde_json::json!("5m");
        value["exit"][0]["timeframe"] = serde_json::json!("5m");
    });
    let out = output_dir("backtest-missing-context");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(
        stderr(&output).contains("the model does not publish the timeframe 3600s"),
        "observed: {}",
        stderr(&output)
    );
    assert!(
        !out.join("backtest/model").exists(),
        "the run must fail before a bundle is written"
    );
}

#[test]
fn backtest_refuses_an_unrelated_file_in_the_root() {
    let out = output_dir("backtest-root-unrelated");
    fs::create_dir_all(out.join("backtest")).expect("the root is created");
    fs::write(out.join("backtest/junk.txt"), b"junk").expect("the junk file is written");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(stderr(&output).contains("must be absent or empty"));
    assert_eq!(
        fs::read(out.join("backtest/junk.txt")).expect("the junk file remains"),
        b"junk"
    );
    for stage in ["model", "strategy", "execution", "portfolio"] {
        assert!(
            !out.join("backtest").join(stage).exists(),
            "the {stage} stage must not be written"
        );
    }
    assert!(
        !out.join("backtest/manifest.json").exists(),
        "the manifest must not be written"
    );
}

#[test]
fn backtest_json_prints_the_envelope() {
    let out = output_dir("backtest-json");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let mut args = backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        strategy.to_str().expect("the strategy path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    )
    .to_vec();
    args.push("--json");
    let output = run(&args);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert!(value["bundle"].as_str().is_some());
    assert!(value["manifest"].as_str().is_some());
}

#[test]
fn backtest_rejects_a_missing_strategy_config() {
    let out = output_dir("backtest-missing-strategy");
    let config = fixture("config-run/run.json");
    let output = run(&backtest_args(
        config.to_str().expect("the config path is UTF-8"),
        "target/no-such-strategy.json",
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(stderr(&output).contains("no-such-strategy.json"));
}
