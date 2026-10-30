//! Binary-level tests for the config-driven `backtest` command.
//!
//! The command must write all five bundles from one run and each bundle must be
//! byte-identical to the standalone `report` bundle for the same config. A
//! non-empty root is refused before any write, and a failure leaves no partial
//! root.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const CONFIG: &str = "config-run/run.json";
const ENGINES: [&str; 5] = ["model", "strategy", "execution", "portfolio", "analytical"];

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
fn one_command_writes_the_five_bundles() {
    let out = output_dir("backtest-command");
    let config = fixture(CONFIG);
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    for engine in ENGINES {
        assert!(
            out.join(engine).join("report.json").exists(),
            "{engine}/report.json is missing"
        );
        assert!(
            out.join(engine).join("provenance.json").exists(),
            "{engine}/provenance.json is missing"
        );
    }

    // The stdout lists the five bundle paths, in contract order.
    let paths: Vec<String> = String::from_utf8(output.stdout)
        .expect("stdout is UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(paths.len(), 5);
    for (path, engine) in paths.iter().zip(ENGINES) {
        assert!(path.ends_with(engine), "{path} does not end in {engine}");
    }
}

#[test]
fn each_bundle_matches_the_standalone_report() {
    let config = fixture(CONFIG);
    let backtest = output_dir("backtest-compare");
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        backtest.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);

    for engine in ENGINES {
        let standalone = output_dir(&format!("backtest-compare-{engine}"));
        let output = run(&[
            "report",
            engine,
            "--config",
            config.to_str().expect("the config path is UTF-8"),
            "--out",
            standalone.to_str().expect("the output path is UTF-8"),
        ]);
        assert_success(&output);
        let left = backtest.join(engine);
        let right = standalone.join(engine);
        let mut names: Vec<String> = fs::read_dir(&left)
            .expect("the bundle directory is readable")
            .map(|entry| {
                entry
                    .expect("the entry is readable")
                    .file_name()
                    .to_str()
                    .expect("the name is UTF-8")
                    .to_owned()
            })
            .collect();
        names.sort();
        for name in names {
            assert_eq!(
                read(&left.join(&name)),
                read(&right.join(&name)),
                "{engine}/{name} differs from the standalone report"
            );
        }
    }
}

#[test]
fn the_json_summary_names_the_bundles_and_the_metrics() {
    let out = output_dir("backtest-json");
    let config = fixture(CONFIG);
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let summary = String::from_utf8(output.stdout).expect("stdout is UTF-8");
    assert!(summary.contains("\"report\":\"backtest\""));
    assert!(summary.contains("\"data_version\":\"test:v1\""));
    assert!(summary.contains("\"final_equity\":10000"));
    for engine in ENGINES {
        assert!(summary.contains(engine), "the summary omits {engine}");
    }
}

#[test]
fn a_non_empty_root_is_refused_before_any_write() {
    let out = output_dir("backtest-non-empty");
    fs::create_dir_all(&out).expect("the root is created");
    fs::write(out.join("old.txt"), b"old").expect("the old file is written");

    let config = fixture(CONFIG);
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert!(error(&output).contains("must be absent or empty"));
    assert_eq!(read(&out.join("old.txt")), b"old");
    assert!(!out.join("model").exists());
}

#[test]
fn a_failed_run_leaves_no_partial_root() {
    let out = output_dir("backtest-failed");
    let config = fixture("config-run/run-missing-source.json");
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert!(!out.exists(), "a failed run left the root behind");
}

#[test]
fn backtest_rejects_a_native_h1_source_before_any_write() {
    let root = output_dir("backtest-h1-source");
    let config_dir = root.join("config");
    fs::create_dir_all(&config_dir).expect("the config directory is created");
    for entry in fs::read_dir(fixture("config-run")).expect("the config fixture is readable") {
        let entry = entry.expect("the config entry is readable");
        fs::copy(entry.path(), config_dir.join(entry.file_name()))
            .expect("the config file is copied");
    }

    // Point the copied config at the native H1 fixture. Its one-row 2025-01-01
    // bound would otherwise hide the coarser cadence.
    let h1 = fixture("timeframes-h1.csv");
    let data_path = config_dir.join("data.json");
    let data = fs::read_to_string(&data_path).expect("data.json is readable");
    let data = data.replace("../state_m1.csv", &h1.display().to_string());
    fs::write(&data_path, data).expect("data.json is rewritten");

    let out = root.join("out");
    let config = config_dir.join("run.json");
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert_eq!(
        error(&output),
        format!(
            "error: {}: source cadence is 3600s, not 60s; provide an M1 source file for the timeframe ladder\n",
            h1.display()
        )
    );
    assert!(!out.exists(), "a rejected source created the bundle root");
}

#[test]
fn backtest_requires_a_config() {
    let output = run(&["backtest"]);
    assert!(!output.status.success());
    assert!(error(&output).contains("--config is required"));
}
