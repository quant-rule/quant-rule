use super::*;
use crate::validate::v3;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn sample() -> Value {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("the v3 example is JSON")
}

fn descriptor(value: Value) -> Descriptor {
    serde_json::from_value(value).expect("the mutated example stays structurally valid")
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../deploy/v3/secrets")
        .join(name)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-deploy-v3-secrets-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn write_mode_600(path: &Path, content: &str) {
    std::fs::write(path, content).expect("write secret");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).expect("mode 600");
    }
}

/// Copy the checked-in fixture files into a temporary directory and point
/// the descriptor at the copy, so the test never changes repository modes.
fn stage(value: &mut Value) -> (Descriptor, PathBuf) {
    let directory = temp_dir();
    let files = value["secrets"]["files"].as_object_mut().expect("files");
    for (name, path) in files.iter_mut() {
        let source = fixture(name);
        let destination = directory.join(name);
        let content = std::fs::read_to_string(&source).expect("fixture secret");
        write_mode_600(&destination, &content);
        *path = serde_json::json!(destination.display().to_string());
    }
    (descriptor(value.clone()), directory)
}

#[test]
fn the_fixture_secret_directory_satisfies_the_file_contract() {
    let (descriptor, directory) = stage(&mut sample());
    validate_files(&descriptor).expect("the fixture secrets validate");
    let _ = std::fs::remove_dir_all(directory);
    assert!(v3::validate(&descriptor).is_ok());
}

#[test]
fn a_group_readable_secret_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    let path = directory.join("gateway_token");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).expect("mode 644");
    }
    let error = validate_files(&descriptor).expect_err("a readable token");
    assert!(error.to_string().contains("deploy.secret.mode"), "{error}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_symlink_secret_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    let path = directory.join("gateway_token");
    std::fs::remove_file(&path).expect("remove");
    #[cfg(unix)]
    std::os::unix::fs::symlink(fixture("gateway_token"), &path).expect("symlink");
    let error = validate_files(&descriptor).expect_err("a symlink");
    assert!(error.to_string().contains("deploy.file.symlink"), "{error}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_oversized_secret_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(&directory.join("gateway_token"), &"a".repeat(8 * 1024 + 1));
    let error = validate_files(&descriptor).expect_err("an oversized token");
    assert!(error.to_string().contains("deploy.file.size"), "{error}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_default_role_credential_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("gateway_database_url"),
        "postgres://qr_gateway_app:quantrule-db-password@postgres:5432/qr_gateway?sslmode=verify-full",
    );
    let error = validate_files(&descriptor).expect_err("a default password");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.default_credential"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn two_roles_cannot_share_one_credential() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("gateway_owner_database_url"),
        "postgres://qr_gateway_owner:gateway-app-4f8c2a1e6d9b7c30@postgres:5432/qr_gateway?sslmode=verify-full",
    );
    let error = validate_files(&descriptor).expect_err("a duplicate password");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.duplicate_credential"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_malformed_or_plaintext_url_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("queue_url"),
        "amqp://qr_queue:queue-1a3c5e7b9d2f4a60@rabbitmq:5671/%2f",
    );
    let error = validate_files(&descriptor).expect_err("a plaintext queue");
    assert!(
        error.to_string().contains("deploy.secret.queue_url"),
        "{error}"
    );

    write_mode_600(
        &directory.join("queue_url"),
        "amqps://qr_queue:queue-1a3c5e7b9d2f4a60@rabbitmq:5671/%2f",
    );
    write_mode_600(
        &directory.join("gateway_database_url"),
        "postgres://qr_gateway_app:gateway-app-4f8c2a1e6d9b7c30@postgres:5432/qr_gateway",
    );
    let error = validate_files(&descriptor).expect_err("a missing sslmode");
    assert!(
        error.to_string().contains("deploy.secret.sslmode"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_queue_url_must_name_the_shared_alias_user_and_vhost() {
    for (url, expected) in [
        (
            "amqps://qr_queue:queue-1a3c5e7b9d2f4a60@rabbitmq-infra-1:5671/%2f",
            "deploy.secret.queue_host",
        ),
        (
            "amqps://guest:queue-1a3c5e7b9d2f4a60@rabbitmq:5671/%2f",
            "deploy.secret.queue_user",
        ),
        (
            "amqps://qr_queue:queue-1a3c5e7b9d2f4a60@rabbitmq:5671/other",
            "deploy.secret.queue_vhost",
        ),
    ] {
        let (descriptor, directory) = stage(&mut sample());
        write_mode_600(&directory.join("queue_url"), url);
        let error = validate_files(&descriptor).expect_err(url);
        assert!(error.to_string().contains(expected), "{url}: {error}");
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn the_erlang_cookie_is_required_strong_and_distinct() {
    let mut value = sample();
    value["secrets"]["files"]
        .as_object_mut()
        .expect("files")
        .remove("rabbitmq_erlang_cookie");
    let error = v3::validate(&descriptor(value)).expect_err("a missing cookie");
    assert!(
        error.to_string().contains("deploy.secrets.missing"),
        "{error}"
    );
    assert!(
        error.to_string().contains("rabbitmq_erlang_cookie"),
        "{error}"
    );

    for (value, expected) in [
        ("short", "deploy.secret.weak_cookie"),
        (
            "replace-with-a-real-erlang-cookie-000000",
            "deploy.secret.default_cookie",
        ),
        (
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "deploy.secret.default_cookie",
        ),
        (
            "stateful:cookie:with:colon:1234567890abcdef",
            "deploy.secret.url_safe",
        ),
    ] {
        let (descriptor, directory) = stage(&mut sample());
        write_mode_600(&directory.join("rabbitmq_erlang_cookie"), value);
        let error = validate_files(&descriptor).expect_err(value);
        assert!(error.to_string().contains(expected), "{value}: {error}");
        let _ = std::fs::remove_dir_all(directory);
    }

    let (descriptor, directory) = stage(&mut sample());
    let superuser =
        std::fs::read_to_string(fixture("postgres_superuser_password")).expect("superuser fixture");
    write_mode_600(&directory.join("rabbitmq_erlang_cookie"), superuser.trim());
    let error = validate_files(&descriptor).expect_err("a duplicate cookie");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.duplicate_credential"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_weak_token_is_refused() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(&directory.join("gateway_token"), "0123456789abcdef");
    let error = validate_files(&descriptor).expect_err("a weak token");
    assert!(
        error.to_string().contains("deploy.secret.weak_token"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_empty_secret_map_is_refused() {
    let mut value = sample();
    value["secrets"]["files"] = serde_json::json!({});
    let error = v3::validate(&descriptor(value)).expect_err("an empty map");
    assert!(
        error.to_string().contains("deploy.secrets.empty"),
        "{error}"
    );
}

#[test]
fn a_missing_or_relative_secret_is_refused_without_file_access() {
    let mut value = sample();
    value["secrets"]["files"]
        .as_object_mut()
        .expect("files")
        .remove("queue_url");
    let error = validate_structure(&descriptor(value)).expect_err("a missing secret");
    assert!(
        error.to_string().contains("deploy.secrets.missing"),
        "{error}"
    );

    let mut value = sample();
    value["secrets"]["files"]["queue_url"] = serde_json::json!("secrets/queue_url");
    let error = validate_structure(&descriptor(value)).expect_err("a relative path");
    assert!(error.to_string().contains("deploy.secrets.path"), "{error}");

    let mut value = sample();
    value["secrets"]["files"]["queue_url"] = serde_json::json!("amqps://user:value@host:5671/%2f");
    let error = validate_structure(&descriptor(value)).expect_err("an inline value");
    assert!(error.to_string().contains("deploy.secrets.path"), "{error}");
}

#[test]
fn a_stateful_secret_is_required_and_distinct() {
    let mut value = sample();
    value["secrets"]["files"]
        .as_object_mut()
        .expect("files")
        .remove("patroni_rest_password");
    let error = v3::validate(&descriptor(value)).expect_err("a missing stateful secret");
    assert!(
        error.to_string().contains("deploy.secrets.missing"),
        "{error}"
    );
    assert!(
        error.to_string().contains("patroni_rest_password"),
        "{error}"
    );

    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("postgres_superuser_password"),
        "gateway-app-4f8c2a1e6d9b7c30",
    );
    let error = validate_files(&descriptor).expect_err("a duplicate stateful password");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.duplicate_credential"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_stateful_secret_is_strong_url_safe_and_not_a_default() {
    for (value, expected) in [
        ("short", "deploy.secret.weak_credential"),
        ("postgres-postgres", "deploy.secret.default_credential"),
        ("has a space in the value", "deploy.file.whitespace"),
        ("stateful:secret/with-colon", "deploy.secret.url_safe"),
    ] {
        let (descriptor, directory) = stage(&mut sample());
        write_mode_600(&directory.join("patroni_rest_password"), value);
        let error = validate_files(&descriptor).expect_err(value);
        assert!(error.to_string().contains(expected), "{value}: {error}");
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn the_minio_root_secrets_are_required_strong_and_scoped() {
    for removed in ["minio_root_user", "minio_root_password"] {
        let mut value = sample();
        value["secrets"]["files"]
            .as_object_mut()
            .expect("files")
            .remove(removed);
        let error = v3::validate(&descriptor(value)).expect_err("a missing root secret");
        assert!(
            error.to_string().contains("deploy.secrets.missing"),
            "{removed}: {error}"
        );
        assert!(error.to_string().contains(removed), "{removed}: {error}");
    }

    for (name, value, expected) in [
        ("minio_root_user", "short", "deploy.secret.weak_credential"),
        (
            "minio_root_user",
            "minioadmin",
            "deploy.secret.default_credential",
        ),
        (
            "minio_root_password",
            "minioadmin",
            "deploy.secret.weak_credential",
        ),
        (
            "minio_root_password",
            "quantrule-minio-password",
            "deploy.secret.default_credential",
        ),
    ] {
        let (descriptor, directory) = stage(&mut sample());
        write_mode_600(&directory.join(name), value);
        let error = validate_files(&descriptor).expect_err(value);
        assert!(
            error.to_string().contains(expected),
            "{name}/{value}: {error}"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    let (descriptor, directory) = stage(&mut sample());
    let root = std::fs::read_to_string(fixture("minio_root_password")).expect("root fixture");
    write_mode_600(&directory.join("object_store_secret_key"), root.trim());
    let error = validate_files(&descriptor).expect_err("a duplicate root password");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.duplicate_credential"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);

    assert_eq!(scope("minio_root_user"), "minio");
    assert_eq!(scope("minio_root_password"), "minio");
    assert_eq!(scope("patroni_rest_password"), "stateful");
    assert_eq!(scope("object_store_access_key"), "app");
}

#[test]
fn the_object_store_access_key_must_be_the_dedicated_app_user() {
    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("object_store_access_key"),
        "qr-access-9f2b4d6c",
    );
    let error = validate_files(&descriptor).expect_err("the wrong app user");
    assert!(
        error
            .to_string()
            .contains("deploy.secret.object_store_user"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(directory);

    let (descriptor, directory) = stage(&mut sample());
    write_mode_600(
        &directory.join("object_store_access_key"),
        OBJECT_STORE_USER,
    );
    validate_files(&descriptor).expect("qr_app is the dedicated user");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_url_parsers_reject_the_wrong_shape() {
    assert!(parse_postgres_url("postgres://role:value@host:5432/db?sslmode=verify-full").is_some());
    assert!(parse_postgres_url("postgres://role@host:5432/db?sslmode=verify-full").is_none());
    assert!(
        parse_postgres_url("postgres://role:value@host:5432/db/extra?sslmode=verify-full")
            .is_none()
    );
    let parsed = parse_amqps_url("amqps://qr_queue:value@rabbitmq:5671/%2f").expect("amqps");
    assert_eq!(parsed.user, "qr_queue");
    assert_eq!(parsed.host, "rabbitmq");
    assert_eq!(parsed.vhost, "%2f");
    assert!(parse_amqps_url("amqps://user@host:5671/%2f").is_none());
}
