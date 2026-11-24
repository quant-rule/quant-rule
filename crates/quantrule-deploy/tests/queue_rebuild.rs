//! Contract tests for the queue rebuild tool.
//!
//! The live rebuild needs a provisioned Swarm, an empty broker, and a stored
//! definitions file; it is not part of the fast gate. These tests pin the
//! Docker/SSH/jq/secret-free plan, the `--confirm` gate, and the absence of a
//! destructive command.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn tool() -> PathBuf {
    repo("tools/queue-rebuild.sh")
}

#[test]
fn the_plan_is_docker_ssh_jq_and_secret_free() {
    let output = Command::new(tool())
        .arg("--plan")
        .output()
        .expect("queue-rebuild --plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "plan (no Docker, SSH, jq, or secret is read)",
        "1 import:",
        "2 user:",
        "3 rearm:",
        "4 report:",
        "never: purge/delete",
        "the queue is not archived",
        "empty-broker drill",
        "Simulated HA is the maximum; Real HA is HA-014",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn a_live_rebuild_requires_confirm_before_any_host_is_contacted() {
    let output = Command::new(tool())
        .output()
        .expect("queue-rebuild without --confirm");
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("requires --confirm"), "{text}");
    assert!(text.contains("no host was contacted"), "{text}");
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
fn no_destructive_or_publishing_command_is_executed() {
    let text = std::fs::read_to_string(tool()).expect("queue-rebuild.sh");
    for forbidden in [
        "docker stack rm",
        "docker volume rm",
        "docker secret rm",
        "docker config rm",
        "docker node rm",
        "docker service rm",
        "docker swarm leave",
        "rm -rf",
        "queue-definitions.sh export",
        "queue-definitions.sh prune",
        "rabbitmqctl purge_queue",
        "rabbitmqctl delete_queue",
        "rabbitmqctl delete_user",
        "rabbitmqctl delete_vhost",
    ] {
        for line in text.lines() {
            let code = line.trim_start();
            if code.starts_with('#') || code.starts_with("queue-rebuild:") {
                continue;
            }
            assert!(
                !code.contains(forbidden),
                "{forbidden} appears in an executable line: {line}"
            );
        }
    }
}
