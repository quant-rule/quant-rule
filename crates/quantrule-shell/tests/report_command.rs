//! Binary-level tests for the config-driven `report` command.
//!
//! The model bundle is compared byte for byte with the legacy `market-report`
//! artifacts for the same data and period. The strategy bundle is checked for
//! its artifact set, its fixed CSV header, determinism, and the boundary that
//! keeps fills, cash, equity, and P&L out of it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const CONFIG: &str = "config-run/run.json";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/report-tests")
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

fn error(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        error(output)
    );
}

fn read(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|source| panic!("cannot read {}: {source}", path.display()))
}

#[test]
fn the_model_bundle_is_byte_identical_to_market_report() {
    let out = output_dir("report-model");
    let config = fixture(CONFIG);
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
        "price-axes.png",
        "events.png",
        "provenance.json",
    ] {
        assert!(bundle.join(name).exists(), "{name} is missing");
    }

    let legacy = output_dir("report-model-legacy");
    let legacy_output = run(&[
        "market-report",
        fixture("state_m1.csv")
            .to_str()
            .expect("the fixture path is UTF-8"),
        "--data-version",
        "test:v1",
        "--from",
        "2025-01-01",
        "--to",
        "2025-01-01",
        "--out",
        legacy.to_str().expect("the legacy path is UTF-8"),
    ]);
    assert_success(&legacy_output);
    for name in ["summary.txt", "report.json", "price-axes.png", "events.png"] {
        assert_eq!(
            read(&bundle.join(name)),
            read(&legacy.join(name)),
            "{name} differs from the legacy report"
        );
    }

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"report\":\"model\""));
    assert!(provenance.contains("\"schema\":\"quantrule.provenance/v1\""));
    assert!(!provenance.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn the_strategy_bundle_has_its_fixed_shape() {
    let out = output_dir("report-strategy");
    let config = fixture(CONFIG);
    let output = run(&[
        "report",
        "strategy",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = out.join("strategy");
    for name in [
        "summary.txt",
        "report.json",
        "decisions.csv",
        "price-decisions.png",
        "provenance.json",
    ] {
        assert!(bundle.join(name).exists(), "{name} is missing");
    }

    let csv = String::from_utf8(read(&bundle.join("decisions.csv"))).expect("csv is UTF-8");
    assert_eq!(
        csv.lines().next(),
        Some(
            "type,side,timeframe,timeframe_secs,bar,datetime,direction,momentum_force,structure_score,reason"
        )
    );
    let summary = String::from_utf8(read(&bundle.join("summary.txt"))).expect("summary is UTF-8");
    assert!(summary.contains("bars: 17"), "{summary}");
    assert!(summary.contains("open_position: flat"), "{summary}");

    let report = String::from_utf8(read(&bundle.join("report.json"))).expect("json is UTF-8");
    assert!(report.contains("\"report\":\"strategy\""));
    for forbidden in [
        "fill",
        "fills",
        "cash",
        "equity",
        "pnl",
        "commission",
        "funding",
        "slippage",
    ] {
        assert!(
            !report.to_ascii_lowercase().contains(forbidden),
            "the strategy report must not publish {forbidden}"
        );
    }

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"report\":\"strategy\""));
    assert!(!provenance.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn repeated_runs_are_byte_identical() {
    let first = output_dir("report-repeat-a");
    let second = output_dir("report-repeat-b");
    let config = fixture(CONFIG);
    for out in [&first, &second] {
        let output = run(&[
            "report",
            "strategy",
            "--config",
            config.to_str().expect("the config path is UTF-8"),
            "--out",
            out.to_str().expect("the output path is UTF-8"),
        ]);
        assert_success(&output);
    }
    for name in [
        "summary.txt",
        "report.json",
        "decisions.csv",
        "price-decisions.png",
        "provenance.json",
    ] {
        assert_eq!(
            read(&first.join("strategy").join(name)),
            read(&second.join("strategy").join(name)),
            "{name} is not deterministic"
        );
    }
}

#[test]
fn the_execution_and_portfolio_bundles_have_their_shape() {
    let config = fixture(CONFIG);
    for (engine, artifacts) in [
        (
            "execution",
            vec![
                "summary.txt",
                "report.json",
                "orders.csv",
                "fills.csv",
                "rejections.csv",
                "price-fills.png",
                "provenance.json",
            ],
        ),
        (
            "portfolio",
            vec![
                "summary.txt",
                "report.json",
                "trades.csv",
                "equity.csv",
                "equity.png",
                "drawdown.png",
                "provenance.json",
            ],
        ),
    ] {
        let out = output_dir(&format!("report-{engine}-shape"));
        let output = run(&[
            "report",
            engine,
            "--config",
            config.to_str().expect("the config path is UTF-8"),
            "--out",
            out.to_str().expect("the output path is UTF-8"),
        ]);
        assert_success(&output);
        let bundle = out.join(engine);
        for artifact in artifacts {
            assert!(
                bundle.join(artifact).exists(),
                "{engine}/{artifact} is missing"
            );
        }
        let report = String::from_utf8(read(&bundle.join("report.json"))).expect("json is UTF-8");
        assert!(report.contains(&format!("\"report\":\"{engine}\"")));
        let lower = report.to_ascii_lowercase();
        if engine == "execution" {
            for forbidden in [
                "cash",
                "equity",
                "pnl",
                "commission",
                "funding",
                "drawdown",
                "trade",
            ] {
                assert!(
                    !lower.contains(forbidden),
                    "execution must not publish {forbidden}"
                );
            }
        } else {
            for forbidden in ["direction", "momentum", "structure", "agreement", "axis"] {
                assert!(
                    !lower.contains(forbidden),
                    "portfolio must not publish {forbidden}"
                );
            }
        }
        let provenance =
            String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
        assert!(provenance.contains(&format!("\"report\":\"{engine}\"")));
        assert!(!provenance.contains(env!("CARGO_MANIFEST_DIR")));
    }

    let out = output_dir("report-execution-headers");
    let output = run(&[
        "report",
        "execution",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);
    let bundle = out.join("execution");
    assert!(String::from_utf8(read(&bundle.join("orders.csv")))
        .expect("csv is UTF-8")
        .starts_with("order_id,submitted_bar,submitted_time,side,quantity,trigger,state,filled_bar,filled_time,fill_id,rejection_reason\n"));
    assert!(String::from_utf8(read(&bundle.join("fills.csv")))
        .expect("csv is UTF-8")
        .starts_with("fill_id,order_id,side,quantity,reference_price,fill_price,slippage,fee,reason,bar,time,position_side,entry\n"));
    assert!(
        String::from_utf8(read(&bundle.join("rejections.csv")))
            .expect("csv is UTF-8")
            .starts_with("bar,time,reason,decision\n")
    );

    let out = output_dir("report-portfolio-headers");
    let output = run(&[
        "report",
        "portfolio",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);
    let bundle = out.join("portfolio");
    assert!(String::from_utf8(read(&bundle.join("trades.csv")))
        .expect("csv is UTF-8")
        .starts_with("trade_id,side,quantity,entry_bar,entry_time,entry_price,exit_bar,exit_time,exit_price,gross_pnl,execution_fees,commission,funding,net_pnl,return,hold_bars,exit_reason\n"));
    assert!(
        String::from_utf8(read(&bundle.join("equity.csv")))
            .expect("csv is UTF-8")
            .starts_with("date,cash,position_value,equity,peak_equity,drawdown\n")
    );
}

#[test]
fn the_analytical_bundle_has_its_shape() {
    let out = output_dir("report-analytical");
    let config = fixture(CONFIG);
    let output = run(&[
        "report",
        "analytical",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = out.join("analytical");
    for name in [
        "summary.txt",
        "report.json",
        "axes.csv",
        "relations.csv",
        "provenance.json",
    ] {
        assert!(bundle.join(name).exists(), "{name} is missing");
    }

    let axes = String::from_utf8(read(&bundle.join("axes.csv"))).expect("csv is UTF-8");
    assert_eq!(
        axes.lines().next(),
        Some(
            "timeframe_secs,timeframe,axis,count,nulls,min,max,mean,ordinal_0,ordinal_1,ordinal_2,ordinal_3,ordinal_4,ordinal_5,ordinal_6,ordinal_7,ordinal_8,ordinal_9,ordinal_nulls"
        )
    );
    assert_eq!(axes.lines().count(), 17, "four timeframes by four axes");

    let relations = String::from_utf8(read(&bundle.join("relations.csv"))).expect("csv is UTF-8");
    assert_eq!(
        relations.lines().next(),
        Some("timeframe_secs,timeframe,left,right,count,correlation")
    );
    assert_eq!(
        relations.lines().count(),
        25,
        "four timeframes by six pairs"
    );

    let report = String::from_utf8(read(&bundle.join("report.json"))).expect("json is UTF-8");
    assert!(report.contains("\"report\":\"analytical\""));
    assert!(report.contains("\"layer1_rows\""));
    assert!(report.contains("\"relations\":[{"));
    let lower = report.to_ascii_lowercase();
    for forbidden in ["strategy", "orders", "fills", "pnl", "cash", "equity"] {
        assert!(
            !lower.contains(forbidden),
            "analytical must not publish {forbidden}"
        );
    }

    let provenance =
        String::from_utf8(read(&bundle.join("provenance.json"))).expect("provenance is UTF-8");
    assert!(provenance.contains("\"report\":\"analytical\""));
    assert!(!provenance.contains(env!("CARGO_MANIFEST_DIR")));
}

#[test]
fn a_non_empty_bundle_target_is_refused() {
    let out = output_dir("report-non-empty");
    let bundle = out.join("model");
    fs::create_dir_all(&bundle).expect("the bundle directory is created");
    fs::write(bundle.join("old.txt"), b"old").expect("the old file is written");

    let config = fixture(CONFIG);
    let output = run(&[
        "report",
        "model",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert!(error(&output).contains("must be absent or empty"));
    assert_eq!(read(&bundle.join("old.txt")), b"old");
    assert!(!bundle.join("report.json").exists());
}

#[test]
fn report_requires_an_engine_and_a_config() {
    let output = run(&["report"]);
    assert!(!output.status.success());
    assert!(error(&output).contains("missing report engine"));

    let output = run(&["report", "model"]);
    assert!(!output.status.success());
    assert!(error(&output).contains("--config is required"));

    let output = run(&["report", "weekly", "--config", "run.json"]);
    assert!(!output.status.success());
    assert!(error(&output).contains("unknown report engine"));
}

#[test]
fn the_analytical_bundle_uses_the_configured_timeframes() {
    let out = output_dir("report-analytical-subset");
    let config = fixture("config-run/run-analytics-subset.json");
    let output = run(&[
        "report",
        "analytical",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    let bundle = out.join("analytical");
    let axes = String::from_utf8(read(&bundle.join("axes.csv"))).expect("csv is UTF-8");
    assert_eq!(axes.lines().count(), 5, "one timeframe by four axes");
    assert!(
        axes.lines().skip(1).all(|line| line.contains(",15m,")),
        "{axes}"
    );

    let relations = String::from_utf8(read(&bundle.join("relations.csv"))).expect("csv is UTF-8");
    assert_eq!(relations.lines().count(), 7, "one timeframe by six pairs");

    let report = String::from_utf8(read(&bundle.join("report.json"))).expect("json is UTF-8");
    assert!(
        report.contains("\"layer3_rows\":0"),
        "no M1 base, no Layer 3: {report}"
    );
    assert!(report.contains("\"timeframes\":[\"15m\"]"), "{report}");
}
