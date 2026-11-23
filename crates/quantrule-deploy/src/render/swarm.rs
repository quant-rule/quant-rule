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

use super::ingress::{
    INGRESS_CERT_CONFIG, INGRESS_KEY_SECRET, TRAEFIK_CONFIG, TRAEFIK_DYNAMIC_CONFIG,
};
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

    let bootstrap = super::bootstrap::bootstrap_script(descriptor);
    let path = out.join("bootstrap.sh");
    write_script(&path, &bootstrap)?;
    written.push(path);

    let jobs = out.join("jobs");
    create_dir(&jobs)?;
    let path = jobs.join("cluster-bootstrap.sh");
    write_script(&path, &super::jobs::cluster_bootstrap(descriptor))?;
    written.push(path);
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
        if crate::validate::secrets::STATEFUL_SECRETS.contains(&name.as_str()) {
            continue;
        }
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
        TRAEFIK_DYNAMIC_CONFIG,
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
                "Traefik ingress configuration is PLAT-002; Patroni, etcd, and HAProxy are PLAT-003; RabbitMQ is PLAT-004; MinIO and observability remain PLAT-005/006",
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
    port: u16,
    host_label: String,
    host_count: usize,
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
        port: service.port,
        host_label: format!("node.labels.quantrule.{name} == true"),
        host_count: service.hosts.len(),
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
    let queue_consumer = matches!(plan.kind, "backtest" | "evidence");
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
                plan.host_count,
                Some(plan.kind),
                queue_consumer,
            ),
        )
        .with("stop_grace_period", Yaml::string("40s"))
        .with("healthcheck", healthcheck(plan.port));
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
    let mut configs = vec![
        config_mount(TRAEFIK_CONFIG, "/etc/traefik/traefik.yml"),
        config_mount(TRAEFIK_DYNAMIC_CONFIG, "/etc/traefik/dynamic/dynamic.yml"),
    ];
    if descriptor.ingress.tls == IngressTls::Cert {
        configs.push(config_mount(
            INGRESS_CERT_CONFIG,
            "/etc/traefik/certs/ingress.crt",
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
                descriptor.ingress.hosts.len(),
                Some("ingress"),
                false,
            ),
        )
        .with("stop_grace_period", Yaml::string("40s"));
    if descriptor.ingress.tls == IngressTls::Cert {
        service = service.with(
            "secrets",
            Yaml::list([mount_yaml(&secret(
                INGRESS_KEY_SECRET,
                "/etc/traefik/certs/ingress.key",
            ))]),
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
    host_count: usize,
    kind: Option<&str>,
    restart_any: bool,
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
    let order = update_order(replicas, host_count, max_per_node);
    let restart_policy = if restart_any {
        // A queue consumer must outlive a broker restart even when its bounded
        // in-process budget is exhausted; Swarm restarts it without a cap.
        Yaml::map()
            .with("condition", Yaml::string("any"))
            .with("delay", Yaml::string("5s"))
    } else {
        Yaml::map()
            .with("condition", Yaml::string("on-failure"))
            .with("delay", Yaml::string("5s"))
            .with("max_attempts", Yaml::number(3_u64))
            .with("window", Yaml::string("120s"))
    };
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
                        Yaml::string(format!(
                            "node.labels.quantrule.deploy == {}",
                            deployment_name(descriptor)
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
                .with("order", Yaml::string(order))
                .with("parallelism", Yaml::number(1_u64))
                .with("delay", Yaml::string("10s"))
                .with("monitor", Yaml::string("180s"))
                .with("failure_action", Yaml::string("rollback"))
                .with("max_failure_ratio", Yaml::number(0_u64)),
        )
        .with(
            "rollback_config",
            Yaml::map()
                .with("order", Yaml::string(order))
                .with("parallelism", Yaml::number(1_u64))
                .with("delay", Yaml::string("5s"))
                .with("monitor", Yaml::string("180s"))
                .with("failure_action", Yaml::string("pause"))
                .with("max_failure_ratio", Yaml::number(0_u64)),
        )
        .with("restart_policy", restart_policy)
        .with(
            "labels",
            Yaml::mapping(
                labels
                    .into_iter()
                    .map(|(key, value)| (key, Yaml::string(value))),
            ),
        )
}

/// The rolling-update order: start-first only when a spare eligible slot exists.
///
/// A service that fills every eligible host (`replicas == hosts`) has no slot
/// for a new task, so a start-first update would wait forever. The stop-first
/// update accepts a short gap and is the only safe default for the reference
/// topology; the lab profile's co-location creates spare slots.
fn update_order(replicas: u32, host_count: usize, max_per_node: u32) -> &'static str {
    let slots = u64::try_from(host_count)
        .unwrap_or(u64::MAX)
        .saturating_mul(u64::from(max_per_node));
    if u64::from(replicas) < slots {
        "start-first"
    } else {
        "stop-first"
    }
}

fn healthcheck(port: u16) -> Yaml {
    Yaml::map()
        .with(
            "test",
            Yaml::list([
                Yaml::string("CMD-SHELL"),
                Yaml::string(format!(
                    "curl -fsS http://127.0.0.1:{port}/health | grep -q '\"state\":\"healthy\"'"
                )),
            ]),
        )
        .with("interval", Yaml::string("15s"))
        .with("timeout", Yaml::string("5s"))
        .with("retries", Yaml::number(6_u64))
        .with("start_period", Yaml::string("60s"))
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
    fn the_ingress_mounts_the_static_dynamic_and_cert_objects() {
        let text = stack_yaml(&descriptor()).expect("stack");
        for expected in [
            "source: \"quantrule_traefik\"",
            "target: \"/etc/traefik/traefik.yml\"",
            "source: \"quantrule_traefik_dynamic\"",
            "target: \"/etc/traefik/dynamic/dynamic.yml\"",
            "source: \"quantrule_ingress_cert\"",
            "target: \"/etc/traefik/certs/ingress.crt\"",
            "source: \"quantrule_ingress_key\"",
            "target: \"/etc/traefik/certs/ingress.key\"",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("/etc/quantrule/tls"), "{text}");
        assert!(!text.contains("published: 80"), "{text}");
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
    fn every_http_service_has_a_healthcheck_on_its_own_listener() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert_eq!(text.matches("healthcheck:").count(), 7, "{text}");
        assert_eq!(
            text.matches("curl -fsS http://127.0.0.1:8080/health")
                .count(),
            6,
            "{text}"
        );
        assert_eq!(
            text.matches("curl -fsS http://127.0.0.1:9090/health")
                .count(),
            1,
            "{text}"
        );
        assert_eq!(text.matches("start_period: \"60s\"").count(), 7, "{text}");
        assert_eq!(text.matches("monitor: \"180s\"").count(), 16, "{text}");
    }

    #[test]
    fn the_queue_consumers_restart_without_a_max_attempt() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert_eq!(text.matches("condition: \"any\"").count(), 2, "{text}");
        assert_eq!(
            text.matches("condition: \"on-failure\"").count(),
            6,
            "{text}"
        );
        assert_eq!(text.matches("max_attempts:").count(), 6, "{text}");
        for (start, end) in [
            ("\n  backtest:\n", "\n  data:\n"),
            ("\n  evidence:\n", "\n  gateway:\n"),
        ] {
            let from = text.find(start).expect("consumer start");
            let to = text.find(end).expect("consumer end");
            let block = &text[from..to];
            assert!(block.contains("condition: \"any\""), "{block}");
            assert!(!block.contains("max_attempts"), "{block}");
            assert!(!block.contains("window:"), "{block}");
        }
        let gateway = text.find("\n  gateway:\n").expect("gateway");
        let ingress = text.find("\n  ingress:\n").expect("ingress");
        assert!(
            text[gateway..ingress].contains("condition: \"on-failure\""),
            "{}",
            &text[gateway..ingress]
        );
    }

    #[test]
    fn the_reference_topology_uses_stop_first_without_a_spare_slot() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert_eq!(text.matches("order: \"stop-first\"").count(), 16, "{text}");
        assert!(!text.contains("order: \"start-first\""), "{text}");
    }

    #[test]
    fn the_lab_uses_start_first_when_co_location_leaves_a_spare_slot() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                .expect("example");
        value["profile"] = serde_json::json!("lab-single-host");
        let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
        let text = stack_yaml(&descriptor).expect("stack");
        assert!(text.contains("order: \"start-first\""), "{text}");
        // The ingress still fills both host-mode 443 slots, so it stays stop-first.
        assert_eq!(text.matches("order: \"stop-first\"").count(), 2, "{text}");
    }

    #[test]
    fn placement_is_scoped_to_the_deployment_and_profile() {
        let text = stack_yaml(&descriptor()).expect("stack");
        assert!(
            text.contains("node.labels.quantrule.deploy == quantrule-ha"),
            "{text}"
        );
        assert!(
            text.contains("node.labels.quantrule.profile == swarm-ha"),
            "{text}"
        );
    }
}
