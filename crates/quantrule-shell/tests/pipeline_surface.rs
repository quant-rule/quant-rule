//! End-to-end verification of the S3.x command surface.
//!
//! The tests run every stage command independently on the fixture data, then run
//! the coupled backtest and its qualification and analytics consumers. They
//! assert the handoff provenance, the bundle layouts, the coupled-run reuse of
//! the stage engines, and byte determinism. No test claims an edge or an
//! acceptance decision.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

const MODEL_FILES: [&str; 8] = [
    "events.csv",
    "legs.csv",
    "levels.csv",
    "provenance.json",
    "readings.csv",
    "report.json",
    "result.json",
    "summary.txt",
];
const STRATEGY_FILES: [&str; 3] = ["decisions.ndjson", "provenance.json", "summary.txt"];
const EXECUTION_FILES: [&str; 5] = [
    "fills.csv",
    "orders.csv",
    "provenance.json",
    "rejections.csv",
    "summary.txt",
];
const PORTFOLIO_FILES: [&str; 7] = [
    "equity.csv",
    "funding.csv",
    "metrics.json",
    "provenance.json",
    "risk_events.csv",
    "summary.txt",
    "trades.csv",
];
const QUALIFICATION_FILES: [&str; 4] = [
    "criteria.csv",
    "provenance.json",
    "qualification.json",
    "qualification.md",
];
const ANALYTICS_FILES: [&str; 11] = [
    "metrics.json",
    "provenance.json",
    "summary.txt",
    "tables/by_timeframe.csv",
    "tables/costs.csv",
    "tables/decisions.csv",
    "tables/equity.csv",
    "tables/funding.csv",
    "tables/monthly_returns.csv",
    "tables/risk_events.csv",
    "tables/trades.csv",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-pipeline-tests")
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

fn arg(path: &Path) -> &str {
    path.to_str().expect("the path is UTF-8")
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

fn file_names(root: &Path) -> Vec<String> {
    tree(root).into_keys().collect()
}

fn read_json(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("the artifact is JSON")
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn decisions(path: &Path) -> Vec<Value> {
    read_text(path)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("the decision is JSON"))
        .collect()
}

fn assert_files(root: &Path, expected: &[&str]) {
    assert_eq!(
        file_names(root),
        expected,
        "bundle layout at {}",
        root.display()
    );
}

fn assert_common_provenance(provenance: &Value, stage: &str, artifact: &str) {
    assert_eq!(provenance["schema"], "quantrule.stage-provenance/v1");
    assert_eq!(provenance["stage"], stage);
    assert_eq!(provenance["artifact"], artifact);
    assert_eq!(provenance["data_version"], "test:v1");
    assert_eq!(provenance["code_commit"], "unknown");
    assert_eq!(provenance["seed"], Value::Null);
    assert_eq!(provenance["period"]["from"], "2025-01-01");
    assert_eq!(provenance["period"]["to"], "2025-01-01");
    assert!(
        provenance["config_id"]
            .as_str()
            .is_some_and(|id| !id.is_empty()),
        "{stage} needs a config identity"
    );
}

fn standalone_chain(base: &Path) {
    fs::create_dir_all(base).expect("the standalone root is created");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    assert_success(&run(&[
        "report",
        "model",
        "--config",
        arg(&config),
        "--out",
        arg(base),
    ]));
    assert_success(&run(&[
        "strategy",
        "run",
        "--model",
        arg(&base.join("model")),
        "--strategy",
        arg(&strategy),
        "--out",
        arg(base),
    ]));
    assert_success(&run(&[
        "execution",
        "run",
        "--decisions",
        arg(&base.join("strategy")),
        "--config",
        arg(&config),
        "--out",
        arg(base),
    ]));
    assert_success(&run(&[
        "portfolio",
        "replay",
        "--fills",
        arg(&base.join("execution")),
        "--config",
        arg(&config),
        "--out",
        arg(base),
    ]));
}

fn coupled_chain(base: &Path) {
    fs::create_dir_all(base).expect("the coupled root is created");
    let config = fixture("config-run/run.json");
    let strategy = fixture("strategy/event-composite.json");
    let criteria = fixture("qualification/pass.json");
    assert_success(&run(&[
        "backtest",
        "run",
        "--config",
        arg(&config),
        "--strategy",
        arg(&strategy),
        "--out",
        arg(base),
    ]));
    assert_success(&run(&[
        "qualify",
        "--evidence",
        arg(&base.join("backtest")),
        "--criteria",
        arg(&criteria),
        "--out",
        arg(base),
    ]));
    assert_success(&run(&[
        "analytics",
        "evaluate",
        "--evidence",
        arg(&base.join("backtest")),
        "--out",
        arg(base),
    ]));
}

#[test]
fn every_stage_runs_on_its_own_and_hands_off_its_artifact() {
    let base = output_dir("standalone").join("run");
    standalone_chain(&base);

    assert_files(&base.join("model"), &MODEL_FILES);
    assert_files(&base.join("strategy"), &STRATEGY_FILES);
    assert_files(&base.join("execution"), &EXECUTION_FILES);
    assert_files(&base.join("portfolio"), &PORTFOLIO_FILES);

    let model = read_json(&base.join("model/provenance.json"));
    assert_eq!(model["schema"], "quantrule.provenance/v1");
    assert_eq!(model["report"], "model");
    assert_eq!(model["data_version"], "test:v1");
    assert!(model["config_hash"].as_str().is_some());

    let strategy = read_json(&base.join("strategy/provenance.json"));
    assert_common_provenance(&strategy, "strategy", "decisions");
    assert_eq!(strategy["source_artifacts"][0]["stage"], "model");
    assert_eq!(
        strategy["source_artifacts"][0]["schema"],
        "quantrule.provenance/v1"
    );
    assert_eq!(
        strategy["source_artifacts"][0]["config_id"],
        model["config_hash"]
    );

    let execution = read_json(&base.join("execution/provenance.json"));
    assert_common_provenance(&execution, "execution", "fills");
    assert_eq!(execution["source_artifacts"][0]["stage"], "strategy");
    assert_eq!(
        execution["source_artifacts"][0]["schema"],
        "quantrule.stage-provenance/v1"
    );
    assert_eq!(
        execution["source_artifacts"][0]["config_id"],
        strategy["config_id"]
    );

    let portfolio = read_json(&base.join("portfolio/provenance.json"));
    assert_common_provenance(&portfolio, "portfolio", "portfolio");
    assert_eq!(portfolio["source_artifacts"][0]["stage"], "execution");
    assert_eq!(
        portfolio["source_artifacts"][0]["config_id"],
        execution["config_id"]
    );
}

#[test]
fn the_coupled_run_reuses_the_stage_engines_and_artifacts() {
    let root = output_dir("coupled");
    let standalone = root.join("standalone");
    let coupled = root.join("coupled");
    standalone_chain(&standalone);
    coupled_chain(&coupled);

    let backtest = coupled.join("backtest");
    assert_eq!(
        file_names(&backtest.join("model")),
        MODEL_FILES,
        "the coupled model must use the model bundle layout"
    );
    assert_eq!(
        file_names(&backtest.join("strategy")),
        STRATEGY_FILES,
        "the coupled strategy must use the decisions bundle layout"
    );
    assert_eq!(
        file_names(&backtest.join("execution")),
        EXECUTION_FILES,
        "the coupled execution must use the fills bundle layout"
    );
    assert_eq!(
        file_names(&backtest.join("portfolio")),
        PORTFOLIO_FILES,
        "the coupled portfolio must use the portfolio bundle layout"
    );
    assert_eq!(
        tree(&backtest.join("model")),
        tree(&standalone.join("model")),
        "the standalone and coupled model bundles must be byte-identical"
    );

    let manifest = read_json(&backtest.join("manifest.json"));
    assert_eq!(manifest["schema"], "quantrule.backtest-manifest/v1");
    assert_eq!(
        manifest["stage_order"]
            .as_array()
            .expect("stage order")
            .len(),
        4
    );
    for stage in manifest["stages"].as_array().expect("stages") {
        assert!(
            stage["provenance_hash"]
                .as_str()
                .is_some_and(|hash| hash.len() == 16),
            "every coupled stage needs a provenance hash"
        );
    }
    let provenance = read_json(&backtest.join("provenance.json"));
    assert_eq!(provenance["stage"], "backtest");
    assert_eq!(
        provenance["source_artifacts"]
            .as_array()
            .expect("source artifacts")
            .len(),
        4
    );

    assert_eq!(
        file_names(&coupled.join("qualification")),
        QUALIFICATION_FILES,
        "the qualification bundle must use its declared layout"
    );
    let qualification = read_json(&coupled.join("qualification/qualification.json"));
    assert_eq!(qualification["schema"], "quantrule.qualification-result/v1");
    assert_eq!(qualification["verdict"], "pass");
    assert_eq!(qualification["human_decision"], Value::Null);
    assert!(
        !coupled.join("qualification/decision.md").exists(),
        "the engine must never write the human decision"
    );
    let qualification_provenance = read_json(&coupled.join("qualification/provenance.json"));
    assert_eq!(qualification_provenance["stage"], "qualification");
    assert_eq!(
        qualification_provenance["source_artifacts"][0]["stage"],
        "backtest"
    );

    assert_eq!(
        file_names(&coupled.join("analytics")),
        ANALYTICS_FILES,
        "the analytics bundle must use its declared layout"
    );
    let analytics = read_json(&coupled.join("analytics/metrics.json"));
    assert_eq!(analytics["schema"], "quantrule.analytics-metrics/v1");
    assert_eq!(analytics["metrics"]["trade_count"], 1);
    assert!(analytics.get("verdict").is_none());
    assert!(analytics.get("human_decision").is_none());
    let analytics_provenance = read_json(&coupled.join("analytics/provenance.json"));
    assert_eq!(analytics_provenance["stage"], "analytics");
    assert_eq!(
        analytics_provenance["source_artifacts"][0]["stage"],
        "backtest"
    );
}

#[test]
fn the_standalone_and_coupled_strategy_decisions_agree_on_the_signal() {
    let root = output_dir("decision-parity");
    let standalone = root.join("standalone");
    let coupled = root.join("coupled");
    standalone_chain(&standalone);
    coupled_chain(&coupled);

    let standalone = decisions(&standalone.join("strategy/decisions.ndjson"));
    let coupled = decisions(&coupled.join("backtest/strategy/decisions.ndjson"));
    assert_eq!(standalone.len(), coupled.len());
    for (standalone, coupled) in standalone.iter().zip(coupled.iter()) {
        assert_eq!(standalone["bar_index"], coupled["bar_index"]);
        assert_eq!(standalone["decision"], coupled["decision"]);
        assert_eq!(standalone["side"], coupled["side"]);
        assert_eq!(standalone["reason"], coupled["reason"]);
    }
}

#[test]
fn the_standalone_pipeline_is_byte_deterministic() {
    let root = output_dir("standalone-deterministic");
    let first = root.join("a");
    let second = root.join("b");
    standalone_chain(&first);
    standalone_chain(&second);
    for stage in ["model", "strategy", "execution", "portfolio"] {
        assert_eq!(
            tree(&first.join(stage)),
            tree(&second.join(stage)),
            "the standalone {stage} stage must be byte-identical across runs"
        );
    }
}

#[test]
fn the_coupled_pipeline_is_byte_deterministic() {
    let root = output_dir("coupled-deterministic");
    let first = root.join("a");
    let second = root.join("b");
    coupled_chain(&first);
    coupled_chain(&second);
    for stage in ["backtest", "qualification", "analytics"] {
        assert_eq!(
            tree(&first.join(stage)),
            tree(&second.join(stage)),
            "the coupled {stage} bundle must be byte-identical across runs"
        );
    }
}

#[test]
fn the_execution_bundles_record_their_sizing_basis() {
    let root = output_dir("sizing");
    let standalone = root.join("standalone");
    let coupled = root.join("coupled");
    standalone_chain(&standalone);
    coupled_chain(&coupled);

    let standalone_summary = read_text(&standalone.join("execution/summary.txt"));
    assert!(
        standalone_summary.contains("sizing_basis: initial_cash"),
        "the standalone execution sizes from initial cash"
    );
    let coupled_summary = read_text(&coupled.join("backtest/execution/summary.txt"));
    assert!(
        coupled_summary.contains("sizing_basis: ledger_current"),
        "the coupled execution sizes from the ledger's current equity"
    );
}

#[test]
fn the_surface_help_lists_every_stage_command() {
    let output = run(&["help"]);
    assert_success(&output);
    let text = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    for command in [
        "candles",
        "candles-tolerant",
        "timeframes",
        "events",
        "report model",
        "strategy explain",
        "strategy run",
        "execution run",
        "portfolio replay",
        "backtest run",
        "qualify",
        "analytics evaluate",
        "policy check",
        "fetch",
        "help",
    ] {
        assert!(text.contains(command), "help must list {command}");
    }
}
