//! Render one env file per service and one for the infrastructure host.
//!
//! The generated files are the only input to the per-role Compose files. They
//! are written with owner-only permissions on Unix because they carry
//! credentials.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::descriptor::Descriptor;
use crate::error::Error;
use crate::validate::{expand, validate_with_secrets};

/// The database one service owns. Model and backtest are absent by design.
const SERVICE_DATABASES: [(&str, &str); 4] = [
    ("gateway", "qr_gateway"),
    ("data", "qr_catalog"),
    ("evidence", "qr_evidence"),
    ("simulation", "qr_simulation"),
];

/// Render every env file under `out`.
///
/// # Errors
///
/// Returns [`Error::MissingSecret`] for an undefined placeholder,
/// [`Error::Invalid`] for a rejected descriptor, and [`Error::Write`] when an
/// output file cannot be written.
pub fn render(
    descriptor: &Descriptor,
    secrets: &BTreeMap<String, String>,
    out: &Path,
) -> Result<Vec<PathBuf>, Error> {
    validate_with_secrets(descriptor, secrets)?;
    std::fs::create_dir_all(out).map_err(|source| Error::Write {
        path: out.to_path_buf(),
        source,
    })?;
    let database_endpoint = expand(&descriptor.endpoints.database, secrets)?;
    let database_password = required(secrets, "QR_POSTGRES_PASSWORD")?;
    let object_store = expand(&descriptor.endpoints.object_store, secrets)?;
    let queue = expand(&descriptor.endpoints.queue, secrets)?;
    let bucket = descriptor
        .endpoints
        .object_bucket
        .clone()
        .or_else(|| secrets.get("QR_OBJECT_STORE_BUCKET").cloned())
        .unwrap_or_else(|| "quantrule".to_owned());

    // The secret files are the only place a resolved credential lands. The
    // env files name the path under `/run/secrets`; the Compose roles mount the
    // rendered directory there.
    let secrets_dir = out.join("secrets");
    reset_secret_directory(&secrets_dir)?;
    let pki_dir = out.join("pki");
    std::fs::create_dir_all(&pki_dir).map_err(|source| Error::Write {
        path: pki_dir.clone(),
        source,
    })?;
    for (name, source) in [
        ("database-ca.crt", &descriptor.endpoints.database_tls_ca),
        ("queue-ca.crt", &descriptor.endpoints.queue_tls_ca),
        (
            "object-store-ca.crt",
            &descriptor.endpoints.object_store_tls_ca,
        ),
    ] {
        copy_ca(Path::new(source), &pki_dir.join(name))?;
    }
    let mut app_urls = BTreeMap::new();
    let mut owner_urls = BTreeMap::new();
    for (service, database) in SERVICE_DATABASES {
        app_urls.insert(
            service,
            database_url(
                &database_endpoint,
                &format!("{database}_app"),
                &database_password,
                database,
            )?,
        );
        owner_urls.insert(
            service,
            database_url(
                &database_endpoint,
                &format!("{database}_owner"),
                &database_password,
                database,
            )?,
        );
    }
    let access_key = required(secrets, "QR_OBJECT_STORE_USER")?;
    let secret_key = required(secrets, "QR_OBJECT_STORE_PASSWORD")?;
    let bearer_token = required(secrets, "QR_BEARER_TOKEN")?;

    for (service, _database) in SERVICE_DATABASES {
        write_secret(
            &secrets_dir,
            &format!("{service}_database_url"),
            app_urls[service].as_str(),
        )?;
        write_secret(
            &secrets_dir,
            &format!("{service}_owner_database_url"),
            owner_urls[service].as_str(),
        )?;
    }
    write_secret(&secrets_dir, "queue_url", &queue)?;
    write_secret(&secrets_dir, "object_store_access_key", &access_key)?;
    write_secret(&secrets_dir, "object_store_secret_key", &secret_key)?;
    write_secret(&secrets_dir, "bearer_token", &bearer_token)?;

    let mut written = Vec::new();
    for name in [
        "gateway",
        "data",
        "model",
        "evidence",
        "simulation",
        "backtest",
    ] {
        let service = &descriptor.services[name];
        let host = &descriptor.hosts[&service.host];
        let mut env = common(descriptor, &object_store, &bucket);
        env.insert("QR_BIND_IP".to_owned(), host.address.clone());
        if let Some(port) = service.port {
            env.insert("QR_BIND_PORT".to_owned(), port.to_string());
            env.insert("QR_BIND".to_owned(), format!("{}:{port}", host.address));
        }
        match name {
            "gateway" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "0".to_owned());
                env.insert("QR_STATIC_DIR".to_owned(), "/srv/static".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                env.insert("QR_MODEL_URL".to_owned(), service_url(descriptor, "model")?);
                env.insert(
                    "QR_EVIDENCE_URL".to_owned(),
                    service_url(descriptor, "evidence")?,
                );
                env.insert(
                    "QR_SIMULATION_URL".to_owned(),
                    service_url(descriptor, "simulation")?,
                );
                env.insert(
                    "QR_DATABASE_URL_FILE".to_owned(),
                    container_secret("gateway_database_url"),
                );
                env.insert(
                    "QR_DATABASE_TLS_CA_PATH".to_owned(),
                    container_pki("database-ca.crt"),
                );
                if descriptor.auth.gateway_token {
                    env.insert(
                        "QR_BEARER_TOKEN_FILE".to_owned(),
                        container_secret("bearer_token"),
                    );
                }
            }
            "data" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_CONFIG_ROOT".to_owned(), descriptor.config_root.clone());
                env.insert(
                    "QR_DATABASE_URL_FILE".to_owned(),
                    container_secret("data_database_url"),
                );
                env.insert(
                    "QR_DATABASE_TLS_CA_PATH".to_owned(),
                    container_pki("database-ca.crt"),
                );
                insert_object_store_files(&mut env);
            }
            "model" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                insert_object_store_files(&mut env);
            }
            "evidence" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                env.insert(
                    "QR_STAGE_RETENTION_DAYS".to_owned(),
                    descriptor.stage_retention_days.unwrap_or(30).to_string(),
                );
                env.insert(
                    "QR_DATABASE_URL_FILE".to_owned(),
                    container_secret("evidence_database_url"),
                );
                env.insert(
                    "QR_DATABASE_TLS_CA_PATH".to_owned(),
                    container_pki("database-ca.crt"),
                );
                env.insert(
                    "QR_QUEUE_URL_FILE".to_owned(),
                    container_secret("queue_url"),
                );
                env.insert(
                    "QR_QUEUE_TLS_CA_PATH".to_owned(),
                    container_pki("queue-ca.crt"),
                );
                insert_object_store_files(&mut env);
            }
            "simulation" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert(
                    "QR_DATABASE_URL_FILE".to_owned(),
                    container_secret("simulation_database_url"),
                );
                env.insert(
                    "QR_DATABASE_TLS_CA_PATH".to_owned(),
                    container_pki("database-ca.crt"),
                );
                insert_object_store_files(&mut env);
            }
            "backtest" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                env.insert("QR_MODEL_URL".to_owned(), service_url(descriptor, "model")?);
                env.insert(
                    "QR_EVIDENCE_URL".to_owned(),
                    service_url(descriptor, "evidence")?,
                );
                env.insert(
                    "QR_BACKTEST_CONCURRENCY".to_owned(),
                    descriptor.backtest_concurrency.unwrap_or(1).to_string(),
                );
                env.insert(
                    "QR_QUEUE_URL_FILE".to_owned(),
                    container_secret("queue_url"),
                );
                env.insert(
                    "QR_QUEUE_TLS_CA_PATH".to_owned(),
                    container_pki("queue-ca.crt"),
                );
                insert_object_store_files(&mut env);
            }
            _ => {}
        }
        let path = out.join(format!("{name}.env"));
        write_env(&path, &env)?;
        written.push(path);
    }
    for (service, _database) in SERVICE_DATABASES {
        let mut env = BTreeMap::new();
        env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
        env.insert(
            "QR_DATABASE_OWNER_URL_FILE".to_owned(),
            container_secret(&format!("{service}_owner_database_url")),
        );
        env.insert(
            "QR_DATABASE_TLS_CA_PATH".to_owned(),
            container_pki("database-ca.crt"),
        );
        let path = out.join(format!("{service}-migrate.env"));
        write_env(&path, &env)?;
        written.push(path);
    }
    let mut infra = BTreeMap::new();
    infra.insert(
        "QR_BIND_IP".to_owned(),
        descriptor.hosts[&descriptor.infra_host].address.clone(),
    );
    for key in [
        "QR_POSTGRES_USER",
        "QR_POSTGRES_PASSWORD",
        "QR_POSTGRES_DB",
        "QR_OBJECT_STORE_USER",
        "QR_OBJECT_STORE_PASSWORD",
        "QR_QUEUE_USER",
        "QR_QUEUE_PASSWORD",
        "QR_QUEUE_VHOST",
    ] {
        infra.insert(key.to_owned(), required(secrets, key)?);
    }
    infra.insert("QR_OBJECT_BUCKET".to_owned(), bucket);
    let path = out.join("infra.env");
    write_env(&path, &infra)?;
    written.push(path);
    Ok(written)
}

/// The in-container path of one rendered secret.
fn container_secret(name: &str) -> String {
    format!("/run/secrets/{name}")
}

/// The in-container path of one rendered CA bundle.
fn container_pki(name: &str) -> String {
    format!("/run/pki/{name}")
}

fn copy_ca(source: &Path, destination: &Path) -> Result<(), Error> {
    std::fs::copy(source, destination)
        .map(|_| ())
        .map_err(|source_error| Error::Read {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(destination, std::fs::Permissions::from_mode(0o644)).map_err(
            |source_error| Error::Write {
                path: destination.to_path_buf(),
                source: source_error,
            },
        )?;
    }
    Ok(())
}

fn insert_object_store_files(env: &mut BTreeMap<String, String>) {
    env.insert(
        "QR_OBJECT_STORE_ACCESS_KEY_FILE".to_owned(),
        container_secret("object_store_access_key"),
    );
    env.insert(
        "QR_OBJECT_STORE_SECRET_KEY_FILE".to_owned(),
        container_secret("object_store_secret_key"),
    );
}

fn reset_secret_directory(path: &Path) -> Result<(), Error> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(Error::Write {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    std::fs::create_dir_all(path).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).map_err(
            |source| Error::Write {
                path: path.to_path_buf(),
                source,
            },
        )?;
    }
    Ok(())
}

fn common(descriptor: &Descriptor, object_store: &str, bucket: &str) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("QR_IMAGE".to_owned(), descriptor.image.reference.clone());
    env.insert(
        "QR_IMAGE_DIGEST".to_owned(),
        descriptor.image.digest.clone(),
    );
    env.insert("QR_PROFILE".to_owned(), "server".to_owned());
    env.insert("QR_OBJECT_STORE_URL".to_owned(), object_store.to_owned());
    env.insert(
        "QR_OBJECT_STORE_TLS_CA_PATH".to_owned(),
        container_pki("object-store-ca.crt"),
    );
    env.insert("QR_OBJECT_BUCKET".to_owned(), bucket.to_owned());
    env.insert("QR_QUEUE_GENERATION".to_owned(), "1".to_owned());
    env
}

/// Build one role's URL from the descriptor's Postgres endpoint.
///
/// The descriptor names one endpoint with one password. The renderer keeps the
/// endpoint's scheme and host:port and replaces the userinfo and database path
/// with the service role and its own database. Nothing is shared between
/// services: a role can connect to one database only.
fn database_url(
    endpoint: &str,
    role: &str,
    password: &str,
    database: &str,
) -> Result<String, Error> {
    let (scheme, rest) = endpoint.split_once("://").ok_or_else(|| Error::Invalid {
        field: "endpoints.database".to_owned(),
        message: "must be a postgres URL with a scheme".to_owned(),
    })?;
    let (authority, _path) = rest.split_once('/').ok_or_else(|| Error::Invalid {
        field: "endpoints.database".to_owned(),
        message: "must name a host, port, and database path".to_owned(),
    })?;
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_userinfo, host)| host);
    if host.is_empty() {
        return Err(Error::Invalid {
            field: "endpoints.database".to_owned(),
            message: "must name a host and port".to_owned(),
        });
    }
    Ok(format!(
        "{scheme}://{role}:{}@{host}/{database}?sslmode=verify-full",
        encode_userinfo(password)
    ))
}

/// Percent-encode the characters that would end the userinfo or authority.
///
/// The secret file carries a raw password. A raw `@`, `/`, or `%` would make
/// the generated URL mean a different host or a broken escape, so it is
/// encoded here rather than left to the operator.
fn encode_userinfo(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let unreserved = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'.'
                    | b'_'
                    | b'~'
                    | b'!'
                    | b'$'
                    | b'&'
                    | b'\''
                    | b'('
                    | b')'
                    | b'*'
                    | b'+'
                    | b','
                    | b';'
                    | b'='
                    | b':'
            );
        if unreserved {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn service_url(descriptor: &Descriptor, name: &str) -> Result<String, Error> {
    let service = &descriptor.services[name];
    let host = &descriptor.hosts[&service.host];
    let Some(port) = service.port else {
        return Err(Error::Invalid {
            field: format!("services.{name}.port"),
            message: "is required for an HTTP service".to_owned(),
        });
    };
    Ok(format!("http://{}:{port}", host.address))
}

fn required(secrets: &BTreeMap<String, String>, name: &str) -> Result<String, Error> {
    secrets
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or_else(|| Error::MissingSecret {
            name: name.to_owned(),
        })
}

fn write_env(path: &Path, env: &BTreeMap<String, String>) -> Result<(), Error> {
    let mut text = String::from("# generated by qr-deploy; do not edit\n");
    for (key, value) in env {
        text.push_str(key);
        text.push('=');
        text.push_str(value);
        text.push('\n');
    }
    write_private(path, text.as_bytes())
}

fn write_secret(dir: &Path, name: &str, value: &str) -> Result<(), Error> {
    let path = dir.join(name);
    let mut text = String::with_capacity(value.len() + 1);
    text.push_str(value);
    text.push('\n');
    write_private(&path, text.as_bytes())
}

fn write_private(path: &Path, contents: &[u8]) -> Result<(), Error> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    use std::io::Write;
    file.write_all(contents).map_err(|source| Error::Write {
        path: path.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, permissions).map_err(|source| Error::Write {
            path: path.to_path_buf(),
            source,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn descriptor() -> Descriptor {
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../deploy/multi-host/deploy.example.json"
        ))
        .expect("example");
        value["image"]["digest"] = serde_json::json!(
            "sha256:3f5a9c1e7b2d4f608a1c3e5d7b9f0a2c4e6d8f1b3a5c7e9d0f2a4c6e8b1d3f5a"
        );
        serde_json::from_value(value).expect("descriptor")
    }

    fn secrets() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("QR_POSTGRES_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_POSTGRES_PASSWORD".to_owned(),
                "db-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_POSTGRES_DB".to_owned(), "quantrule".to_owned()),
            ("QR_OBJECT_STORE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_OBJECT_STORE_PASSWORD".to_owned(),
                "object-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_QUEUE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_QUEUE_PASSWORD".to_owned(),
                "queue-secret-0123456789abcdef".to_owned(),
            ),
            ("QR_QUEUE_VHOST".to_owned(), "/".to_owned()),
            (
                "QR_BEARER_TOKEN".to_owned(),
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
            ),
        ])
    }

    fn temp_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "qr-deploy-render-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    #[test]
    fn render_writes_service_envs_migration_envs_and_private_secret_files() {
        let out = temp_dir();
        std::fs::create_dir_all(&out).expect("output");
        let ca = out.join("ca.crt");
        std::fs::write(
            &ca,
            "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
        )
        .expect("CA fixture");
        let mut descriptor = descriptor();
        descriptor.endpoints.database_tls_ca = ca.display().to_string();
        descriptor.endpoints.queue_tls_ca = ca.display().to_string();
        descriptor.endpoints.object_store_tls_ca = ca.display().to_string();
        let files = render(&descriptor, &secrets(), &out).expect("render");
        assert_eq!(files.len(), 11);
        let gateway = std::fs::read_to_string(out.join("gateway.env")).expect("gateway");
        assert!(
            gateway.contains("QR_BEARER_TOKEN_FILE=/run/secrets/bearer_token"),
            "{gateway}"
        );
        assert!(gateway.contains("QR_DATA_URL=http://10.10.0.11:18081"));
        assert!(gateway.contains("QR_INTERNAL_ONLY=0"));
        assert!(gateway.contains("QR_MIGRATE=0"));
        assert!(gateway.contains("QR_DATABASE_URL_FILE=/run/secrets/gateway_database_url"));
        assert!(gateway.contains("QR_DATABASE_TLS_CA_PATH=/run/pki/database-ca.crt"));
        assert!(gateway.contains("QR_OBJECT_STORE_TLS_CA_PATH=/run/pki/object-store-ca.crt"));
        assert!(!gateway.contains("QR_DATABASE_URL="));
        assert!(!gateway.contains("QR_DATABASE_OWNER_URL"));
        assert!(!gateway.contains("AWS_ACCESS_KEY_ID"));
        assert!(!gateway.contains("AWS_ACCESS_KEY_ID"));
        assert!(!gateway.contains("token=0123456789abcdef"));
        let data = std::fs::read_to_string(out.join("data.env")).expect("data");
        assert!(data.contains("QR_INTERNAL_ONLY=1"));
        assert!(data.contains("QR_BIND=10.10.0.11:18081"));
        assert!(data.contains("QR_MIGRATE=0"));
        assert!(data.contains("QR_DATABASE_URL_FILE=/run/secrets/data_database_url"));
        assert!(
            data.contains("QR_OBJECT_STORE_ACCESS_KEY_FILE=/run/secrets/object_store_access_key")
        );
        assert!(
            data.contains("QR_OBJECT_STORE_SECRET_KEY_FILE=/run/secrets/object_store_secret_key")
        );
        assert!(!data.contains("QR_DATABASE_OWNER_URL"));
        let model = std::fs::read_to_string(out.join("model.env")).expect("model");
        assert!(!model.contains("QR_DATABASE_URL"));
        assert!(!model.contains("QR_DATABASE_OWNER_URL"));
        assert!(model.contains("QR_OBJECT_STORE_ACCESS_KEY_FILE"));
        let evidence = std::fs::read_to_string(out.join("evidence.env")).expect("evidence");
        assert!(evidence.contains("QR_DATABASE_URL_FILE=/run/secrets/evidence_database_url"));
        assert!(evidence.contains("QR_DATABASE_TLS_CA_PATH=/run/pki/database-ca.crt"));
        assert!(evidence.contains("QR_QUEUE_URL_FILE=/run/secrets/queue_url"));
        assert!(evidence.contains("QR_QUEUE_TLS_CA_PATH=/run/pki/queue-ca.crt"));
        assert!(!evidence.contains("QR_DATABASE_OWNER_URL"));
        let simulation = std::fs::read_to_string(out.join("simulation.env")).expect("simulation");
        assert!(simulation.contains("QR_DATABASE_URL_FILE=/run/secrets/simulation_database_url"));
        assert!(!simulation.contains("QR_DATABASE_OWNER_URL"));
        let backtest = std::fs::read_to_string(out.join("backtest.env")).expect("backtest");
        assert!(backtest.contains("QR_BACKTEST_CONCURRENCY=1"));
        assert!(!backtest.contains("QR_BIND="));
        assert!(!backtest.contains("QR_DATABASE_URL"));
        assert!(!backtest.contains("QR_DATABASE_OWNER_URL"));
        assert!(backtest.contains("QR_QUEUE_URL_FILE=/run/secrets/queue_url"));

        for (service, database) in SERVICE_DATABASES {
            let env = std::fs::read_to_string(out.join(format!("{service}-migrate.env")))
                .expect("migration env");
            assert!(env.contains("QR_MIGRATE=0"));
            assert!(env.contains(&format!(
                "QR_DATABASE_OWNER_URL_FILE=/run/secrets/{service}_owner_database_url"
            )));
            assert!(!env.contains("QR_DATABASE_URL="));
            let secret = std::fs::read_to_string(
                out.join("secrets")
                    .join(format!("{service}_owner_database_url")),
            )
            .expect("owner secret");
            assert!(secret.contains(&format!(
                "postgres://{database}_owner:db-secret-0123456789abcdef@10.10.0.20:5432/{database}?sslmode=verify-full"
            )));
        }

        let infra = std::fs::read_to_string(out.join("infra.env")).expect("infra");
        assert!(infra.contains("QR_BIND_IP=10.10.0.20"));
        assert!(infra.contains("QR_OBJECT_BUCKET=quantrule"));

        let app_secret =
            std::fs::read_to_string(out.join("secrets/gateway_database_url")).expect("app secret");
        assert!(app_secret.contains(
            "postgres://qr_gateway_app:db-secret-0123456789abcdef@10.10.0.20:5432/qr_gateway?sslmode=verify-full"
        ));
        let queue = std::fs::read_to_string(out.join("secrets/queue_url")).expect("queue secret");
        assert!(queue.contains("queue-secret-0123456789abcdef"));
        let object = std::fs::read_to_string(out.join("secrets/object_store_secret_key"))
            .expect("object secret");
        assert!(object.contains("object-secret-0123456789abcdef"));
        let token =
            std::fs::read_to_string(out.join("secrets/bearer_token")).expect("token secret");
        assert!(token.contains("0123456789abcdef0123456789abcdef"));
        for name in ["database-ca.crt", "queue-ca.crt", "object-store-ca.crt"] {
            let copied = std::fs::read_to_string(out.join("pki").join(name)).expect("copied CA");
            assert!(copied.contains("BEGIN CERTIFICATE"), "{name}");
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for (path, expected) in [
                (out.join("gateway.env"), 0o600),
                (out.join("secrets"), 0o700),
                (out.join("secrets/gateway_database_url"), 0o600),
                (out.join("secrets/bearer_token"), 0o600),
            ] {
                let mode = std::fs::metadata(&path)
                    .expect("metadata")
                    .permissions()
                    .mode()
                    & 0o777;
                assert_eq!(mode, expected, "{}", path.display());
            }
        }
        std::fs::remove_dir_all(&out).expect("cleanup");
    }

    #[test]
    fn render_reports_a_missing_placeholder() {
        let out = temp_dir();
        let mut value: Value = serde_json::from_str(include_str!(
            "../../../deploy/multi-host/deploy.example.json"
        ))
        .expect("example");
        value["image"]["digest"] = serde_json::json!(
            "sha256:3f5a9c1e7b2d4f608a1c3e5d7b9f0a2c4e6d8f1b3a5c7e9d0f2a4c6e8b1d3f5a"
        );
        value["endpoints"]["database"] = serde_json::json!(
            "postgres://${MISSING}@10.10.0.20:5432/quantrule?sslmode=verify-full"
        );
        let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
        let error = render(&descriptor, &secrets(), &out).expect_err("missing secret");
        assert!(error.to_string().contains("MISSING"), "{error}");
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn a_role_url_keeps_the_host_and_encodes_the_password() {
        let url = database_url(
            "postgres://quantrule:unused@10.10.0.20:5432/quantrule",
            "qr_catalog_app",
            "p@ss/word%",
            "qr_catalog",
        )
        .expect("url");
        assert_eq!(
            url,
            "postgres://qr_catalog_app:p%40ss%2Fword%25@10.10.0.20:5432/qr_catalog?sslmode=verify-full"
        );
    }

    #[test]
    fn a_database_endpoint_without_a_path_is_refused() {
        let error = database_url(
            "postgres://quantrule:secret@10.10.0.20:5432",
            "qr_catalog_app",
            "secret",
            "qr_catalog",
        )
        .expect_err("no path");
        assert!(error.to_string().contains("host, port, and database path"));
    }
}
