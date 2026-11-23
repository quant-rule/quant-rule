//! Shell contract for the stateful operator wrapper.
//!
//! The fast tier never starts Docker or SSH. These tests prove `bash -n`, the
//! Docker-free `--plan`, the absence of a destructive command, and that no
//! fixture secret value appears in the output.

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
        "qr-stateful-tool-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run(args: &[&str], out: &Path) -> std::process::Output {
    Command::new("bash")
        .arg(repo("tools/stateful.sh"))
        .args(args)
        .env("OUT", out)
        .env("QR_DEPLOY_BIN", "/bin/false")
        .env("QR_STATEFUL_SSH", "false")
        .env("QR_STATEFUL_SUDO", "")
        .env("QR_STATEFUL_DOCKER", "false")
        .output()
        .expect("run tools/stateful.sh")
}

#[test]
fn the_tool_is_valid_shell() {
    let output = Command::new("bash")
        .args(["-n", repo("tools/stateful.sh").to_str().expect("path")])
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
        "network",
        "copy",
        "etcd -> Patroni -> HAProxy",
        "seed the first RabbitMQ broker",
        "etcdctl endpoint health",
        "patronictl list",
        "RabbitMQ cluster_status (3 running, no alarms)",
        "queue-bootstrap",
        "psql -h postgres",
        "no stack, volume, secret, config, node, or service is removed",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_copy_selector_keeps_broker_material_on_brokers() {
    let manifest =
        repo("crates/quantrule-deploy/tests/fixtures/render-v3/expected/stateful-manifest.json");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest).expect("stateful manifest"))
            .expect("manifest JSON");
    let files = value["files"].as_array().expect("files");
    let selector = |host: &str| {
        files
            .iter()
            .filter(|entry| {
                entry["hosts"]
                    .as_array()
                    .is_some_and(|hosts| hosts.iter().any(|value| value == host))
            })
            .filter_map(|entry| entry["source"].as_str())
            .collect::<Vec<_>>()
    };
    let edge = selector("edge-1");
    assert!(edge.contains(&"haproxy-edge-1.cfg"), "{edge:?}");
    assert!(edge.contains(&"secrets/patroni_rest_password"), "{edge:?}");
    assert!(
        !edge.contains(&"secrets/rabbitmq_erlang_cookie"),
        "{edge:?}"
    );
    assert!(!edge.contains(&"rabbitmq-infra-1.conf"), "{edge:?}");
    assert!(!edge.contains(&"pki/rabbitmq/server.key"), "{edge:?}");
    assert!(!edge.contains(&"patroni-infra-1.yml"), "{edge:?}");

    let broker = selector("infra-1");
    assert!(broker.contains(&"rabbitmq-infra-1.conf"), "{broker:?}");
    assert!(
        broker.contains(&"secrets/rabbitmq_erlang_cookie"),
        "{broker:?}"
    );
    assert!(broker.contains(&"pki/rabbitmq/server.key"), "{broker:?}");
    assert!(!broker.contains(&"haproxy-edge-1.cfg"), "{broker:?}");

    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    assert!(text.contains(".hosts | index($host)"), "{text}");
    assert!(text.contains("--arg host"), "{text}");
}

#[test]
fn the_start_seeds_one_broker_before_the_rest() {
    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    assert!(
        text.contains("start_phase \"rabbitmq-broker-$seed\" \"$seed\""),
        "{text}"
    );
    assert!(text.contains("wait_rabbitmq_seed \"$seed\""), "{text}");
    assert!(
        text.contains("[ \"$host\" = \"$seed\" ] && continue"),
        "{text}"
    );
    assert!(text.contains("rabbitmq-diagnostics -q ping"), "{text}");
    assert!(text.contains("rabbitmqctl await_startup"), "{text}");
}

#[test]
fn the_wait_checks_three_running_nodes_and_no_alarms() {
    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    assert!(
        text.contains("length(rabbit_nodes:list_running())."),
        "{text}"
    );
    assert!(
        text.contains("length(rabbit_alarm:get_alarms())."),
        "{text}"
    );
    assert!(text.contains("rabbitmq_cluster_ready"), "{text}");
    assert!(text.contains("cluster_status"), "{text}");
}

#[test]
fn the_queue_bootstrap_never_passes_the_password_as_an_argument() {
    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    assert!(text.contains("IFS= read -r url"), "{text}");
    assert!(text.contains("rabbitmqctl -q hash_password"), "{text}");
    assert!(
        text.contains("rabbit_auth_backend_internal:add_user"),
        "{text}"
    );
    assert!(text.contains("set_permissions"), "{text}");
    assert!(text.contains("exec -i -u rabbitmq"), "{text}");
    assert!(text.contains("exec -u rabbitmq"), "{text}");
    assert!(!text.contains("set -x"), "{text}");
    for forbidden in [
        "rabbitmqctl -q add_user",
        "rabbitmqctl add_user",
        "add_user \"$user\" \"$password\"",
        "add_user '$user'",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
    assert!(
        text.contains("printf '%s\\n' \"$url\" | $ssh_bin"),
        "the URL must travel on stdin: {text}"
    );
}

#[test]
fn the_tool_has_no_destructive_command() {
    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    for forbidden in [
        "stack rm",
        "secret rm",
        "config rm",
        "volume rm",
        "down -v",
        "docker system prune",
        "rm -rf",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
}

#[test]
fn the_optional_data_owner_is_env_gated() {
    let text = std::fs::read_to_string(repo("tools/stateful.sh")).expect("tool");
    assert!(text.contains("QR_STATEFUL_DATA_OWNER"), "{text}");
    assert!(
        text.contains("chown $QR_STATEFUL_DATA_OWNER"),
        "the owner change must be an explicit optional step: {text}"
    );
}

#[test]
fn the_plan_prints_no_secret_value() {
    let directory = temp_dir();
    std::fs::create_dir_all(directory.join("secrets")).expect("secrets dir");
    let marker = "stateful-plan-must-not-print-7f3c1a9e";
    std::fs::write(
        directory.join("secrets/postgres_superuser_password"),
        format!("{marker}\n"),
    )
    .expect("secret");
    let output = run(&["--plan"], &directory);
    assert!(output.status.success());
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!printed.contains(marker), "the plan printed a secret");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    let directory = temp_dir();
    let output = run(&["--surprise"], &directory);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown argument"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}
