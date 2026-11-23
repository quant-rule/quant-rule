//! Unit tests for the stateful check rows.
//!
//! The fixture certificates are fake and the `CertificateQuery` is a seam; no
//! live `openssl`, Docker daemon, or secret value is read.

use super::*;
use crate::render::infra;

use crate::render::stateful::stateful_sans;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct FakeCertificates {
    names: Vec<String>,
    error: Option<String>,
}

impl FakeCertificates {
    fn valid() -> Self {
        Self {
            names: stateful_sans(&descriptor()),
            error: None,
        }
    }
}

impl CertificateQuery for FakeCertificates {
    fn inspect(&self, _path: &Path) -> Result<super::super::ingress::CertificateFacts, String> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        Ok(super::super::ingress::CertificateFacts {
            dns_names: self.names.clone(),
            valid_for_days: true,
        })
    }
}

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("v3 example")
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-check-stateful-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn stage_tree(descriptor: &Descriptor) -> PathBuf {
    let directory = temp_dir();
    for host in &descriptor.stateful.postgres.nodes {
        let text = stateful::patroni_yaml(descriptor, host).expect("patroni");
        std::fs::write(directory.join(format!("patroni-{host}.yml")), text).expect("patroni");
    }
    for host in &descriptor.stateful.etcd.nodes {
        let text = stateful::etcd_yaml(descriptor, host).expect("etcd");
        std::fs::write(directory.join(format!("etcd-{host}.conf.yml")), text).expect("etcd");
    }
    for host in &descriptor.stateful.haproxy.nodes {
        let text = stateful::haproxy_cfg(descriptor, host).expect("haproxy");
        std::fs::write(directory.join(format!("haproxy-{host}.cfg")), text).expect("haproxy");
    }
    std::fs::create_dir_all(directory.join("infra")).expect("infra");
    for host in &descriptor.stateful.postgres.nodes {
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("patroni unit"),
        )
        .expect("patroni unit");
    }
    for host in &descriptor.stateful.etcd.nodes {
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("etcd unit"),
        )
        .expect("etcd unit");
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
    std::fs::create_dir_all(directory.join("pki/postgres")).expect("postgres pki");
    std::fs::create_dir_all(directory.join("pki/etcd")).expect("etcd pki");
    for name in [
        "pki/database-ca.crt",
        "pki/postgres/server.crt",
        "pki/postgres/server.key",
        "pki/etcd/server.crt",
        "pki/etcd/server.key",
        "pki/etcd/peer.crt",
        "pki/etcd/peer.key",
        "pki/etcd/client.crt",
        "pki/etcd/client.key",
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
fn a_valid_sample_passes_every_stateful_row() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    for (subject, state) in [
        ("patroni", "synchronous-mode"),
        ("patroni", "synchronous-mode-strict"),
        ("patroni", "synchronous-node-count"),
        ("patroni", "rest-auth"),
        ("patroni", "watchdog"),
        ("etcd", "members"),
        ("etcd", "client-tls"),
        ("haproxy", "health-check"),
        ("haproxy", "servers"),
        ("manifest", "sans"),
        ("postgres certificate", "san"),
        ("etcd server certificate", "san"),
        ("etcd peer certificate", "san"),
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
fn the_stateful_rows_match_their_golden_fixture() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    let actual: String = rows.iter().map(row_text).collect();
    assert_eq!(
        actual,
        include_str!("../../../tests/fixtures/check-v3/stateful.txt"),
        "{actual}"
    );
}

#[test]
fn a_mutated_rendered_config_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("patroni-infra-1.yml");
    let text = std::fs::read_to_string(&path).expect("patroni");
    std::fs::write(
        &path,
        text.replace(
            "synchronous_mode_strict: true",
            "synchronous_mode_strict: false",
        ),
    )
    .expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("synchronous_mode_strict")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_rendered_file_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    std::fs::remove_file(directory.join("haproxy-edge-1.cfg")).expect("remove");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("haproxy-edge-1.cfg")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|row| row.contains("render tree")),
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
        failures(&rows)
            .iter()
            .any(|row| row.contains("sans-mismatch")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_patroni_san_is_a_failed_row() {
    let mut certificates = FakeCertificates::valid();
    certificates.names.retain(|name| name != "patroni-infra-2");
    let rows = rows(&descriptor(), None, &certificates);
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("postgres certificate") && row.contains("patroni-infra-2")),
        "{failures:?}"
    );
}

#[test]
fn a_missing_openssl_is_a_limited_failure() {
    let certificates = FakeCertificates {
        names: Vec::new(),
        error: Some("cannot run openssl x509: No such file or directory".to_owned()),
    };
    let rows = rows(&descriptor(), None, &certificates);
    assert_eq!(
        rows.iter()
            .filter(|row| row.state == "limited" && row.is_failure())
            .count(),
        3
    );
}

#[test]
fn a_haproxy_unit_without_the_postgres_alias_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/edge-1.yaml");
    let text = std::fs::read_to_string(&path).expect("unit");
    std::fs::write(&path, text.replace("\"postgres\"", "\"not-postgres\"")).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures.iter().any(|row| row.contains("postgres")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
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
