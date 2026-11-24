//! `quantrule.deploy/v3` backup validation.
//!
//! The repository, schedule, backup host, cipher, and object-archive rules live
//! here so the general v3 validator keeps its size and the backup contract has
//! one module of its own.

use crate::descriptor::v3::{BackupRepoKind, Descriptor};
use crate::error::Error;

use super::v3::{host_by_name, rule};

/// Validate the backup block without opening a file.
///
/// # Errors
///
/// Returns the first `deploy.backup.*` or `deploy.host.undeclared` rule.
pub(super) fn validate(descriptor: &Descriptor) -> Result<(), Error> {
    if !descriptor.backup.pitr {
        return Err(rule(
            "deploy.backup.pitr",
            "backup.pitr",
            "must be true; the HA contract requires PITR",
        ));
    }
    if descriptor.backup.retention_days == 0 || descriptor.backup.retention_days > 3650 {
        return Err(rule(
            "deploy.backup.retention_days",
            "backup.retention_days",
            "must be between 1 and 3650 days",
        ));
    }
    if descriptor.backup.object_replication {
        return Err(rule(
            "deploy.backup.object_replication",
            "backup.object_replication",
            "must be false in v1.5.10; PLAT-005 owns local erasure and versioning, and HA-013 owns off-host/archive copies and any second-site replication",
        ));
    }
    validate_schedule(
        &descriptor.backup.schedule.full_at,
        "backup.schedule.full_at",
    )?;
    validate_schedule(
        &descriptor.backup.schedule.diff_at,
        "backup.schedule.diff_at",
    )?;
    if descriptor.backup.schedule.full_at == descriptor.backup.schedule.diff_at {
        return Err(rule(
            "deploy.backup.schedule",
            "backup.schedule.diff_at",
            "must differ from backup.schedule.full_at; a full and a differential backup cannot share a minute",
        ));
    }
    if let Some(host) = &descriptor.backup.host {
        host_by_name(descriptor, host, "backup.host")?;
    }
    validate_repo(descriptor)?;
    validate_object_archive(descriptor)?;
    Ok(())
}

fn validate_schedule(value: &str, field: &str) -> Result<(), Error> {
    let bytes = value.as_bytes();
    let valid = bytes.len() == 5
        && bytes[2] == b':'
        && bytes[0..2].iter().all(u8::is_ascii_digit)
        && bytes[3..].iter().all(u8::is_ascii_digit)
        && value
            .get(..2)
            .and_then(|hour| hour.parse::<u8>().ok())
            .is_some_and(|hour| hour <= 23)
        && value
            .get(3..)
            .and_then(|minute| minute.parse::<u8>().ok())
            .is_some_and(|minute| minute <= 59);
    if !valid {
        return Err(rule(
            "deploy.backup.schedule",
            field,
            "must be a UTC HH:MM time",
        ));
    }
    Ok(())
}

fn validate_repo(descriptor: &Descriptor) -> Result<(), Error> {
    let repo = &descriptor.backup.repo;
    match repo.kind {
        BackupRepoKind::Posix => {
            if repo.endpoint.is_some() {
                return Err(rule(
                    "deploy.backup.repo.endpoint",
                    "backup.repo.endpoint",
                    "must be absent for a posix repository",
                ));
            }
            if repo.bucket.is_some() {
                return Err(rule(
                    "deploy.backup.repo.bucket",
                    "backup.repo.bucket",
                    "must be absent for a posix repository",
                ));
            }
            if repo.tls_ca.is_some() {
                return Err(rule(
                    "deploy.backup.repo.tls_ca",
                    "backup.repo.tls_ca",
                    "must be absent for a posix repository",
                ));
            }
            if let Some(path) = repo.path.as_deref() {
                validate_absolute_path(path, "backup.repo.path")?;
            }
        }
        BackupRepoKind::S3 => {
            let Some(endpoint) = repo.endpoint.as_deref() else {
                return Err(rule(
                    "deploy.backup.repo.endpoint",
                    "backup.repo.endpoint",
                    "must name the S3 host and port, without a scheme",
                ));
            };
            if !is_endpoint_host(endpoint) {
                return Err(rule(
                    "deploy.backup.repo.endpoint",
                    "backup.repo.endpoint",
                    "must be host or host:port, without a scheme, path, or userinfo",
                ));
            }
            let Some(bucket) = repo.bucket.as_deref() else {
                return Err(rule(
                    "deploy.backup.repo.bucket",
                    "backup.repo.bucket",
                    "must name the S3 bucket",
                ));
            };
            if !is_bucket_name(bucket) {
                return Err(rule(
                    "deploy.backup.repo.bucket",
                    "backup.repo.bucket",
                    "must be a lowercase S3 bucket name",
                ));
            }
            if let Some(path) = repo.path.as_deref() {
                validate_absolute_path(path, "backup.repo.path")?;
            }
            let Some(tls_ca) = repo.tls_ca.as_deref() else {
                return Err(rule(
                    "deploy.backup.repo.tls_ca",
                    "backup.repo.tls_ca",
                    "must name the absolute CA bundle used to verify the S3 endpoint",
                ));
            };
            validate_absolute_path(tls_ca, "backup.repo.tls_ca")?;
        }
    }
    Ok(())
}

fn validate_object_archive(descriptor: &Descriptor) -> Result<(), Error> {
    let archive = &descriptor.backup.object_archive;
    if !archive.enabled {
        return Ok(());
    }
    let Some(target) = archive.target.as_deref() else {
        return Err(rule(
            "deploy.backup.object_archive.target",
            "backup.object_archive.target",
            "must name the archive target; it is required when the archive is enabled",
        ));
    };
    if !is_bucket_name(target) {
        return Err(rule(
            "deploy.backup.object_archive.target",
            "backup.object_archive.target",
            "must be a lowercase bucket name",
        ));
    }
    if target == crate::descriptor::v3::APP_OBJECT_BUCKET {
        return Err(rule(
            "deploy.backup.object_archive.target",
            "backup.object_archive.target",
            "must differ from the application bucket; an archive is not a second copy in the same bucket",
        ));
    }
    let days = archive.retention_days.ok_or_else(|| {
        rule(
            "deploy.backup.object_archive.retention_days",
            "backup.object_archive.retention_days",
            "must name the archive retention in days when the archive is enabled",
        )
    })?;
    if !(1..=3650).contains(&days) {
        return Err(rule(
            "deploy.backup.object_archive.retention_days",
            "backup.object_archive.retention_days",
            "must be between 1 and 3650 days",
        ));
    }
    Ok(())
}

fn validate_absolute_path(value: &str, field: &str) -> Result<(), Error> {
    if value == "/"
        || value.chars().any(char::is_whitespace)
        || !std::path::Path::new(value).is_absolute()
    {
        return Err(rule(
            "deploy.backup.path",
            field,
            "must be an absolute path, not the filesystem root, without whitespace",
        ));
    }
    Ok(())
}

fn is_endpoint_host(value: &str) -> bool {
    if value.is_empty()
        || value.contains("://")
        || value.contains('/')
        || value.contains('@')
        || value.contains('[')
        || value.contains(']')
        || value.chars().any(char::is_whitespace)
    {
        return false;
    }
    let (host, port) = value.rsplit_once(':').unwrap_or((value, ""));
    if host.is_empty()
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return false;
    }
    port.is_empty()
        || (port.bytes().all(|byte| byte.is_ascii_digit())
            && port.parse::<u16>().is_ok_and(|port| port != 0))
}

fn is_bucket_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn sample() -> Value {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("the v3 example is JSON")
    }

    fn descriptor(value: Value) -> Descriptor {
        serde_json::from_value(value).expect("the mutated example stays structurally valid")
    }

    fn reject(value: Value) -> Error {
        validate(&descriptor(value)).expect_err("the descriptor must be refused")
    }

    #[test]
    fn backup_defaults_are_stable_and_posix_when_the_block_is_omitted() {
        let mut value = sample();
        value["backup"] = serde_json::json!({
            "pitr": true,
            "retention_days": 30,
            "object_replication": false
        });
        let descriptor = descriptor(value);
        assert_eq!(descriptor.backup.repo.kind, BackupRepoKind::Posix);
        assert!(descriptor.backup.repo.path.is_none());
        assert_eq!(
            crate::render::backup::posix_path(&descriptor),
            Some("/srv/quantrule/backup/pgbackrest")
        );
        assert_eq!(descriptor.backup.cipher.as_str(), "aes-256-cbc");
        assert_eq!(descriptor.backup.schedule.full_at, "02:00");
        assert_eq!(descriptor.backup.schedule.diff_at, "14:00");
        assert_eq!(
            descriptor.backup.host(&descriptor),
            Some(descriptor.swarm.managers[0].as_str())
        );
        assert!(!descriptor.backup.object_archive.enabled);
    }

    #[test]
    fn an_s3_repository_without_a_path_does_not_inherit_the_posix_default() {
        let mut value = sample();
        value["backup"]["repo"] = serde_json::json!({
            "kind": "s3",
            "endpoint": "minio:9000",
            "bucket": "quantrule-backup",
            "tls_ca": "/etc/quantrule/pki/ca.crt"
        });
        let descriptor = descriptor(value);
        let config = crate::render::backup::config(&descriptor, "infra-1").expect("config");
        assert!(config.contains("repo1-path=/\n"), "{config}");
        assert!(
            !config.contains("/srv/quantrule/backup/pgbackrest"),
            "{config}"
        );
    }

    #[test]
    fn a_pre_ha013_descriptor_still_validates_with_the_default_block() {
        let mut value = sample();
        value["backup"] = serde_json::json!({
            "pitr": true,
            "retention_days": 30,
            "object_replication": false
        });
        value["secrets"]["files"]
            .as_object_mut()
            .expect("secrets")
            .remove("pgbackrest_cipher_pass");
        crate::validate::v3::validate(&descriptor(value))
            .expect("an existing v3 descriptor still validates");
    }

    #[test]
    fn the_backup_repository_schedule_and_archive_are_bounded() {
        let mut value = sample();
        value["backup"]["repo"] = serde_json::json!({"kind": "s3", "path": "/quantrule"});
        value["secrets"]["files"]["pgbackrest_s3_access_key"] =
            serde_json::json!("/etc/quantrule/secrets/pgbackrest_s3_access_key");
        value["secrets"]["files"]["pgbackrest_s3_secret_key"] =
            serde_json::json!("/etc/quantrule/secrets/pgbackrest_s3_secret_key");
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.backup.repo.endpoint"),
            "{error}"
        );

        let mut value = sample();
        value["backup"]["repo"] = serde_json::json!({"kind": "posix", "path": "relative"});
        let error = reject(value);
        assert!(error.to_string().contains("deploy.backup.path"), "{error}");

        let mut value = sample();
        value["backup"]["schedule"]["full_at"] = serde_json::json!("2:00");
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.backup.schedule"),
            "{error}"
        );

        let mut value = sample();
        value["backup"]["host"] = serde_json::json!("ghost");
        let error = reject(value);
        assert!(
            error.to_string().contains("deploy.host.undeclared"),
            "{error}"
        );

        let mut value = sample();
        value["backup"]["object_archive"] =
            serde_json::json!({"enabled": true, "target": "quantrule", "retention_days": 30});
        let error = reject(value);
        assert!(
            error
                .to_string()
                .contains("deploy.backup.object_archive.target"),
            "{error}"
        );

        let mut value = sample();
        value["backup"]["object_archive"] =
            serde_json::json!({"enabled": true, "target": "archive"});
        let error = reject(value);
        assert!(
            error
                .to_string()
                .contains("deploy.backup.object_archive.retention_days"),
            "{error}"
        );
    }
}
