//! Secret file structure and content rules for `quantrule.deploy/v3`.
//!
//! The descriptor maps a logical Swarm secret name to an absolute host path.
//! Structural validation needs no file access, so a descriptor can be reviewed
//! before its secret files are copied. `validate --files` adds the local file
//! contract: regular, non-symlink, mode 600, at most 8 KiB, and a non-default,
//! non-duplicate credential for every role.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::Metadata;
use std::path::Path;

use crate::descriptor::v3::Descriptor;
use crate::error::Error;

use super::v3::{is_secret_name, rule};

/// The service to logical database mapping used by the four role pairs.
const SERVICE_DATABASES: [(&str, &str); 4] = [
    ("gateway", "gateway"),
    ("data", "catalog"),
    ("evidence", "evidence"),
    ("simulation", "simulation"),
];

/// The four stateful secrets Patroni and the cluster bootstrap consume.
pub const STATEFUL_SECRETS: [&str; 4] = [
    "postgres_superuser_password",
    "patroni_replication_password",
    "patroni_rewind_password",
    "patroni_rest_password",
];

/// The rendered scope of a secret: `stateful` for the database tier, `app` otherwise.
#[must_use]
pub fn scope(name: &str) -> &'static str {
    if STATEFUL_SECRETS.contains(&name) {
        "stateful"
    } else {
        "app"
    }
}

/// Validate the `secrets.files` map without opening a file.
///
/// # Errors
///
/// Returns [`Error::Rule`] for an empty map, an invalid name, a relative or
/// whitespace-bearing path, or a missing required secret.
pub fn validate_structure(descriptor: &Descriptor) -> Result<(), Error> {
    let files = &descriptor.secrets.files;
    if files.is_empty() {
        return Err(rule(
            "deploy.secrets.empty",
            "secrets.files",
            "must map at least the application secrets to absolute host paths",
        ));
    }
    for (name, path) in files {
        if !is_secret_name(name) {
            return Err(rule(
                "deploy.secrets.name",
                format!("secrets.files.{name}"),
                "must be a lowercase secret name using letters, digits and underscores",
            ));
        }
        if path.trim().is_empty() || !Path::new(path).is_absolute() {
            return Err(rule(
                "deploy.secrets.path",
                format!("secrets.files.{name}"),
                "must be an absolute host path, never an inline value",
            ));
        }
        if path.chars().any(char::is_whitespace) {
            return Err(rule(
                "deploy.secrets.path",
                format!("secrets.files.{name}"),
                "must not contain whitespace",
            ));
        }
    }
    for name in required_names(descriptor) {
        if !files.contains_key(&name) {
            return Err(rule(
                "deploy.secrets.missing",
                format!("secrets.files.{name}"),
                format!("{name} is required for the v3 deployment"),
            ));
        }
    }
    Ok(())
}

fn required_names(descriptor: &Descriptor) -> BTreeSet<String> {
    let mut names = BTreeSet::from([
        "queue_url".to_owned(),
        "object_store_access_key".to_owned(),
        "object_store_secret_key".to_owned(),
    ]);
    for (service, _) in SERVICE_DATABASES {
        names.insert(format!("{service}_database_url"));
        names.insert(format!("{service}_owner_database_url"));
    }
    for name in STATEFUL_SECRETS {
        names.insert(name.to_owned());
    }
    names.insert(descriptor.auth.gateway_token_secret.clone());
    names
}

/// Read every secret file exactly once, in map order.
///
/// The renderer calls this and then [`validate_values`], so one invocation
/// reads each source secret once and writes the copy from the same value.
///
/// # Errors
///
/// Returns [`Error::Rule`] for an unsafe file, an unreadable value, or an empty
/// value. The value is never included in the message.
pub fn load_values(descriptor: &Descriptor) -> Result<BTreeMap<String, String>, Error> {
    let mut values = BTreeMap::new();
    for (name, path) in &descriptor.secrets.files {
        values.insert(
            name.clone(),
            read_secret_file(path, &format!("secrets.files.{name}"), true)?,
        );
    }
    Ok(values)
}

/// Check every secret file and the credentials it carries.
///
/// # Errors
///
/// Returns [`Error::Rule`] for an unsafe file, a malformed credential URL, a
/// default credential, or two roles sharing one credential.
#[cfg(test)]
pub fn validate_files(descriptor: &Descriptor) -> Result<(), Error> {
    let values = load_values(descriptor)?;
    validate_values(descriptor, &values)
}

/// Check the credentials carried by already-loaded secret values.
///
/// # Errors
///
/// Returns [`Error::Rule`] for a malformed credential URL, a default
/// credential, or two roles sharing one credential.
pub fn validate_values(
    descriptor: &Descriptor,
    values: &BTreeMap<String, String>,
) -> Result<(), Error> {
    let token = values
        .get(&descriptor.auth.gateway_token_secret)
        .ok_or_else(|| {
            rule(
                "deploy.secrets.missing",
                format!("secrets.files.{}", descriptor.auth.gateway_token_secret),
                "the gateway token file is required",
            )
        })?;
    check_token(
        &format!("secrets.files.{}", descriptor.auth.gateway_token_secret),
        token,
    )?;

    let mut credentials: BTreeMap<String, String> = BTreeMap::new();
    for (service, database_key) in SERVICE_DATABASES {
        let database = &descriptor.databases[database_key];
        let app_name = format!("{service}_database_url");
        let owner_name = format!("{service}_owner_database_url");
        for (name, role, expected_database) in [
            (
                &app_name,
                database.app_role.as_str(),
                database.database.as_str(),
            ),
            (
                &owner_name,
                database.owner_role.as_str(),
                database.database.as_str(),
            ),
        ] {
            let field = format!("secrets.files.{name}");
            let value = values.get(name).ok_or_else(|| {
                rule(
                    "deploy.secrets.missing",
                    &field,
                    format!("{name} is required for the v3 deployment"),
                )
            })?;
            let parsed = parse_postgres_url(value).ok_or_else(|| {
                rule(
                    "deploy.secret.database_url",
                    &field,
                    "must be a postgres:// role:password@host:5432/database URL with sslmode=verify-full",
                )
            })?;
            if parsed.user != role {
                return Err(rule(
                    "deploy.secret.role",
                    &field,
                    format!("must use {role}; found {}", parsed.user),
                ));
            }
            if parsed.database != expected_database {
                return Err(rule(
                    "deploy.secret.database",
                    &field,
                    format!("must name {expected_database}; found {}", parsed.database),
                ));
            }
            if parsed.port != "5432" {
                return Err(rule(
                    "deploy.secret.database_port",
                    &field,
                    "must use the TLS port 5432",
                ));
            }
            if !parsed
                .query
                .split('&')
                .any(|pair| pair == "sslmode=verify-full")
            {
                return Err(rule(
                    "deploy.secret.sslmode",
                    &field,
                    "must carry sslmode=verify-full",
                ));
            }
            check_password(&field, parsed.password)?;
            check_duplicate(&mut credentials, parsed.password, &field)?;
        }
    }

    let queue_field = "secrets.files.queue_url";
    let queue = values.get("queue_url").ok_or_else(|| {
        rule(
            "deploy.secrets.missing",
            queue_field,
            "queue_url is required for the v3 deployment",
        )
    })?;
    let parsed = parse_amqps_url(queue).ok_or_else(|| {
        rule(
            "deploy.secret.queue_url",
            queue_field,
            "must be an amqps:// user:password@host:5671/vhost URL",
        )
    })?;
    if parsed.port != "5671" {
        return Err(rule(
            "deploy.secret.queue_port",
            queue_field,
            "must use the TLS port 5671",
        ));
    }
    check_password(queue_field, parsed.password)?;
    check_duplicate(&mut credentials, parsed.password, queue_field)?;

    let access_field = "secrets.files.object_store_access_key";
    let access = values.get("object_store_access_key").ok_or_else(|| {
        rule(
            "deploy.secrets.missing",
            access_field,
            "object_store_access_key is required",
        )
    })?;
    check_access_key(access_field, access)?;

    let secret_field = "secrets.files.object_store_secret_key";
    let secret = values.get("object_store_secret_key").ok_or_else(|| {
        rule(
            "deploy.secrets.missing",
            secret_field,
            "object_store_secret_key is required",
        )
    })?;
    check_password(secret_field, secret)?;
    check_duplicate(&mut credentials, secret, secret_field)?;

    for name in STATEFUL_SECRETS {
        let field = format!("secrets.files.{name}");
        let value = values.get(name).ok_or_else(|| {
            rule(
                "deploy.secrets.missing",
                &field,
                format!("{name} is required for the v3 database tier"),
            )
        })?;
        check_stateful_password(&field, value)?;
        check_duplicate(&mut credentials, value, &field)?;
    }
    Ok(())
}

/// A stateful password is strong, URL-safe, and not a documented default.
fn check_stateful_password(field: &str, value: &str) -> Result<(), Error> {
    check_password(field, value)?;
    let lower = value.to_ascii_lowercase();
    if lower.contains("postgres") || lower.contains("patroni") {
        return Err(rule(
            "deploy.secret.default_credential",
            field,
            "must not use a documented default or template stateful credential",
        ));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~'))
    {
        return Err(rule(
            "deploy.secret.url_safe",
            field,
            "must contain only ASCII letters, digits, and -._~ so it is safe in a URL and a config file",
        ));
    }
    Ok(())
}

fn check_duplicate(
    credentials: &mut BTreeMap<String, String>,
    value: &str,
    field: &str,
) -> Result<(), Error> {
    if let Some(other) = credentials.insert(value.to_owned(), field.to_owned()) {
        return Err(rule(
            "deploy.secret.duplicate_credential",
            field,
            format!("shares a credential with {other}; every role needs its own value"),
        ));
    }
    Ok(())
}

fn check_password(field: &str, value: &str) -> Result<(), Error> {
    let lower = value.to_ascii_lowercase();
    if value.len() < 16 {
        return Err(rule(
            "deploy.secret.weak_credential",
            field,
            "must be at least 16 characters",
        ));
    }
    if lower.contains("replace-with")
        || lower.contains("change-me")
        || lower.contains("changeme")
        || lower.contains("example")
        || lower.contains("password")
        || lower.contains("quantrule")
        || lower.contains("minioadmin")
    {
        return Err(rule(
            "deploy.secret.default_credential",
            field,
            "must not use a documented default or template credential",
        ));
    }
    Ok(())
}

fn check_token(field: &str, value: &str) -> Result<(), Error> {
    let lower = value.to_ascii_lowercase();
    let repeated = value
        .chars()
        .next()
        .is_some_and(|first| value.chars().all(|character| character == first));
    if value.len() < 32 {
        return Err(rule(
            "deploy.secret.weak_token",
            field,
            "must be at least 32 characters",
        ));
    }
    if repeated
        || lower.contains("replace-with")
        || lower.contains("change-me")
        || lower.contains("example")
    {
        return Err(rule(
            "deploy.secret.default_token",
            field,
            "must not be a template or repeated-character token",
        ));
    }
    Ok(())
}

fn check_access_key(field: &str, value: &str) -> Result<(), Error> {
    let lower = value.to_ascii_lowercase();
    if value.len() < 8 {
        return Err(rule(
            "deploy.secret.weak_credential",
            field,
            "must be at least 8 characters",
        ));
    }
    if lower.contains("replace-with")
        || lower.contains("change-me")
        || lower.contains("example")
        || lower == "minioadmin"
        || lower == "quantrule"
        || lower == "accesskey"
    {
        return Err(rule(
            "deploy.secret.default_credential",
            field,
            "must not use a documented default or template access key",
        ));
    }
    Ok(())
}

struct PostgresUrl<'a> {
    user: &'a str,
    password: &'a str,
    port: &'a str,
    database: &'a str,
    query: &'a str,
}

fn parse_postgres_url(value: &str) -> Option<PostgresUrl<'_>> {
    let rest = value
        .strip_prefix("postgres://")
        .or_else(|| value.strip_prefix("postgresql://"))?;
    let (authority, path_query) = rest.split_once('/')?;
    if path_query.contains('/') {
        return None;
    }
    let (userinfo, host_port) = authority.rsplit_once('@')?;
    let (user, password) = userinfo.split_once(':')?;
    let (host, port) = host_port.rsplit_once(':')?;
    let (database, query) = path_query.split_once('?').unwrap_or((path_query, ""));
    if user.is_empty() || password.is_empty() || host.is_empty() || database.is_empty() {
        return None;
    }
    Some(PostgresUrl {
        user,
        password,
        port,
        database,
        query,
    })
}

struct AmqpsUrl<'a> {
    password: &'a str,
    port: &'a str,
}

fn parse_amqps_url(value: &str) -> Option<AmqpsUrl<'_>> {
    let rest = value.strip_prefix("amqps://")?;
    let (authority, vhost) = rest.split_once('/')?;
    let (userinfo, host_port) = authority.rsplit_once('@')?;
    let (user, password) = userinfo.split_once(':')?;
    let (host, port) = host_port.rsplit_once(':')?;
    if user.is_empty() || password.is_empty() || host.is_empty() || vhost.is_empty() {
        return None;
    }
    Some(AmqpsUrl { password, port })
}

/// Read one mode-600 secret file and apply the runtime file contract.
///
/// # Errors
///
/// Returns [`Error::Rule`] naming the file field for a symlink, non-regular
/// file, unsafe mode, oversized file, invalid encoding, NUL, empty value or
/// whitespace. The value is never included in the message.
pub fn read_secret_file(path: &str, field: &str, require_mode: bool) -> Result<String, Error> {
    read_file(path, field, require_mode, 8 * 1024, false)
}

/// Read one PEM private key, which has internal newlines but is still secret.
///
/// # Errors
///
/// Returns [`Error::Rule`] for a symlink, non-regular or oversized file, a
/// missing or non-600 mode, or an unreadable or empty value.
pub fn read_pem_file(path: &str, field: &str, require_mode: bool) -> Result<String, Error> {
    read_file(path, field, require_mode, 8 * 1024, true)
}

/// Read one non-secret file, such as an ingress certificate.
///
/// # Errors
///
/// Returns [`Error::Rule`] for a symlink, non-regular or oversized file, or an
/// unreadable or empty value.
pub fn read_public_file(path: &str, field: &str) -> Result<String, Error> {
    read_file(path, field, false, 64 * 1024, true)
}

fn safe_metadata(path: &str, field: &str) -> Result<Metadata, Error> {
    let metadata = std::fs::symlink_metadata(path).map_err(|source| {
        rule(
            "deploy.file.unreadable",
            field,
            format!("cannot read {path}: {source}"),
        )
    })?;
    if metadata.file_type().is_symlink() {
        return Err(rule(
            "deploy.file.symlink",
            field,
            "must be a regular file, not a symlink",
        ));
    }
    if !metadata.is_file() {
        return Err(rule(
            "deploy.file.not_file",
            field,
            "must be a regular file",
        ));
    }
    Ok(metadata)
}

fn check_mode(metadata: &Metadata, field: &str) -> Result<(), Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        if mode != 0o600 {
            return Err(rule(
                "deploy.secret.mode",
                field,
                format!("must be mode 600; found {mode:o}"),
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = (metadata, field);
    Ok(())
}

fn check_size(metadata: &Metadata, field: &str, maximum: u64) -> Result<(), Error> {
    if metadata.len() > maximum {
        return Err(rule(
            "deploy.file.size",
            field,
            format!("must be at most {maximum} bytes; found {}", metadata.len()),
        ));
    }
    Ok(())
}

fn read_file(
    path: &str,
    field: &str,
    require_mode: bool,
    maximum: u64,
    allow_whitespace: bool,
) -> Result<String, Error> {
    let metadata = safe_metadata(path, field)?;
    if require_mode {
        check_mode(&metadata, field)?;
    }
    check_size(&metadata, field, maximum)?;
    let bytes = std::fs::read(path).map_err(|source| {
        rule(
            "deploy.file.unreadable",
            field,
            format!("cannot read {path}: {source}"),
        )
    })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| rule("deploy.file.encoding", field, "must be UTF-8 text"))?;
    if text.contains('\0') {
        return Err(rule(
            "deploy.file.nul",
            field,
            "must not contain a NUL byte",
        ));
    }
    let value = text
        .strip_suffix('\n')
        .map_or(text, |value| value.strip_suffix('\r').unwrap_or(value));
    if value.is_empty() {
        return Err(rule("deploy.file.empty", field, "must not be empty"));
    }
    if !allow_whitespace && value.chars().any(char::is_whitespace) {
        return Err(rule(
            "deploy.file.whitespace",
            field,
            "must not contain whitespace",
        ));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::v3;
    use serde_json::Value;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn sample() -> Value {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
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
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .expect("mode 600");
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
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))
                .expect("mode 644");
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
        value["secrets"]["files"]["queue_url"] =
            serde_json::json!("amqps://user:value@host:5671/%2f");
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
    fn the_url_parsers_reject_the_wrong_shape() {
        assert!(
            parse_postgres_url("postgres://role:value@host:5432/db?sslmode=verify-full").is_some()
        );
        assert!(parse_postgres_url("postgres://role@host:5432/db?sslmode=verify-full").is_none());
        assert!(
            parse_postgres_url("postgres://role:value@host:5432/db/extra?sslmode=verify-full")
                .is_none()
        );
        assert!(parse_amqps_url("amqps://user:value@host:5671/%2f").is_some());
        assert!(parse_amqps_url("amqps://user@host:5671/%2f").is_none());
    }
}
