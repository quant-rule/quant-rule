//! Binary-level tests for the `lab summary` command.
//!
//! Each test copies the checked-in lab fixture into `target/`, runs the real
//! binary, and asserts on stdout, stderr, the written summary, and the exit
//! status. No test reads `data/`, the wall clock, or the network.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const DATA_VERSION: &str = "test:v1";
const CREATED: &str = "2026-10-29T12:00:00Z";
const CONFIG: &str = "config/examples/solusd-2025/run.json";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn lab_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/report-tests/lab-summary")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the lab fixture directory is created");
    copy_directory(&fixture("lab"), &path);
    path
}

fn copy_directory(source: &Path, target: &Path) {
    for entry in fs::read_dir(source).expect("the fixture directory is readable") {
        let entry = entry.expect("the fixture entry is readable");
        let target = target.join(entry.file_name());
        if entry
            .file_type()
            .expect("the fixture entry type is readable")
            .is_dir()
        {
            fs::create_dir_all(&target).expect("the fixture subdirectory is created");
            copy_directory(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("the fixture file is copied");
        }
    }
}

fn arguments(directory: &Path) -> Vec<String> {
    vec![
        String::from("lab"),
        String::from("summary"),
        directory
            .to_str()
            .expect("the lab path is UTF-8")
            .to_owned(),
        String::from("--role"),
        String::from("researcher"),
        String::from("--data-version"),
        String::from(DATA_VERSION),
        String::from("--config"),
        String::from(CONFIG),
        String::from("--created-utc"),
        String::from(CREATED),
    ]
}

fn run(arguments: &[String]) -> Output {
    Command::new(BIN)
        .current_dir(workspace_root())
        .args(arguments)
        .output()
        .expect("the quantrule binary runs")
}

fn run_interactive(input: &str) -> Output {
    let mut child = Command::new(BIN)
        .current_dir(workspace_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the quantrule binary starts");
    {
        use std::io::Write as _;
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(input.as_bytes())
            .expect("the interactive input is written");
    }
    child.wait_with_output().expect("the binary finishes")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure; stdout: {}",
        stdout(output)
    );
}

fn summary(directory: &Path) -> Vec<u8> {
    fs::read(directory.join("summary.txt")).expect("summary.txt is readable")
}

#[test]
fn writes_the_golden_summary() {
    let directory = lab_dir("golden");
    let output = run(&arguments(&directory));

    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("wrote {}\n", directory.join("summary.txt").display())
    );
    assert_eq!(
        summary(&directory),
        include_bytes!("fixtures/lab-summary.txt")
    );
}

#[test]
fn a_second_run_refuses_to_overwrite_without_force() {
    let directory = lab_dir("existing");
    assert_success(&run(&arguments(&directory)));
    let first = summary(&directory);

    let output = run(&arguments(&directory));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("already exists; use --force"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert_eq!(summary(&directory), first);
}

#[test]
fn force_overwrites_an_existing_summary() {
    let directory = lab_dir("force");
    fs::write(directory.join("summary.txt"), b"stale").expect("the stale summary is written");

    let mut arguments = arguments(&directory);
    arguments.push(String::from("--force"));
    let output = run(&arguments);

    assert_success(&output);
    assert_eq!(
        summary(&directory),
        include_bytes!("fixtures/lab-summary.txt")
    );
}

#[test]
fn a_missing_bundle_fails_before_writing() {
    let directory = lab_dir("missing-bundle");
    fs::remove_file(directory.join("model/report.json")).expect("the model report is removed");

    let output = run(&arguments(&directory));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("missing report bundle"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert!(!directory.join("summary.txt").exists());
}

#[test]
fn a_data_version_mismatch_fails_before_writing() {
    let directory = lab_dir("version-mismatch");
    let mut arguments = arguments(&directory);
    let position = arguments
        .iter()
        .position(|value| value == "--data-version")
        .expect("the data version option is present");
    arguments[position + 1] = String::from("other:v1");

    let output = run(&arguments);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("data version mismatch"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert!(!directory.join("summary.txt").exists());
}

#[test]
fn a_provenance_data_version_mismatch_fails_before_writing() {
    let directory = lab_dir("provenance-mismatch");
    let path = directory.join("portfolio/provenance.json");
    let text = fs::read_to_string(&path).expect("the provenance is readable");
    fs::write(&path, text.replace(DATA_VERSION, "other:v1")).expect("the provenance is rewritten");

    let output = run(&arguments(&directory));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("provenance.json: data version mismatch"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert!(!directory.join("summary.txt").exists());
}

fn insert_config_hash(directory: &Path, bundle: &str, hash: &str) {
    let path = directory.join(bundle).join("provenance.json");
    let text = fs::read_to_string(&path).expect("the provenance is readable");
    let mut value: serde_json::Value =
        serde_json::from_str(&text).expect("the provenance is valid JSON");
    value["config_hash"] = serde_json::Value::String(hash.to_owned());
    let text = serde_json::to_string_pretty(&value).expect("the provenance is serializable");
    fs::write(&path, text).expect("the provenance is rewritten");
}

#[test]
fn a_config_hash_mismatch_fails_before_writing() {
    let directory = lab_dir("config-hash-mismatch");
    insert_config_hash(&directory, "model", "aaaaaaaaaaaaaaaa");
    insert_config_hash(&directory, "strategy", "bbbbbbbbbbbbbbbb");

    let output = run(&arguments(&directory));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("config_hash"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert!(!directory.join("summary.txt").exists());
}

#[test]
fn a_model_version_mismatch_fails_before_writing() {
    let directory = lab_dir("model-version-mismatch");
    let path = directory.join("portfolio/report.json");
    let text = fs::read_to_string(&path).expect("the portfolio report is readable");
    let mut value: serde_json::Value =
        serde_json::from_str(&text).expect("the portfolio report is valid JSON");
    value["model_version"] = serde_json::Value::String(String::from("9.9.9"));
    fs::write(
        &path,
        serde_json::to_string_pretty(&value).expect("the portfolio report is serializable"),
    )
    .expect("the portfolio report is rewritten");

    let output = run(&arguments(&directory));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("model_version"),
        "unexpected stderr: {}",
        stderr(&output)
    );
    assert!(!directory.join("summary.txt").exists());
}

#[test]
fn out_of_sample_and_rerun_evidence_are_checked() {
    let directory = lab_dir("evidence");
    let oos = directory.join("oos");
    let rerun = directory.join("rerun");
    fs::create_dir_all(oos.join("portfolio")).expect("the oos directory is created");
    fs::create_dir_all(rerun.join("portfolio")).expect("the rerun directory is created");
    copy_directory(&directory.join("portfolio"), &oos.join("portfolio"));
    copy_directory(&directory.join("portfolio"), &rerun.join("portfolio"));

    // A copied report keeps the main data version, so it is not evidence.
    let output = run(&arguments(&directory));

    assert_success(&output);
    let text = String::from_utf8(summary(&directory)).expect("the summary is UTF-8");
    assert!(text.contains("[ ] out-of-sample evidence (oos/ uses the same data version)"));
    assert!(text.contains("[x] independent rerun (rerun/ matches)"));

    // A distinct data version is evidence, and the second write needs --force.
    let oos_report = oos.join("portfolio/report.json");
    let text = fs::read_to_string(&oos_report).expect("the oos report is readable");
    let mut value: serde_json::Value =
        serde_json::from_str(&text).expect("the oos report is valid JSON");
    value["data_version"] = serde_json::Value::String(String::from("oos:v1"));
    fs::write(
        &oos_report,
        serde_json::to_string_pretty(&value).expect("the oos report is serializable"),
    )
    .expect("the oos report is rewritten");

    let mut arguments = arguments(&directory);
    arguments.push(String::from("--force"));
    let output = run(&arguments);

    assert_success(&output);
    let text = String::from_utf8(summary(&directory)).expect("the summary is UTF-8");
    assert!(text.contains("[x] out-of-sample evidence (oos/)"));
    assert!(text.contains("[x] independent rerun (rerun/ matches)"));
}

#[test]
fn notes_are_included_verbatim() {
    let directory = lab_dir("notes");
    let mut arguments = arguments(&directory);
    arguments.push(String::from("--notes"));
    arguments.push(
        fixture("lab/notes.md")
            .to_str()
            .expect("the notes path is UTF-8")
            .to_owned(),
    );

    let output = run(&arguments);

    assert_success(&output);
    let text = String::from_utf8(summary(&directory)).expect("the summary is UTF-8");
    assert!(text.contains("  Hypothesis: the ladder-context rule over the configured ladder."));
    assert!(text.contains("  In-sample: 2025-01-01..2025-12-31."));
    assert!(text.contains("  Out-of-sample: not run."));
    assert!(text.contains("  Variations tried: one."));
}

#[test]
fn the_operator_is_recorded() {
    let directory = lab_dir("operator");
    let mut arguments = arguments(&directory);
    arguments.push(String::from("--operator"));
    arguments.push(String::from("agent"));

    let output = run(&arguments);

    assert_success(&output);
    let text = String::from_utf8(summary(&directory)).expect("the summary is UTF-8");
    assert!(text.contains("  operator: agent\n"));
}

#[test]
fn repeated_runs_are_byte_identical() {
    let first = lab_dir("determinism-first");
    let second = lab_dir("determinism-second");

    assert_success(&run(&arguments(&first)));
    assert_success(&run(&arguments(&second)));

    assert_eq!(summary(&first), summary(&second));
}

#[test]
fn interactive_and_non_interactive_output_are_byte_identical() {
    let first = lab_dir("parity-non-interactive");
    let second = lab_dir("parity-interactive");

    let non_interactive = run(&arguments(&first));
    assert_success(&non_interactive);
    assert_eq!(
        stdout(&non_interactive),
        format!("wrote {}\n", first.join("summary.txt").display())
    );

    let line = arguments(&second).join(" ");
    let interactive = run_interactive(&format!("{line}\nquit\n"));
    assert_success(&interactive);
    assert_eq!(
        stdout(&interactive),
        format!("wrote {}\n", second.join("summary.txt").display())
    );
    assert_eq!(summary(&first), summary(&second));
    assert!(stderr(&interactive).contains("quantrule> "));
}

#[test]
fn an_unknown_role_fails_before_any_read() {
    let directory = lab_dir("unknown-role");
    let mut arguments = arguments(&directory);
    let position = arguments
        .iter()
        .position(|value| value == "--role")
        .expect("the role option is present");
    arguments[position + 1] = String::from("wizard");

    let output = run(&arguments);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("--role expects researcher, developer, or validator"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn an_unknown_operator_fails_before_any_read() {
    let directory = lab_dir("unknown-operator");
    let mut arguments = arguments(&directory);
    arguments.push(String::from("--operator"));
    arguments.push(String::from("robot"));

    let output = run(&arguments);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("--operator expects human or agent"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn a_non_canonical_created_utc_fails_before_any_read() {
    let directory = lab_dir("bad-created");
    let mut arguments = arguments(&directory);
    let position = arguments
        .iter()
        .position(|value| value == "--created-utc")
        .expect("the created option is present");
    arguments[position + 1] = String::from("2026-10-29T12:00:00+00:00");

    let output = run(&arguments);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("--created-utc expects YYYY-MM-DDTHH:MM:SSZ"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn an_impossible_created_utc_fails_before_any_read() {
    let directory = lab_dir("impossible-created");
    let mut arguments = arguments(&directory);
    let position = arguments
        .iter()
        .position(|value| value == "--created-utc")
        .expect("the created option is present");
    arguments[position + 1] = String::from("2025-02-30T00:00:00Z");

    let output = run(&arguments);

    assert_failure(&output);
    assert!(
        stderr(&output).contains("invalid --created-utc"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}

#[test]
fn a_missing_lab_directory_fails() {
    let directory = lab_dir("missing-directory");
    let absent = directory.join("absent");
    let output = run(&arguments(&absent));

    assert_failure(&output);
    assert!(
        stderr(&output).contains("is not a directory"),
        "unexpected stderr: {}",
        stderr(&output)
    );
}
