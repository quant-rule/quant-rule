//! Fake-Docker and shell tests for the opt-in HA lab harness.
//!
//! The fast tier never starts a Swarm. These tests prove the plan is
//! Docker-free, the preflight refuses an unavailable daemon or a two-manager
//! cluster with `UNVERIFIED`, and the harness text never contains a destructive
//! Docker command.

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
        "qr-ha-harness-{}-{}",
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

fn run(args: &[&str], docker: Option<&Path>) -> Output {
    let mut command = Command::new("bash");
    command.arg(repo("deploy/v3/test.sh")).args(args);
    if let Some(docker) = docker {
        command.env("QR_SWARM_DOCKER", docker);
    }
    command.output().expect("run ha harness")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_ha_harness_is_valid_bash() {
    let output = Command::new("bash")
        .args(["-n", repo("deploy/v3/test.sh").to_str().expect("path")])
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_plan_is_docker_free_and_names_the_sequence() {
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
    let output = run(&["--plan"], Some(&docker));
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "plan (no Docker is contacted)",
        "preflight",
        "bootstrap",
        "drain",
        "stop",
        "Simulated HA",
        "Real HA is HA-014",
        "not evidence",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!marker.exists(), "the plan contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    let output = run(&["--explode"], None);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("unknown argument"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_docker_executable_is_unverified() {
    let directory = temp_dir();
    let missing = directory.join("no-docker-here");
    let output = run(&[], Some(&missing));
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("UNVERIFIED"), "{printed}");
    assert!(printed.contains("not available"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_inactive_swarm_is_unverified() {
    let directory = temp_dir();
    let docker = directory.join("inactive-docker");
    write_mode(
        &docker,
        r#"#!/bin/sh
set -eu
case "$1" in
    info)
        case "${3:-}" in
            *LocalNodeState*) printf 'inactive\n' ;;
            *ControlAvailable*) printf 'false\n' ;;
            *) exit 0 ;;
        esac
        ;;
    *) exit 0 ;;
esac
"#,
        0o755,
    );
    let output = run(&[], Some(&docker));
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("UNVERIFIED"), "{printed}");
    assert!(printed.contains("Swarm is inactive"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_two_manager_cluster_is_unverified() {
    let directory = temp_dir();
    let docker = directory.join("two-manager-docker");
    write_mode(
        &docker,
        r#"#!/bin/sh
set -eu
case "$1" in
    info)
        case "${3:-}" in
            *LocalNodeState*) printf 'active\n' ;;
            *ControlAvailable*) printf 'true\n' ;;
            *) exit 0 ;;
        esac
        ;;
    node)
        case "$2" in
            ls) printf 'n1 h1 manager active Leader\nn2 h2 manager active Reachable\n' ;;
            *) exit 0 ;;
        esac
        ;;
    *) exit 0 ;;
esac
"#,
        0o755,
    );
    let output = run(&[], Some(&docker));
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("UNVERIFIED"), "{printed}");
    assert!(printed.contains("manager count 2"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_harness_never_contains_a_destructive_command() {
    let text = std::fs::read_to_string(repo("deploy/v3/test.sh")).expect("harness");
    for forbidden in [
        "docker stack rm",
        "docker volume rm",
        "docker secret rm",
        "docker config rm",
        "docker node rm",
        "docker swarm leave",
        "docker service rm",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    for required in ["--confirm", "loss checks not run", "UNVERIFIED"] {
        assert!(text.contains(required), "{required}");
    }
}
