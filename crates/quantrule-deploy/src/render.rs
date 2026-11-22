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
        let mut env = common(descriptor, secrets, &object_store, &queue, &bucket);
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
                if descriptor.auth.gateway_token {
                    env.insert(
                        "QR_BEARER_TOKEN".to_owned(),
                        required(secrets, "QR_BEARER_TOKEN")?,
                    );
                }
            }
            "data" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_CONFIG_ROOT".to_owned(), descriptor.config_root.clone());
            }
            "model" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            }
            "evidence" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                env.insert(
                    "QR_STAGE_RETENTION_DAYS".to_owned(),
                    descriptor.stage_retention_days.unwrap_or(30).to_string(),
                );
            }
            "simulation" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
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
            }
            _ => {}
        }
        if let Some(database) = SERVICE_DATABASES
            .iter()
            .find_map(|(service, database)| (*service == name).then_some(*database))
        {
            env.insert(
                "QR_DATABASE_URL".to_owned(),
                database_url(
                    &database_endpoint,
                    &format!("{database}_app"),
                    &database_password,
                    database,
                )?,
            );
        }
        let path = out.join(format!("{name}.env"));
        write_env(&path, &env)?;
        written.push(path);
    }
    for (service, database) in SERVICE_DATABASES {
        let mut env = BTreeMap::new();
        env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
        env.insert(
            "QR_DATABASE_OWNER_URL".to_owned(),
            database_url(
                &database_endpoint,
                &format!("{database}_owner"),
                &database_password,
                database,
            )?,
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

fn common(
    descriptor: &Descriptor,
    secrets: &BTreeMap<String, String>,
    object_store: &str,
    queue: &str,
    bucket: &str,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("QR_IMAGE".to_owned(), descriptor.image.reference.clone());
    env.insert(
        "QR_IMAGE_DIGEST".to_owned(),
        descriptor.image.digest.clone(),
    );
    env.insert("QR_PROFILE".to_owned(), "server".to_owned());
    env.insert("QR_OBJECT_STORE_URL".to_owned(), object_store.to_owned());
    env.insert("QR_OBJECT_BUCKET".to_owned(), bucket.to_owned());
    env.insert("QR_QUEUE_URL".to_owned(), queue.to_owned());
    env.insert(
        "AWS_ACCESS_KEY_ID".to_owned(),
        secrets
            .get("QR_OBJECT_STORE_USER")
            .cloned()
            .unwrap_or_default(),
    );
    env.insert(
        "AWS_SECRET_ACCESS_KEY".to_owned(),
        secrets
            .get("QR_OBJECT_STORE_PASSWORD")
            .cloned()
            .unwrap_or_default(),
    );
    env.insert("AWS_REGION".to_owned(), "us-east-1".to_owned());
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
        "{scheme}://{role}:{}@{host}/{database}",
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
    file.write_all(text.as_bytes())
        .map_err(|source| Error::Write {
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
    fn render_writes_service_and_migration_envs() {
        let out = temp_dir();
        let files = render(&descriptor(), &secrets(), &out).expect("render");
        assert_eq!(files.len(), 11);
        let gateway = std::fs::read_to_string(out.join("gateway.env")).expect("gateway");
        assert!(gateway.contains("QR_BEARER_TOKEN=0123456789abcdef"));
        assert!(gateway.contains("QR_DATA_URL=http://10.10.0.11:18081"));
        assert!(gateway.contains("QR_INTERNAL_ONLY=0"));
        assert!(gateway.contains("QR_MIGRATE=0"));
        assert!(gateway.contains(
            "QR_DATABASE_URL=postgres://qr_gateway_app:db-secret-0123456789abcdef@10.10.0.20:5432/qr_gateway"
        ));
        assert!(!gateway.contains("QR_DATABASE_OWNER_URL"));
        let data = std::fs::read_to_string(out.join("data.env")).expect("data");
        assert!(data.contains("QR_INTERNAL_ONLY=1"));
        assert!(data.contains("QR_BIND=10.10.0.11:18081"));
        assert!(data.contains("QR_MIGRATE=0"));
        assert!(data.contains(
            "QR_DATABASE_URL=postgres://qr_catalog_app:db-secret-0123456789abcdef@10.10.0.20:5432/qr_catalog"
        ));
        assert!(!data.contains("QR_DATABASE_OWNER_URL"));
        let model = std::fs::read_to_string(out.join("model.env")).expect("model");
        assert!(!model.contains("QR_DATABASE_URL"));
        assert!(!model.contains("QR_DATABASE_OWNER_URL"));
        let evidence = std::fs::read_to_string(out.join("evidence.env")).expect("evidence");
        assert!(evidence.contains(
            "QR_DATABASE_URL=postgres://qr_evidence_app:db-secret-0123456789abcdef@10.10.0.20:5432/qr_evidence"
        ));
        assert!(!evidence.contains("QR_DATABASE_OWNER_URL"));
        let simulation = std::fs::read_to_string(out.join("simulation.env")).expect("simulation");
        assert!(simulation.contains(
            "QR_DATABASE_URL=postgres://qr_simulation_app:db-secret-0123456789abcdef@10.10.0.20:5432/qr_simulation"
        ));
        assert!(!simulation.contains("QR_DATABASE_OWNER_URL"));
        let backtest = std::fs::read_to_string(out.join("backtest.env")).expect("backtest");
        assert!(backtest.contains("QR_BACKTEST_CONCURRENCY=1"));
        assert!(!backtest.contains("QR_BIND="));
        assert!(!backtest.contains("QR_DATABASE_URL"));
        assert!(!backtest.contains("QR_DATABASE_OWNER_URL"));

        for (service, database) in SERVICE_DATABASES {
            let env = std::fs::read_to_string(out.join(format!("{service}-migrate.env")))
                .expect("migration env");
            assert!(env.contains("QR_MIGRATE=0"));
            assert!(env.contains(&format!(
                "QR_DATABASE_OWNER_URL=postgres://{database}_owner:db-secret-0123456789abcdef@10.10.0.20:5432/{database}"
            )));
            assert!(!env.contains("QR_DATABASE_URL="));
        }

        let infra = std::fs::read_to_string(out.join("infra.env")).expect("infra");
        assert!(infra.contains("QR_BIND_IP=10.10.0.20"));
        assert!(infra.contains("QR_OBJECT_BUCKET=quantrule"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(out.join("gateway.env"))
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
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
        value["endpoints"]["database"] =
            serde_json::json!("postgres://${MISSING}@10.10.0.20:5432/quantrule");
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
            "postgres://qr_catalog_app:p%40ss%2Fword%25@10.10.0.20:5432/qr_catalog"
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
