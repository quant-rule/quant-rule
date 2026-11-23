//! The image entrypoint's private-bind resolution.
//!
//! These run the real shell entrypoint with a fake `id` (so secret staging is
//! skipped) and a fake `getent` (so no container or Swarm task is needed).

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn entrypoint() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docker/entrypoint.sh")
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-entrypoint-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn executable(path: &Path, body: &str) {
    std::fs::write(path, body).expect("write script");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("mode");
}

fn fake_path(directory: &Path, getent: &str) -> String {
    executable(&directory.join("id"), "#!/bin/sh\nprintf '1000\\n'\n");
    executable(&directory.join("getent"), &format!("#!/bin/sh\n{getent}\n"));
    format!(
        "{}:{}",
        directory.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

fn run(directory: &Path, bind: &str, metrics: Option<&str>, internal: bool) -> Output {
    let mut command = Command::new("sh");
    command
        .arg(entrypoint())
        .arg("sh")
        .arg("-c")
        .arg("printf '%s|%s' \"$QR_BIND\" \"${QR_METRICS_BIND:-unset}\"")
        .env("PATH", fake_path(directory, "printf '10.1.2.3 task\\n'"))
        .env("QR_BIND", bind)
        .env_remove("QR_METRICS_BIND");
    if let Some(metrics) = metrics {
        command.env("QR_METRICS_BIND", metrics);
    }
    if internal {
        command.env("QR_INTERNAL_ONLY", "1");
    } else {
        command.env_remove("QR_INTERNAL_ONLY");
    }
    command.output().expect("run entrypoint")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn an_unspecified_bind_becomes_the_task_address_on_both_listeners() {
    let directory = temp_dir();
    let output = run(&directory, "0.0.0.0:8080", Some("0.0.0.0:9100"), true);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "10.1.2.3:8080|10.1.2.3:9100");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_port_is_preserved_and_an_ipv6_address_is_bracketed() {
    let directory = temp_dir();
    executable(&directory.join("id"), "#!/bin/sh\nprintf '1000\\n'\n");
    executable(
        &directory.join("getent"),
        "#!/bin/sh\nprintf 'fd00::10 task\\n'\n",
    );
    let output = Command::new("sh")
        .arg(entrypoint())
        .arg("sh")
        .arg("-c")
        .arg("printf '%s' \"$QR_BIND\"")
        .env(
            "PATH",
            format!(
                "{}:{}",
                directory.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("QR_BIND", "0.0.0.0:18081")
        .env_remove("QR_METRICS_BIND")
        .env("QR_INTERNAL_ONLY", "1")
        .output()
        .expect("run entrypoint");
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "[fd00::10]:18081");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unset_metrics_bind_stays_unset() {
    let directory = temp_dir();
    let output = run(&directory, "0.0.0.0:8080", None, true);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "10.1.2.3:8080|unset");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_private_address_fails_closed() {
    let directory = temp_dir();
    let mut command = Command::new("sh");
    command
        .arg(entrypoint())
        .arg("true")
        .env("PATH", fake_path(&directory, "exit 1"))
        .env("QR_BIND", "0.0.0.0:8080")
        .env("QR_INTERNAL_ONLY", "1");
    let output = command.output().expect("run entrypoint");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("cannot resolve a non-loopback address"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_loopback_only_hostname_fails_closed() {
    let directory = temp_dir();
    executable(&directory.join("id"), "#!/bin/sh\nprintf '1000\\n'\n");
    executable(
        &directory.join("getent"),
        "#!/bin/sh\nprintf '127.0.1.1 task\\n'\n",
    );
    let output = Command::new("sh")
        .arg(entrypoint())
        .arg("true")
        .env(
            "PATH",
            format!(
                "{}:{}",
                directory.display(),
                std::env::var("PATH").unwrap_or_default()
            ),
        )
        .env("QR_BIND", "0.0.0.0:8080")
        .env("QR_INTERNAL_ONLY", "1")
        .output()
        .expect("run entrypoint");
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("cannot resolve a non-loopback address"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_resolution_is_only_applied_when_internal_only_is_set() {
    let directory = temp_dir();
    let output = run(&directory, "0.0.0.0:8080", None, false);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "0.0.0.0:8080|unset");
    let _ = std::fs::remove_dir_all(directory);
}
