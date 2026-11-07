//! Binary-level tests for `portfolio replay`.
//!
//! Each test builds the model, strategy, and execution artifacts with the real
//! commands, then replays the fills through the portfolio ledger. The tests
//! assert the bundle shape, the accounting invariant, determinism, the
//! non-empty-target refusal, the data-version handoff, the null metric rule, and
//! short-fill acceptance.

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
        .join("../../target/p3-portfolio-tests")
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

fn build_fills(name: &str) -> (PathBuf, PathBuf) {
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
    assert_success(&run(&[
        "execution",
        "run",
        "--decisions",
        root.join("strategy")
            .to_str()
            .expect("the decisions path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        root.to_str().expect("the output path is UTF-8"),
    ]));
    (root.join("execution"), root)
}

fn replay_args<'a>(fills: &'a str, config: &'a str, out: &'a str) -> [&'a str; 8] {
    [
        "portfolio",
        "replay",
        "--fills",
        fills,
        "--config",
        config,
        "--out",
        out,
    ]
}

#[test]
fn replay_writes_the_portfolio_bundle() {
    let (fills, root) = build_fills("portfolio-bundle");
    let config = fixture("config-run/run.json");
    let output = run(&replay_args(
        fills.to_str().expect("the fills path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        root.to_str().expect("the output path is UTF-8"),
    ));
    assert_success(&output);

    let bundle = root.join("portfolio");
    let files = bundle_files(&bundle);
    for name in [
        "trades.csv",
        "equity.csv",
        "funding.csv",
        "risk_events.csv",
        "metrics.json",
        "summary.txt",
        "provenance.json",
    ] {
        assert!(files.contains_key(name), "{name} is missing");
    }

    let metrics: serde_json::Value =
        serde_json::from_slice(&files["metrics.json"]).expect("metrics are JSON");
    assert_eq!(metrics["schema"], "quantrule.portfolio-metrics/v1");
    assert_eq!(metrics["trades"], 1);
    let initial = metrics["initial_cash"].as_f64().expect("initial cash");
    let final_equity = metrics["final_equity"].as_f64().expect("final equity");
    let net_pnl = metrics["net_pnl"].as_f64().expect("net pnl");
    assert!((final_equity - (initial + net_pnl)).abs() < 1e-9);

    let summary = String::from_utf8(files["summary.txt"].clone()).expect("summary is UTF-8");
    assert!(summary.contains("trades: 1"));
    assert!(summary.contains("net_pnl:"));

    let provenance: serde_json::Value =
        serde_json::from_slice(&files["provenance.json"]).expect("provenance is JSON");
    assert_eq!(provenance["stage"], "portfolio");
    assert_eq!(provenance["artifact"], "portfolio");
    assert_eq!(provenance["source_artifacts"][0]["stage"], "execution");
}

#[test]
fn replay_is_byte_deterministic() {
    let (fills_a, root_a) = build_fills("portfolio-deterministic-a");
    let (fills_b, root_b) = build_fills("portfolio-deterministic-b");
    let config = fixture("config-run/run.json");
    for (fills, root) in [(&fills_a, &root_a), (&fills_b, &root_b)] {
        assert_success(&run(&replay_args(
            fills.to_str().expect("the fills path is UTF-8"),
            config.to_str().expect("the config path is UTF-8"),
            root.to_str().expect("the output path is UTF-8"),
        )));
    }
    assert_eq!(
        bundle_files(&root_a.join("portfolio")),
        bundle_files(&root_b.join("portfolio"))
    );
}

#[test]
fn replay_refuses_a_non_empty_target() {
    let (fills, root) = build_fills("portfolio-non-empty");
    let config = fixture("config-run/run.json");
    let args = replay_args(
        fills.to_str().expect("the fills path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        root.to_str().expect("the output path is UTF-8"),
    );
    assert_success(&run(&args));
    let second = run(&args);
    assert_failure(&second);
    assert!(stderr(&second).contains("must be absent or empty"));
}

#[test]
fn replay_json_prints_the_artifact_envelope() {
    let (fills, root) = build_fills("portfolio-json");
    let config = fixture("config-run/run.json");
    let mut args = replay_args(
        fills.to_str().expect("the fills path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        root.to_str().expect("the output path is UTF-8"),
    )
    .to_vec();
    args.push("--json");
    let output = run(&args);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_str(stdout(&output).trim()).expect("one JSON object");
    assert!(value["bundle"].as_str().is_some());
    assert_eq!(value["artifacts"][0], "trades.csv");
}

#[test]
fn replay_rejects_a_missing_fills_bundle() {
    let config = fixture("config-run/run.json");
    let out = output_dir("portfolio-missing-output");
    let output = run(&replay_args(
        "target/no-such-fills-bundle",
        config.to_str().expect("the config path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(stderr(&output).contains("cannot read"));
}

#[test]
fn replay_rejects_a_data_version_mismatch() {
    let (fills, _) = build_fills("portfolio-version-source");
    let bundle = output_dir("portfolio-version-bundle");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    fs::copy(fills.join("fills.csv"), bundle.join("fills.csv")).expect("the fills copy");
    let mut provenance: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(fills.join("provenance.json")).expect("the provenance is readable"),
    )
    .expect("the provenance is JSON");
    provenance["data_version"] = serde_json::json!("other:v1");
    fs::write(
        bundle.join("provenance.json"),
        format!(
            "{}\n",
            serde_json::to_string(&provenance).expect("the value is JSON")
        ),
    )
    .expect("the provenance is written");

    let config = fixture("config-run/run.json");
    let out = output_dir("portfolio-version-output");
    let output = run(&replay_args(
        bundle.to_str().expect("the bundle path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_failure(&output);
    assert!(stderr(&output).contains("does not match the run"));
}

#[test]
fn replay_reports_undefined_metrics_as_null() {
    let (fills, _) = build_fills("portfolio-null-source");
    let bundle = output_dir("portfolio-null-bundle");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    fs::copy(
        fills.join("provenance.json"),
        bundle.join("provenance.json"),
    )
    .expect("the provenance copy");
    fs::write(
        bundle.join("fills.csv"),
        "fill_id,side,quantity,reference_price,price,slippage,fee,bar,time,reason\n",
    )
    .expect("the empty fills are written");

    let config = fixture("config-run/run.json");
    let out = output_dir("portfolio-null-output");
    let output = run(&replay_args(
        bundle.to_str().expect("the bundle path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_success(&output);
    let metrics: serde_json::Value = serde_json::from_slice(
        &fs::read(out.join("portfolio").join("metrics.json")).expect("metrics are readable"),
    )
    .expect("metrics are JSON");
    assert_eq!(metrics["trades"], 0);
    assert!(metrics["win_rate"].is_null());
    assert!(metrics["profit_factor"].is_null());
    assert!(metrics["average_trade"].is_null());
    let summary =
        fs::read_to_string(out.join("portfolio").join("summary.txt")).expect("summary is readable");
    assert!(summary.contains("win_rate: null"));
    assert!(summary.contains("profit_factor: null"));
}

#[test]
fn replay_accepts_a_short_fill() {
    let (fills, _) = build_fills("portfolio-short-source");
    let bundle = output_dir("portfolio-short-bundle");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    fs::copy(
        fills.join("provenance.json"),
        bundle.join("provenance.json"),
    )
    .expect("the provenance copy");

    let text = fs::read_to_string(fills.join("fills.csv")).expect("the fills are readable");
    let mut short_fills = String::new();
    for (index, line) in text.lines().enumerate() {
        if index == 0 {
            short_fills.push_str(line);
            short_fills.push('\n');
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let side = if index == 1 { "sell" } else { "buy" };
        let reference: f64 = fields[3].parse().expect("the reference price");
        let slippage: f64 = fields[5].parse().expect("the slippage");
        let price = if side == "sell" {
            reference - slippage
        } else {
            reference + slippage
        };
        short_fills.push_str(&format!(
            "{},{side},{},{},{price:.12},{},{},{},{},{}\n",
            fields[0], fields[2], fields[3], fields[5], fields[6], fields[7], fields[8], fields[9]
        ));
    }
    fs::write(bundle.join("fills.csv"), short_fills).expect("the short fills are written");

    let config = fixture("config-run/run.json");
    let out = output_dir("portfolio-short-output");
    let output = run(&replay_args(
        bundle.to_str().expect("the bundle path is UTF-8"),
        config.to_str().expect("the config path is UTF-8"),
        out.to_str().expect("the output path is UTF-8"),
    ));
    assert_success(&output);

    let trades = fs::read_to_string(out.join("portfolio").join("trades.csv"))
        .expect("the trades are readable");
    let side = trades
        .lines()
        .nth(1)
        .and_then(|line| line.split(',').nth(1))
        .expect("the first trade side");
    assert_eq!(side, "short");

    let provenance = fs::read_to_string(out.join("portfolio").join("provenance.json"))
        .expect("the provenance is readable");
    assert!(!provenance.contains("allow_short"));
}
