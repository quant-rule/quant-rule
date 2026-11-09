//! Binary-level tests for `analytics evaluate`.
//!
//! Every test first runs the real `backtest run` command on the fixture data,
//! then evaluates the resulting evidence bundle. The tests assert the bundle
//! layout, the golden metrics and tables, the decision aggregation, determinism,
//! the evidence refusals, and the absence of a verdict or acceptance.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");

const TABLES: [&str; 9] = [
    "benchmarks.csv",
    "by_timeframe.csv",
    "costs.csv",
    "decisions.csv",
    "equity.csv",
    "funding.csv",
    "monthly_returns.csv",
    "risk_events.csv",
    "trades.csv",
];

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/p3-analytics-tests")
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

fn analytics(evidence: &Path, out: &Path) -> Output {
    run(&[
        "analytics",
        "evaluate",
        "--evidence",
        evidence.to_str().expect("the evidence path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ])
}

fn metrics(out: &Path) -> Value {
    let path = out.join("analytics/metrics.json");
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("the analytics metrics are JSON")
}

#[test]
fn analytics_writes_the_bundle_and_matches_the_golden_fixture() {
    let evidence = evidence("golden-evidence");
    let out = output_dir("golden-result");
    let output = analytics(&evidence, &out);
    assert_success(&output);

    let root = out.join("analytics");
    let files = tree(&root);
    for name in ["metrics.json", "summary.txt", "provenance.json"] {
        assert!(files.contains_key(name), "{name} is missing");
    }
    for table in TABLES {
        assert!(
            files.contains_key(&format!("tables/{table}")),
            "tables/{table} is missing"
        );
    }
    for unsupported in ["by_regime.csv", "by_instrument.csv"] {
        assert!(
            !files.contains_key(&format!("tables/{unsupported}")),
            "unsupported table {unsupported} must not be synthesized"
        );
    }

    for name in ["metrics.json", "summary.txt", "provenance.json"] {
        let expected = fs::read(fixture(&format!("analytics/expected/{name}")))
            .unwrap_or_else(|error| panic!("read expected {name}: {error}"));
        assert_eq!(
            files[name], expected,
            "{name} must match the checked-in golden fixture"
        );
    }
    for table in TABLES {
        let name = format!("tables/{table}");
        let expected = fs::read(fixture(&format!("analytics/expected/{name}")))
            .unwrap_or_else(|error| panic!("read expected {name}: {error}"));
        assert_eq!(
            files[&name], expected,
            "{name} must match the checked-in golden fixture"
        );
    }

    let value = metrics(&out);
    assert_eq!(value["schema"], "quantrule.analytics-metrics/v1");
    assert_eq!(value["stage"], "analytics");
    assert_eq!(value["metrics"]["trade_count"], 1);
    assert_eq!(value["metrics"]["win_rate"], 0.0);
    assert_eq!(value["metrics"]["profit_factor"], 0.0);
    assert_eq!(value["metrics"]["decisions"]["hold"], 51);
    assert_eq!(value["metrics"]["decisions"]["enter"], 1);
    assert!(
        value.get("verdict").is_none(),
        "analytics writes no verdict"
    );
    assert!(
        value.get("human_decision").is_none(),
        "analytics writes no human decision"
    );
    let provenance: Value =
        serde_json::from_slice(&files["provenance.json"]).expect("the provenance is JSON");
    assert!(
        provenance.get("verdict").is_none(),
        "the provenance writes no verdict"
    );
    assert!(
        provenance.get("human_decision").is_none(),
        "the provenance writes no human decision"
    );
    let summary = String::from_utf8(files["summary.txt"].clone()).expect("the summary is UTF-8");
    assert!(
        !summary.to_lowercase().contains("verdict"),
        "the summary writes no verdict"
    );
    assert!(
        !summary.contains("human_decision"),
        "the summary writes no human decision"
    );
    assert_eq!(value["unsupported"][0]["name"], "by_regime");
    assert_eq!(
        value["unsupported"][0]["reason_code"],
        "regime_label_not_published"
    );
    assert_eq!(value["unsupported"][1]["name"], "by_instrument");
    assert_eq!(
        value["unsupported"][1]["reason_code"],
        "instrument_identity_not_published"
    );
}

#[test]
fn analytics_decision_counts_match_the_strategy_bundle() {
    let evidence = evidence("decision-evidence");
    let out = output_dir("decision-result");
    assert_success(&analytics(&evidence, &out));

    let decisions_text = fs::read_to_string(evidence.join("strategy/decisions.ndjson"))
        .expect("the decisions are readable");
    let mut total = 0_u64;
    let mut hold = 0_u64;
    let mut enter = 0_u64;
    for line in decisions_text
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let value: Value = serde_json::from_str(line).expect("a decision is JSON");
        total += 1;
        match value["decision"].as_str().expect("a decision kind") {
            "hold" => hold += 1,
            "enter" => enter += 1,
            other => panic!("unexpected decision {other}"),
        }
    }
    let value = metrics(&out);
    assert_eq!(value["metrics"]["decisions"]["hold"], hold);
    assert_eq!(value["metrics"]["decisions"]["enter"], enter);
    let by_timeframe = fs::read_to_string(out.join("analytics/tables/by_timeframe.csv"))
        .expect("the by-timeframe table is readable");
    let row = by_timeframe
        .lines()
        .nth(1)
        .expect("the by-timeframe table has one row");
    let cells = row.split(',').collect::<Vec<_>>();
    assert_eq!(cells[0], "5m");
    assert_eq!(cells[1], total.to_string());
    assert_eq!(cells[2], hold.to_string());
    assert_eq!(cells[3], enter.to_string());
}

#[test]
fn analytics_is_byte_deterministic() {
    let evidence = evidence("deterministic-evidence");
    let first = output_dir("deterministic-a");
    let second = output_dir("deterministic-b");
    assert_success(&analytics(&evidence, &first));
    assert_success(&analytics(&evidence, &second));
    assert_eq!(
        tree(&first.join("analytics")),
        tree(&second.join("analytics"))
    );
}

#[test]
fn analytics_json_prints_the_envelope() {
    let evidence = evidence("json-evidence");
    let out = output_dir("json-result");
    let output = run(&[
        "analytics",
        "evaluate",
        "--evidence",
        evidence.to_str().expect("the evidence path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: Value = serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert_eq!(value["trade_count"], 1);
    assert_eq!(value["table_count"], 9);
    assert!(value["metrics"].as_str().is_some());
    assert!(value["tables"].as_str().is_some());
}

#[test]
fn analytics_refuses_a_missing_manifest() {
    let out = output_dir("missing-manifest");
    fs::create_dir_all(&out).expect("the output root is created");
    let output = analytics(&out, &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("manifest.json"));
}

#[test]
fn analytics_refuses_a_tampered_provenance() {
    let evidence = evidence("tampered-evidence");
    let copied = output_dir("tampered-copy");
    copy_tree(&evidence, &copied.join("backtest"));
    let provenance = copied.join("backtest/portfolio/provenance.json");
    let mut bytes = fs::read(&provenance).expect("the provenance is readable");
    bytes.push(b' ');
    fs::write(&provenance, bytes).expect("the provenance is rewritten");

    let out = output_dir("tampered-result");
    let output = analytics(&copied.join("backtest"), &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("provenance hash"));
}

#[test]
fn analytics_refuses_a_missing_table() {
    let evidence = evidence("missing-table-evidence");
    let copied = output_dir("missing-table-copy");
    copy_tree(&evidence, &copied.join("backtest"));
    fs::remove_file(copied.join("backtest/portfolio/trades.csv"))
        .expect("the trades table is removed");

    let out = output_dir("missing-table-result");
    let output = analytics(&copied.join("backtest"), &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("trades.csv"));
}

#[test]
fn analytics_refuses_an_unsupported_metrics_schema() {
    let evidence = evidence("schema-evidence");
    let copied = output_dir("schema-copy");
    copy_tree(&evidence, &copied.join("backtest"));
    let path = copied.join("backtest/portfolio/metrics.json");
    let text = fs::read_to_string(&path).expect("the metrics are readable");
    let value = text.replace(
        "quantrule.portfolio-metrics/v1",
        "quantrule.portfolio-metrics/v0",
    );
    fs::write(&path, value).expect("the metrics are rewritten");

    let out = output_dir("schema-result");
    let output = analytics(&copied.join("backtest"), &out);
    assert_failure(&output);
    assert!(stderr(&output).contains("quantrule.portfolio-metrics/v1"));
}

#[test]
fn analytics_tables_declare_their_headers() {
    let evidence = evidence("header-evidence");
    let out = output_dir("header-result");
    assert_success(&analytics(&evidence, &out));

    let value = metrics(&out);
    for table in value["tables"].as_array().expect("tables") {
        let path = out
            .join("analytics")
            .join(table["path"].as_str().expect("the table path is a string"));
        let text = fs::read_to_string(&path).expect("the table is readable");
        let header = text.lines().next().expect("the table has a header");
        let columns = table["columns"]
            .as_array()
            .expect("the columns are an array")
            .iter()
            .map(|column| column.as_str().expect("a column is a string"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(header, columns, "{} header drift", table["name"]);
        let rows = text
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .count();
        assert_eq!(
            table["rows"].as_u64().expect("a row count") as usize,
            rows,
            "{} row count drift",
            table["name"]
        );
    }
}
