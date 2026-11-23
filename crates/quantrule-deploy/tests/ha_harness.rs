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
fn the_plan_names_the_database_failover_and_the_skip_opt_out() {
    let output = run(&["--plan"], None);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "database preflight",
        "database baseline",
        "database loss",
        "docker pause the primary Patroni container",
        "database control",
        "surviving edge with PGHOSTADDR",
        "database negative controls",
        "database restore",
        "database evidence",
        "QR_SWARM_EVIDENCE",
        "Unverified",
        "--skip-database",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(
        !printed.contains("--watchdog"),
        "a destructive watchdog phase is not part of this tier: {printed}"
    );
}

#[test]
fn the_ha_tier_defaults_to_the_lab_descriptor() {
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
    assert!(
        !printed.contains("deploy.example.json"),
        "the tier default must be the lab descriptor: {printed}"
    );
}

#[test]
fn the_failover_helper_is_valid_bash() {
    let output = Command::new("bash")
        .args(["-n", repo("deploy/v3/failover.sh").to_str().expect("path")])
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_haproxy_control_dials_the_surviving_edge() {
    let text = std::fs::read_to_string(repo("deploy/v3/failover.sh")).expect("failover");
    let start = text
        .find("db_haproxy_control() {")
        .expect("haproxy control");
    let end = start
        + text[start..]
            .find("db_restore() {")
            .expect("restore after control");
    let control = &text[start..end];
    assert!(
        control.contains("survivor=") && control.contains("survivor_container="),
        "the control must resolve the non-paused HAProxy: {control}"
    );
    assert!(
        control.contains("db_psql_via_hostaddr"),
        "the control must dial the resolved survivor: {control}"
    );
    assert!(
        !control.contains("db_psql_through_postgres"),
        "the control must not dial the shared alias once: {control}"
    );
    assert!(
        text.contains("PGHOSTADDR=$3"),
        "the survivor path must set PGHOSTADDR while keeping the postgres hostname: {text}"
    );
    assert!(
        text.contains("PGSSLMODE=verify-full"),
        "the control and app paths must stay TLS with verify-full: {text}"
    );
    assert!(
        control.contains("sslmode=verify-full"),
        "the evidence path must name verify-full: {control}"
    );
}

#[test]
fn the_failover_requires_the_watchdog_off_and_keeps_secrets_off_argv() {
    let text = std::fs::read_to_string(repo("deploy/v3/failover.sh")).expect("failover");
    assert!(
        text.contains("stateful.postgres.watchdog=off"),
        "the pause phase must refuse a non-off watchdog: {text}"
    );
    assert!(
        text.contains("lab-only"),
        "the pause phase must not claim real-host evidence: {text}"
    );
    assert!(
        text.contains("PGPASSWORD=\\$(cat /run/secrets/postgres_superuser_password)"),
        "the superuser password must be read remotely, not passed locally: {text}"
    );
    assert!(
        !text.contains("PGPASSWORD=postgres")
            && !text.contains("PGPASSWORD=patroni")
            && !text.contains("PGPASSWORD=qr_"),
        "a literal password reached the script: {text}"
    );
    for forbidden in [
        "stack rm",
        "volume rm",
        "secret rm",
        "config rm",
        "node rm",
        "service rm",
        "swarm leave",
        "down -v",
        "system prune",
        "rm -rf",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
}

fn run_db_preflight(descriptor: &Path, profile: &str) -> Output {
    let failover = repo("deploy/v3/failover.sh");
    let script = format!(
        "unverified() {{ printf 'UNVERIFIED: %s\\n' \"$*\" >&2; exit 1; }}\n\
         profile='{profile}'\n\
         descriptor='{}'\n\
         . '{}'\n\
         db_preflight\n",
        descriptor.display(),
        failover.display()
    );
    Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("run db_preflight")
}

#[test]
fn the_database_phase_refuses_a_non_off_watchdog() {
    let directory = temp_dir();
    let mut value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.lab.json")).expect("lab"),
    )
    .expect("lab JSON");
    value["stateful"]["postgres"]["watchdog"] = serde_json::json!("automatic");
    let descriptor = directory.join("deploy.json");
    std::fs::write(&descriptor, value.to_string()).expect("descriptor");
    let output = run_db_preflight(&descriptor, "lab-single-host");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("watchdog=off"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_database_phase_refuses_a_non_lab_profile() {
    let directory = temp_dir();
    let mut value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.lab.json")).expect("lab"),
    )
    .expect("lab JSON");
    value["profile"] = serde_json::json!("swarm-ha");
    let descriptor = directory.join("deploy.json");
    std::fs::write(&descriptor, value.to_string()).expect("descriptor");
    let output = run_db_preflight(&descriptor, "swarm-ha");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("lab-only"), "{}", stderr(&output));
    let _ = std::fs::remove_dir_all(directory);
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
        "ingress baseline",
        "ingress loss",
        "negative control",
        "VIP",
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
fn a_missing_vip_is_unverified() {
    let directory = temp_dir();
    let mut value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.example.json")).expect("v3 example"),
    )
    .expect("v3 JSON");
    value["ingress"]["vip"] = serde_json::Value::Null;
    let descriptor = directory.join("deploy.json");
    std::fs::write(&descriptor, value.to_string()).expect("descriptor");
    let docker = directory.join("fake-docker");
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
    *) exit 0 ;;
esac
"#,
        0o755,
    );
    let output = Command::new("bash")
        .arg(repo("deploy/v3/test.sh"))
        .env("QR_SWARM_DOCKER", &docker)
        .env("DESCRIPTOR", &descriptor)
        .output()
        .expect("run ha harness");
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("UNVERIFIED"), "{printed}");
    assert!(printed.contains("ingress.vip is missing"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_direct_edge_control_uses_the_public_address() {
    let text = std::fs::read_to_string(repo("deploy/v3/test.sh")).expect("harness");
    let start = text
        .find("negative_controls() {")
        .expect("negative controls");
    let end = text.find("ingress_loss() {").expect("ingress loss");
    let controls = &text[start..end];
    assert!(controls.contains("public_address \"$host\""), "{controls}");
    assert!(
        controls.contains("has no public_address"),
        "a missing public address must be UNVERIFIED: {controls}"
    );
    assert!(
        !controls.contains("host_address \"$host\""),
        "the direct-edge control must not test the private SSH/Swarm address: {controls}"
    );
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
