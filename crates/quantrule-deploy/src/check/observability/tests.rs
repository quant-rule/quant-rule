//! Unit tests for the observability check rows.
//!
//! Every test uses the in-memory renderer or a temporary staged tree. No
//! Docker daemon, live scrape, secret value, or real certificate is read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::render::observability;
use crate::render::stateful;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("v3 example")
}

fn lab_descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.lab.json")).expect("v3 lab")
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-check-observability-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn stage_tree(descriptor: &Descriptor) -> PathBuf {
    let directory = temp_dir();
    let mut written = Vec::new();
    observability::render(descriptor, &directory, &mut written).expect("observability render");
    infra::render(descriptor, &directory, &mut written).expect("infra render");
    for host in &descriptor.stateful.haproxy.nodes {
        let text = stateful::haproxy_cfg(descriptor, host).expect("haproxy");
        std::fs::write(directory.join(format!("haproxy-{host}.cfg")), text).expect("haproxy");
    }
    std::fs::write(
        directory.join("stack.yaml"),
        crate::render::swarm::stack_yaml(descriptor).expect("stack"),
    )
    .expect("stack");
    directory
}

fn row_text(row: &CheckRow) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\n",
        row.status(),
        row.group,
        row.subject,
        row.state,
        row.detail
            .as_deref()
            .or(row.error.as_deref())
            .unwrap_or_default()
    )
}

fn failures(rows: &[CheckRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.is_failure())
        .map(row_text)
        .collect()
}

fn mutate(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).expect("read");
    assert!(text.contains(from), "missing mutation target {from:?}");
    std::fs::write(path, text.replacen(from, to, 1)).expect("mutate");
}

#[test]
fn a_valid_sample_passes_every_observability_row() {
    let rows = rows(&descriptor(), None);
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    for (subject, state) in [
        ("placement", "spread"),
        ("prometheus unit", "pinned"),
        ("alertmanager unit", "pinned"),
        ("loki unit", "pinned"),
        ("grafana unit", "pinned"),
        ("agents", "per-host"),
        ("prometheus.yml", "scrape"),
        ("prometheus.yml", "auth-and-tls"),
        ("prometheus.yml", "overlay-only"),
        ("alert-rules.yml", "coverage"),
        ("alert-rules.yml", "labels"),
        ("alert-rules.yml", "metrics"),
        ("alert-rules.yml", "ha-coverage"),
        ("alertmanager.yml", "receiver"),
        ("alertmanager.yml", "routing"),
        ("loki.yml", "retention"),
        ("promtail", "whitelist"),
        ("blackbox.yml", "probes"),
        ("grafana datasources", "fixed-uids"),
        ("grafana provider", "read-only"),
        ("grafana dashboards", "four"),
        ("grafana access", "anonymous-viewer"),
        ("manifest", "files"),
        ("manifest", "no-secret"),
        ("app logging", "json-file"),
        ("traefik", "metrics"),
        ("haproxy", "metrics"),
        ("docker socket", "read-only"),
        ("ports", "overlay-only"),
    ] {
        assert!(
            rows.iter()
                .any(|row| row.subject == subject && row.state == state && !row.is_failure()),
            "{subject}/{state}: {}",
            rows.iter().map(row_text).collect::<String>()
        );
    }
}

#[test]
fn the_observability_rows_match_their_golden_fixture() {
    let rows = rows(&descriptor(), None);
    let actual: String = rows.iter().map(row_text).collect();
    assert_eq!(
        actual,
        include_str!("../../../tests/fixtures/check-v3/observability.txt"),
        "{actual}"
    );
}

#[test]
#[ignore = "rewrites the checked-in observability check fixture"]
fn regenerate_the_observability_check_fixture() {
    let rows = rows(&descriptor(), None);
    let actual: String = rows.iter().map(row_text).collect();
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check-v3/observability.txt");
    std::fs::write(path, actual).expect("write the observability fixture");
}

#[test]
fn a_missing_alert_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "alert: \"GatewayReplicaDown\"",
        "alert: \"GatewayReplicaDownRetired\"",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("GatewayReplicaDown")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_failure_row_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "          failure_row: \"F02\"\n",
        "",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures.iter().any(|row| row.contains("F02 has no rule")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unknown_failure_row_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "failure_row: \"F03\"",
        "failure_row: \"F18\"",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures.iter().any(|row| row.contains("F18")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_duplicate_alert_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "alert: \"DrainStuck\"",
        "alert: \"ServiceTargetDown\"",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("duplicate alert ServiceTargetDown")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_scrape_job_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("prometheus.yml"),
        "job_name: \"rabbitmq\"",
        "job_name: \"rabbitmq_retired\"",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("missing scrape job rabbitmq")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_wrong_patroni_metric_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "patroni_primary",
        "patroni_master",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("retired metric patroni_master")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_wrong_minio_metric_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("alert-rules.yml"),
        "minio_cluster_drive_offline_total",
        "minio_cluster_disk_offline_total",
    );
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("retired metric minio_cluster_disk_offline_total")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_dashboard_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    std::fs::remove_file(directory.join("grafana/dashboards/capacity.json")).expect("remove");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures.iter().any(|row| row.contains("capacity.json")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_public_port_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/infra-1.yaml");
    let mut text = std::fs::read_to_string(&path).expect("unit");
    text.push_str("    ports:\n      - \"9090:9090\"\n");
    std::fs::write(&path, text).expect("mutate");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("publishes a port") || row.contains("published port")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_external_receiver_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("alertmanager.yml");
    let mut text = std::fs::read_to_string(&path).expect("alertmanager");
    text.push_str("webhook_configs:\n  - url: https://example.invalid/hook\n");
    std::fs::write(&path, text).expect("mutate");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("external receiver key webhook_configs")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_manifest_entry_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("observability-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("manifest")).expect("JSON");
    value["files"]
        .as_array_mut()
        .expect("files")
        .retain(|entry| entry["source"] != "grafana/dashboards/overview.json");
    std::fs::write(&path, value.to_string()).expect("mutate");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("grafana/dashboards/overview.json")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_writable_docker_socket_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/core-1.yaml");
    let text = std::fs::read_to_string(&path).expect("unit");
    std::fs::write(
        &path,
        text.replacen(
            "read_only: true\n        source: \"/var/run/docker.sock\"",
            "read_only: false\n        source: \"/var/run/docker.sock\"",
            1,
        ),
    )
    .expect("mutate");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures
            .iter()
            .any(|row| row.contains("writable") || row.contains("read-only")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_rows_use_the_lab_names() {
    let descriptor = lab_descriptor();
    let rows = rows(&descriptor, None);
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    let text = rows.iter().map(row_text).collect::<String>();
    assert!(text.contains("lab-1"), "{text}");
    assert!(!text.contains("infra-1"), "{text}");
}
