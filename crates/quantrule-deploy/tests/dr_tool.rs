//! Contract tests for the backup/restore/DR drill tool.
//!
//! The live drill needs a disposable lab, a resolved derived image, a
//! pgBackRest repository, and an object archive. These tests pin the plan, the
//! confirmation gates, the target selection, and the absence of a destructive
//! command.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn tool() -> PathBuf {
    repo("deploy/v3/dr.sh")
}

#[test]
fn the_scratch_plan_names_the_phases_evidence_and_labels_without_docker() {
    let output = Command::new(tool())
        .args(["--plan", "--skip-objects", "--skip-queue", "--skip-digest"])
        .env("QR_DEPLOY_BIN", "/nonexistent/qr-deploy")
        .output()
        .expect("dr --plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "plan (no Docker, SSH, jq, qr-deploy, or secret file is read)",
        "target: scratch",
        "1 preflight:",
        "2 digest:",
        "3 markers:",
        "4 restore:",
        "5 queue: skipped, row Unverified",
        "6 digest:",
        "7 evidence:",
        "labels: Measured for executed rows",
        "Simulated HA at most for the three-VM lab",
        "Unverified when skipped or missing",
        "Real HA is HA-014",
        "this plan is not evidence",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn the_in_place_plan_is_available_without_confirm_but_a_live_run_is_not() {
    let output = Command::new(tool())
        .args(["--plan", "--target", "in-place"])
        .output()
        .expect("dr in-place --plan");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("target: in-place"), "{text}");
    assert!(
        text.contains("stop apps and every lab Patroni node"),
        "{text}"
    );

    let output = Command::new(tool())
        .arg("--in-place")
        .output()
        .expect("dr --in-place without confirm");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("pass --confirm"), "{text}");
    assert!(text.contains("destructive"), "{text}");

    let output = Command::new(tool())
        .args(["--in-place", "--confirm", "--skip-backup"])
        .output()
        .expect("dr --in-place --skip-backup");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("requires a pre-restore backup"), "{text}");
}

#[test]
fn a_live_scratch_drill_requires_confirm() {
    let output = Command::new(tool()).arg("--target").arg("scratch").output();
    let output = output.expect("dr scratch");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("pass --confirm"), "{text}");
    assert!(text.contains("performs a restore"), "{text}");
}

#[test]
fn an_unknown_target_is_refused() {
    let output = Command::new(tool())
        .args(["--target", "production"])
        .output()
        .expect("dr bad target");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("must be scratch or in-place"), "{text}");
}

#[test]
fn the_dr_scripts_are_valid_shell() {
    for script in [
        "deploy/v3/dr.sh",
        "deploy/v3/dr/evidence.sh",
        "deploy/v3/dr/target.sh",
        "deploy/v3/dr/scratch.sh",
        "deploy/v3/dr/inplace.sh",
        "deploy/v3/dr/digest.sh",
    ] {
        let output = Command::new("bash")
            .arg("-n")
            .arg(repo(script))
            .output()
            .expect("bash -n");
        assert!(
            output.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn no_forbidden_destructive_command_is_executed() {
    for script in [
        "deploy/v3/dr.sh",
        "deploy/v3/dr/evidence.sh",
        "deploy/v3/dr/target.sh",
        "deploy/v3/dr/scratch.sh",
        "deploy/v3/dr/inplace.sh",
        "deploy/v3/dr/digest.sh",
    ] {
        let text = std::fs::read_to_string(repo(script)).expect(script);
        for forbidden in [
            "docker stack rm",
            "docker volume rm",
            "docker secret rm",
            "docker config rm",
            "docker node rm",
            "docker service rm",
            "docker swarm leave",
            "pgbackrest expire",
            "stanza-delete",
            "mc rm",
            "DROP DATABASE",
            "DROP SCHEMA",
            "rm -rf",
        ] {
            for line in text.lines() {
                let code = line.trim_start();
                if code.starts_with('#') || code.starts_with("dr:") {
                    continue;
                }
                assert!(
                    !code.contains(forbidden),
                    "{script}: {forbidden} appears in an executable line: {line}"
                );
            }
        }
    }
}
