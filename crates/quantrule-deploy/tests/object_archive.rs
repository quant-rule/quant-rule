//! Contract tests for the one-way object archive tool.
//!
//! The live mirror needs an operator-provided source and target; it is not part
//! of the fast gate. These tests pin the plan, the confirmation gate, the
//! distinct-bucket refusal, the absence of a destructive or replication flag,
//! and the no-secret-on-argv rule.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn tool() -> PathBuf {
    repo("tools/object-archive.sh")
}

#[test]
fn the_plan_is_docker_free_and_names_one_way_mirror_and_evidence() {
    let output = Command::new(tool())
        .arg("--plan")
        .env("QR_OBJECT_ARCHIVE_MC", "/nonexistent/mc")
        .env("QR_OBJECT_ARCHIVE_IMAGE", "missing")
        .output()
        .expect("object-archive --plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "plan (no Docker, jq, mc, SSH, or secret file is read)",
        "mode: archive",
        "mc mirror --overwrite --summary",
        "no --remove, no --watch, no mc replicate",
        "count/bytes and a SHA-256 for every key copied by this run",
        "target identity redacted",
        "Measured for this run only",
        "archive, not object replication",
        "object_replication stays false",
        "claim a second site, version history, or Real HA",
        "restore mode mirrors the archive into a fresh scratch bucket before rows",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn a_live_archive_requires_confirm_before_any_host_is_contacted() {
    let output = Command::new(tool()).output().expect("object-archive");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("requires --confirm"), "{text}");
    assert!(text.contains("no host was contacted"), "{text}");
}

#[test]
fn the_source_and_target_bucket_must_differ() {
    let output = Command::new(tool())
        .args([
            "--plan",
            "--source-bucket",
            "quantrule",
            "--target-bucket",
            "quantrule",
        ])
        .output()
        .expect("object-archive same bucket");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.contains("source and target buckets must differ"),
        "{text}"
    );
}

#[test]
fn restore_mode_is_one_way_and_does_not_delete_the_archive() {
    let output = Command::new(tool())
        .args(["--plan", "--restore"])
        .output()
        .expect("object-archive --restore --plan");
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("mode: restore"), "{text}");
    assert!(
        text.contains("it never deletes the archive"),
        "the restore plan must not delete the archive: {text}"
    );
}

#[test]
fn the_tool_is_valid_shell() {
    let output = Command::new("bash")
        .arg("-n")
        .arg(tool())
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn no_destructive_or_replication_flag_is_executed() {
    let text = std::fs::read_to_string(tool()).expect("object-archive.sh");
    for forbidden in [
        "--remove",
        "--watch",
        "mc replicate",
        "mc rm",
        "mc delete",
        "docker stack rm",
        "docker volume rm",
        "docker secret rm",
        "docker config rm",
        "docker node rm",
        "docker service rm",
        "docker swarm leave",
        "mc admin policy remove local quantrule-app",
    ] {
        for line in text.lines() {
            let code = line.trim_start();
            if code.starts_with('#') || code.starts_with("object-archive:") {
                continue;
            }
            assert!(
                !code.contains(forbidden),
                "{forbidden} appears in an executable line: {line}"
            );
        }
    }
}

#[test]
fn no_secret_is_passed_on_a_command_line() {
    let text = std::fs::read_to_string(tool()).expect("object-archive.sh");
    for forbidden in [
        "mc alias set",
        "--secret-key",
        "--access-key",
        "--secret_key",
        "--access_key",
        "AWS_SECRET_ACCESS_KEY=",
        "AWS_ACCESS_KEY_ID=",
    ] {
        for line in text.lines() {
            let code = line.trim_start();
            if code.starts_with('#') || code.starts_with("object-archive:") {
                continue;
            }
            assert!(
                !code.contains(forbidden),
                "{forbidden} appears in an executable line: {line}"
            );
        }
    }
}
