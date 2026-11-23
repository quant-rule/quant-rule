//! The v3 Swarm stack: seven stateless services plus one 443-only ingress.
//!
//! The stack carries no secret value. Every credential is an external Swarm
//! secret mounted under `/run/secrets`; every CA bundle is an external config
//! mounted under `/run/quantrule/pki`. The gateway publishes no port: the one
//! ingress service owns the host-mode 443 binding. The bootstrap script labels
//! the declared hosts, checks placement, creates the external objects, deploys
//! the stack, and runs the one-shot database jobs.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::descriptor::v3::{Descriptor, IngressTls, Profile, Service};
use crate::error::Error;

use super::yaml::Yaml;
use super::{
    APP_SERVICES, app_image, create_dir, evidence_tier, pinned, safe_component,
    stack_name as deployment_name, write_public, write_script,
};

/// The internal metrics port every app service exposes on the overlay.
pub const METRICS_PORT: u16 = 9100;

/// The in-container path of the database CA config.
pub const DATABASE_CA_PATH: &str = "/run/quantrule/pki/database-ca.crt";
/// The in-container path of the queue CA config.
pub const QUEUE_CA_PATH: &str = "/run/quantrule/pki/queue-ca.crt";
/// The in-container path of the object-store CA config.
pub const OBJECT_STORE_CA_PATH: &str = "/run/quantrule/pki/object-store-ca.crt";

/// The external config names the bootstrap script creates.
pub const DATABASE_CA_CONFIG: &str = "quantrule_database_ca";
/// The external config name for the queue CA.
pub const QUEUE_CA_CONFIG: &str = "quantrule_queue_ca";
/// The external config name for the object-store CA.
pub const OBJECT_STORE_CA_CONFIG: &str = "quantrule_object_store_ca";
/// The external config name PLAT-002 owns for the Traefik file provider.
pub const TRAEFIK_CONFIG: &str = "quantrule_traefik";
/// The external config name for an operator-supplied ingress certificate.
pub const INGRESS_CERT_CONFIG: &str = "quantrule_ingress_cert";
/// The external secret name for an operator-supplied ingress key.
pub const INGRESS_KEY_SECRET: &str = "quantrule_ingress_key";

/// Write the Swarm stack, its bootstrap script, and the ordered job scripts.
pub fn render(
    descriptor: &Descriptor,
    out: &std::path::Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    let stack = stack_yaml(descriptor)?;
    let path = out.join("stack.yaml");
    write_public(&path, stack.as_bytes())?;
    written.push(path);

    let bootstrap = bootstrap_script(descriptor);
    let path = out.join("bootstrap.sh");
    write_script(&path, &bootstrap)?;
    written.push(path);

    let jobs = out.join("jobs");
    create_dir(&jobs)?;
    let path = jobs.join("db-bootstrap.sh");
    write_script(&path, &super::jobs::db_bootstrap(descriptor))?;
    written.push(path);
    let path = jobs.join("migrate.sh");
    write_script(&path, &super::jobs::migrate(descriptor))?;
    written.push(path);
    Ok(())
}

/// Build the Compose 3.9 stack document.
///
/// # Errors
///
/// Returns a rule when a descriptor service cannot be planned. The descriptor
/// was validated before this function is called, so the failure is defensive.
pub fn stack_yaml(descriptor: &Descriptor) -> Result<String, Error> {
    let mut services = BTreeMap::new();
    for (name, binary) in APP_SERVICES {
        let plan = service_plan(descriptor, name, binary)?;
        services.insert(name.to_owned(), service_yaml(descriptor, &plan));
    }
    services.insert("ingress".to_owned(), ingress_yaml(descriptor));

    let mut secrets = BTreeMap::new();
    for name in descriptor.secrets.files.keys() {
        safe_component(name, &format!("secrets.files.{name}"))?;
        secrets.insert(name.clone(), external(name));
    }
    if descriptor.ingress.tls == IngressTls::Cert {
        secrets.insert(INGRESS_KEY_SECRET.to_owned(), external(INGRESS_KEY_SECRET));
    }

    let mut configs = BTreeMap::new();
    for name in [
        DATABASE_CA_CONFIG,
        QUEUE_CA_CONFIG,
        OBJECT_STORE_CA_CONFIG,
        TRAEFIK_CONFIG,
    ] {
        configs.insert(name.to_owned(), external(name));
    }
    if descriptor.ingress.tls == IngressTls::Cert {
        configs.insert(
            INGRESS_CERT_CONFIG.to_owned(),
            external(INGRESS_CERT_CONFIG),
        );
    }

    let root = Yaml::map()
        .with("x-quantrule", metadata(descriptor))
        .with(
            "networks",
            Yaml::mapping([(
                descriptor.network.overlay.clone(),
                external(&descriptor.network.overlay),
            )]),
        )
        .with("secrets", Yaml::Map(secrets))
        .with("configs", Yaml::Map(configs))
        .with("services", Yaml::Map(services));
    Ok(root.render())
}

fn metadata(descriptor: &Descriptor) -> Yaml {
    Yaml::map()
        .with("owner", Yaml::string("HA-012"))
        .with("profile", Yaml::string(descriptor.profile.as_str()))
        .with("evidence", Yaml::string(evidence_tier(descriptor.profile)))
        .with(
            "network",
            Yaml::string(
                "overlay, attachable, encrypted; created by bootstrap.sh and checked before deploy",
            ),
        )
        .with("network_encrypted", Yaml::Bool(true))
        .with("network_attachable", Yaml::Bool(true))
        .with(
            "platform",
            Yaml::string(
                "Traefik/ACME, Patroni, RabbitMQ, MinIO, and observability configuration belong to PLAT-002..006",
            ),
        )
}

fn external(name: &str) -> Yaml {
    Yaml::map()
        .with("external", Yaml::Bool(true))
        .with("name", Yaml::string(name))
}

#[derive(Debug)]
struct Mount {
    source: String,
    target: String,
    mode: u64,
}

struct Plan {
    binary: &'static str,
    kind: &'static str,
    replicas: u32,
    host_label: String,
    env: BTreeMap<String, String>,
    secrets: Vec<Mount>,
    configs: Vec<Mount>,
    max_per_node: u32,
}

fn service_plan(
    descriptor: &Descriptor,
    name: &'static str,
    binary: &'static str,
) -> Result<Plan, Error> {
    let service = descriptor.services.get(name).ok_or_else(|| {
        Error::rule(
            "deploy.render.service",
            format!("services.{name}"),
            "the service is not declared",
        )
    })?;
    let mut plan = Plan {
        binary,
        kind: name,
        replicas: service.replicas,
        host_label: format!("node.labels.quantrule.{name} == true"),
        env: common_env(descriptor, service),
        secrets: Vec::new(),
        configs: Vec::new(),
        max_per_node: match descriptor.profile {
            Profile::LabSingleHost => service.replicas.max(1),
            Profile::SwarmHa | Profile::SingleAz => 1,
        },
    };
    plan.env.insert("QR_MIGRATE".to_owned(), "0".to_owned());
    match name {
        "gateway" => {
            plan.env
                .insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            plan.env
                .insert("QR_MODEL_URL".to_owned(), service_url(descriptor, "model")?);
            plan.env.insert(
                "QR_STRATEGY_URL".to_owned(),
                service_url(descriptor, "strategy")?,
            );
            plan.env.insert(
                "QR_EVIDENCE_URL".to_owned(),
                service_url(descriptor, "evidence")?,
            );
            plan.env.insert(
                "QR_SIMULATION_URL".to_owned(),
                service_url(descriptor, "simulation")?,
            );
            plan.env
                .insert("QR_STATIC_DIR".to_owned(), "/srv/static".to_owned());
            plan.env.insert(
                "QR_DATABASE_URL_FILE".to_owned(),
                container_secret("gateway_database_url"),
            );
            plan.env.insert(
                "QR_DATABASE_TLS_CA_PATH".to_owned(),
                DATABASE_CA_PATH.to_owned(),
            );
            plan.env.insert(
                "QR_BEARER_TOKEN_FILE".to_owned(),
                container_secret(&descriptor.auth.gateway_token_secret),
            );
            plan.secrets
                .push(secret("gateway_database_url", "gateway_database_url"));
            plan.secrets.push(secret(
                &descriptor.auth.gateway_token_secret,
                &descriptor.auth.gateway_token_secret,
            ));
            plan.configs
                .push(config(DATABASE_CA_CONFIG, DATABASE_CA_PATH));
        }
        "data" => {
            plan.env
                .insert("QR_CONFIG_ROOT".to_owned(), "/srv/config".to_owned());
            plan.env.insert(
                "QR_DATABASE_URL_FILE".to_owned(),
                container_secret("data_database_url"),
            );
            plan.env.insert(
                "QR_DATABASE_TLS_CA_PATH".to_owned(),
                DATABASE_CA_PATH.to_owned(),
            );
            insert_object_store(&mut plan);
            plan.secrets
                .push(secret("data_database_url", "data_database_url"));
            plan.configs
                .push(config(DATABASE_CA_CONFIG, DATABASE_CA_PATH));
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        "model" => {
            plan.env
                .insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            insert_object_store(&mut plan);
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        "strategy" => {
            plan.env
                .insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            plan.env
                .insert("QR_MODEL_URL".to_owned(), service_url(descriptor, "model")?);
            insert_object_store(&mut plan);
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        "backtest" => {
            plan.env
                .insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            plan.env
                .insert("QR_MODEL_URL".to_owned(), service_url(descriptor, "model")?);
            plan.env.insert(
                "QR_EVIDENCE_URL".to_owned(),
                service_url(descriptor, "evidence")?,
            );
            plan.env
                .insert("QR_BACKTEST_CONCURRENCY".to_owned(), "1".to_owned());
            insert_queue(&mut plan);
            insert_object_store(&mut plan);
            plan.secrets.push(secret("queue_url", "queue_url"));
            plan.configs.push(config(QUEUE_CA_CONFIG, QUEUE_CA_PATH));
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        "evidence" => {
            plan.env
                .insert("QR_DATA_URL".to_owned(), service_url(descriptor, "data")?);
            plan.env
                .insert("QR_STAGE_RETENTION_DAYS".to_owned(), "30".to_owned());
            plan.env.insert(
                "QR_DATABASE_URL_FILE".to_owned(),
                container_secret("evidence_database_url"),
            );
            plan.env.insert(
                "QR_DATABASE_TLS_CA_PATH".to_owned(),
                DATABASE_CA_PATH.to_owned(),
            );
            insert_queue(&mut plan);
            insert_object_store(&mut plan);
            plan.secrets
                .push(secret("evidence_database_url", "evidence_database_url"));
            plan.secrets.push(secret("queue_url", "queue_url"));
            plan.configs
                .push(config(DATABASE_CA_CONFIG, DATABASE_CA_PATH));
            plan.configs.push(config(QUEUE_CA_CONFIG, QUEUE_CA_PATH));
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        "simulation" => {
            plan.env.insert(
                "QR_DATABASE_URL_FILE".to_owned(),
                container_secret("simulation_database_url"),
            );
            plan.env.insert(
                "QR_DATABASE_TLS_CA_PATH".to_owned(),
                DATABASE_CA_PATH.to_owned(),
            );
            insert_object_store(&mut plan);
            plan.secrets
                .push(secret("simulation_database_url", "simulation_database_url"));
            plan.configs
                .push(config(DATABASE_CA_CONFIG, DATABASE_CA_PATH));
            plan.configs
                .push(config(OBJECT_STORE_CA_CONFIG, OBJECT_STORE_CA_PATH));
        }
        other => {
            return Err(Error::rule(
                "deploy.render.service",
                format!("services.{other}"),
                "is not one of the seven services",
            ));
        }
    }
    Ok(plan)
}

fn common_env(descriptor: &Descriptor, service: &Service) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert(
        "QR_IMAGE_DIGEST".to_owned(),
        descriptor.image.digest.clone(),
    );
    env.insert("QR_PROFILE".to_owned(), "server".to_owned());
    env.insert("QR_INTERNAL_ONLY".to_owned(), "1".to_owned());
    env.insert("QR_BIND".to_owned(), format!("0.0.0.0:{}", service.port));
    env.insert(
        "QR_METRICS_BIND".to_owned(),
        format!("0.0.0.0:{METRICS_PORT}"),
    );
    env.insert("QR_LOG_FORMAT".to_owned(), "json".to_owned());
    env.insert("QR_QUEUE_GENERATION".to_owned(), "1".to_owned());
    env.insert(
        "QR_OBJECT_STORE_URL".to_owned(),
        format!(
            "{}://{}:{}",
            descriptor.endpoints.object_store.scheme,
            descriptor.endpoints.object_store.host,
            descriptor.endpoints.object_store.port
        ),
    );
    env.insert(
        "QR_OBJECT_STORE_TLS_CA_PATH".to_owned(),
        OBJECT_STORE_CA_PATH.to_owned(),
    );
    env.insert("QR_OBJECT_BUCKET".to_owned(), "quantrule".to_owned());
    env
}

fn insert_object_store(plan: &mut Plan) {
    plan.env.insert(
        "QR_OBJECT_STORE_ACCESS_KEY_FILE".to_owned(),
        container_secret("object_store_access_key"),
    );
    plan.env.insert(
        "QR_OBJECT_STORE_SECRET_KEY_FILE".to_owned(),
        container_secret("object_store_secret_key"),
    );
    plan.secrets
        .push(secret("object_store_access_key", "object_store_access_key"));
    plan.secrets
        .push(secret("object_store_secret_key", "object_store_secret_key"));
}

fn insert_queue(plan: &mut Plan) {
    plan.env.insert(
        "QR_QUEUE_URL_FILE".to_owned(),
        container_secret("queue_url"),
    );
    plan.env
        .insert("QR_QUEUE_TLS_CA_PATH".to_owned(), QUEUE_CA_PATH.to_owned());
}

fn service_url(descriptor: &Descriptor, name: &str) -> Result<String, Error> {
    let service = descriptor.services.get(name).ok_or_else(|| {
        Error::rule(
            "deploy.render.service",
            format!("services.{name}"),
            "the service is not declared",
        )
    })?;
    Ok(format!("http://{name}:{}", service.port))
}

fn container_secret(name: &str) -> String {
    format!("/run/secrets/{name}")
}

fn secret(source: &str, target: &str) -> Mount {
    Mount {
        source: source.to_owned(),
        target: target.to_owned(),
        mode: 0o400,
    }
}

fn config(source: &str, target: &str) -> Mount {
    Mount {
        source: source.to_owned(),
        target: target.to_owned(),
        mode: 0o444,
    }
}

fn service_yaml(descriptor: &Descriptor, plan: &Plan) -> Yaml {
    let mut service = Yaml::map()
        .with("image", Yaml::string(app_image(descriptor)))
        .with("command", Yaml::list([Yaml::string(plan.binary)]))
        .with(
            "environment",
            Yaml::mapping(
                plan.env
                    .iter()
                    .map(|(key, value)| (key.clone(), Yaml::string(value))),
            ),
        )
        .with(
            "networks",
            Yaml::list([Yaml::string(descriptor.network.overlay.as_str())]),
        )
        .with(
            "deploy",
            deploy(
                descriptor,
                &plan.host_label,
                plan.replicas,
                plan.max_per_node,
                Some(plan.kind),
            ),
        )
        .with("stop_grace_period", Yaml::string("40s"));
    if !plan.secrets.is_empty() {
        service = service.with("secrets", Yaml::list(plan.secrets.iter().map(mount_yaml)));
    }
    if !plan.configs.is_empty() {
        service = service.with("configs", Yaml::list(plan.configs.iter().map(mount_yaml)));
    }
    service
}

fn mount_yaml(mount: &Mount) -> Yaml {
    Yaml::map()
        .with("source", Yaml::string(mount.source.as_str()))
        .with("target", Yaml::string(mount.target.as_str()))
        .with("mode", Yaml::number(mount.mode))
}

fn ingress_yaml(descriptor: &Descriptor) -> Yaml {
    let mut configs = vec![config_mount(TRAEFIK_CONFIG, "/etc/traefik/traefik.yml")];
    if descriptor.ingress.tls == IngressTls::Cert {
        configs.push(config_mount(
            INGRESS_CERT_CONFIG,
            "/etc/quantrule/tls/tls.crt",
        ));
    }
    let mut service = Yaml::map()
        .with("image", Yaml::string(pinned(&descriptor.images["traefik"])))
        .with(
            "ports",
            Yaml::list([Yaml::map()
                .with("target", Yaml::number(443_u64))
                .with("published", Yaml::number(443_u64))
                .with("protocol", Yaml::string("tcp"))
                .with("mode", Yaml::string("host"))]),
        )
        .with(
            "networks",
            Yaml::list([Yaml::string(descriptor.network.overlay.as_str())]),
        )
        .with("configs", Yaml::list(configs))
        .with(
            "deploy",
            deploy(
                descriptor,
                "node.labels.quantrule.ingress == true",
                descriptor.ingress.replicas,
                1,
                Some("ingress"),
            ),
        )
        .with("stop_grace_period", Yaml::string("40s"));
    if descriptor.ingress.tls == IngressTls::Cert {
        service = service.with(
            "secrets",
            Yaml::list([mount_yaml(&secret(INGRESS_KEY_SECRET, INGRESS_KEY_SECRET))]),
        );
    }
    service
}

fn config_mount(source: &str, target: &str) -> Yaml {
    mount_yaml(&config(source, target))
}

fn deploy(
    descriptor: &Descriptor,
    host_label: &str,
    replicas: u32,
    max_per_node: u32,
    kind: Option<&str>,
) -> Yaml {
    let mut labels = BTreeMap::new();
    labels.insert(
        "quantrule.profile".to_owned(),
        descriptor.profile.as_str().to_owned(),
    );
    labels.insert(
        "quantrule.evidence".to_owned(),
        evidence_tier(descriptor.profile).to_owned(),
    );
    if let Some(kind) = kind {
        labels.insert("quantrule.kind".to_owned(), kind.to_owned());
    }
    Yaml::map()
        .with("replicas", Yaml::number(replicas))
        .with(
            "placement",
            Yaml::map()
                .with(
                    "constraints",
                    Yaml::list([
                        Yaml::string(host_label),
                        Yaml::string(format!(
                            "node.labels.quantrule.profile == {}",
                            descriptor.profile.as_str()
                        )),
                    ]),
                )
                .with("max_replicas_per_node", Yaml::number(max_per_node))
                .with(
                    "preferences",
                    Yaml::list([
                        Yaml::map().with("spread", Yaml::string("node.labels.quantrule.domain"))
                    ]),
                ),
        )
        .with(
            "update_config",
            Yaml::map()
                .with("order", Yaml::string("start-first"))
                .with("parallelism", Yaml::number(1_u64))
                .with("delay", Yaml::string("10s"))
                .with("monitor", Yaml::string("30s"))
                .with("failure_action", Yaml::string("rollback"))
                .with("max_failure_ratio", Yaml::number(0_u64)),
        )
        .with(
            "rollback_config",
            Yaml::map()
                .with("order", Yaml::string("start-first"))
                .with("parallelism", Yaml::number(1_u64))
                .with("delay", Yaml::string("5s"))
                .with("failure_action", Yaml::string("pause")),
        )
        .with(
            "restart_policy",
            Yaml::map()
                .with("condition", Yaml::string("on-failure"))
                .with("delay", Yaml::string("5s"))
                .with("max_attempts", Yaml::number(3_u64))
                .with("window", Yaml::string("120s")),
        )
        .with(
            "labels",
            Yaml::mapping(
                labels
                    .into_iter()
                    .map(|(key, value)| (key, Yaml::string(value))),
            ),
        )
}

/// One host labelled by the bootstrap script.
#[derive(Debug, Clone)]
pub struct NodeRow {
    /// The descriptor host key.
    pub host: String,
    /// The private address.
    pub address: String,
    /// The failure domain.
    pub domain: String,
    /// The services that may run on the host.
    pub services: Vec<&'static str>,
    /// Whether the ingress may run on the host.
    pub ingress: bool,
}

/// The host-to-service table the bootstrap script applies.
#[must_use]
pub fn node_rows(descriptor: &Descriptor) -> Vec<NodeRow> {
    descriptor
        .hosts
        .iter()
        .map(|(host, value)| NodeRow {
            host: host.clone(),
            address: value.address.clone(),
            domain: value.failure_domain.clone(),
            services: APP_SERVICES
                .iter()
                .filter(|(name, _)| {
                    descriptor
                        .services
                        .get(*name)
                        .is_some_and(|service| service.hosts.contains(host))
                })
                .map(|(name, _)| *name)
                .collect(),
            ingress: descriptor.ingress.hosts.contains(host),
        })
        .collect()
}

fn bootstrap_script(descriptor: &Descriptor) -> String {
    let overlay = &descriptor.network.overlay;
    let stack = deployment_name(descriptor);
    let profile = descriptor.profile.as_str();
    let app_image = app_image(descriptor);
    let mut nodes = String::new();
    for row in node_rows(descriptor) {
        let services = if row.services.is_empty() {
            "-".to_owned()
        } else {
            row.services.join(",")
        };
        let ingress = if row.ingress { "1" } else { "0" };
        nodes.push_str(&format!(
            "{} {} {} {} {}\n",
            row.host, row.address, row.domain, services, ingress
        ));
    }
    let mut secret_names: Vec<&String> = descriptor.secrets.files.keys().collect();
    secret_names.sort();
    let secrets = secret_names
        .iter()
        .map(|name| name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let mut config_creates = String::from(
        "create_config quantrule_database_ca pki/database-ca.crt\n\
         create_config quantrule_queue_ca pki/queue-ca.crt\n\
         create_config quantrule_object_store_ca pki/object-store-ca.crt\n",
    );
    let mut ingress_check = String::new();
    if descriptor.ingress.tls == IngressTls::Cert {
        config_creates.push_str("create_config quantrule_ingress_cert pki/ingress.crt\n");
        config_creates.push_str("create_secret quantrule_ingress_key pki/ingress.key\n");
    }
    ingress_check.push_str(
        "docker config inspect quantrule_traefik >/dev/null 2>&1 || \\\n    fail \"PLAT-002 must create the Traefik config quantrule_traefik before deploy\"\n",
    );
    let replicas = APP_SERVICES
        .iter()
        .map(|(name, _)| {
            let count = descriptor
                .services
                .get(*name)
                .map_or(0, |service| service.replicas);
            format!("{name}:{count}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let host_counts = APP_SERVICES
        .iter()
        .map(|(name, _)| {
            let count = descriptor
                .services
                .get(*name)
                .map_or(0, |service| service.hosts.len());
            format!("{name}:{count}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    let domain_min = match descriptor.profile {
        Profile::SwarmHa => "replicas",
        Profile::SingleAz => "replicas",
        Profile::LabSingleHost => "1",
    };
    format!(
        r#"#!/bin/sh
# Generated by qr-deploy; run on a Swarm manager.
#
# The script creates only the external objects HA-012 owns. Traefik's file
# provider is PLAT-002; Patroni, RabbitMQ, MinIO and observability files are
# PLAT-003..006. No secret value is read, printed, or passed as an argument.
set -eu
OVERLAY={overlay}
STACK={stack}
PROFILE={profile}
EXPECTED_IMAGE={image}
REPLICAS="{replicas}"
HOST_COUNTS="{host_counts}"
DOMAIN_MIN={domain_min}

fail() {{
    printf 'qr-deploy bootstrap: %s\n' "$*" >&2
    exit 1
}}

command -v docker >/dev/null 2>&1 || fail "docker is required on the Swarm manager"
docker info >/dev/null 2>&1 || fail "cannot reach the Docker daemon"
docker node ls >/dev/null 2>&1 || fail "run this script on a Swarm manager"

if docker network inspect "$OVERLAY" >/dev/null 2>&1; then
    [ "$(docker network inspect --format '{{{{.Driver}}}}' "$OVERLAY")" = "overlay" ] || \
        fail "network $OVERLAY exists but is not an overlay"
    case "$(docker network inspect --format '{{{{json .Options}}}}' "$OVERLAY")" in
        *'"encrypted":"true"'*) ;;
        *) fail "network $OVERLAY must be encrypted" ;;
    esac
    [ "$(docker network inspect --format '{{{{.Attachable}}}}' "$OVERLAY")" = "true" ] || \
        fail "network $OVERLAY must be attachable"
else
    docker network create --driver overlay --attachable --opt encrypted "$OVERLAY" >/dev/null
fi

create_secret() {{
    name=$1
    file=$2
    if docker secret inspect "$name" >/dev/null 2>&1; then
        return 0
    fi
    [ -f "$file" ] || fail "missing rendered secret file $file"
    docker secret create "$name" "$file" >/dev/null
}}

create_config() {{
    name=$1
    file=$2
    if docker config inspect "$name" >/dev/null 2>&1; then
        return 0
    fi
    [ -f "$file" ] || fail "missing rendered config file $file"
    docker config create "$name" "$file" >/dev/null
}}

resolve_node() {{
    name=$1
    address=$2
    if docker node inspect "$name" >/dev/null 2>&1; then
        printf '%s' "$name"
        return 0
    fi
    for node in $(docker node ls -q); do
        if [ "$(docker node inspect --format '{{{{.Status.Addr}}}}' "$node")" = "$address" ]; then
            printf '%s' "$node"
            return 0
        fi
    done
    fail "cannot map host $name ($address) to a Swarm node; label it manually"
}}

label_host() {{
    name=$1
    address=$2
    domain=$3
    services=$4
    ingress=$5
    [ "$services" = "-" ] && services=""
    node=$(resolve_node "$name" "$address")
    docker node update --label-add "quantrule.host=$name" \
        --label-add "quantrule.domain=$domain" \
        --label-add "quantrule.profile=$PROFILE" "$node" >/dev/null
    for service in gateway data model strategy backtest evidence simulation; do
        docker node update --label-rm "quantrule.$service" "$node" >/dev/null 2>&1 || true
    done
    docker node update --label-rm quantrule.ingress "$node" >/dev/null 2>&1 || true
    old_ifs=$IFS
    IFS=,
    for service in $services; do
        [ -n "$service" ] || continue
        docker node update --label-add "quantrule.$service=true" "$node" >/dev/null
    done
    IFS=$old_ifs
    if [ "$ingress" = "1" ]; then
        docker node update --label-add quantrule.ingress=true "$node" >/dev/null
    fi
}}

count_label() {{
    docker node ls --filter "node.label=$1=true" -q | wc -l | tr -d ' '
}}

domain_count() {{
    for node in $(docker node ls --filter "node.label=$1=true" -q); do
        docker node inspect --format '{{{{index .Spec.Labels "quantrule.domain"}}}}' "$node"
    done | sort -u | wc -l | tr -d ' '
}}

check_placement() {{
    for entry in $HOST_COUNTS; do
        service=${{entry%%:*}}
        wanted=${{entry##*:}}
        have=$(count_label "quantrule.$service")
        [ "$have" -eq "$wanted" ] || \
            fail "service $service is labelled on $have hosts; the descriptor names $wanted"
    done
    for entry in $REPLICAS; do
        service=${{entry%%:*}}
        wanted=${{entry##*:}}
        have=$(count_label "quantrule.$service")
        if [ "$DOMAIN_MIN" = "replicas" ]; then
            [ "$have" -ge "$wanted" ] || \
                fail "service $service needs $wanted labelled hosts; found $have"
            domains=$(domain_count "quantrule.$service")
            [ "$domains" -ge "$wanted" ] || \
                fail "service $service needs $wanted failure domains; found $domains"
        else
            [ "$have" -ge 1 ] || fail "service $service has no labelled host"
        fi
    done
    ingress_have=$(count_label quantrule.ingress)
    [ "$ingress_have" -eq {ingress_hosts} ] || \
        fail "ingress is labelled on $ingress_have hosts; the descriptor names {ingress_hosts}"
    if [ "$PROFILE" != "lab-single-host" ]; then
        ingress_domains=$(domain_count quantrule.ingress)
        [ "$ingress_domains" -ge 2 ] || \
            fail "the ingress needs two failure domains; found $ingress_domains"
    fi
}}

for name in {secrets}; do
    create_secret "$name" "secrets/$name"
done
{config_creates}{ingress_check}
label_nodes() {{
    while read -r host address domain services ingress; do
        [ -n "$host" ] || continue
        label_host "$host" "$address" "$domain" "$services" "$ingress"
    done <<'NODES'
{nodes}NODES
}}
label_nodes
check_placement

docker stack deploy --with-registry-auth -c stack.yaml "$STACK"
printf 'stack %s deployed; image %s; profile %s\n' "$STACK" "$EXPECTED_IMAGE" "$PROFILE"

attempt=1
while ! sh jobs/db-bootstrap.sh; do
    [ "$attempt" -lt 30 ] || fail "db-bootstrap did not converge"
    attempt=$((attempt + 1))
    sleep 2
done
sh jobs/migrate.sh
printf 'bootstrap complete: databases and migrations applied\n'
"#,
        overlay = shell_quote(overlay),
        stack = shell_quote(stack),
        profile = shell_quote(profile),
        image = shell_quote(&app_image),
        replicas = replicas,
        host_counts = host_counts,
        domain_min = domain_min,
        ingress_hosts = descriptor.ingress.hosts.len(),
        secrets = secrets,
        config_creates = config_creates,
        ingress_check = ingress_check,
        nodes = nodes,
    )
}

pub(crate) fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b'/' | b':' | b'@')
        })
    {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    #[test]
    fn the_stack_has_one_ingress_and_no_other_published_port() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert!(text.contains("published: 443"), "{text}");
        assert!(text.contains("mode: \"host\""), "{text}");
        assert!(!text.contains("published: 8080"), "{text}");
        assert!(!text.contains("published: 80"), "{text}");
        assert_eq!(text.matches("published:").count(), 1, "{text}");
    }

    #[test]
    fn the_stack_has_exactly_seven_services_and_one_ingress() {
        let text = stack_yaml(&descriptor()).expect("stack");
        for name in [
            "gateway",
            "data",
            "model",
            "strategy",
            "backtest",
            "evidence",
            "simulation",
        ] {
            assert_eq!(
                text.matches(&format!("\n  {name}:\n    command:")).count(),
                1,
                "{name}: {text}"
            );
        }
        assert_eq!(
            text.matches("\n  ingress:\n    configs:").count(),
            1,
            "{text}"
        );
    }

    #[test]
    fn every_service_and_image_is_pinned() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert!(text.contains("@sha256:"), "{text}");
        assert!(text.contains("QR_MIGRATE: \"0\""), "{text}");
        assert!(text.contains("stop_grace_period: \"40s\""), "{text}");
        assert!(text.contains("max_replicas_per_node: 1"), "{text}");
        assert!(text.contains("network_encrypted: true"), "{text}");
        assert!(text.contains("network_attachable: true"), "{text}");
    }

    #[test]
    fn the_bootstrap_script_never_contains_a_secret_value() {
        let script = bootstrap_script(&descriptor());
        assert!(script.contains("create_secret \"$name\" \"secrets/$name\""));
        assert!(!script.contains("postgres://"));
        assert!(!script.contains("amqps://"));
        let token = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../deploy/v3/secrets/gateway_token"),
        )
        .expect("fixture token");
        assert!(
            !script.contains(token.trim()),
            "the bootstrap script copied a token"
        );
    }
}
