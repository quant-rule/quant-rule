//! The local gate and CI run the same Rust checker set.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repository root")
}

#[test]
fn the_gate_and_ci_run_the_same_checker_entry_point() {
    let root = root();
    let gate = fs::read_to_string(root.join("tools/gate.sh")).expect("gate.sh");
    let ci = fs::read_to_string(root.join(".github/workflows/ci.yml")).expect("ci.yml");
    assert!(
        gate.contains("quantrule-governance") && gate.contains("check --all"),
        "tools/gate.sh does not run the Rust checker set"
    );
    assert!(
        ci.contains("quantrule-governance") && ci.contains("check --all"),
        "CI does not run the Rust checker set"
    );
}

#[test]
fn a_seeded_doc_length_failure_exits_nonzero() {
    let root = std::env::temp_dir().join(format!("qr-gate-doc-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("docs/architecture")).expect("tree");
    fs::write(
        root.join("docs/architecture/long.md"),
        format!("# Long\n\n{}", "line\n".repeat(200)),
    )
    .expect("fixture");
    let output = Command::new(env!("CARGO_BIN_EXE_qr-governance"))
        .args(["check", "--checker", "doc-length", "--root"])
        .arg(&root)
        .output()
        .expect("run checker");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cap"), "{stderr}");
    fs::remove_dir_all(&root).expect("cleanup");
}
