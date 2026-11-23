//! Unit tests for the RabbitMQ queue check rows.
//!
//! The fixture certificates are fake and the `CertificateQuery` is a seam; no
//! live `openssl`, Docker daemon, broker, or secret value is read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::check::ingress::CertificateFacts;
use crate::render::queue::queue_sans;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct FakeCertificates {
    names: Vec<String>,
    valid_for_days: bool,
    error: Option<String>,
}

impl FakeCertificates {
    fn valid() -> Self {
        Self {
            names: queue_sans(&descriptor()),
            valid_for_days: true,
            error: None,
        }
    }
}

impl CertificateQuery for FakeCertificates {
    fn inspect(&self, _path: &Path) -> Result<CertificateFacts, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(CertificateFacts {
            dns_names: self.names.clone(),
            valid_for_days: self.valid_for_days,
        })
    }
}

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("v3 example")
}

fn lab_descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.lab.json")).expect("v3 lab")
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-check-queue-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn stage_tree(descriptor: &Descriptor) -> PathBuf {
    let directory = temp_dir();
    for host in &descriptor.stateful.rabbitmq.nodes {
        let text = queue::rabbitmq_conf(descriptor, host).expect("rabbitmq");
        std::fs::write(directory.join(format!("rabbitmq-{host}.conf")), text).expect("rabbitmq");
    }
    std::fs::write(
        directory.join("rabbitmq-enabled_plugins"),
        queue::enabled_plugins(),
    )
    .expect("plugins");
    for host in &descriptor.stateful.haproxy.nodes {
        let text = stateful::haproxy_cfg(descriptor, host).expect("haproxy");
        std::fs::write(directory.join(format!("haproxy-{host}.cfg")), text).expect("haproxy");
    }
    std::fs::create_dir_all(directory.join("infra")).expect("infra");
    for host in &descriptor.stateful.rabbitmq.nodes {
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("broker unit"),
        )
        .expect("broker unit");
    }
    for host in &descriptor.stateful.haproxy.nodes {
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("haproxy unit"),
        )
        .expect("haproxy unit");
    }
    let manifest = stateful::stateful_manifest(descriptor).expect("manifest");
    std::fs::write(directory.join("stateful-manifest.json"), manifest).expect("manifest");
    std::fs::create_dir_all(directory.join("pki/rabbitmq")).expect("rabbitmq pki");
    for name in [
        "pki/rabbitmq/ca.crt",
        "pki/rabbitmq/server.crt",
        "pki/rabbitmq/server.key",
    ] {
        std::fs::write(directory.join(name), "fixture\n").expect("pki file");
    }
    directory
}

fn failures(rows: &[CheckRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.is_failure())
        .map(|row| {
            format!(
                "{}\t{}\t{}\t{}",
                row.subject,
                row.state,
                row.detail.as_deref().unwrap_or_default(),
                row.error.as_deref().unwrap_or_default()
            )
        })
        .collect()
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

#[test]
fn a_valid_sample_passes_every_queue_row() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    for (subject, state) in [
        ("rabbitmq", "tls-only"),
        ("rabbitmq", "guest-loopback"),
        ("rabbitmq", "no-policy"),
        ("rabbitmq", "cluster"),
        ("rabbitmq", "partition-handling"),
        ("rabbitmq", "queue-default"),
        ("rabbitmq", "alarms"),
        ("rabbitmq", "metrics"),
        ("rabbitmq", "unit"),
        ("haproxy", "frontend"),
        ("haproxy", "backend"),
        ("haproxy", "unit"),
        ("manifest", "hosts"),
        ("manifest", "broker-modes"),
        ("manifest", "sans"),
        ("rabbitmq certificate", "san"),
    ] {
        assert!(
            rows.iter()
                .any(|row| row.subject == subject && row.state == state && !row.is_failure()),
            "{subject}/{state}: {:?}",
            rows.iter().map(row_text).collect::<String>()
        );
    }
}

#[test]
fn the_queue_rows_match_their_golden_fixture() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    let actual: String = rows.iter().map(row_text).collect();
    assert_eq!(
        actual,
        include_str!("../../../tests/fixtures/check-v3/queue.txt"),
        "{actual}"
    );
}

#[test]
fn a_mutated_broker_config_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("rabbitmq-infra-1.conf");
    let text = std::fs::read_to_string(&path).expect("conf");
    std::fs::write(
        &path,
        text.replace("listeners.tcp = none", "listeners.tcp.default = 5672"),
    )
    .expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("listeners.tcp = none")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_operator_policy_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("rabbitmq-infra-1.conf");
    let text = std::fs::read_to_string(&path).expect("conf");
    std::fs::write(
        &path,
        format!("{text}\npolicy.runs.ha = {{\"pattern\":\"^runs\\\\.\",\"definition\":{{}}}}\n"),
    )
    .expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("policy.") || row.contains("runs.")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_peer_and_shutdown_sessions_are_failed_rows() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("haproxy-edge-1.cfg");
    let text = std::fs::read_to_string(&path).expect("haproxy");
    let mutated = text
        .replace(
            "  server rabbitmq-infra-2 rabbitmq-infra-2:5671 check\n",
            "",
        )
        .replace(
            "  server rabbitmq-infra-3 rabbitmq-infra-3:5671 check\n  on-marked-down shutdown-sessions\n\nfrontend minio",
            "  server rabbitmq-infra-3 rabbitmq-infra-3:5671 check\n\nfrontend minio",
        );
    std::fs::write(&path, mutated).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("rabbitmq-infra-2:5671")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|row| row.contains("shutdown-sessions")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_broker_unit_with_a_long_name_or_shared_alias_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/infra-1.yaml");
    let text = std::fs::read_to_string(&path).expect("unit");
    let mutated = text
        .replace(
            "RABBITMQ_NODENAME: \"rabbit@rabbitmq-infra-1\"",
            "RABBITMQ_NODENAME: \"rabbit@rabbitmq-infra-1\"\n      RABBITMQ_USE_LONGNAME: \"true\"",
        )
        .replace(
            "            \"rabbitmq-infra-1\"",
            "            \"rabbitmq\"",
        );
    std::fs::write(&path, mutated).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("RABBITMQ_USE_LONGNAME")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|row| row.contains("\"rabbitmq\"")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_manifest_that_sends_the_cookie_to_every_host_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("stateful-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("manifest")).expect("JSON");
    for entry in value["files"].as_array_mut().expect("files") {
        if entry["source"] == "secrets/rabbitmq_erlang_cookie" {
            entry["hosts"] = serde_json::json!(["infra-1", "infra-2", "infra-3", "edge-1"]);
        }
    }
    std::fs::write(&path, value.to_string()).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("rabbitmq_erlang_cookie")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_manifest_sans_mismatch_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("stateful-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("manifest")).expect("JSON");
    value["sans"].as_array_mut().expect("sans").pop();
    std::fs::write(&path, value.to_string()).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows).iter().any(|row| row.contains("missing [")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_broker_san_is_a_failed_row() {
    let mut certificates = FakeCertificates::valid();
    certificates.names.retain(|name| name != "rabbitmq-infra-2");
    let rows = rows(&descriptor(), None, &certificates);
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("rabbitmq certificate") && row.contains("rabbitmq-infra-2")),
        "{failures:?}"
    );
}

#[test]
fn an_expiring_or_missing_openssl_certificate_is_a_failed_row() {
    let mut certificates = FakeCertificates::valid();
    certificates.valid_for_days = false;
    let expiring = rows(&descriptor(), None, &certificates);
    assert!(
        expiring
            .iter()
            .any(|row| row.subject == "rabbitmq certificate" && row.state == "expiring"),
        "{:?}",
        failures(&expiring)
    );

    let certificates = FakeCertificates {
        names: Vec::new(),
        valid_for_days: true,
        error: Some("cannot run openssl x509: No such file or directory".to_owned()),
    };
    let rows = rows(&descriptor(), None, &certificates);
    assert!(
        rows.iter()
            .any(|row| row.subject == "rabbitmq certificate" && row.state == "limited"),
        "{:?}",
        failures(&rows)
    );
}

#[test]
fn the_lab_rows_use_the_lab_names() {
    let descriptor = lab_descriptor();
    let rows = rows(
        &descriptor,
        None,
        &FakeCertificates {
            names: queue_sans(&descriptor),
            valid_for_days: true,
            error: None,
        },
    );
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    let text = rows.iter().map(row_text).collect::<String>();
    assert!(text.contains("rabbitmq-lab-1"), "{text}");
    assert!(!text.contains("rabbitmq-infra-1"), "{text}");
}

#[test]
fn the_report_carries_no_fixture_secret_value() {
    let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "image");
    append(&mut report, &descriptor(), None, &FakeCertificates::valid());
    let text = report.render();
    for secret in stateful::STATEFUL_SECRETS {
        let value =
            std::fs::read_to_string(format!("../deploy/v3/secrets/{secret}")).unwrap_or_default();
        let value = value.trim();
        if !value.is_empty() {
            assert!(!text.contains(value), "the report printed {secret}");
        }
    }
}
