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

/// The five stateful secrets the database and broker tiers consume.
pub const STATEFUL_SECRETS: [&str; 5] = [
    "postgres_superuser_password",
    "patroni_replication_password",
    "patroni_rewind_password",
    "patroni_rest_password",
    "rabbitmq_erlang_cookie",
];

/// The queue user the application URL must name.
pub const QUEUE_USER: &str = "qr_queue";

/// The encoded default vhost in the application URL; `%2f` is `/`.
pub const QUEUE_VHOST: &str = "%2f";

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
    if parsed.host != descriptor.endpoints.queue.host {
        return Err(rule(
            "deploy.secret.queue_host",
            queue_field,
            format!(
                "must use the shared {} alias; found {}",
                descriptor.endpoints.queue.host, parsed.host
            ),
        ));
    }
    if parsed.user != QUEUE_USER {
        return Err(rule(
            "deploy.secret.queue_user",
            queue_field,
            format!(
                "must use the {QUEUE_USER} application user; found {}",
                parsed.user
            ),
        ));
    }
    if !parsed.vhost.eq_ignore_ascii_case(QUEUE_VHOST) {
        return Err(rule(
            "deploy.secret.queue_vhost",
            queue_field,
            "must use the default vhost encoded as %2f",
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
                format!("{name} is required for the v3 stateful tier"),
            )
        })?;
        if name == "rabbitmq_erlang_cookie" {
            check_erlang_cookie(&field, value)?;
        } else {
            check_stateful_password(&field, value)?;
        }
        check_duplicate(&mut credentials, value, &field)?;
    }
    Ok(())
}

/// An Erlang cookie is strong, URL-safe, and long enough to be unique.
fn check_erlang_cookie(field: &str, value: &str) -> Result<(), Error> {
    let lower = value.to_ascii_lowercase();
    let repeated = value
        .chars()
        .next()
        .is_some_and(|first| value.chars().all(|character| character == first));
    if value.len() < 32 {
        return Err(rule(
            "deploy.secret.weak_cookie",
            field,
            "must be at least 32 characters",
        ));
    }
    if repeated
        || lower.contains("replace-with")
        || lower.contains("change-me")
        || lower.contains("changeme")
        || lower.contains("example")
        || lower.contains("quantrule")
    {
        return Err(rule(
            "deploy.secret.default_cookie",
            field,
            "must not be a documented default or repeated-character cookie",
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
    user: &'a str,
    password: &'a str,
    host: &'a str,
    port: &'a str,
    vhost: &'a str,
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
    Some(AmqpsUrl {
        user,
        password,
        host,
        port,
        vhost,
    })
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
mod tests;
