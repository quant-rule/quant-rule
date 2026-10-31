//! Binary-level tests for the cross-engine diagnosis.
//!
//! The tests use the small checked-in run fixture, so they exercise the real
//! command, the real pipeline, and the real artifact writer without the full
//! year of source data.

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

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn artifact_names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .expect("the diagnosis directory is readable")
        .map(|entry| {
            entry
                .expect("the directory entry is readable")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn run_diagnose(name: &str) -> PathBuf {
    let out = output_dir(name);
    let config = fixture("config-run/run.json");
    let output = run(&[
        "backtest",
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
        "--diagnose",
    ]);
    assert_success(&output);
    out
}

fn run_lab_diagnose(lab: &Path, name: &str) -> PathBuf {
    let out = output_dir(name);
    let config = fixture("config-run/run.json");
    let output = run(&[
        "lab",
        "diagnose",
        lab.to_str().expect("the lab path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);
    out
}

fn check_status(diagnosis: &Path, id: &str) -> String {
    let json: serde_json::Value = serde_json::from_slice(
        &fs::read(diagnosis.join("diagnosis.json")).expect("diagnosis.json is readable"),
    )
    .expect("diagnosis.json is valid JSON");
    json["checks"]["items"]
        .as_array()
        .expect("the checks are an array")
        .iter()
        .find(|check| check["id"] == id)
        .expect("the check exists")["status"]
        .as_str()
        .expect("the status is a string")
        .to_owned()
}

#[test]
fn backtest_diagnose_writes_the_expected_artifact_set() {
    let out = run_diagnose("diagnosis-command");
    let diagnosis = out.join("diagnosis");
    let names = artifact_names(&diagnosis);
    let expected = [
        "00-run-scorecard.png",
        "10-data-quality.png",
        "30-strategy-funnel.png",
        "40-execution-quality.png",
        "50-portfolio-attribution.png",
        "checks.csv",
        "data-quality.csv",
        "diagnosis.json",
        "execution-rejections.csv",
        "funnel.csv",
        "manifest.txt",
        "model-health.csv",
        "portfolio-monthly.csv",
        "strategy-exit-reasons.csv",
        "strategy-funnel.csv",
        "summary.txt",
        "trades_enriched.csv",
    ];
    assert_eq!(names, expected);

    let summary = fs::read_to_string(diagnosis.join("summary.txt"))
        .expect("the diagnosis summary is readable");
    assert!(summary.starts_with("QuantRule diagnosis\n"));
    assert!(summary.contains("Scorecard\n"));
    assert!(summary.contains("Funnel\n"));
    assert!(summary.contains("Limitations\n"));

    let json: serde_json::Value = serde_json::from_slice(
        &fs::read(diagnosis.join("diagnosis.json")).expect("diagnosis.json is readable"),
    )
    .expect("diagnosis.json is valid JSON");
    assert_eq!(json["diagnosis_version"], "2026-10-31");
    assert!(json["checks"]["items"].as_array().is_some());
    assert!(json["trades"].as_array().is_some());
}

#[test]
fn manifest_lists_every_artifact_including_itself() {
    let out = run_diagnose("diagnosis-manifest");
    let diagnosis = out.join("diagnosis");
    let manifest =
        fs::read_to_string(diagnosis.join("manifest.txt")).expect("the manifest is readable");
    let names = artifact_names(&diagnosis);

    let header = manifest
        .lines()
        .find(|line| line.starts_with("artifacts: "))
        .expect("the artifact count is present");
    assert_eq!(
        header,
        format!("artifacts: {} (including this manifest)", names.len())
    );

    let mut rows = BTreeMap::new();
    let mut in_table = false;
    for line in manifest.lines() {
        if line == "name,bytes" {
            in_table = true;
            continue;
        }
        if !in_table {
            continue;
        }
        let (name, bytes) = line.split_once(',').expect("a manifest row");
        rows.insert(
            name.to_owned(),
            bytes.parse::<u64>().expect("a byte length"),
        );
    }

    let mut listed = rows.keys().cloned().collect::<Vec<_>>();
    listed.sort();
    assert_eq!(listed, names);
    for (name, bytes) in &rows {
        let length = fs::metadata(diagnosis.join(name))
            .expect("the listed artifact exists")
            .len();
        assert_eq!(*bytes, length, "{name}");
    }
}

#[test]
fn two_diagnoses_of_the_same_run_are_byte_identical() {
    let first = run_diagnose("diagnosis-command-a").join("diagnosis");
    let second = run_diagnose("diagnosis-command-b").join("diagnosis");
    let names = artifact_names(&first);
    assert_eq!(names, artifact_names(&second));
    for name in names {
        assert_eq!(
            fs::read(first.join(&name)).expect("the first artifact is readable"),
            fs::read(second.join(&name)).expect("the second artifact is readable"),
            "artifact {name} differs"
        );
    }
}

#[test]
fn lab_diagnose_requires_a_distinct_oos_data_version() {
    let lab = run_diagnose("diagnosis-oos");
    let main =
        fs::read(lab.join("portfolio/report.json")).expect("the portfolio report is readable");
    let mut oos: serde_json::Value =
        serde_json::from_slice(&main).expect("the portfolio report is valid JSON");
    let main_version = oos["data_version"]
        .as_str()
        .expect("the main report records a data version")
        .to_owned();
    let oos_dir = lab.join("oos/portfolio");
    fs::create_dir_all(&oos_dir).expect("the oos directory is created");
    fs::write(
        oos_dir.join("report.json"),
        serde_json::to_vec_pretty(&oos).expect("the oos report is serializable"),
    )
    .expect("the oos report is written");

    let same = run_lab_diagnose(&lab, "diagnosis-oos-same");
    assert_eq!(check_status(&same, "evidence.out_of_sample"), "unknown");

    oos["data_version"] = serde_json::Value::String(format!("{main_version}:oos"));
    fs::write(
        oos_dir.join("report.json"),
        serde_json::to_vec_pretty(&oos).expect("the oos report is serializable"),
    )
    .expect("the oos report is rewritten");

    let distinct = run_lab_diagnose(&lab, "diagnosis-oos-distinct");
    assert_eq!(check_status(&distinct, "evidence.out_of_sample"), "pass");
}

#[test]
fn lab_list_prints_a_json_array() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/report-tests");
    fs::create_dir_all(&root).expect("the lab root is created");
    let output = run(&[
        "lab",
        "list",
        "--root",
        root.to_str().expect("the root path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the lab list is valid JSON");
    assert!(value.as_array().is_some());
}

#[test]
fn lab_list_validates_a_present_config_hash() {
    let lab = run_diagnose("lab-list-hash");
    let root = output_dir("lab-list-hash-root");
    let target = root.join("bad-lab");
    fs::create_dir_all(target.join("portfolio")).expect("the lab portfolio is created");
    fs::copy(
        lab.join("portfolio/report.json"),
        target.join("portfolio/report.json"),
    )
    .expect("the report is copied");
    let provenance = fs::read_to_string(lab.join("portfolio/provenance.json"))
        .expect("the provenance is readable");
    let mut value: serde_json::Value =
        serde_json::from_str(&provenance).expect("the provenance is valid JSON");

    // Eleven ASCII bytes, two two-byte characters, and one ASCII byte: 16 bytes,
    // but byte 12 falls inside a multi-byte character.
    value["config_hash"] = serde_json::Value::String(String::from("aaaaaaaaaaaééa"));
    fs::write(
        target.join("portfolio/provenance.json"),
        serde_json::to_vec_pretty(&value).expect("the provenance is serializable"),
    )
    .expect("the provenance is written");

    let output = run(&[
        "lab",
        "list",
        "--root",
        root.to_str().expect("the root path is UTF-8"),
    ]);
    assert!(!output.status.success());
    assert_ne!(
        output.status.code(),
        Some(101),
        "the command must not panic"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("config_hash"),
        "unexpected stderr: {stderr}"
    );

    value["config_hash"] = serde_json::Value::String(String::from("0123456789abcdef"));
    fs::write(
        target.join("portfolio/provenance.json"),
        serde_json::to_vec_pretty(&value).expect("the provenance is serializable"),
    )
    .expect("the provenance is rewritten");
    let output = run(&[
        "lab",
        "list",
        "--root",
        root.to_str().expect("the root path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let listed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the lab list is valid JSON");
    assert_eq!(listed[0]["config_hash"], "0123456789abcdef");

    fs::remove_file(target.join("portfolio/provenance.json")).expect("the provenance is removed");
    let output = run(&[
        "lab",
        "list",
        "--root",
        root.to_str().expect("the root path is UTF-8"),
        "--json",
    ]);
    assert_success(&output);
    let listed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the lab list is valid JSON");
    assert_eq!(listed[0]["config_hash"], "");
}

#[test]
fn lab_diagnose_reproduces_the_backtest_diagnosis() {
    let lab = run_diagnose("diagnosis-command-lab");
    let out = output_dir("diagnosis-command-lab-diagnose");
    let config = fixture("config-run/run.json");
    let output = run(&[
        "lab",
        "diagnose",
        lab.to_str().expect("the lab path is UTF-8"),
        "--config",
        config.to_str().expect("the config path is UTF-8"),
        "--out",
        out.to_str().expect("the output path is UTF-8"),
    ]);
    assert_success(&output);
    let first = lab.join("diagnosis");
    let second = out;
    let names = artifact_names(&first);
    assert_eq!(names, artifact_names(&second));
    for name in names {
        assert_eq!(
            fs::read(first.join(&name)).expect("the lab artifact is readable"),
            fs::read(second.join(&name)).expect("the rerun artifact is readable"),
            "artifact {name} differs"
        );
    }
}
