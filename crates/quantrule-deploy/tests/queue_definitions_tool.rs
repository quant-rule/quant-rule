//! Shell and fake-broker tests for `tools/queue-definitions.sh`.
//!
//! The fast tier never contacts Docker or SSH. The fake `ssh` serves a captured
//! definitions fixture, so the import preflight and the live re-verify run
//! without a broker. The fixture is topology only and carries no credential.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-queue-definitions-{}-{}",
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

fn run(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new("bash");
    command.arg(repo("tools/queue-definitions.sh")).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run queue-definitions")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn fixture() -> PathBuf {
    repo("crates/quantrule-deploy/tests/fixtures/queue-definitions/topology.json")
}

fn fixture_value() -> Value {
    serde_json::from_str(&std::fs::read_to_string(fixture()).expect("fixture")).expect("JSON")
}

#[test]
fn the_tool_is_valid_shell() {
    let output = Command::new("bash")
        .args([
            "-n",
            repo("tools/queue-definitions.sh").to_str().expect("path"),
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
fn the_plan_is_docker_ssh_and_jq_free() {
    let directory = temp_dir();
    let marker = directory.join("called");
    for name in ["jq", "docker", "ssh"] {
        write_mode(
            &directory.join(name),
            &format!(
                "#!/bin/sh\nprintf '%s\\n' '{}' >> '{}'\nexit 1\n",
                name,
                marker.display()
            ),
            0o755,
        );
    }
    let path = format!(
        "{}:{}",
        directory.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let output = run(&["--plan"], &[("PATH", &path)]);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    for expected in [
        "no Docker, SSH, jq, or secret file is read",
        "export_definitions",
        "users, permissions, and topic_permissions",
        "delivery limit 3",
        "no runs.*/stages.* policy",
        "mode 600",
        "empty broker/cluster",
        "--force",
        "older than 30 days",
        "HA-013",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!marker.exists(), "the plan called an external tool");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_valid_topology_verifies() {
    let output = run(&["verify", fixture().to_str().expect("fixture")], &[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stderr(&output);
    assert!(printed.contains("runs.backtest.g1"), "{printed}");
    assert!(printed.contains("no runs.*/stages.* policy"), "{printed}");
}

#[test]
fn a_policy_on_runs_is_refused() {
    let directory = temp_dir();
    let mut value = fixture_value();
    value["policies"] = serde_json::json!([{
        "name": "runs-ha",
        "pattern": "^runs\\.",
        "apply-to": "queues",
        "definition": {}
    }]);
    let path = directory.join("policy.json");
    std::fs::write(&path, value.to_string()).expect("policy fixture");
    let output = run(&["verify", path.to_str().expect("path")], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("a policy targets runs.* or stages.*"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_user_hash_is_refused_and_not_printed() {
    let directory = temp_dir();
    let marker = "fake-password-hash-must-not-print-4d8e";
    let mut value = fixture_value();
    value["users"] = serde_json::json!([{
        "name": "qr_queue",
        "password_hash": marker,
        "hashing_algorithm": "rabbit_password_hashing_sha256",
        "tags": []
    }]);
    let path = directory.join("users.json");
    std::fs::write(&path, value.to_string()).expect("user fixture");
    let output = run(&["verify", path.to_str().expect("path")], &[]);
    assert_eq!(output.status.code(), Some(1));
    let printed = format!("{}{}", stdout(&output), stderr(&output));
    assert!(printed.contains("users are still present"), "{printed}");
    assert!(
        !printed.contains(marker),
        "the tool printed a password hash"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_queue_without_the_quorum_arguments_is_refused() {
    let directory = temp_dir();
    let mut value = fixture_value();
    value["queues"][0]["arguments"]["x-queue-type"] = serde_json::json!("classic");
    value["queues"][1]["arguments"]["x-delivery-limit"] = serde_json::json!(5);
    let path = directory.join("mutated.json");
    std::fs::write(&path, value.to_string()).expect("mutated fixture");
    let output = run(&["verify", path.to_str().expect("path")], &[]);
    assert_eq!(output.status.code(), Some(1));
    let printed = stderr(&output);
    assert!(printed.contains("is not quorum"), "{printed}");
    assert!(printed.contains("lacks x-delivery-limit 3"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn prune_deletes_only_old_definition_files() {
    let directory = temp_dir();
    for name in [
        "2026-10-01T000000Z.json",
        "2026-10-01T000000Z.manifest.json",
        "2026-11-01T000000Z.json",
        "2026-11-01T000000Z.manifest.json",
        "manifest.json",
        "notes.json",
    ] {
        std::fs::write(directory.join(name), "{}\n").expect("definition file");
    }
    let output = run(
        &[
            "prune",
            "--dir",
            directory.to_str().expect("dir"),
            "--now",
            "2026-11-23T000000Z",
            "--days",
            "30",
        ],
        &[],
    );
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!directory.join("2026-10-01T000000Z.json").exists());
    assert!(!directory.join("2026-10-01T000000Z.manifest.json").exists());
    assert!(directory.join("2026-11-01T000000Z.json").exists());
    assert!(directory.join("2026-11-01T000000Z.manifest.json").exists());
    assert!(directory.join("manifest.json").exists());
    assert!(directory.join("notes.json").exists());
    let _ = std::fs::remove_dir_all(directory);
}

fn descriptor(directory: &Path) -> PathBuf {
    let path = directory.join("deploy.json");
    std::fs::write(
        &path,
        serde_json::json!({
            "stateful": {"rabbitmq": {"nodes": ["infra-1"]}}
        })
        .to_string(),
    )
    .expect("descriptor");
    path
}

fn fake_ssh(directory: &Path, queues: &str) -> PathBuf {
    let script = directory.join("fake-ssh");
    write_mode(
        &script,
        &format!(
            r#"#!/bin/sh
set -eu
case "$*" in
    *export_definitions*) cat '{}' ;;
    *list_queues*) printf '%b' '{}' ;;
    *rabbitmq-diagnostics*) exit 0 ;;
    *import_definitions*) cat >/dev/null; exit 0 ;;
    *) exit 0 ;;
esac
"#,
            fixture().display(),
            queues
        ),
        0o755,
    );
    script
}

#[test]
fn export_strips_credentials_and_stores_the_manifest() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory);
    let copies = directory.join("copies");
    std::fs::create_dir_all(&copies).expect("copies");
    let marker = "fake-password-hash-export-must-not-store-9a1c";
    let mut raw = fixture_value();
    raw["users"] = serde_json::json!([{
        "name": "qr_queue",
        "password_hash": marker,
        "hashing_algorithm": "rabbit_password_hashing_sha256",
        "tags": []
    }]);
    raw["permissions"] = serde_json::json!([{
        "user": "qr_queue",
        "vhost": "/",
        "configure": ".*",
        "write": ".*",
        "read": ".*"
    }]);
    let raw_path = directory.join("raw.json");
    std::fs::write(&raw_path, raw.to_string()).expect("raw fixture");
    let ssh = directory.join("fake-export-ssh");
    write_mode(
        &ssh,
        &format!(
            r#"#!/bin/sh
set -eu
case "$*" in
    *export_definitions*) cat '{}' ;;
    *sh\ -c*) cat > "$(mktemp "$QR_FAKE_COPY_DIR/copy.XXXXXX")" ;;
    *) exit 0 ;;
esac
"#,
            raw_path.display()
        ),
        0o755,
    );
    let output = run(
        &["export"],
        &[
            ("DESCRIPTOR", descriptor.to_str().expect("descriptor")),
            ("QR_QUEUE_DEFINITIONS_SSH", ssh.to_str().expect("ssh")),
            ("QR_QUEUE_DEFINITIONS_STAMP", "2026-11-23T120000Z"),
            ("QR_FAKE_COPY_DIR", copies.to_str().expect("copies")),
        ],
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    let printed = stdout(&output);
    assert!(
        printed.contains("exported 2026-11-23T120000Z.json"),
        "{printed}"
    );
    assert!(printed.contains("users/permissions stripped"), "{printed}");
    let files = std::fs::read_dir(&copies)
        .expect("copies")
        .map(|entry| entry.expect("entry").path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 2, "{files:?}");
    let definitions = files
        .iter()
        .find(|path| {
            std::fs::read_to_string(path).is_ok_and(|text| text.contains("\"runs.backtest.g1\""))
        })
        .expect("definitions copy");
    let text = std::fs::read_to_string(definitions).expect("definitions");
    assert!(!text.contains("\"users\""), "{text}");
    assert!(!text.contains("\"permissions\""), "{text}");
    assert!(!text.contains(marker), "the export stored a password hash");
    let manifest = files
        .iter()
        .find(|path| {
            std::fs::read_to_string(path).is_ok_and(|text| text.contains("\"definitions\""))
        })
        .expect("manifest copy");
    let manifest_text = std::fs::read_to_string(manifest).expect("manifest");
    assert!(!manifest_text.contains(marker), "{manifest_text}");
    assert!(manifest_text.contains("HA-013"), "{manifest_text}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn import_refuses_a_live_generation_without_force() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory);
    let ssh = fake_ssh(&directory, "runs.backtest.g1\\n");
    let output = run(
        &["import", fixture().to_str().expect("fixture")],
        &[
            ("DESCRIPTOR", descriptor.to_str().expect("descriptor")),
            ("QR_QUEUE_DEFINITIONS_SSH", ssh.to_str().expect("ssh")),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("live queue generation"),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("--force"), "{}", stderr(&output));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn import_refuses_a_non_empty_cluster_without_a_generation() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory);
    let ssh = fake_ssh(&directory, "legacy.queue\\n");
    let output = run(
        &["import", fixture().to_str().expect("fixture")],
        &[
            ("DESCRIPTOR", descriptor.to_str().expect("descriptor")),
            ("QR_QUEUE_DEFINITIONS_SSH", ssh.to_str().expect("ssh")),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(stderr(&output).contains("not empty"), "{}", stderr(&output));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn import_into_an_empty_cluster_reverifies_live_topology() {
    let directory = temp_dir();
    let descriptor = descriptor(&directory);
    let ssh = fake_ssh(&directory, "");
    let output = run(
        &["import", fixture().to_str().expect("fixture")],
        &[
            ("DESCRIPTOR", descriptor.to_str().expect("descriptor")),
            ("QR_QUEUE_DEFINITIONS_SSH", ssh.to_str().expect("ssh")),
        ],
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("verified live topology"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_tool_has_no_destructive_or_secret_command() {
    let text = std::fs::read_to_string(repo("tools/queue-definitions.sh")).expect("tool");
    for forbidden in [
        "purge_queue",
        "delete_user",
        "delete_vhost",
        "policy delete",
        "rm -rf",
        "stack rm",
        "volume rm",
        "secret rm",
        "config rm",
        "down -v",
        "set -x",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}");
    }
    for required in [
        "rabbitmqctl export_definitions",
        "del(.users, .permissions, .topic_permissions)",
        "rabbitmqctl import_definitions",
        "chmod \"$mode\"",
        "chown --reference=\"$dir\"",
        "queue-bootstrap",
        "HA-013",
    ] {
        assert!(text.contains(required), "{required}");
    }
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    let output = run(&["--surprise"], &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("unknown argument"),
        "{}",
        stderr(&output)
    );
}
