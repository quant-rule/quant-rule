use super::*;

fn with_fixture_secrets(value: &mut serde_json::Value) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let files = value["secrets"]["files"]
        .as_object_mut()
        .expect("secrets.files");
    for (name, path) in files.iter_mut() {
        *path = serde_json::json!(
            root.join("deploy/v3/secrets")
                .join(name)
                .display()
                .to_string()
        );
    }
}

fn descriptor() -> Descriptor {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example");
    with_fixture_secrets(&mut value);
    serde_json::from_value(value).expect("descriptor")
}

fn lab_descriptor() -> Descriptor {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.lab.json"))
            .expect("v3 lab");
    with_fixture_secrets(&mut value);
    serde_json::from_value(value).expect("descriptor")
}

fn row_text(row: &CheckRow) -> String {
    let tail = row
        .error
        .as_deref()
        .or(row.detail.as_deref())
        .unwrap_or_default();
    format!(
        "{}\t{}\t{}\t{}\t{}\n",
        row.status(),
        row.group,
        row.subject,
        row.state,
        tail
    )
}

fn failures(rows: &[CheckRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.is_failure())
        .map(row_text)
        .collect()
}

#[test]
fn a_valid_sample_passes_every_backup_row() {
    let rows = rows(&descriptor(), None);
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    for (subject, state) in [
        ("pitr", "configured"),
        ("stanza", "configured"),
        ("archive", "configured"),
        ("placeholder", "removed"),
        ("repository", "configured"),
        ("host", "declared"),
        ("off-host", "off-host"),
        ("object", "no-replication"),
        ("image", "labelled-fixture"),
        ("secrets", "no-values"),
        ("script", "rendered"),
        ("schedule", "utc"),
        ("unit", "configured"),
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
fn a_lab_local_patroni_image_is_reported_not_failed() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.lab.json"))
            .expect("lab descriptor");
    value["images"]["patroni"]["reference"] = serde_json::json!("10.10.0.2:5000/quantrule/patroni");
    let descriptor: Descriptor = serde_json::from_value(value).expect("lab");
    let row = image_row(&descriptor);
    assert!(!row.is_failure(), "{row:?}");
    assert_eq!(row.state, "local-lab");
}

#[test]
fn the_backup_rows_match_their_golden_fixture() {
    let rows = rows(&descriptor(), None);
    let actual: String = rows.iter().map(row_text).collect();
    assert_eq!(
        actual,
        include_str!("../../tests/fixtures/check-v3/backup.txt"),
        "{actual}"
    );
}

#[test]
fn cipher_none_without_a_passphrase_passes_the_backup_rows() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("example");
    with_fixture_secrets(&mut value);
    value["backup"]["cipher"] = serde_json::json!("none");
    value["backup"]["repo"] = serde_json::json!({
        "kind": "posix",
        "path": "/srv/quantrule/backup/pgbackrest"
    });
    let files = value["secrets"]["files"].as_object_mut().expect("secrets");
    files.remove("pgbackrest_cipher_pass");
    files.remove("pgbackrest_s3_access_key");
    files.remove("pgbackrest_s3_secret_key");
    let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
    let rows = rows(&descriptor, None);
    assert!(failures(&rows).is_empty(), "{:?}", failures(&rows));
    assert!(
        rows.iter()
            .any(|row| row.subject == "secrets" && row.state == "no-values")
    );
}

#[test]
fn the_lab_in_cluster_repository_is_labelled_not_off_host() {
    let rows = rows(&lab_descriptor(), None);
    assert!(
        rows.iter().any(|row| row.subject == "off-host"
            && row.state == "lab-not-off-host"
            && !row.is_failure()),
        "{:?}",
        failures(&rows)
    );
}

#[test]
fn an_in_cluster_repository_outside_the_lab_fails() {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("example");
    value["backup"]["repo"] = serde_json::json!({
        "kind": "s3",
        "endpoint": "minio:9000",
        "bucket": "quantrule-backup",
        "path": "/quantrule",
        "tls_ca": "/etc/quantrule/pki/ca.crt"
    });
    value["secrets"]["files"]["pgbackrest_s3_access_key"] =
        serde_json::json!("/etc/quantrule/secrets/pgbackrest_s3_access_key");
    value["secrets"]["files"]["pgbackrest_s3_secret_key"] =
        serde_json::json!("/etc/quantrule/secrets/pgbackrest_s3_secret_key");
    let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
    let failures = failures(&rows(&descriptor, None));
    assert!(
        failures.iter().any(|row| row.contains("in-cluster")),
        "{failures:?}"
    );
}

#[test]
fn a_lab_in_cluster_repository_without_the_label_fails() {
    let descriptor = lab_descriptor();
    let directory = stage_tree(&descriptor);
    for host in &descriptor.stateful.postgres.nodes {
        let path = directory.join(backup::config_source(host));
        let text = std::fs::read_to_string(&path).expect("lab config");
        std::fs::write(
                &path,
                text.replace(
                    "# not-off-host: lab-only in-cluster MinIO repository; never a production off-host claim\n",
                    "",
                ),
            )
            .expect("mutate");
    }
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures.iter().any(|row| row.contains("not-off-host")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_report_carries_no_backup_secret_value() {
    let descriptor = descriptor();
    let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "image");
    append(&mut report, &descriptor, None);
    let text = report.render();
    for name in std::iter::once(BACKUP_CIPHER_SECRET).chain(BACKUP_S3_SECRETS.iter().copied()) {
        let Some(path) = descriptor.secrets.files.get(name) else {
            continue;
        };
        let value = std::fs::read_to_string(path).unwrap_or_default();
        let value = value.trim();
        if !value.is_empty() {
            assert!(!text.contains(value), "the report printed {name}");
        }
    }
}

#[test]
fn a_mutated_archive_command_is_a_failed_row() {
    let descriptor = descriptor();
    let directory = stage_tree(&descriptor);
    let path = directory.join("patroni-infra-1.yml");
    let text = std::fs::read_to_string(&path).expect("patroni");
    std::fs::write(
        &path,
        text.replace("archive_timeout: 60", "archive_timeout: 0"),
    )
    .expect("mutate");
    let failures = failures(&rows(&descriptor, Some(&directory)));
    assert!(
        failures.iter().any(|row| row.contains("archive_timeout")),
        "{failures:?}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
#[ignore = "rewrites the checked-in backup check fixture"]
fn regenerate_the_backup_check_fixture() {
    let rows = rows(&descriptor(), None);
    let actual: String = rows.iter().map(row_text).collect();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check-v3/backup.txt");
    std::fs::write(path, actual).expect("write the backup fixture");
}

fn stage_tree(descriptor: &Descriptor) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let directory = std::env::temp_dir().join(format!(
        "qr-backup-check-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("infra")).expect("tree");
    for host in &descriptor.stateful.postgres.nodes {
        std::fs::write(
            directory.join(format!("patroni-{host}.yml")),
            crate::render::stateful::patroni_yaml(descriptor, host).expect("patroni"),
        )
        .expect("patroni");
        std::fs::write(
            directory.join(backup::config_source(host)),
            backup::config(descriptor, host).expect("pgbackrest config"),
        )
        .expect("pgbackrest config");
        std::fs::write(
            directory.join("infra").join(format!("{host}.yaml")),
            infra::unit_yaml(descriptor, host).expect("patroni unit"),
        )
        .expect("patroni unit");
    }
    let script = directory.join(backup::SCRIPT_SOURCE);
    std::fs::write(&script, backup::script(descriptor).expect("script")).expect("script");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).expect("script mode");
    std::fs::write(
        directory.join("stateful-manifest.json"),
        crate::render::stateful::stateful_manifest(descriptor).expect("manifest"),
    )
    .expect("manifest");
    directory
}
