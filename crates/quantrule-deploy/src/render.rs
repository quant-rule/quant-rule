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
    let database = expand(&descriptor.endpoints.database, secrets)?;
    let object_store = expand(&descriptor.endpoints.object_store, secrets)?;
    let queue = expand(&descriptor.endpoints.queue, secrets)?;
    let bucket = descriptor
        .endpoints
        .object_bucket
        .clone()
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
        let mut env = common(
            descriptor,
            secrets,
            &database,
            &object_store,
            &queue,
            &bucket,
        );
        env.insert("QR_BIND_IP".to_owned(), host.address.clone());
        if let Some(port) = service.port {
            env.insert("QR_BIND_PORT".to_owned(), port.to_string());
            env.insert("QR_BIND".to_owned(), format!("{}:{port}", host.address));
        }
        match name {
            "gateway" => {
                env.insert("QR_MIGRATE".to_owned(), "1".to_owned());
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
                env.insert("QR_MIGRATE".to_owned(), "1".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_CONFIG_ROOT".to_owned(), descriptor.config_root.clone());
            }
            "model" => {
                env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            }
            "evidence" => {
                env.insert("QR_MIGRATE".to_owned(), "1".to_owned());
                env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
                env.insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
                env.insert(
                    "QR_STAGE_RETENTION_DAYS".to_owned(),
                    descriptor.stage_retention_days.unwrap_or(30).to_string(),
                );
            }
            "simulation" => {
                env.insert("QR_MIGRATE".to_owned(), "1".to_owned());
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
        let path = out.join(format!("{name}.env"));
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
    database: &str,
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
    env.insert("QR_DATABASE_URL".to_owned(), database.to_owned());
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
    std::fs::write(path, text).map_err(|source| Error::Write {
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
        serde_json::from_str(include_str!(
            "../../../deploy/multi-host/deploy.example.json"
        ))
        .expect("example")
    }

    fn secrets() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("QR_POSTGRES_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_POSTGRES_PASSWORD".to_owned(),
                "long-random-db".to_owned(),
            ),
            ("QR_POSTGRES_DB".to_owned(), "quantrule".to_owned()),
            ("QR_OBJECT_STORE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_OBJECT_STORE_PASSWORD".to_owned(),
                "long-random-object".to_owned(),
            ),
            ("QR_QUEUE_USER".to_owned(), "quantrule".to_owned()),
            (
                "QR_QUEUE_PASSWORD".to_owned(),
                "long-random-queue".to_owned(),
            ),
            ("QR_QUEUE_VHOST".to_owned(), "/".to_owned()),
            ("QR_BEARER_TOKEN".to_owned(), "long-random-token".to_owned()),
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
    fn render_writes_one_env_per_service_and_the_infra_env() {
        let out = temp_dir();
        let files = render(&descriptor(), &secrets(), &out).expect("render");
        assert_eq!(files.len(), 7);
        let gateway = std::fs::read_to_string(out.join("gateway.env")).expect("gateway");
        assert!(gateway.contains("QR_BEARER_TOKEN=long-random-token"));
        assert!(gateway.contains("QR_DATA_URL=http://10.10.0.11:18081"));
        assert!(gateway.contains("QR_INTERNAL_ONLY=0"));
        let data = std::fs::read_to_string(out.join("data.env")).expect("data");
        assert!(data.contains("QR_INTERNAL_ONLY=1"));
        assert!(data.contains("QR_BIND=10.10.0.11:18081"));
        assert!(data.contains("QR_MIGRATE=1"));
        let backtest = std::fs::read_to_string(out.join("backtest.env")).expect("backtest");
        assert!(backtest.contains("QR_BACKTEST_CONCURRENCY=1"));
        assert!(!backtest.contains("QR_BIND="));
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
        value["endpoints"]["database"] =
            serde_json::json!("postgres://${MISSING}@10.10.0.20:5432/quantrule");
        let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
        let error = render(&descriptor, &secrets(), &out).expect_err("missing secret");
        assert!(error.to_string().contains("MISSING"), "{error}");
        let _ = std::fs::remove_dir_all(&out);
    }
}
