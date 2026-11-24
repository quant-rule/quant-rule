//! pgBackRest and backup-age rows of a v3 check.
//!
//! The rows read the same rendered text the operator copies: one
//! `pgbackrest-<host>.conf` per Patroni node, the backup-host script, the
//! Patroni unit and its secret mounts, and the stateful manifest. The default is
//! Docker-free. A missing `openssl` is unrelated here; no certificate is queried.
//!
//! The live backup, verify, restore, and PITR drill remain `Unverified`. These
//! rows prove the rendered contract, not that a repository received a byte.

use std::path::Path;

use crate::descriptor::v3::{
    APP_OBJECT_BUCKET, BackupCipher, BackupRepoKind, Descriptor, PATRONI_BASE_DIGEST,
    PGBACKREST_FIXTURE_DIGEST, PGBACKREST_IMAGE_REFERENCE, Profile,
};
use crate::render::backup;
use crate::render::infra;
use crate::validate::secrets::{BACKUP_CIPHER_SECRET, BACKUP_S3_SECRETS};

use super::report::{CheckReport, CheckRow};
use super::stateful::read_text;

/// Append every backup row from the rendered tree or the in-memory render.
pub(crate) fn append(report: &mut CheckReport, descriptor: &Descriptor, out: Option<&Path>) {
    for row in rows(descriptor, out) {
        report.push(row);
    }
}

/// The backup rows in check order.
pub(crate) fn rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    vec![
        pitr_row(descriptor),
        stanza_row(descriptor, out),
        archive_row(descriptor, out),
        placeholder_row(descriptor, out),
        repository_row(descriptor, out),
        host_row(descriptor, out),
        off_host_row(descriptor, out),
        object_row(descriptor, out),
        image_row(descriptor),
        secrets_row(descriptor, out),
        script_row(descriptor, out),
        schedule_row(descriptor, out),
        unit_row(descriptor, out),
    ]
}

fn pitr_row(descriptor: &Descriptor) -> CheckRow {
    if !descriptor.backup.pitr {
        return CheckRow::failed(
            "backup",
            "pitr",
            "disabled",
            "backup.pitr must be true; the HA contract requires PITR",
        );
    }
    CheckRow::ok(
        "backup",
        "pitr",
        "configured",
        format!(
            "stanza {} covers all four databases; {} day retention; the live restore/PITR drill is Unverified",
            backup::STANZA,
            descriptor.backup.retention_days
        ),
    )
}

fn patroni_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("patroni-{host}.yml")),
                "Patroni config",
            )?,
            None => crate::render::stateful::patroni_yaml(descriptor, host)
                .map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn config_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        let name = backup::config_source(host);
        let text = match out {
            Some(directory) => read_text(&directory.join(&name), "pgBackRest config")?,
            None => backup::config(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn stanza_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let texts = match config_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return CheckRow::failed("backup", "stanza", "unreadable", error),
    };
    let mut missing = Vec::new();
    for (host, text) in &texts {
        for needle in [
            format!("[{}]", backup::STANZA),
            "pg1-path=/var/lib/postgresql/data".to_owned(),
            "pg1-port=5432".to_owned(),
            "pg1-socket-path=/run/postgresql".to_owned(),
        ] {
            if !text.contains(&needle) {
                missing.push(format!("{host} lacks {needle}"));
            }
        }
    }
    if missing.is_empty() {
        CheckRow::ok(
            "backup",
            "stanza",
            "configured",
            format!(
                "stanza {}; {} Patroni config(s) with pg1-path, pg1-port 5432, and the socket directory",
                backup::STANZA,
                texts.len()
            ),
        )
    } else {
        CheckRow::failed("backup", "stanza", "configuration", missing.join("; "))
    }
}

fn archive_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let texts = match patroni_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return CheckRow::failed("backup", "archive", "unreadable", error),
    };
    let mut missing = Vec::new();
    for (host, text) in &texts {
        for needle in [
            "archive_mode: \"on\"",
            "archive_timeout: 60",
            "wal_level: \"replica\"",
            "pgbackrest --stanza=quantrule archive-push %p",
            "pgbackrest --stanza=quantrule archive-get %f",
        ] {
            if !text.contains(needle) {
                missing.push(format!("{host} lacks {needle:?}"));
            }
        }
    }
    if missing.is_empty() {
        CheckRow::ok(
            "backup",
            "archive",
            "configured",
            "archive_mode on, archive_timeout 60, wal_level replica; archive-push and archive-get use the quantrule stanza",
        )
    } else {
        CheckRow::failed("backup", "archive", "configuration", missing.join("; "))
    }
}

fn placeholder_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    let texts = match patroni_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return CheckRow::failed("backup", "placeholder", "unreadable", error),
    };
    for (host, text) in &texts {
        for forbidden in ["/srv/quantrule/wal-archive", "test ! -f"] {
            if text.contains(forbidden) {
                issues.push(format!("{host} still carries {forbidden}"));
            }
        }
    }
    for host in &descriptor.stateful.postgres.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        if text.contains("wal-archive") {
            issues.push(format!("infra/{host}.yaml still mounts a wal-archive path"));
        }
        if text.contains("/srv/quantrule/patroni/wal-archive") {
            issues.push(format!(
                "infra/{host}.yaml still names the node-local WAL path"
            ));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "backup",
            "placeholder",
            "removed",
            "no node-local WAL archive remains in the Patroni config or unit",
        )
    } else {
        CheckRow::failed("backup", "placeholder", "configuration", issues.join("; "))
    }
}

fn repository_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let texts = match config_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return CheckRow::failed("backup", "repository", "unreadable", error),
    };
    let mut required = vec![
        "repo1-retention-full-type=time".to_owned(),
        format!("repo1-retention-full={}", descriptor.backup.retention_days),
        "repo1-retention-diff=7".to_owned(),
        format!("repo1-cipher-type={}", descriptor.backup.cipher.as_str()),
    ];
    match descriptor.backup.repo.kind {
        BackupRepoKind::Posix => required.push(format!(
            "repo1-path={}",
            backup::posix_path(descriptor).unwrap_or(crate::descriptor::v3::DEFAULT_BACKUP_PATH)
        )),
        BackupRepoKind::S3 => {
            required.push(format!(
                "repo1-s3-endpoint={}",
                descriptor.backup.repo.endpoint.as_deref().unwrap_or("")
            ));
            required.push(format!(
                "repo1-s3-bucket={}",
                descriptor.backup.repo.bucket.as_deref().unwrap_or("")
            ));
            required.push(format!(
                "repo1-path={}",
                descriptor.backup.repo.path.as_deref().unwrap_or("/")
            ));
            required.push("repo1-storage-verify-tls=y".to_owned());
            required.push(format!(
                "repo1-storage-ca-file={}",
                descriptor.backup.repo.tls_ca.as_deref().unwrap_or("")
            ));
        }
    }
    let mut missing = Vec::new();
    for (host, text) in &texts {
        for needle in &required {
            if !text.contains(needle.as_str()) {
                missing.push(format!("{host} lacks {needle:?}"));
            }
        }
    }
    if missing.is_empty() {
        CheckRow::ok(
            "backup",
            "repository",
            "configured",
            format!(
                "{} repository; cipher {}; full retention {} days; differential retention 7; {}",
                descriptor.backup.repo.kind.as_str(),
                descriptor.backup.cipher.as_str(),
                descriptor.backup.retention_days,
                if descriptor.backup.repo.kind == BackupRepoKind::Posix {
                    "the posix path is mounted read-write on every Patroni node and must be one shared mount"
                } else {
                    "the S3 endpoint is verified with the configured CA"
                }
            ),
        )
    } else {
        CheckRow::failed("backup", "repository", "configuration", missing.join("; "))
    }
}

fn host_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let Some(host) = backup::backup_host(descriptor) else {
        return CheckRow::failed(
            "backup",
            "host",
            "missing",
            "backup.host resolves to no host; the default is swarm.managers[0]",
        );
    };
    let mut issues = Vec::new();
    if !descriptor.hosts.contains_key(host) {
        issues.push(format!("backup host {host} is not declared in hosts"));
    }
    match unit_text(descriptor, out, host) {
        Ok(text) => {
            if !text.contains(&format!("\n  node-exporter-{host}:\n")) {
                issues.push(format!(
                    "{host} has no node_exporter unit for the age stamp"
                ));
            }
        }
        Err(error) => issues.push(error),
    }
    match script_text(descriptor, out) {
        Ok(text) => {
            if !text.contains(&format!("# backup host: {host}")) {
                issues.push(format!("{host} is not named in the backup script header"));
            }
        }
        Err(error) => issues.push(error),
    }
    if issues.is_empty() {
        CheckRow::ok(
            "backup",
            "host",
            "declared",
            format!("{host}; node_exporter present; backup script copied by tools/stateful.sh"),
        )
    } else {
        CheckRow::failed("backup", "host", "configuration", issues.join("; "))
    }
}

fn off_host_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let in_cluster = backup::in_cluster_repo(descriptor);
    if !in_cluster {
        return CheckRow::ok(
            "backup",
            "off-host",
            "off-host",
            format!(
                "{} repository is outside the object-store nodes",
                descriptor.backup.repo.kind.as_str()
            ),
        );
    }
    let label_text = match descriptor.backup.repo.kind {
        BackupRepoKind::S3 => config_texts(descriptor, out).map(|texts| {
            texts
                .into_iter()
                .map(|(_, text)| text)
                .collect::<Vec<_>>()
                .join("\n")
        }),
        BackupRepoKind::Posix => script_text(descriptor, out),
    };
    match label_text {
        Ok(text) => {
            if descriptor.profile == Profile::LabSingleHost {
                if text.contains("not-off-host") {
                    CheckRow::ok(
                        "backup",
                        "off-host",
                        "lab-not-off-host",
                        "lab-only in-cluster MinIO repository; the rendered file carries the not-off-host label and no production claim",
                    )
                } else {
                    CheckRow::failed(
                        "backup",
                        "off-host",
                        "missing-label",
                        "the lab in-cluster repository must carry the not-off-host label in the rendered file",
                    )
                }
            } else {
                CheckRow::failed(
                    "backup",
                    "off-host",
                    "in-cluster",
                    format!(
                        "{} uses the in-cluster object store; swarm-ha and single-az require an off-host S3 repository or the dedicated backup-host posix path",
                        descriptor.backup.repo.kind.as_str()
                    ),
                )
            }
        }
        Err(error) => CheckRow::failed("backup", "off-host", "unreadable", error),
    }
}

fn object_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    if descriptor.backup.object_replication {
        return CheckRow::failed(
            "backup",
            "object",
            "replication",
            "backup.object_replication must be false; HA-013 does not install a second-site object copy",
        );
    }
    let archive = &descriptor.backup.object_archive;
    let script = match script_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => return CheckRow::failed("backup", "object", "unreadable", error),
    };
    if archive.enabled {
        let target = archive.target.as_deref().unwrap_or("");
        if target == APP_OBJECT_BUCKET {
            return CheckRow::failed(
                "backup",
                "object",
                "target",
                "the object archive target must differ from the application bucket",
            );
        }
        let expected = format!(
            "object_archive: enabled target={target} retention_days={}",
            archive.retention_days.unwrap_or(0)
        );
        if !script.contains(&expected) {
            return CheckRow::failed(
                "backup",
                "object",
                "render",
                format!("the backup script does not carry {expected:?}"),
            );
        }
        CheckRow::ok(
            "backup",
            "object",
            "archive-phase-3",
            format!(
                "object_replication false; archive target {target} differs from {APP_OBJECT_BUCKET}; the phase-3 tool is not rendered or run"
            ),
        )
    } else if script.contains("object_archive: disabled") {
        CheckRow::ok(
            "backup",
            "object",
            "no-replication",
            "object_replication false; the phase-3 object archive is disabled and no tool is run",
        )
    } else {
        CheckRow::failed(
            "backup",
            "object",
            "render",
            "the backup script does not name the disabled object archive",
        )
    }
}

fn image_row(descriptor: &Descriptor) -> CheckRow {
    let pin = &descriptor.images["patroni"];
    if pin.reference != PGBACKREST_IMAGE_REFERENCE {
        return CheckRow::failed(
            "backup",
            "image",
            "base",
            format!(
                "images.patroni.reference must be {PGBACKREST_IMAGE_REFERENCE}; found {}",
                pin.reference
            ),
        );
    }
    if pin.digest == PATRONI_BASE_DIGEST {
        return CheckRow::failed(
            "backup",
            "image",
            "base",
            "images.patroni still names the base image without pgBackRest",
        );
    }
    let state = if pin.digest == PGBACKREST_FIXTURE_DIGEST {
        "labelled-fixture"
    } else {
        "pinned"
    };
    let detail = if pin.digest == PGBACKREST_FIXTURE_DIGEST {
        "derived image is a labelled blocker; --release refuses it and deploy/v3/images/patroni/build.sh prints the real digest"
    } else {
        "derived image reference is pinned by digest"
    };
    CheckRow::ok("backup", "image", state, detail)
}

fn secrets_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut texts = Vec::new();
    match script_text(descriptor, out) {
        Ok(text) => texts.push(text),
        Err(error) => return CheckRow::failed("backup", "secrets", "unreadable", error),
    }
    match config_texts(descriptor, out) {
        Ok(configs) => texts.extend(configs.into_iter().map(|(_, text)| text)),
        Err(error) => return CheckRow::failed("backup", "secrets", "unreadable", error),
    }
    for host in &descriptor.stateful.postgres.nodes {
        match unit_text(descriptor, out, host) {
            Ok(text) => texts.push(text),
            Err(error) => return CheckRow::failed("backup", "secrets", "unreadable", error),
        }
    }
    let cipher_declared = descriptor.secrets.files.contains_key(BACKUP_CIPHER_SECRET);
    let require_cipher = cipher_declared || descriptor.backup.cipher != BackupCipher::None;
    let names: Vec<&str> = std::iter::once(BACKUP_CIPHER_SECRET)
        .chain(BACKUP_S3_SECRETS.iter().copied())
        .filter(|name| {
            (*name == BACKUP_CIPHER_SECRET && require_cipher)
                || descriptor.backup.repo.kind == BackupRepoKind::S3
        })
        .collect();
    let mut checked = 0;
    let mut issues = Vec::new();
    for name in names {
        let Some(path) = descriptor.secrets.files.get(name) else {
            issues.push(format!("{name} is not declared"));
            continue;
        };
        let Ok(value) = std::fs::read_to_string(path) else {
            issues.push(format!("{name} cannot be read; no positive secret check"));
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            issues.push(format!("{name} is empty"));
            continue;
        }
        checked += 1;
        for text in &texts {
            if text.contains(value) {
                issues.push(format!("a rendered backup file contains the {name} value"));
                break;
            }
        }
    }
    if issues.is_empty() && (checked > 0 || !require_cipher) {
        CheckRow::ok(
            "backup",
            "secrets",
            "no-values",
            if checked == 0 {
                "cipher none needs no passphrase; no rendered file carries a backup secret"
                    .to_owned()
            } else {
                format!(
                    "{checked} pgBackRest secret file(s) checked; no value appears in a config, the script, or a unit"
                )
            },
        )
    } else {
        CheckRow::failed("backup", "secrets", "no-values", issues.join("; "))
    }
}

fn script_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let text = match script_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => return CheckRow::failed("backup", "script", "unreadable", error),
    };
    let mut missing: Vec<String> = [
        "--type=full",
        "--type=diff",
        "pgbackrest_args=(verify)",
        "pgbackrest_args=(info)",
        "pgbackrest_args=(restore --delta)",
        "quantrule_backup_last_success_timestamp_seconds",
        "temporary=$(mktemp",
        "chmod 644",
        "mv \"$temporary\" \"$textfile\"",
        "dry-run: no secret is read, no repository is written, and no stamp is created",
        "phase 1a",
    ]
    .into_iter()
    .filter(|needle| !text.contains(needle))
    .map(|needle| format!("the script lacks {needle:?}"))
    .collect();
    if let Some(directory) = out {
        let path = directory.join(backup::SCRIPT_SOURCE);
        match std::fs::metadata(&path) {
            Ok(metadata) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mode = metadata.permissions().mode() & 0o777;
                    if mode != 0o700 {
                        missing.push(format!("{} is mode {mode:o}; expected 700", path.display()));
                    }
                }
                #[cfg(not(unix))]
                let _ = (metadata, &path);
            }
            Err(error) => missing.push(format!("{}: {error}", path.display())),
        }
    }
    if missing.is_empty() {
        CheckRow::ok(
            "backup",
            "script",
            "rendered",
            "full, diff, verify, info, restore; a mode-644 textfile is written atomically only after a real backup; dry-run never stamps",
        )
    } else {
        CheckRow::failed("backup", "script", "configuration", missing.join("; "))
    }
}

fn schedule_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let text = match script_text(descriptor, out) {
        Ok(text) => text,
        Err(error) => return CheckRow::failed("backup", "schedule", "unreadable", error),
    };
    let expected = format!(
        "# Schedule: full {} UTC, diff {} UTC; install a host systemd timer or cron.",
        descriptor.backup.schedule.full_at, descriptor.backup.schedule.diff_at
    );
    if text.contains(&expected) {
        CheckRow::ok(
            "backup",
            "schedule",
            "utc",
            format!(
                "full {} UTC and diff {} UTC; the operator installs the host timer or cron, no scheduler runs in a container",
                descriptor.backup.schedule.full_at, descriptor.backup.schedule.diff_at
            ),
        )
    } else {
        CheckRow::failed(
            "backup",
            "schedule",
            "render",
            "the backup script does not carry the rendered UTC schedule",
        )
    }
}

fn unit_row(descriptor: &Descriptor, out: Option<&Path>) -> CheckRow {
    let mut issues = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                issues.push(error);
                continue;
            }
        };
        for needle in [
            "source: \"/etc/pgbackrest/pgbackrest.conf\"".to_owned(),
            format!("target: \"{}\"", backup::CONFIG_TARGET),
            "PGBACKREST_REPO1_CIPHER_PASS".to_owned(),
        ] {
            if !text.contains(&needle) {
                issues.push(format!("infra/{host}.yaml lacks {needle}"));
            }
        }
        if descriptor.secrets.files.contains_key(BACKUP_CIPHER_SECRET) {
            let needle = format!("source: \"/etc/quantrule/secrets/{BACKUP_CIPHER_SECRET}\"");
            if !text.contains(&needle) {
                issues.push(format!("infra/{host}.yaml lacks {needle}"));
            }
        }
        if let Some(path) = backup::posix_path(descriptor) {
            for needle in [format!("source: \"{path}\""), format!("target: \"{path}\"")] {
                if !text.contains(&needle) {
                    issues.push(format!(
                        "infra/{host}.yaml lacks the shared posix repository mount {needle}"
                    ));
                }
            }
        }
        if descriptor.backup.repo.kind == BackupRepoKind::S3 {
            for secret in BACKUP_S3_SECRETS {
                let needle = format!("source: \"/etc/quantrule/secrets/{secret}\"");
                if !text.contains(&needle) {
                    issues.push(format!("infra/{host}.yaml lacks {needle}"));
                }
            }
            if let Some(tls_ca) = descriptor.backup.repo.tls_ca.as_deref()
                && tls_ca != "/etc/quantrule/pki/ca.crt"
            {
                let needle = format!("source: \"{tls_ca}\"");
                if !text.contains(&needle) {
                    issues.push(format!(
                        "infra/{host}.yaml lacks the repository CA {needle}"
                    ));
                }
            }
        }
        if text.contains("wal-archive") {
            issues.push(format!("infra/{host}.yaml still mounts wal-archive"));
        }
        if text.contains("ports:") {
            issues.push(format!("infra/{host}.yaml publishes a port"));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "backup",
            "unit",
            "configured",
            "every Patroni unit mounts pgbackrest.conf and the mode-600 backup secrets, exports the passphrase, and has no WAL placeholder or published port",
        )
    } else {
        CheckRow::failed("backup", "unit", "configuration", issues.join("; "))
    }
}

fn script_text(descriptor: &Descriptor, out: Option<&Path>) -> Result<String, String> {
    match out {
        Some(directory) => read_text(
            &directory.join(backup::SCRIPT_SOURCE),
            "pgBackRest backup script",
        ),
        None => backup::script(descriptor).map_err(|error| error.to_string()),
    }
}

fn unit_text(descriptor: &Descriptor, out: Option<&Path>, host: &str) -> Result<String, String> {
    match out {
        Some(directory) => read_text(
            &directory.join("infra").join(format!("{host}.yaml")),
            "host unit",
        ),
        None => infra::unit_yaml(descriptor, host)
            .ok_or_else(|| format!("the renderer produced no unit for {host}")),
    }
}

#[cfg(test)]
mod tests {
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
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700))
            .expect("script mode");
        std::fs::write(
            directory.join("stateful-manifest.json"),
            crate::render::stateful::stateful_manifest(descriptor).expect("manifest"),
        )
        .expect("manifest");
        directory
    }
}
