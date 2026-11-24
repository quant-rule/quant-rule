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
        "runtime digest",
        "all-service digest row",
        "database preflight",
        "--allow-unverified bootstrap",
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
        "queue preflight",
        "queue baseline",
        "queue loss",
        "quorum leader",
        "qr.lab.queue-probe",
        "queue restore",
        "--skip-queue",
        "Unverified",
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
fn the_plan_names_the_observability_phase_and_its_skip_opt_out() {
    let output = run(&["--plan"], None);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "observability: copy/start/wait/verify",
        "promtool rule-unit proof of GatewaySyntheticDown",
        "SwarmManagerQuorumAtRisk",
        "GatewayReplicaDown",
        "pause on the non-VIP gateway",
        "VIP ready probe 1",
        "--skip-observability",
        "Simulated HA only when both alerts fired and resolved",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
}

#[test]
fn the_observability_lab_phase_is_valid_bash() {
    for script in ["deploy/v3/observability.sh", "tools/observability.sh"] {
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
fn the_queue_failover_helper_is_valid_bash() {
    let output = Command::new("bash")
        .args([
            "-n",
            repo("deploy/v3/queue-failover.sh").to_str().expect("path"),
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
fn the_queue_phase_refuses_a_non_lab_profile() {
    let script = format!(
        "unverified() {{ printf 'UNVERIFIED: %s\\n' \"$*\" >&2; exit 1; }}\n\
         profile='swarm-ha'\n\
         descriptor='{}'\n\
         . '{}'\n\
         queue_preflight\n",
        repo("deploy/v3/deploy.lab.json").display(),
        repo("deploy/v3/queue-failover.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("run queue_preflight");
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("lab-only"), "{}", stderr(&output));
}

#[test]
fn the_queue_cleanup_starts_the_stopped_broker() {
    let directory = temp_dir();
    let marker = directory.join("started");
    let script = format!(
        "unverified() {{ exit 1; }}\n\
         remote() {{ printf '%s\\n' \"$*\" > '{}'; }}\n\
         docker_bin=docker\n\
         . '{}'\n\
         q_stopped_host=infra-1\n\
         q_cleanup\n",
        marker.display(),
        repo("deploy/v3/queue-failover.sh").display()
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .output()
        .expect("run q_cleanup");
    assert!(output.status.success(), "{}", stderr(&output));
    let text = std::fs::read_to_string(&marker).expect("marker");
    assert!(text.contains("start"), "{text}");
    assert!(text.contains("quantrule-rabbitmq-infra-1"), "{text}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_queue_script_keeps_the_evidence_scoped_and_value_free() {
    let text = std::fs::read_to_string(repo("deploy/v3/queue-failover.sh")).expect("queue");
    for expected in [
        "scope: $scope",
        "queue_level: \"Simulated HA\"",
        "end_to_end: $end_to_end",
        "Unverified until the MinIO phase and tools/verify.sh succeed",
        "HA-014 only",
        "users/permissions stripped",
        "q_cleanup",
        "docker_bin stop",
        "docker_bin start",
        "q_survivor_host",
        "probe_host=${q_survivor_host:-$q_first_broker}",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    assert!(
        !text.contains("docker rm")
            && !text.contains("docker volume")
            && !text.contains("purge_queue")
            && !text.contains("delete_user")
            && !text.contains("delete_vhost"),
        "{text}"
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
fn the_runtime_digest_probe_covers_all_seven_services_after_deploy_and_restore() {
    let text = std::fs::read_to_string(repo("deploy/v3/test.sh")).expect("harness");
    for expected in [
        "qr_deploy check \"$descriptor\" --remote docker --runtime",
        "for service in gateway data model strategy backtest evidence simulation",
        "digest_status=\"Measured\"",
        "digest_all \"after deploy\"",
        "digest_all \"after restore\"",
        "all-service image digest: $digest_status",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
    assert!(
        text.contains("$1 == \"ok\" && $3 == subject"),
        "the probe must parse only a successful runtime row: {text}"
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
        "runtime digest",
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

#[test]
fn the_lab_plan_is_docker_free_and_names_the_remote_phase() {
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
    let output = Command::new("bash")
        .arg(repo("deploy/v3/lab.sh"))
        .arg("--plan")
        .env("QR_LAB_DOCKER", &docker)
        .output()
        .expect("run lab plan");
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "lab: reference: lab",
        "lab: references: three-az and single-az are designs; lab is the three-VM profile",
        "lab: descriptor: deploy/v3/deploy.lab.json",
        "tools/stateful.sh network",
        "queue-bootstrap, object-bootstrap",
        "tools/swarm.sh --allow-unverified bootstrap",
        "tools/observability.sh copy, start, wait, verify",
        "--remote docker --overlay --runtime",
        "deploy/v3/test.sh --confirm --skip-deploy",
        "remote client",
        "QUANTRULE_API_URL",
        "QUANTRULE_CA_CERT",
        "QUANTRULE_TOKEN",
        "openssl s_client",
        "curl --cacert /dev/null",
        "browser (separate",
        "Simulated HA",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!printed.contains("deploy.example.json"), "{printed}");
    assert!(!marker.exists(), "the plan contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_preflight_stops_at_the_fixture_without_docker() {
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
    let deploy = directory.join("fake-qr-deploy");
    write_mode(
        &deploy,
        "#!/bin/sh\nprintf 'qr-deploy: deploy.image.fixture at image.digest: is the labelled application fixture\\n' >&2\nexit 1\n",
        0o755,
    );
    let output = Command::new("bash")
        .arg(repo("deploy/v3/lab.sh"))
        .args(["--reference", "lab"])
        .env("QR_DEPLOY_BIN", &deploy)
        .env("QR_LAB_DOCKER", &docker)
        .output()
        .expect("run lab preflight");
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(
        printed.contains("deploy.image.fixture at image.digest"),
        "{printed}"
    );
    assert!(printed.contains("no host is contacted"), "{printed}");
    assert!(printed.contains("Unverified"), "{printed}");
    assert!(!marker.exists(), "the fixture blocker contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_refuses_a_non_lab_profile_even_with_a_descriptor_override() {
    let directory = temp_dir();
    let mut value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.lab.json")).expect("lab descriptor"),
    )
    .expect("lab JSON");
    value["profile"] = serde_json::json!("swarm-ha");
    let descriptor = directory.join("deploy.json");
    std::fs::write(&descriptor, value.to_string()).expect("descriptor");
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
    let deploy = directory.join("fake-qr-deploy");
    write_mode(&deploy, "#!/bin/sh\nexit 0\n", 0o755);
    let output = Command::new("bash")
        .arg(repo("deploy/v3/lab.sh"))
        .args(["--reference", "lab"])
        .env("DESCRIPTOR", &descriptor)
        .env("QR_DEPLOY_BIN", &deploy)
        .env("QR_LAB_DOCKER", &docker)
        .output()
        .expect("run lab profile guard");
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(
        printed.contains("only the lab-single-host profile has lab hosts"),
        "{printed}"
    );
    assert!(printed.contains("Unverified"), "{printed}");
    assert!(!marker.exists(), "the profile guard contacted Docker");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_remote_client_requires_a_provisioned_lab() {
    let output = Command::new("bash")
        .arg(repo("deploy/v3/lab.sh"))
        .args(["--remote-client", "--confirm"])
        .env_remove("QR_LAB_PROVISIONED")
        .env_remove("QUANTRULE_API_URL")
        .env_remove("QUANTRULE_CA_CERT")
        .env_remove("QUANTRULE_TOKEN")
        .output()
        .expect("run remote gate");
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("QR_LAB_PROVISIONED=1"), "{printed}");
    assert!(printed.contains("Unverified"), "{printed}");

    let output = Command::new("bash")
        .arg(repo("deploy/v3/lab.sh"))
        .args(["--remote-client", "--confirm"])
        .env("QR_LAB_PROVISIONED", "1")
        .env_remove("QUANTRULE_API_URL")
        .output()
        .expect("run remote URL gate");
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("QUANTRULE_API_URL must be https://quantrule.dev/api/v1"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn the_lab_script_is_valid_bash() {
    for script in [
        "deploy/v3/lab.sh",
        "deploy/v3/lab/evidence.sh",
        "deploy/v3/lab/remote.sh",
    ] {
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

#[test]
fn the_lab_keeps_the_provisioning_order_and_avoids_destructive_commands() {
    let text = std::fs::read_to_string(repo("deploy/v3/lab.sh")).expect("lab harness");
    let markers = [
        "./tools/stateful.sh network",
        "./tools/stateful.sh copy",
        "./tools/stateful.sh start",
        "./tools/stateful.sh wait",
        "./tools/stateful.sh queue-bootstrap",
        "./tools/stateful.sh object-bootstrap",
        "./tools/swarm.sh \"${bootstrap[@]}\"",
        "./tools/observability.sh copy",
        "./tools/observability.sh start",
        "./tools/observability.sh wait",
        "./tools/observability.sh verify",
        "qr_deploy check \"$descriptor\" --remote docker --overlay --runtime",
        "./deploy/v3/test.sh \"${args[@]}\"",
    ];
    let mut previous = 0;
    for marker in markers {
        let position = text.find(marker).unwrap_or_else(|| panic!("{marker}"));
        assert!(position > previous, "{marker} is out of order");
        previous = position;
    }
    assert!(text.contains("--allow-unverified bootstrap"), "{text}");
    assert!(text.contains("--remote-client"), "{text}");
    assert!(text.contains("QUANTRULE_API_URL"), "{text}");
    for script in [
        "deploy/v3/lab.sh",
        "deploy/v3/lab/evidence.sh",
        "deploy/v3/lab/remote.sh",
    ] {
        let text = std::fs::read_to_string(repo(script)).expect("lab script");
        for forbidden in [
            "docker stack rm",
            "docker volume rm",
            "docker secret rm",
            "docker config rm",
            "docker node rm",
            "docker swarm leave",
            "docker service rm",
            "down -v",
            "rm -rf",
        ] {
            assert!(!text.contains(forbidden), "{script}: {forbidden}");
        }
    }
}

#[test]
fn the_ha_tier_plan_lists_the_lab_reference_and_remote_phases() {
    let output = Command::new("bash")
        .arg(repo("tools/test_tiers.sh"))
        .args(["ha", "--plan"])
        .env("DESCRIPTOR", "deploy/v3/deploy.lab.json")
        .output()
        .expect("run ha tier plan");
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "lab: reference: lab",
        "lab: descriptor: deploy/v3/deploy.lab.json",
        "lab: 6 remote client",
        "lab: 7 browser (separate, lab-gated)",
        "deploy/v3/test.sh --plan",
        "Real HA is HA-014",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!printed.contains("deploy.example.json"), "{printed}");
}
