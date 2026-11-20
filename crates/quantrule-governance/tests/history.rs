//! Integration tests for `qr-governance history`.
//!
//! Every test builds a temporary repository tree and runs the real binary, so
//! the CLI, the manifest, and the filesystem behavior are exercised together.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn root() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("qr-governance-history-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create the temporary root");
    root
}

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("a parent")).expect("create the parent");
    fs::write(path, text).expect("write the fixture");
}

fn run(root: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_qr-governance"));
    command.args(args).arg("--root").arg(root);
    command.output().expect("run qr-governance")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn seed(root: &Path) {
    write(
        root,
        "config/research/PROGRAM-2026-11-05-03/program.json",
        "{\"id\":\"old\"}\n",
    );
    write(root, "config/sr1/run.json", "{\"id\":\"undated\"}\n");
    write(
        root,
        "docs/execution-plans/2026-10-01/old-plan.md",
        "# Old plan\n",
    );
    write(
        root,
        "docs/execution-plans/2026-11-20/new-plan.md",
        "# New plan\n",
    );
    write(root, "docs/improvement/2026-10-01/run.md", "# Run\n");
}

#[test]
fn list_filters_by_date_and_can_include_undated_files() {
    let root = root();
    seed(&root);
    let output = run(&root, &["history", "list", "--kind", "config"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.contains("PROGRAM-2026-11-05-03/program.json"),
        "{text}"
    );
    assert!(text.contains("sr1/run.json"), "{text}");

    let output = run(
        &root,
        &[
            "history",
            "list",
            "--kind",
            "config",
            "--before",
            "2026-11-01",
            "--include-undated",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        !text.contains("PROGRAM-2026-11-05-03/program.json"),
        "{text}"
    );
    assert!(text.contains("sr1/run.json"), "{text}");

    let output = run(
        &root,
        &[
            "history",
            "list",
            "--kind",
            "config",
            "--before",
            "2026-11-06",
            "--include-undated",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(
        text.contains("PROGRAM-2026-11-05-03/program.json"),
        "{text}"
    );
    assert!(text.contains("sr1/run.json"), "{text}");
}

#[test]
fn archive_writes_a_manifest_and_keeps_the_source() {
    let root = root();
    seed(&root);
    let archive = root.join("archive");
    let output = run(
        &root,
        &[
            "history",
            "archive",
            "--kind",
            "docs",
            "--before",
            "2026-11-01",
            "--out",
            archive.to_str().expect("UTF-8"),
            "--created-unix",
            "0",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        root.join("docs/execution-plans/2026-10-01/old-plan.md")
            .exists()
    );
    assert!(
        archive
            .join("docs/docs/execution-plans/2026-10-01/old-plan.md")
            .exists()
    );
    let manifest_path = archive.join("docs/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("manifest")).expect("JSON");
    assert_eq!(manifest["schema"], "quantrule.history-manifest/v1");
    assert_eq!(manifest["kind"], "docs");
    assert_eq!(manifest["files"].as_array().expect("files").len(), 1);
    assert!(
        manifest["archive_id"]
            .as_str()
            .expect("id")
            .starts_with("sha256:")
    );
}

#[test]
fn archive_requires_a_date_bound_or_all() {
    let root = root();
    seed(&root);
    let archive = root.join("archive");
    let output = run(
        &root,
        &[
            "history",
            "archive",
            "--kind",
            "docs",
            "--out",
            archive.to_str().expect("UTF-8"),
        ],
    );
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--before or --all"));
}

#[test]
fn erase_refuses_a_wrong_confirmation_and_a_tampered_file() {
    let root = root();
    seed(&root);
    let archive = root.join("archive");
    let output = run(
        &root,
        &[
            "history",
            "archive",
            "--kind",
            "docs",
            "--before",
            "2026-11-01",
            "--out",
            archive.to_str().expect("UTF-8"),
            "--created-unix",
            "0",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let manifest_path = archive.join("docs/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("manifest")).expect("JSON");
    let id = manifest["archive_id"].as_str().expect("id");

    let wrong = run(
        &root,
        &[
            "history",
            "erase",
            "--manifest",
            manifest_path.to_str().expect("UTF-8"),
            "--confirm",
            "sha256:wrong",
        ],
    );
    assert!(!wrong.status.success());
    assert!(
        root.join("docs/execution-plans/2026-10-01/old-plan.md")
            .exists()
    );

    let output = run(
        &root,
        &[
            "history",
            "erase",
            "--manifest",
            manifest_path.to_str().expect("UTF-8"),
            "--confirm",
            id,
            "--dry-run",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("would erase"));
    assert!(
        root.join("docs/execution-plans/2026-10-01/old-plan.md")
            .exists()
    );

    fs::write(
        root.join("docs/execution-plans/2026-10-01/old-plan.md"),
        "# Tampered\n",
    )
    .expect("tamper");
    let tampered = run(
        &root,
        &[
            "history",
            "erase",
            "--manifest",
            manifest_path.to_str().expect("UTF-8"),
            "--confirm",
            id,
        ],
    );
    assert!(!tampered.status.success());
    assert!(stderr(&tampered).contains("manifest does not match"));
}

#[test]
fn erase_removes_only_a_verified_manifest() {
    let root = root();
    seed(&root);
    let archive = root.join("archive");
    let output = run(
        &root,
        &[
            "history",
            "archive",
            "--kind",
            "docs",
            "--before",
            "2026-11-01",
            "--out",
            archive.to_str().expect("UTF-8"),
            "--created-unix",
            "0",
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let manifest_path = archive.join("docs/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&manifest_path).expect("manifest")).expect("JSON");
    let id = manifest["archive_id"].as_str().expect("id");
    let output = run(
        &root,
        &[
            "history",
            "erase",
            "--manifest",
            manifest_path.to_str().expect("UTF-8"),
            "--confirm",
            id,
        ],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        !root
            .join("docs/execution-plans/2026-10-01/old-plan.md")
            .exists()
    );
    assert!(
        root.join("docs/execution-plans/2026-11-20/new-plan.md")
            .exists()
    );
    assert!(manifest_path.exists());
}
