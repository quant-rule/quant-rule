//! Unit tests for the MinIO check rows.
//!
//! The fixture certificates are fake and the `CertificateQuery` is a seam; no
//! live `openssl`, Docker daemon, MinIO server, or secret value is read.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::check::ingress::CertificateFacts;
use crate::render::minio::minio_sans;

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct FakeCertificates {
    names: Vec<String>,
    valid_for_days: bool,
    error: Option<String>,
}

impl FakeCertificates {
    fn valid() -> Self {
        Self {
            names: minio_sans(&descriptor()),
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

fn local_paths(mut descriptor: Descriptor) -> Descriptor {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/v3/secrets");
    for (name, path) in &mut descriptor.secrets.files {
        *path = root.join(name).display().to_string();
    }
    descriptor
}

fn descriptor() -> Descriptor {
    let text = include_str!("../../../../../deploy/v3/deploy.example.json");
    local_paths(serde_json::from_str(text).expect("v3 example"))
}

fn lab_descriptor() -> Descriptor {
    let text = include_str!("../../../../../deploy/v3/deploy.lab.json");
    local_paths(serde_json::from_str(text).expect("v3 lab"))
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-check-minio-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn stage_tree(descriptor: &Descriptor) -> PathBuf {
    let directory = temp_dir();
    for host in &descriptor.stateful.minio.nodes {
        let text = minio::minio_conf(descriptor, host).expect("minio");
        std::fs::write(directory.join(format!("minio-{host}.conf")), text).expect("minio");
    }
    std::fs::write(directory.join("minio-policy.json"), minio::bucket_policy()).expect("policy");
    for host in &descriptor.stateful.haproxy.nodes {
        let text = stateful::haproxy_cfg(descriptor, host).expect("haproxy");
        std::fs::write(directory.join(format!("haproxy-{host}.cfg")), text).expect("haproxy");
    }
    std::fs::create_dir_all(directory.join("infra")).expect("infra");
    for host in &descriptor.stateful.minio.nodes {
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("minio unit"),
        )
        .expect("minio unit");
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
    std::fs::write(
        directory.join("stack.yaml"),
        swarm::stack_yaml(descriptor).expect("stack"),
    )
    .expect("stack");
    std::fs::create_dir_all(directory.join("pki/minio")).expect("minio pki");
    for name in ["pki/minio/public.crt", "pki/minio/private.key"] {
        std::fs::write(directory.join(name), "fixture\n").expect("pki file");
    }
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
fn a_valid_sample_passes_every_minio_row() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    for (subject, state) in [
        ("minio", "conf"),
        ("minio", "no-secret"),
        ("minio", "overlay-only"),
        ("minio", "quorum"),
        ("minio", "unit"),
        ("haproxy", "frontend"),
        ("haproxy", "backend"),
        ("haproxy", "unit"),
        ("manifest", "hosts"),
        ("manifest", "minio-modes"),
        ("manifest", "sans"),
        ("minio certificate", "san"),
        ("app", "object-store"),
        ("app", "credentials"),
        ("app", "replication"),
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
fn the_minio_rows_match_their_golden_fixture() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    let actual: String = rows.iter().map(row_text).collect();
    assert_eq!(
        actual,
        include_str!("../../../tests/fixtures/check-v3/minio.txt"),
        "{actual}"
    );
}

#[test]
#[ignore = "rewrites the checked-in MinIO check fixture"]
fn regenerate_the_minio_check_fixture() {
    let rows = rows(&descriptor(), None, &FakeCertificates::valid());
    let actual: String = rows.iter().map(row_text).collect();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check-v3/minio.txt");
    std::fs::write(path, actual).expect("write the MinIO fixture");
}

#[test]
fn a_mutated_conf_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    mutate(
        &directory.join("minio-infra-1.conf"),
        "MINIO_BROWSER=\"off\"",
        "MINIO_BROWSER=\"on\"",
    );
    let rows = self::rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.contains("MINIO_BROWSER")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_secret_value_in_the_conf_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let password = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/v3/secrets/minio_root_password"),
    )
    .expect("fixture password");
    let path = directory.join("minio-infra-1.conf");
    let mut text = std::fs::read_to_string(&path).expect("conf");
    text.push_str(&format!("MINIO_ROOT_PASSWORD=\"{}\"\n", password.trim()));
    std::fs::write(&path, text).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.contains("minio_root_password")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unsafe_drive_layout_is_refused() {
    let mut descriptor = lab_descriptor();
    descriptor.stateful.minio.drives = vec![2, 1, 1];
    descriptor.stateful.minio.parity = "auto".to_owned();
    let rows = rows(&descriptor, None, &FakeCertificates::valid());
    assert!(
        rows.iter().any(|row| row.subject == "minio"
            && row.state == "layout"
            && row
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("write quorum")),
        "{:?}",
        failures(&rows)
    );
}

#[test]
fn a_minio_unit_with_a_shared_alias_or_published_port_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/infra-1.yaml");
    mutate(
        &path,
        "            \"minio-infra-1\"\n",
        "            \"minio\"\n",
    );
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows).iter().any(|row| row.contains("\"minio\"")),
        "{:?}",
        failures(&rows)
    );

    let directory = stage_tree(&descriptor);
    let path = directory.join("infra/infra-1.yaml");
    mutate(
        &path,
        "    hostname: \"minio-infra-1\"\n",
        "    hostname: \"minio-infra-1\"\n    ports:\n      - \"9000:9000\"\n",
    );
    let rows = self::rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows).iter().any(|row| row.contains("ports:")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_haproxy_without_a_minio_backend_server_or_shutdown_sessions_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("haproxy-edge-1.cfg");
    let text = std::fs::read_to_string(&path).expect("haproxy");
    let mutated = text
        .replace("  server minio-infra-2 minio-infra-2:9000 check\n", "")
        .replace(
            "  on-marked-down shutdown-sessions\n\nlisten stats",
            "\nlisten stats",
        );
    std::fs::write(&path, mutated).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    let failures = failures(&rows);
    assert!(
        failures
            .iter()
            .any(|row| row.contains("minio-infra-2:9000")),
        "{failures:?}"
    );
    assert!(
        failures.iter().any(|row| row.contains("shutdown-sessions")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_manifest_that_sends_the_root_files_to_every_host_fails() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("stateful-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("manifest")).expect("JSON");
    for entry in value["files"].as_array_mut().expect("files") {
        if entry["source"] == "secrets/minio_root_password" {
            entry["hosts"] = serde_json::json!(["edge-1", "infra-1"]);
        }
    }
    std::fs::write(&path, value.to_string()).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.contains("minio_root_password")),
        "{:?}",
        failures(&rows)
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
    value["sans"]
        .as_array_mut()
        .expect("sans")
        .retain(|name| name != "minio-infra-2");
    std::fs::write(&path, value.to_string()).expect("mutate");
    let rows = rows(&descriptor, Some(&directory), &FakeCertificates::valid());
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.contains("missing [minio-infra-2]")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_minio_san_is_a_failed_row() {
    let mut certificates = FakeCertificates::valid();
    certificates.names.retain(|name| name != "minio-infra-2");
    let rows = rows(&descriptor(), None, &certificates);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.contains("minio certificate") && row.contains("minio-infra-2")),
        "{:?}",
        failures(&rows)
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
            .any(|row| row.subject == "minio certificate" && row.state == "expiring"),
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
            .any(|row| row.subject == "minio certificate" && row.state == "limited"),
        "{:?}",
        failures(&rows)
    );
}

#[test]
fn a_non_qr_app_access_key_fails() {
    let directory = temp_dir();
    let path = directory.join("object_store_access_key");
    std::fs::write(&path, "someone_else\n").expect("access key");
    let mut descriptor = descriptor();
    descriptor.secrets.files.insert(
        "object_store_access_key".to_owned(),
        path.display().to_string(),
    );
    let rows = rows(&descriptor, None, &FakeCertificates::valid());
    assert!(
        failures(&rows).iter().any(|row| row.contains("not qr_app")),
        "{:?}",
        failures(&rows)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_lab_rows_use_the_lab_names() {
    let descriptor = lab_descriptor();
    let rows = rows(
        &descriptor,
        None,
        &FakeCertificates {
            names: minio_sans(&descriptor),
            valid_for_days: true,
            error: None,
        },
    );
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    let text = rows.iter().map(row_text).collect::<String>();
    assert!(text.contains("minio-lab-1"), "{text}");
    assert!(text.contains("EC:3"), "{text}");
    assert!(!text.contains("minio-infra-1"), "{text}");
}

#[test]
fn the_report_carries_no_fixture_secret_value() {
    let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "image");
    append(&mut report, &descriptor(), None, &FakeCertificates::valid());
    let text = report.render();
    for secret in stateful::STATEFUL_SECRETS
        .into_iter()
        .chain(stateful::MINIO_SECRETS)
        .chain(["object_store_secret_key"])
    {
        let value =
            std::fs::read_to_string(format!("../deploy/v3/secrets/{secret}")).unwrap_or_default();
        let value = value.trim();
        if !value.is_empty() {
            assert!(!text.contains(value), "the report printed {secret}");
        }
    }
}
