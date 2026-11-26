//! Contract tests for the soak harness.
//!
//! The full run needs a live Compose stack and 72 hours; these tests pin the
//! Docker-free plan and the argument checks that fail before Docker, a secret,
//! or a host is read.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn tool() -> PathBuf {
    repo("tools/soak.sh")
}

#[test]
fn the_plan_is_docker_free_and_names_the_gate() {
    let output = Command::new(tool())
        .arg("--plan")
        .output()
        .expect("soak --plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "plan (no Docker, network, secret, or host is read)",
        "MemAvailable >= SOAK_MIN_MEM_MB",
        "tools/verify.sh --expected",
        "72h is the release gate",
        "a shorter --duration is a sample, not the gate",
        "value-free JSON",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn a_bad_duration_is_refused_before_preflight() {
    let output = Command::new(tool())
        .args(["--duration", "5x"])
        .output()
        .expect("soak --duration");
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("duration must be"), "{text}");
}

#[test]
fn a_zero_worker_cycle_is_refused() {
    let output = Command::new(tool())
        .args(["--duration", "1s", "--cycles", "0"])
        .output()
        .expect("soak --cycles");
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("worker counts must be at least 1"), "{text}");
}
