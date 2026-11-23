//! Fake-Docker and shell tests for the MinIO node-loss lab phase.
//!
//! The fast tier never starts a Swarm. These tests prove the plan is
//! Docker-free, the phase refuses a non-lab profile, an unsafe drive layout, a
//! missing `mc` client, and an empty MinIO node list, the cleanup only starts a
//! stopped node, and the script contains no destructive command.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-minio-harness-{}-{}",
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn plan(args: &[&str], docker: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(repo("deploy/v3/test.sh")).args(args);
    if let Some(docker) = docker {
        command.env("QR_SWARM_DOCKER", docker);
    }
    command.output().expect("run ha harness")
}

fn descriptor(directory: &Path, mutate: impl FnOnce(&mut serde_json::Value)) -> PathBuf {
    let mut value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.lab.json")).expect("v3 lab"),
    )
    .expect("v3 JSON");
    mutate(&mut value);
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    path
}

fn run_phase(descriptor: &Path, profile: &str, call: &str, envs: &[(&str, &str)]) -> Output {
    let script = format!(
        "unverified() {{ printf 'UNVERIFIED: %s\\n' \"$*\" >&2; exit 1; }}\n\
         profile='{profile}'\n\
         descriptor='{}'\n\
         . '{}'\n\
         {call}\n",
        descriptor.display(),
        repo("deploy/v3/minio-failover.sh").display()
    );
    let mut command = Command::new("bash");
    command.arg("-c").arg(script);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run minio phase")
}

#[test]
fn the_plan_names_the_minio_node_loss_and_the_skip_opt_out() {
    let directory = temp_dir();
    let marker = directory.join("docker-called");
    let docker = directory.join("fake-docker");
    write_mode(
        &docker,
        &format!(
            "#!/bin/sh\nprintf 'called\\n' > '{}'\nexit 1\n",
            marker.display()
        ),
        0o755,
    );
    let output = plan(&["--plan"], Some(&docker));
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "object preflight",
        "three MinIO nodes",
        "six drives, EC:3",
        "write quorum",
        "mc admin info",
        "object baseline",
        "https://minio:9000 with qr_app",
        "SHA-256",
        "object loss",
        "docker stop",
        "reduced mc admin info",
        "object restore",
        "docker start",
        "mc admin heal -r qr/quantrule",
        "zero healing",
        "--skip-object",
        "Unverified",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!marker.exists(), "the plan contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_ha_tier_plan_defaults_to_the_lab_and_names_the_object_phase() {
    let output = Command::new("bash")
        .arg(repo("tools/test_tiers.sh"))
        .args(["ha", "--plan"])
        .output()
        .expect("run ha tier");
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(
        printed.contains("descriptor: deploy/v3/deploy.lab.json"),
        "{printed}"
    );
    assert!(printed.contains("object preflight"), "{printed}");
    assert!(!printed.contains("deploy.example.json"), "{printed}");
}

#[test]
fn the_minio_phase_refuses_a_non_lab_profile() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory, |_| {});
    let output = run_phase(&descriptor, "swarm-ha", "o_preflight", &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("lab-only"), "{}", stderr(&output));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_minio_phase_refuses_an_unsafe_drive_layout() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory, |value| {
        value["stateful"]["minio"]["drives"] = serde_json::json!([2, 1, 1]);
        value["stateful"]["minio"]["parity"] = serde_json::json!("auto");
    });
    let output = run_phase(&descriptor, "lab-single-host", "o_layout", &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("write quorum"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_minio_phase_refuses_a_missing_mc_client() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory, |_| {});
    let missing = directory.join("no-mc-here");
    let output = run_phase(
        &descriptor,
        "lab-single-host",
        "o_client_environment",
        &[("QR_SWARM_MC", missing.to_str().expect("missing mc"))],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("not executable"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_minio_phase_refuses_an_empty_node_list() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory, |value| {
        value["stateful"]["minio"]["nodes"] = serde_json::json!([]);
        value["stateful"]["minio"]["drives"] = serde_json::json!([]);
    });
    let output = run_phase(&descriptor, "lab-single-host", "o_layout", &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("declares no MinIO nodes"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_minio_cleanup_starts_the_stopped_node() {
    let directory = temp_dir();
    let marker = directory.join("started");
    let script = format!(
        "unverified() {{ exit 1; }}\n\
         remote() {{ printf '%s\\n' \"$*\" > '{}'; }}\n\
         docker_bin=docker\n\
         . '{}'\n\
         o_stopped_host=lab-1\n\
         o_cleanup\n",
        marker.display(),
        repo("deploy/v3/minio-failover.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("run o_cleanup");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = std::fs::read_to_string(&marker).expect("marker");
    assert!(text.contains("start"), "{text}");
    assert!(text.contains("quantrule-minio-lab-1"), "{text}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_minio_script_keeps_the_evidence_scoped_and_value_free() {
    let text = std::fs::read_to_string(repo("deploy/v3/minio-failover.sh")).expect("minio");
    for expected in [
        "scope: \"object_store\"",
        "object_store: \"Simulated HA\"",
        "end_to_end: $status",
        "real_ha: \"HA-014 only\"",
        "write_quorum: $quorum",
        "sha256: $hash",
        "SSL_CERT_FILE=/certs/ca.crt",
        "qr_app",
        "Blocked",
        "Unverified",
        "trap o_cleanup EXIT",
        "docker_bin stop",
        "docker_bin start",
        "mc admin heal -r",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    for forbidden in [
        "docker rm",
        "docker stack rm",
        "docker volume rm",
        "docker secret rm",
        "docker config rm",
        "docker service rm",
        "docker node rm",
        "docker swarm leave",
        "mc rm",
        "mc rb",
        "rm -rf",
        "--insecure",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn the_minio_helper_is_valid_bash() {
    let output = Command::new("bash")
        .args([
            "-n",
            repo("deploy/v3/minio-failover.sh").to_str().expect("path"),
        ])
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_harness_never_removes_a_minio_object_or_container() {
    let text = std::fs::read_to_string(repo("deploy/v3/test.sh")).expect("test.sh");
    for forbidden in [
        "docker rm",
        "docker stack rm",
        "docker volume rm",
        "docker secret rm",
        "docker config rm",
        "docker service rm",
        "docker node rm",
        "docker swarm leave",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    for required in ["--skip-object", "object_status", "object_failover"] {
        assert!(text.contains(required), "{required}");
    }
}
