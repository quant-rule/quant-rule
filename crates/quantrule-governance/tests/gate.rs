//! CLI tests for `qr-governance gate plan`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qr-governance"))
        .args(args)
        .output()
        .expect("run qr-governance")
}

fn temp_root(name: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "qr-gate-plan-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("tree");
    root
}

#[test]
fn files_mode_reports_docs() {
    let output = run(&[
        "gate",
        "plan",
        "--files",
        "docs/example.md",
        "--field",
        "mode",
    ]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "docs");
}

#[test]
fn files_mode_reports_json() {
    let output = run(&["gate", "plan", "--files", "docs/example.md", "--json"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("gate plan JSON");
    assert_eq!(value["schema"], "quantrule.gate-plan/v1");
    assert_eq!(value["mode"], "docs");
    assert_eq!(value["checks"], serde_json::json!(["docs"]));
}

#[test]
fn an_unknown_path_is_full() {
    let output = run(&[
        "gate",
        "plan",
        "--files",
        "unknown/path.bin",
        "--field",
        "mode",
    ]);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "full");
}

#[test]
fn a_missing_choice_is_a_usage_error() {
    let output = run(&["gate", "plan"]);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn changed_reads_the_git_range() {
    let root = temp_root("changed");
    fs::create_dir_all(root.join("docs")).expect("docs");
    fs::write(root.join("docs/one.md"), "# One\n").expect("one");
    let init = Command::new("git")
        .args(["-C", root.to_str().expect("path"), "init", "-q"])
        .status()
        .expect("git init");
    assert!(init.success());
    let add = Command::new("git")
        .args(["-C", root.to_str().expect("path"), "add", "."])
        .status()
        .expect("git add");
    assert!(add.success());
    let commit = Command::new("git")
        .args([
            "-C",
            root.to_str().expect("path"),
            "-c",
            "user.email=gate@example.invalid",
            "-c",
            "user.name=Gate",
            "commit",
            "-qm",
            "init",
        ])
        .status()
        .expect("git commit");
    assert!(commit.success());
    fs::write(root.join("docs/two.md"), "# Two\n").expect("two");
    let add = Command::new("git")
        .args(["-C", root.to_str().expect("path"), "add", "."])
        .status()
        .expect("git add");
    assert!(add.success());
    let output = run(&[
        "gate",
        "plan",
        "--changed",
        "HEAD",
        "--root",
        root.to_str().expect("path"),
        "--field",
        "mode",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "docs");
    fs::remove_dir_all(root).expect("cleanup");
}
