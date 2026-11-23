//! The four-database topology guard.
//!
//! The local Compose stack and the multi-host bootstrap must agree on four
//! logical databases, one owner role and one application role per database,
//! and no database for the stateless services. The owner URL belongs only to a
//! one-shot migration job; the long-running service holds only the application
//! URL. This test reads the checked-in files, so a service renamed to a fifth
//! database, a shared URL, or an owner credential in a long-running service
//! fails the fast gate instead of surfacing during a deployment.

use std::path::{Path, PathBuf};

/// One database, its owner schema set, and the Compose service that owns it.
const DATABASES: [(&str, &str, &[&str]); 4] = [
    ("qr_gateway", "gateway", &["gateway"]),
    ("qr_catalog", "data", &["catalog"]),
    ("qr_evidence", "evidence", &["runs", "research", "stage"]),
    ("qr_simulation", "simulation", &["simulation"]),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the runtime manifest has two ancestors")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The lines of one top-level Compose service.
fn service_block<'a>(compose: &'a str, name: &str) -> &'a str {
    let marker = format!("  {name}:\n");
    let start = compose
        .match_indices(&marker)
        .find(|(offset, _)| *offset == 0 || compose.as_bytes()[offset - 1] == b'\n')
        .map(|(offset, _)| offset + marker.len())
        .unwrap_or_else(|| panic!("Compose has no top-level service {name}"));
    let rest = &compose[start..];
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        let top_level_service = line.starts_with("  ") && line.as_bytes().get(2) != Some(&b' ');
        if top_level_service {
            return &rest[..offset];
        }
        offset += line.len();
    }
    rest
}

#[test]
fn services_hold_only_the_app_url_and_migration_jobs_only_the_owner_url() {
    let compose = read("compose.yaml");
    assert_eq!(
        compose.matches("QR_DATABASE_URL_FILE:").count(),
        4,
        "exactly four long-running services own a database"
    );
    assert_eq!(
        compose.matches("QR_DATABASE_OWNER_URL_FILE:").count(),
        4,
        "exactly four one-shot migration jobs hold the owner URL"
    );
    for (database, service, _schemas) in DATABASES {
        let block = service_block(&compose, service);
        assert!(
            block.contains(&format!(
                "QR_DATABASE_URL_FILE: /run/secrets/{database}_database_url"
            )),
            "{service} has no {database}_app file"
        );
        assert!(
            block.contains(&format!("/run/secrets/{database}_database_url")),
            "{service} does not mount {database}_app"
        );
        assert!(
            block.contains("QR_MIGRATE: \"0\""),
            "{service} must not migrate"
        );
        assert!(
            !block.contains("QR_DATABASE_OWNER_URL"),
            "{service} carries the owner credential"
        );

        let migrate = service_block(&compose, &format!("{service}-migrate"));
        assert!(
            migrate.contains("--migrate-only"),
            "{service}-migrate has no migration-only command"
        );
        assert!(
            migrate.contains(&format!(
                "QR_DATABASE_OWNER_URL_FILE: /run/secrets/{database}_owner_database_url"
            )),
            "{service}-migrate has no {database}_owner file"
        );
        assert!(
            !migrate.contains("QR_DATABASE_URL:"),
            "{service}-migrate carries the application URL"
        );
        assert!(
            block.contains(&format!(
                "{service}-migrate:\n        condition: service_completed_successfully"
            )),
            "{service} does not wait for its migration job"
        );
    }
}

#[test]
fn application_services_receive_only_file_backed_secrets() {
    let compose = read("compose.yaml");
    for service in [
        "gateway",
        "data",
        "model",
        "evidence",
        "simulation",
        "backtest",
        "strategy",
    ] {
        let block = service_block(&compose, service);
        for forbidden in [
            "QR_DATABASE_URL:",
            "QR_DATABASE_OWNER_URL:",
            "QR_QUEUE_URL:",
            "QR_BEARER_TOKEN:",
            "QR_OBJECT_STORE_ACCESS_KEY:",
            "QR_OBJECT_STORE_SECRET_KEY:",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_REGION",
        ] {
            assert!(
                !block.contains(forbidden),
                "{service} receives the plain secret {forbidden}"
            );
        }
        for line in block.lines().filter(|line| line.contains("_FILE:")) {
            assert!(
                line.contains("/run/secrets/"),
                "{service} names a non-mounted secret: {line}"
            );
        }
    }
}

#[test]
fn stateless_services_have_no_database() {
    let compose = read("compose.yaml");
    for service in ["model", "backtest", "strategy"] {
        let block = service_block(&compose, service);
        assert!(
            !block.contains("QR_DATABASE_URL"),
            "{service} must not receive a database URL"
        );
        assert!(
            !block.contains("QR_DATABASE_OWNER_URL"),
            "{service} must not receive an owner URL"
        );
    }
}

#[test]
fn the_strategy_service_receives_no_database_queue_or_config_mount() {
    let compose = read("compose.yaml");
    let block = service_block(&compose, "strategy");
    for forbidden in [
        "QR_DATABASE_URL",
        "QR_DATABASE_OWNER_URL",
        "QR_QUEUE_URL",
        "QR_CONFIG_ROOT",
    ] {
        assert!(
            !block.contains(forbidden),
            "strategy must not receive {forbidden}"
        );
    }
    assert!(block.contains("QR_DATA_URL: http://data:8080"));
    assert!(block.contains("QR_MODEL_URL: http://model:8080"));
    assert!(block.contains("QR_OBJECT_STORE_URL: https://minio:9000"));
    assert!(block.contains("QR_OBJECT_STORE_TLS_CA_PATH: /run/pki/ca.crt"));
    assert!(block.contains("/run/pki/ca.crt:ro"));
    assert!(
        block.contains("QR_OBJECT_STORE_ACCESS_KEY_FILE: /run/secrets/object_store_access_key")
    );
    assert!(
        block.contains("QR_OBJECT_STORE_SECRET_KEY_FILE: /run/secrets/object_store_secret_key")
    );
}

#[test]
fn the_local_stateful_endpoints_are_tls() {
    let compose = read("compose.yaml");
    assert!(
        compose.contains("QR_OBJECT_STORE_URL: https://minio:9000"),
        "the object store must use https"
    );
    assert!(
        !compose.contains("QR_OBJECT_STORE_URL: http://"),
        "a plaintext object-store URL is a defect"
    );
    assert!(compose.contains("QR_DATABASE_TLS_CA_PATH"));
    assert!(compose.contains("QR_QUEUE_TLS_CA_PATH"));
    assert!(compose.contains("QR_OBJECT_STORE_TLS_CA_PATH"));
    assert!(compose.contains("PGSSLMODE: verify-full"));

    let secrets = read("tools/secrets.sh");
    assert_eq!(
        secrets.matches("?sslmode=verify-full").count(),
        8,
        "every generated database URL must use verify-full"
    );
    assert!(
        secrets.contains("amqps://quantrule:$QR_RABBITMQ_PASSWORD@rabbitmq:5671/%2f"),
        "the generated queue URL must use amqps on 5671"
    );
    assert!(
        !secrets.contains("amqp://quantrule:$QR_RABBITMQ_PASSWORD@rabbitmq:5672"),
        "a plaintext AMQP URL is a defect"
    );
}

#[test]
fn the_bootstrap_runs_before_every_migration_job() {
    let compose = read("compose.yaml");
    let bootstrap = service_block(&compose, "db-bootstrap");
    assert!(bootstrap.contains("./deploy/postgres/bootstrap.sh:/bootstrap/bootstrap.sh:ro"));
    assert!(bootstrap.contains("QR_DB_ROLE_PASSWORD"));
    assert_eq!(
        compose
            .matches("db-bootstrap:\n        condition: service_completed_successfully")
            .count(),
        4,
        "all four migration jobs wait for the bootstrap"
    );
    for (_database, service, _schemas) in DATABASES {
        let migrate = service_block(&compose, &format!("{service}-migrate"));
        assert!(
            migrate.contains("db-bootstrap:\n        condition: service_completed_successfully"),
            "{service}-migrate does not wait for the bootstrap"
        );
    }
}

#[test]
fn every_declared_secret_is_a_file_and_every_mount_is_declared() {
    let compose = read("compose.yaml");
    let start = compose
        .find("\nsecrets:\n")
        .expect("a top-level secrets block");
    let block = &compose[start..];
    for name in [
        "qr_gateway_database_url",
        "qr_catalog_database_url",
        "qr_evidence_database_url",
        "qr_simulation_database_url",
        "qr_gateway_owner_database_url",
        "qr_catalog_owner_database_url",
        "qr_evidence_owner_database_url",
        "qr_simulation_owner_database_url",
        "queue_url",
        "object_store_access_key",
        "object_store_secret_key",
    ] {
        assert!(
            block.contains(&format!("  {name}:\n    file: ")),
            "the top-level secret {name} is not a file reference"
        );
    }
    assert!(
        !block.contains("environment:"),
        "a top-level secret carries an inline value"
    );

    for service in [
        "gateway",
        "data",
        "model",
        "evidence",
        "simulation",
        "backtest",
        "strategy",
    ] {
        let service_block = service_block(&compose, service);
        let Some((_, rest)) = service_block.split_once("    secrets:\n") else {
            panic!("{service} has no secrets mount")
        };
        let mut mounted = Vec::new();
        for line in rest.lines() {
            if let Some(name) = line.strip_prefix("      - ") {
                mounted.push(name.trim().to_owned());
            } else {
                break;
            }
        }
        for line in service_block
            .lines()
            .filter(|line| line.contains("_FILE: /run/secrets/"))
        {
            let name = line
                .rsplit_once("/run/secrets/")
                .map(|(_, name)| name.trim())
                .unwrap_or_default();
            assert!(
                mounted.iter().any(|mounted| mounted == name),
                "{service} names {name} but does not mount it"
            );
        }
    }

    let remote = read("compose.remote.yaml");
    assert!(remote.contains("QR_BEARER_TOKEN_FILE: /run/secrets/bearer_token"));
    assert!(!remote.contains("QR_BEARER_TOKEN:"));
}

#[test]
fn the_bootstrap_creates_four_databases_and_isolated_roles() {
    let script = read("deploy/postgres/bootstrap.sh");
    for (database, _service, schemas) in DATABASES {
        assert!(
            script.contains(&format!("'{database}', '{}_owner'", database)),
            "the bootstrap does not create {database}"
        );
        assert!(
            script.contains(&format!("CREATE ROLE %I LOGIN', '{database}_owner'")),
            "the bootstrap does not create the {database} owner role"
        );
        assert!(
            script.contains(&format!("CREATE ROLE %I LOGIN', '{database}_app'")),
            "the bootstrap does not create the {database} application role"
        );
        assert!(
            script.contains(&format!("REVOKE ALL ON DATABASE {database} FROM PUBLIC;")),
            "PUBLIC can still connect to {database}"
        );
        for schema in schemas {
            assert!(
                script.contains(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {schema} AUTHORIZATION {database}_owner;"
                )),
                "the bootstrap does not own {schema}"
            );
            assert!(
                script.contains(&format!("REVOKE ALL ON SCHEMA {schema} FROM PUBLIC;")),
                "PUBLIC can still use {schema}"
            );
            assert!(
                script.contains(&format!(
                    "GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA {schema} TO {database}_app;"
                )),
                "the {database}_app role has no DML on {schema}"
            );
            assert!(
                script.contains(&format!(
                    "GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA {schema} TO {database}_app;"
                )),
                "the {database}_app role cannot advance {schema} sequences"
            );
            assert!(
                script.contains(&format!(
                    "ALTER DEFAULT PRIVILEGES FOR ROLE {database}_owner IN SCHEMA {schema}"
                )),
                "new {schema} objects would not be granted to {database}_app"
            );
        }
    }
}
