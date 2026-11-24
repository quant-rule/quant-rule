//! Fast tests for the HA-014 chaos and Real HA gates.
//!
//! The fast tier never starts a Swarm. These tests prove that the plans are
//! Docker-free, `--chaos` is opt-in behind `--confirm`, the matrix covers all
//! 23 rows, and the Real HA gate refuses a lab descriptor or a missing human
//! attestation. They do not run a chaos case or claim `Real HA`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-chaos-harness-{name}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn write_mode(path: &Path, content: &str, mode: u32) {
    std::fs::write(path, content).expect("write file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }
    #[cfg(not(unix))]
    let _ = mode;
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn run_plan(script: &str, args: &[&str], docker: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(repo(script)).args(args);
    if let Some(docker) = docker {
        command.env("QR_SWARM_DOCKER", docker);
    }
    command.output().expect("run plan")
}

#[test]
fn the_chaos_plan_names_every_live_row_and_the_f_row_mappings() {
    let output = run_plan("deploy/v3/chaos.sh", &["--plan"], None);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "HA-18",
        "HA-19",
        "HA-20",
        "HA-21",
        "HA-22",
        "HA-23",
        "F01",
        "F01..F17",
        "F03",
        "F06",
        "F07",
        "F11",
        "AZ1",
        "accepted risk",
        "Simulated HA only after a pass",
        "no Real HA",
        "--chaos requires --confirm",
        "--skip-chaos",
        "QR_HA_EVIDENCE",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
}

#[test]
fn the_ha_plan_is_docker_free_and_names_the_chaos_cases() {
    let directory = temp_dir("plan");
    let marker = directory.join("docker-called");
    let docker = directory.join("fake-docker");
    write_mode(
        &docker,
        &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
        0o755,
    );
    let output = run_plan("deploy/v3/test.sh", &["--plan"], Some(&docker));
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!marker.exists(), "the plan contacted Docker");
    let printed = stdout(&output);
    for expected in [
        "HA-18",
        "HA-19",
        "HA-20",
        "HA-21",
        "HA-22",
        "HA-23",
        "F01",
        "F11",
        "--chaos requires --confirm",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn chaos_without_confirm_is_refused_before_docker() {
    let directory = temp_dir("confirm");
    let marker = directory.join("docker-called");
    let docker = directory.join("fake-docker");
    write_mode(
        &docker,
        &format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
        0o755,
    );
    let output = run_plan("deploy/v3/test.sh", &["--chaos"], Some(&docker));
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--chaos requires --confirm"),
        "{}",
        stderr(&output)
    );
    assert!(!marker.exists(), "the refusal contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn lab_chaos_without_confirm_is_refused_before_provisioning() {
    let output = run_plan("deploy/v3/lab.sh", &["--chaos"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("--chaos requires --confirm"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn skip_chaos_is_the_default_and_the_lab_forwards_it() {
    let test_sh = std::fs::read_to_string(repo("deploy/v3/test.sh")).expect("test.sh");
    assert!(test_sh.contains("skip_chaos=1"));
    assert!(test_sh.contains("--skip-chaos"));
    let lab_sh = std::fs::read_to_string(repo("deploy/v3/lab.sh")).expect("lab.sh");
    assert!(lab_sh.contains("args+=(--chaos)"));
    assert!(lab_sh.contains("args+=(--skip-chaos)"));
    let output = run_plan("deploy/v3/lab.sh", &["--plan", "--chaos"], None);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(printed.contains("5d chaos"), "{printed}");
    assert!(
        printed.contains("deploy/v3/chaos.sh --plan") || printed.contains("HA-18 worker SIGKILL"),
        "{printed}"
    );
}

#[test]
fn the_matrix_covers_all_23_rows_and_the_live_claims_are_unverified() {
    let text = std::fs::read_to_string(repo("docs/config/2026-11-24/chaos-matrix.json"))
        .expect("chaos matrix");
    let matrix: serde_json::Value = serde_json::from_str(&text).expect("matrix JSON");
    let rows = matrix["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 23);
    for (index, row) in rows.iter().enumerate() {
        let expected = format!("HA-{:02}", index + 1);
        assert_eq!(row["id"].as_str(), Some(expected.as_str()));
    }
    for row in rows.iter().skip(1) {
        if let Some(id) = row["id"].as_str()
            && id >= "HA-18"
        {
            assert_eq!(row["status"], json!("Unverified"), "{id}");
            assert_eq!(row["live_evidence"], json!("Unverified"), "{id}");
        }
    }
    let risks = matrix["accepted_risks"].as_array().expect("risks");
    assert!(
        risks.iter().any(|risk| {
            let text = format!(
                "{} {} {}",
                risk["risk"].as_str().unwrap_or(""),
                risk["impact"].as_str().unwrap_or(""),
                risk["residual_risk"].as_str().unwrap_or("")
            );
            text.contains("AZ1") && text.contains("MinIO") && text.contains("ingress")
        }),
        "{risks:?}"
    );
}

#[test]
fn the_scripts_keep_destructive_commands_out() {
    for script in [
        "deploy/v3/chaos.sh",
        "deploy/v3/chaos/lib.sh",
        "deploy/v3/chaos/cases.sh",
        "deploy/v3/real-ha.sh",
    ] {
        let text = std::fs::read_to_string(repo(script)).expect(script);
        for forbidden in [
            "docker stack rm",
            "docker volume rm",
            "docker secret rm",
            "docker config rm",
            "docker node rm",
            "docker service rm",
            "swarm leave",
            "down -v",
            "system prune",
            "mkfs",
            "shutdown",
            "poweroff",
            "reboot",
            "rm -rf /",
        ] {
            assert!(!text.contains(forbidden), "{script}: {forbidden}");
        }
        let output = Command::new("bash")
            .args(["-n", repo(script).to_str().expect("path")])
            .output()
            .expect("bash -n");
        assert!(
            output.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn real_ha_evidence(descriptor: &Path) -> serde_json::Value {
    json!({
        "schema": "quantrule.real-ha/v1",
        "descriptor": descriptor.to_string_lossy(),
        "recorded_on": "2026-11-24",
        "failure_domains": 3,
        "hosts": [
            {"host": "edge-1", "machine_id": "machine-1", "product_uuid": "uuid-1"},
            {"host": "core-1", "machine_id": "machine-2", "product_uuid": "uuid-2"},
            {"host": "infra-3", "machine_id": "machine-3", "product_uuid": "uuid-3"}
        ],
        "digest_before": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "digest_after": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "attestation": {
            "by": "human",
            "signed_on": "2026-11-24",
            "statement": "I observed the before and after digest and the one finalisation"
        }
    })
}

fn run_real_ha(descriptor: &Path, evidence: &Path) -> Output {
    Command::new("bash")
        .arg(repo("deploy/v3/real-ha.sh"))
        .args(["--confirm-real-ha", "--descriptor"])
        .arg(descriptor)
        .arg("--evidence")
        .arg(evidence)
        .output()
        .expect("run real-ha")
}

#[test]
fn real_ha_refuses_the_lab_descriptor() {
    let directory = temp_dir("real-lab");
    let descriptor = repo("deploy/v3/deploy.lab.json");
    let evidence = directory.join("evidence.json");
    std::fs::write(&evidence, real_ha_evidence(&descriptor).to_string()).expect("evidence");
    let output = run_real_ha(&descriptor, &evidence);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("lab-single-host"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn real_ha_refuses_a_missing_attestation() {
    let directory = temp_dir("real-attestation");
    let descriptor = repo("deploy/v3/reference/three-az.json");
    let mut evidence = real_ha_evidence(&descriptor);
    evidence
        .as_object_mut()
        .expect("object")
        .remove("attestation");
    let path = directory.join("evidence.json");
    std::fs::write(&path, evidence.to_string()).expect("evidence");
    let output = run_real_ha(&descriptor, &path);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("human attestation"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn real_ha_accepts_a_synthetic_attestation_only_as_a_gate_check() {
    let directory = temp_dir("real-valid");
    let descriptor = repo("deploy/v3/reference/three-az.json");
    let path = directory.join("evidence.json");
    std::fs::write(&path, real_ha_evidence(&descriptor).to_string()).expect("evidence");
    let output = run_real_ha(&descriptor, &path);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(printed.contains("gate satisfied"), "{printed}");
    assert!(printed.contains("no failure was executed"), "{printed}");
    assert!(printed.contains("remains Unverified"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}
