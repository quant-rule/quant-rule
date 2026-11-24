//! Shell contract for the observability operator wrapper.
//!
//! The fast tier never starts Docker or SSH. These tests prove `bash -n`, the
//! Docker-free `--plan`, the start order, the manager-quorum path, the absence
//! of a destructive command, and that no fixture secret value appears in the
//! output.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-observability-tool-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run(args: &[&str], out: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(repo("tools/observability.sh"))
        .args(args)
        .env("OUT", out)
        .env("QR_DEPLOY_BIN", "/bin/false")
        .env("QR_OBSERVABILITY_SSH", "false")
        .env("QR_OBSERVABILITY_SUDO", "")
        .env("QR_OBSERVABILITY_DOCKER", "false")
        .output()
        .expect("run tools/observability.sh")
}

#[test]
fn the_tool_is_valid_shell() {
    let output = Command::new("bash")
        .args(["-n", repo("tools/observability.sh").to_str().expect("path")])
        .output()
        .expect("bash -n");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn the_plan_is_docker_free_and_names_the_boot_order() {
    let directory = temp_dir();
    let output = run(&["--plan"], &directory);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "no Docker, SSH, jq, or secret file is read",
        "copy",
        "Alertmanager -> node_exporter + blackbox -> Prometheus -> Loki -> promtail (every host) -> Grafana",
        "Prometheus /-/ready",
        "38 rules across 17 failure_row values",
        "Alertmanager /-/ready",
        "Loki /-/ready",
        "promtail /ready",
        "Grafana /api/health",
        "promtool check rules",
        "amtool check-config",
        "manager-quorum",
        "docker node ls",
        "no stack, volume, secret, config, node, or service is removed",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_tool_has_no_destructive_command_or_secret_echo() {
    let text = std::fs::read_to_string(repo("tools/observability.sh")).expect("tool");
    assert!(!text.contains("set -x"), "{text}");
    for forbidden in [
        "stack rm",
        "down -v",
        "volume rm",
        "secret rm",
        "config rm",
        "service rm",
        "node rm",
        "rm -rf",
        "docker rm",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
    for expected in [
        "docker node ls",
        "quantrule_swarm_managers_ready",
        "textfile",
        "observability-manifest.json",
        "QR_OBSERVABILITY_STAMP",
        "quantrule_verify_last_success_timestamp_seconds",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn the_start_order_is_alertmanager_exporters_prometheus_loki_promtail_grafana() {
    let text = std::fs::read_to_string(repo("tools/observability.sh")).expect("tool");
    let positions = [
        "start_unit \"$(unit_host alertmanager)\" alertmanager",
        "start_unit \"$host\" \"node-exporter-$host\"",
        "start_unit \"$(unit_host prometheus)\" blackbox",
        "start_unit \"$(unit_host prometheus)\" prometheus",
        "start_unit \"$(unit_host loki)\" loki",
        "start_unit \"$host\" \"promtail-$host\"",
        "start_unit \"$(unit_host grafana)\" grafana",
    ];
    let mut last = 0;
    for expected in positions {
        let position = text.find(expected).expect(expected);
        assert!(position >= last, "{expected} is out of order");
        last = position;
    }
}

#[test]
fn the_manager_quorum_uses_a_textfile_not_a_container_socket() {
    let text = std::fs::read_to_string(repo("tools/observability.sh")).expect("tool");
    assert!(text.contains("docker_bin node ls --format"), "{text}");
    assert!(text.contains("quantrule_swarm_managers_ready"), "{text}");
    assert!(
        text.contains("/srv/quantrule/node_exporter/textfile"),
        "{text}"
    );
    assert!(!text.contains("docker.sock:/var/run/docker.sock"), "{text}");
}

#[test]
fn the_copy_selector_honors_the_manifest_hosts() {
    let text = std::fs::read_to_string(repo("tools/observability.sh")).expect("tool");
    for expected in [
        "observability-manifest.json",
        ".hosts | index($host)",
        "--arg host",
        "cat > $target",
        "chmod $mode $target",
        "install -d",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn no_fixture_secret_value_appears_in_the_plan_output() {
    let directory = temp_dir();
    let output = run(&["--plan"], &directory);
    let text = String::from_utf8_lossy(&output.stdout);
    for name in [
        "patroni_rest_password",
        "postgres_superuser_password",
        "gateway_token",
        "minio_root_password",
    ] {
        let value = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}")))
            .expect("fixture secret")
            .trim()
            .to_owned();
        assert!(!text.contains(&value), "{name} value leaked into the plan");
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_make_targets_forward_to_the_tool() {
    let makefile = std::fs::read_to_string(repo("Makefile")).expect("Makefile");
    assert!(makefile.contains("observability: ##"), "{makefile}");
    assert!(
        makefile.contains("./tools/observability.sh $(ARGS)"),
        "{makefile}"
    );
    assert!(makefile.contains("observability-plan: ##"), "{makefile}");
}
