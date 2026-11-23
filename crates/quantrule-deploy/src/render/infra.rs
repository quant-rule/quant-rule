//! Pinned per-host Compose units for the stateful and observability tiers.
//!
//! The descriptor places the units; this renderer writes one file per host. The
//! Patroni, etcd, and HAProxy units carry their PLAT-003 commands, config, PKI,
//! and secret mounts; their configuration bodies live in [`super::stateful`].
//! The RabbitMQ unit carries its PLAT-004 config, plugin list, cookie, PKI, and
//! data/definitions directories; its configuration body lives in
//! [`super::queue`]. MinIO, Prometheus, Grafana, Loki, and Alertmanager remain
//! pinned skeletons for PLAT-005/006. Every unit attaches to the encrypted
//! overlay, publishes no port, and keeps its data on a host-local path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::{Descriptor, Watchdog};
use crate::error::Error;
use crate::validate::secrets::STATEFUL_SECRETS;

use super::queue::{
    RABBITMQ_CONFIG_TARGET, RABBITMQ_COOKIE_TARGET, RABBITMQ_DATA_PKI, RABBITMQ_DATA_TARGET,
    RABBITMQ_DEFINITIONS_TARGET, RABBITMQ_PKI_TARGET, RABBITMQ_PLUGINS_TARGET,
};
use super::stateful::{ETCD_CONFIG_TARGET, HAPROXY_CONFIG_TARGET, PATRONI_CONFIG_TARGET};
use super::yaml::Yaml;
use super::{create_dir, evidence_tier, pinned, safe_component, write_public};

/// The Patroni supervisor command. The four password files are read before exec
/// and passed to Patroni through its documented `PATRONI_*` environment; no
/// password enters the rendered YAML.
const PATRONI_COMMAND: &str = "set -eu\n\
umask 077\n\
export PATRONI_SUPERUSER_PASSWORD=\"$$(cat /run/secrets/postgres_superuser_password)\"\n\
export PATRONI_REPLICATION_PASSWORD=\"$$(cat /run/secrets/patroni_replication_password)\"\n\
export PATRONI_REWIND_PASSWORD=\"$$(cat /run/secrets/patroni_rewind_password)\"\n\
export PATRONI_RESTAPI_PASSWORD=\"$$(cat /run/secrets/patroni_rest_password)\"\n\
exec patroni /etc/patroni/patroni.yml";

/// The HAProxy command. It expands the REST basic-auth header into a runtime
/// copy of the configuration; the rendered file carries only a placeholder.
const HAPROXY_COMMAND: &str = "set -eu\n\
umask 077\n\
auth=\"patroni:$$(cat /run/secrets/patroni_rest_password)\"\n\
encoded=$$(printf '%s' \"$$auth\" | base64 | tr -d '\\n')\n\
sed \"s|__PATRONI_REST_AUTH__|$$encoded|g\" /usr/local/etc/haproxy/haproxy.cfg > /tmp/haproxy.cfg\n\
exec haproxy -W -db -f /tmp/haproxy.cfg";

/// Write `infra/<host>.yaml` for every host that owns a unit.
pub fn render(
    descriptor: &Descriptor,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    let directory = out.join("infra");
    create_dir(&directory)?;
    for (host, units) in host_units(descriptor) {
        safe_component(&host, "hosts")?;
        let text = host_yaml(descriptor, &host, &units);
        let path = directory.join(format!("{host}.yaml"));
        write_public(&path, text.as_bytes())?;
        written.push(path);
    }
    Ok(())
}

#[derive(Debug)]
struct Unit {
    name: String,
    image: String,
    container_name: String,
    hostname: Option<String>,
    environment: BTreeMap<String, String>,
    user: Option<String>,
    volumes: Vec<Volume>,
    aliases: Vec<String>,
    host_network: bool,
    cap_add: Vec<&'static str>,
    command: Vec<String>,
    devices: Vec<String>,
}

#[derive(Debug)]
struct Volume {
    source: String,
    target: String,
    read_only: bool,
}

fn volume(source: impl Into<String>, target: impl Into<String>, read_only: bool) -> Volume {
    Volume {
        source: source.into(),
        target: target.into(),
        read_only,
    }
}

fn unit(
    descriptor: &Descriptor,
    name: &str,
    image_key: &str,
    host: &str,
    volumes: Vec<Volume>,
    aliases: Vec<String>,
) -> Unit {
    Unit {
        name: name.to_owned(),
        image: pinned(&descriptor.images[image_key]),
        container_name: format!("quantrule-{name}-{host}"),
        hostname: None,
        environment: BTreeMap::new(),
        user: None,
        volumes,
        aliases,
        host_network: false,
        cap_add: Vec::new(),
        command: Vec::new(),
        devices: Vec::new(),
    }
}

/// A host-network unit with the two capabilities keepalived needs.
fn host_unit(
    descriptor: &Descriptor,
    name: &str,
    image_key: &str,
    host: &str,
    volumes: Vec<Volume>,
) -> Unit {
    Unit {
        name: name.to_owned(),
        image: pinned(&descriptor.images[image_key]),
        container_name: format!("quantrule-{name}-{host}"),
        hostname: None,
        environment: BTreeMap::new(),
        user: None,
        volumes,
        aliases: Vec::new(),
        host_network: true,
        cap_add: vec!["NET_ADMIN", "NET_BROADCAST"],
        command: Vec::new(),
        devices: Vec::new(),
    }
}

/// The units assigned to each host.
fn host_units(descriptor: &Descriptor) -> BTreeMap<String, Vec<Unit>> {
    let mut hosts: BTreeMap<String, Vec<Unit>> = BTreeMap::new();
    for host in &descriptor.stateful.postgres.nodes {
        let mut volumes = vec![
            volume(
                "/srv/quantrule/patroni/data",
                "/var/lib/postgresql/data",
                false,
            ),
            volume(
                "/srv/quantrule/patroni/wal-archive",
                "/srv/quantrule/wal-archive",
                false,
            ),
            volume(
                "/etc/quantrule/patroni/patroni.yml",
                PATRONI_CONFIG_TARGET,
                true,
            ),
            volume(
                "/etc/quantrule/pki/ca.crt",
                "/etc/quantrule/pki/ca.crt",
                true,
            ),
            volume(
                "/etc/quantrule/pki/postgres",
                "/etc/quantrule/pki/postgres",
                true,
            ),
            volume("/etc/quantrule/pki/etcd", "/etc/quantrule/pki/etcd", true),
        ];
        for secret in STATEFUL_SECRETS {
            volumes.push(volume(
                format!("/etc/quantrule/secrets/{secret}"),
                format!("/run/secrets/{secret}"),
                true,
            ));
        }
        let mut patroni = unit(
            descriptor,
            "patroni",
            "patroni",
            host,
            volumes,
            vec![format!("patroni-{host}")],
        );
        patroni.command = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            PATRONI_COMMAND.to_owned(),
        ];
        if descriptor.stateful.postgres.watchdog == Watchdog::Required {
            patroni
                .devices
                .push("/dev/watchdog:/dev/watchdog".to_owned());
        }
        hosts.entry(host.clone()).or_default().push(patroni);
    }
    for host in &descriptor.stateful.etcd.nodes {
        let mut etcd = unit(
            descriptor,
            "etcd",
            "etcd",
            host,
            vec![
                volume("/srv/quantrule/etcd/data", "/etcd-data", false),
                volume(
                    "/etc/quantrule/etcd/etcd.conf.yml",
                    ETCD_CONFIG_TARGET,
                    true,
                ),
                volume(
                    "/etc/quantrule/pki/ca.crt",
                    "/etc/quantrule/pki/ca.crt",
                    true,
                ),
                volume("/etc/quantrule/pki/etcd", "/etc/quantrule/pki/etcd", true),
            ],
            vec![format!("etcd-{host}")],
        );
        etcd.command = vec![
            "etcd".to_owned(),
            "--config-file".to_owned(),
            ETCD_CONFIG_TARGET.to_owned(),
        ];
        hosts.entry(host.clone()).or_default().push(etcd);
    }
    let database_host = descriptor.endpoints.database.host.clone();
    let queue_host = descriptor.endpoints.queue.host.clone();
    for host in &descriptor.stateful.haproxy.nodes {
        let mut haproxy = unit(
            descriptor,
            "haproxy",
            "haproxy",
            host,
            vec![
                volume(
                    "/etc/quantrule/haproxy/haproxy.cfg",
                    HAPROXY_CONFIG_TARGET,
                    true,
                ),
                volume(
                    "/etc/quantrule/secrets/patroni_rest_password",
                    "/run/secrets/patroni_rest_password",
                    true,
                ),
            ],
            vec![
                format!("haproxy-{host}"),
                database_host.clone(),
                queue_host.clone(),
            ],
        );
        haproxy.command = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            HAPROXY_COMMAND.to_owned(),
        ];
        hosts.entry(host.clone()).or_default().push(haproxy);
    }
    for host in &descriptor.ingress.hosts {
        hosts.entry(host.clone()).or_default().push(host_unit(
            descriptor,
            "keepalived",
            "keepalived",
            host,
            vec![volume(
                "/etc/quantrule/keepalived.conf",
                "/etc/keepalived/keepalived.conf",
                true,
            )],
        ));
    }
    for host in &descriptor.stateful.rabbitmq.nodes {
        let service = format!("rabbitmq-broker-{host}");
        let mut rabbitmq = unit(
            descriptor,
            &service,
            "rabbitmq",
            host,
            vec![
                volume("/srv/quantrule/rabbitmq/data", RABBITMQ_DATA_TARGET, false),
                volume(
                    "/srv/quantrule/rabbitmq/definitions",
                    RABBITMQ_DEFINITIONS_TARGET,
                    false,
                ),
                volume(
                    "/srv/quantrule/rabbitmq/conf.d",
                    "/etc/rabbitmq/conf.d",
                    true,
                ),
                volume(
                    "/etc/quantrule/rabbitmq/rabbitmq.conf",
                    RABBITMQ_CONFIG_TARGET,
                    true,
                ),
                volume(
                    "/etc/quantrule/rabbitmq/enabled_plugins",
                    RABBITMQ_PLUGINS_TARGET,
                    true,
                ),
                volume(
                    "/etc/quantrule/rabbitmq/erlang.cookie",
                    RABBITMQ_COOKIE_TARGET,
                    true,
                ),
                volume("/etc/quantrule/pki/rabbitmq", RABBITMQ_PKI_TARGET, true),
            ],
            vec![format!("rabbitmq-{host}")],
        );
        // The Compose service key must not be `rabbitmq`: Compose adds the
        // service name as an implicit network alias, which would put the shared
        // front-door name on a broker. It is unique per host for the same
        // reason. The container name stays stable for the operator tool.
        rabbitmq.container_name = format!("quantrule-rabbitmq-{host}");
        rabbitmq.hostname = Some(format!("rabbitmq-{host}"));
        rabbitmq.environment.insert(
            "RABBITMQ_NODENAME".to_owned(),
            format!("rabbit@rabbitmq-{host}"),
        );
        rabbitmq.environment.insert(
            "RABBITMQ_CONFIG_FILE".to_owned(),
            format!("{RABBITMQ_DATA_TARGET}/rabbitmq.conf"),
        );
        rabbitmq.environment.insert(
            "RABBITMQ_ENABLED_PLUGINS_FILE".to_owned(),
            format!("{RABBITMQ_DATA_TARGET}/enabled_plugins"),
        );
        // The official image runs as the rabbitmq user. The unit starts as root
        // only to install the cookie and the PKI copy, then drops to rabbitmq.
        rabbitmq.user = Some("0:0".to_owned());
        rabbitmq.command = vec![
            "/bin/sh".to_owned(),
            "-c".to_owned(),
            format!(
                "set -eu\n\
                 umask 077\n\
                 install -d -o rabbitmq -g rabbitmq -m 700 {RABBITMQ_DATA_PKI}\n\
                 install -o rabbitmq -g rabbitmq -m 600 {RABBITMQ_COOKIE_TARGET} {RABBITMQ_DATA_TARGET}/.erlang.cookie\n\
                 install -o rabbitmq -g rabbitmq -m 600 /etc/quantrule/rabbitmq/rabbitmq.conf {RABBITMQ_DATA_TARGET}/rabbitmq.conf\n\
                 install -o rabbitmq -g rabbitmq -m 600 /etc/quantrule/rabbitmq/enabled_plugins {RABBITMQ_DATA_TARGET}/enabled_plugins\n\
                 install -o rabbitmq -g rabbitmq -m 644 {RABBITMQ_PKI_TARGET}/ca.crt {RABBITMQ_DATA_PKI}/ca.crt\n\
                 install -o rabbitmq -g rabbitmq -m 644 {RABBITMQ_PKI_TARGET}/server.crt {RABBITMQ_DATA_PKI}/server.crt\n\
                 install -o rabbitmq -g rabbitmq -m 600 {RABBITMQ_PKI_TARGET}/server.key {RABBITMQ_DATA_PKI}/server.key\n\
                 chown -R rabbitmq:rabbitmq {RABBITMQ_DATA_TARGET} {RABBITMQ_DEFINITIONS_TARGET}\n\
                 export RABBITMQ_CONFIG_FILE={RABBITMQ_DATA_TARGET}/rabbitmq.conf\n\
                 export RABBITMQ_ENABLED_PLUGINS_FILE={RABBITMQ_DATA_TARGET}/enabled_plugins\n\
                 exec gosu rabbitmq rabbitmq-server"
            ),
        ];
        hosts.entry(host.clone()).or_default().push(rabbitmq);
    }
    let object_host = descriptor.endpoints.object_store.host.clone();
    for (index, host) in descriptor.stateful.minio.nodes.iter().enumerate() {
        let drives = descriptor
            .stateful
            .minio
            .drives
            .get(index)
            .copied()
            .unwrap_or(0);
        let mut volumes = Vec::new();
        for drive in 1..=drives {
            volumes.push(volume(
                format!("/srv/quantrule/minio/{host}/drive{drive}"),
                format!("/data{drive}"),
                false,
            ));
        }
        volumes.push(volume(
            "/etc/quantrule/minio/minio.conf",
            "/etc/minio/minio.conf",
            true,
        ));
        volumes.push(volume(
            "/etc/quantrule/pki/minio",
            "/etc/quantrule/pki",
            true,
        ));
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            "minio",
            "minio",
            host,
            volumes,
            vec![object_host.clone()],
        ));
    }

    for (unit_name, image_key, host, volumes) in [
        (
            "prometheus",
            "prometheus",
            &descriptor.observability.prometheus.host,
            vec![
                volume("/srv/quantrule/prometheus/data", "/prometheus", false),
                volume(
                    "/etc/quantrule/prometheus/prometheus.yml",
                    "/etc/prometheus/prometheus.yml",
                    true,
                ),
            ],
        ),
        (
            "grafana",
            "grafana",
            &descriptor.observability.grafana.host,
            vec![
                volume("/srv/quantrule/grafana/data", "/var/lib/grafana", false),
                volume("/etc/quantrule/grafana", "/etc/grafana/provisioning", true),
            ],
        ),
        (
            "loki",
            "loki",
            &descriptor.observability.loki.host,
            vec![
                volume("/srv/quantrule/loki/data", "/loki", false),
                volume("/etc/quantrule/loki/loki.yml", "/etc/loki/loki.yml", true),
            ],
        ),
        (
            "alertmanager",
            "alertmanager",
            &descriptor.observability.alertmanager.host,
            vec![
                volume("/srv/quantrule/alertmanager/data", "/alertmanager", false),
                volume(
                    "/etc/quantrule/alertmanager/alertmanager.yml",
                    "/etc/alertmanager/alertmanager.yml",
                    true,
                ),
            ],
        ),
    ] {
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            unit_name,
            image_key,
            host,
            volumes,
            Vec::new(),
        ));
    }
    hosts.retain(|_, units| !units.is_empty());
    hosts
}

/// The rendered Compose unit for one host, or `None` when the host owns no unit.
///
/// The check reads the same in-memory text the renderer writes, so a row can
/// prove the `postgres` alias without a rendered tree on disk.
pub(crate) fn unit_yaml(descriptor: &Descriptor, host: &str) -> Option<String> {
    let units = host_units(descriptor);
    units
        .get(host)
        .map(|units| host_yaml(descriptor, host, units))
}

fn host_yaml(descriptor: &Descriptor, host: &str, units: &[Unit]) -> String {
    let mut services = BTreeMap::new();
    for value in units {
        let mut service = Yaml::map()
            .with("image", Yaml::string(value.image.as_str()))
            .with(
                "container_name",
                Yaml::string(value.container_name.as_str()),
            )
            .with("restart", Yaml::string("unless-stopped"))
            .with("stop_grace_period", Yaml::string("40s"));
        if let Some(hostname) = &value.hostname {
            service = service.with("hostname", Yaml::string(hostname.as_str()));
        }
        if let Some(user) = &value.user {
            service = service.with("user", Yaml::string(user.as_str()));
        }
        if !value.environment.is_empty() {
            service = service.with(
                "environment",
                Yaml::mapping(
                    value
                        .environment
                        .iter()
                        .map(|(key, value)| (key.clone(), Yaml::string(value))),
                ),
            );
        }
        if value.host_network {
            service = service.with("network_mode", Yaml::string("host"));
        }
        if !value.cap_add.is_empty() {
            service = service.with(
                "cap_add",
                Yaml::list(value.cap_add.iter().map(|cap| Yaml::string(*cap))),
            );
        }
        if !value.command.is_empty() {
            service = service.with(
                "command",
                Yaml::list(value.command.iter().map(Yaml::string)),
            );
        }
        if !value.devices.is_empty() {
            service = service.with(
                "devices",
                Yaml::list(value.devices.iter().map(Yaml::string)),
            );
        }
        if !value.volumes.is_empty() {
            service = service.with(
                "volumes",
                Yaml::list(value.volumes.iter().map(|volume| {
                    Yaml::map()
                        .with("type", Yaml::string("bind"))
                        .with("source", Yaml::string(volume.source.as_str()))
                        .with("target", Yaml::string(volume.target.as_str()))
                        .with("read_only", Yaml::Bool(volume.read_only))
                })),
            );
        }
        if !value.host_network {
            if value.aliases.is_empty() {
                service = service.with(
                    "networks",
                    Yaml::list([Yaml::string(descriptor.network.overlay.as_str())]),
                );
            } else {
                service = service.with(
                    "networks",
                    Yaml::mapping([(
                        descriptor.network.overlay.clone(),
                        Yaml::map().with(
                            "aliases",
                            Yaml::list(value.aliases.iter().map(Yaml::string)),
                        ),
                    )]),
                );
            }
        }
        services.insert(value.name.clone(), service);
    }
    let root = Yaml::map()
        .with(
            "name",
            Yaml::string(format!("quantrule-infra-{host}")),
        )
        .with(
            "x-quantrule",
            Yaml::map()
                .with("owner", Yaml::string("PLAT-003/PLAT-004"))
                .with("profile", Yaml::string(descriptor.profile.as_str()))
                .with("evidence", Yaml::string(evidence_tier(descriptor.profile)))
                .with(
                    "network",
                    Yaml::string("overlay, attachable, encrypted; created by bootstrap.sh"),
                )
                .with(
                    "platform",
                    Yaml::string(
                        "keepalived is PLAT-002; Patroni, etcd, and HAProxy are configured by PLAT-003; RabbitMQ is PLAT-004; MinIO and observability remain PLAT-005/006",
                    ),
                ),
        )
        .with("services", Yaml::Map(services))
        .with(
            "networks",
            Yaml::mapping([(
                descriptor.network.overlay.clone(),
                external(&descriptor.network.overlay),
            )]),
        );
    root.render()
}

fn external(name: &str) -> Yaml {
    Yaml::map()
        .with("external", Yaml::Bool(true))
        .with("name", Yaml::string(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    #[test]
    fn every_infra_host_has_no_published_port_and_pins_its_image() {
        let units = host_units(&descriptor());
        for (host, values) in &units {
            let text = host_yaml(&descriptor(), host, values);
            assert!(!text.contains("ports:"), "{host}: {text}");
            assert!(text.contains("@sha256:"), "{host}: {text}");
            assert!(text.contains("restart: \"unless-stopped\""), "{host}");
            assert!(text.contains("external: true"), "{host}");
            assert!(text.contains("overlay, attachable, encrypted"), "{host}");
        }
    }

    #[test]
    fn the_edge_hosts_run_keepalived_with_host_networking() {
        let units = host_units(&descriptor());
        for host in ["edge-1", "edge-2"] {
            let text = host_yaml(&descriptor(), host, &units[host]);
            assert!(text.contains("\n  keepalived:\n"), "{host}: {text}");
            assert!(text.contains("network_mode: \"host\""), "{host}: {text}");
            assert!(text.contains("NET_ADMIN"), "{host}: {text}");
            assert!(text.contains("NET_BROADCAST"), "{host}: {text}");
            assert!(
                text.contains("/etc/quantrule/keepalived.conf"),
                "{host}: {text}"
            );
            assert!(!text.contains("ports:"), "{host}: {text}");
            assert!(!text.contains("deploy:"), "{host}: {text}");
        }
        let core = host_yaml(&descriptor(), "core-1", &[]);
        assert!(!core.contains("keepalived:"), "{core}");
    }

    #[test]
    fn the_stateful_units_use_the_new_images_commands_and_aliases() {
        let descriptor = descriptor();
        let units = host_units(&descriptor);
        let patroni = host_yaml(&descriptor, "infra-1", &units["infra-1"]);
        assert!(patroni.contains("\n  patroni:\n"), "{patroni}");
        assert!(patroni.contains("patroni/patroni@sha256:"), "{patroni}");
        assert!(!patroni.contains("postgres@sha256:"), "{patroni}");
        assert!(patroni.contains("\"patroni-infra-1\""), "{patroni}");
        assert!(
            patroni.contains("/etc/quantrule/patroni/patroni.yml"),
            "{patroni}"
        );
        assert!(
            patroni.contains("/run/secrets/patroni_rest_password"),
            "{patroni}"
        );
        assert!(patroni.contains("PATRONI_RESTAPI_PASSWORD"), "{patroni}");
        assert!(!patroni.contains("/dev/watchdog"), "{patroni}");
        assert!(!patroni.contains("devices:"), "{patroni}");

        let etcd = host_yaml(&descriptor, "infra-1", &units["infra-1"]);
        assert!(etcd.contains("\n  etcd:\n"), "{etcd}");
        assert!(etcd.contains("--config-file"), "{etcd}");
        assert!(etcd.contains("\"etcd-infra-1\""), "{etcd}");
        assert!(etcd.contains("/etc/quantrule/etcd/etcd.conf.yml"), "{etcd}");
        assert!(etcd.contains("/etc/quantrule/pki/etcd"), "{etcd}");

        let haproxy = host_yaml(&descriptor, "edge-1", &units["edge-1"]);
        assert!(haproxy.contains("\n  haproxy:\n"), "{haproxy}");
        assert!(haproxy.contains("\"haproxy-edge-1\""), "{haproxy}");
        assert!(haproxy.contains("\"postgres\""), "{haproxy}");
        assert!(haproxy.contains("\"rabbitmq\""), "{haproxy}");
        assert!(
            haproxy.contains("/run/secrets/patroni_rest_password"),
            "{haproxy}"
        );
        assert!(haproxy.contains("command:"), "{haproxy}");
        assert!(!haproxy.contains("ports:"), "{haproxy}");
    }

    #[test]
    fn the_rabbitmq_unit_uses_the_short_node_name_and_broker_only_alias() {
        let descriptor = descriptor();
        let units = host_units(&descriptor);
        let text = host_yaml(&descriptor, "infra-1", &units["infra-1"]);
        for expected in [
            "\n  rabbitmq-broker-infra-1:\n",
            "hostname: \"rabbitmq-infra-1\"",
            "RABBITMQ_NODENAME: \"rabbit@rabbitmq-infra-1\"",
            "RABBITMQ_CONFIG_FILE: \"/var/lib/rabbitmq/rabbitmq.conf\"",
            "RABBITMQ_ENABLED_PLUGINS_FILE: \"/var/lib/rabbitmq/enabled_plugins\"",
            "user: \"0:0\"",
            "/etc/quantrule/rabbitmq/rabbitmq.conf",
            "/etc/quantrule/rabbitmq/enabled_plugins",
            "/etc/quantrule/rabbitmq/erlang.cookie",
            "/etc/quantrule/pki/rabbitmq",
            "/srv/quantrule/rabbitmq/data",
            "/srv/quantrule/rabbitmq/definitions",
            "/srv/quantrule/rabbitmq/conf.d",
            "install -o rabbitmq -g rabbitmq -m 600 /etc/quantrule/rabbitmq/rabbitmq.conf /var/lib/rabbitmq/rabbitmq.conf",
            "export RABBITMQ_CONFIG_FILE=/var/lib/rabbitmq/rabbitmq.conf",
            "gosu rabbitmq rabbitmq-server",
            "rabbitmq-infra-1",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("\"rabbitmq\""), "{text}");
        assert!(!text.contains("\n  rabbitmq:\n"), "{text}");
        assert!(text.contains("\n  rabbitmq-broker-infra-1:\n"), "{text}");
        assert!(!text.contains("RABBITMQ_USE_LONGNAME"), "{text}");
        assert!(!text.contains("ports:"), "{text}");
        assert!(!text.contains("RABBITMQ_ERLANG_COOKIE:"), "{text}");
        let edge = host_yaml(&descriptor, "edge-1", &units["edge-1"]);
        assert!(edge.contains("\"rabbitmq\""), "{edge}");
    }

    #[test]
    fn the_watchdog_mapping_is_required_only() {
        let automatic = descriptor();
        let automatic_yaml =
            super::super::stateful::patroni_yaml(&automatic, "infra-1").expect("automatic patroni");
        assert!(
            automatic_yaml.contains("mode: \"automatic\""),
            "{automatic_yaml}"
        );
        assert!(
            automatic_yaml.contains("device: \"/dev/watchdog\""),
            "{automatic_yaml}"
        );
        let automatic_infra = host_yaml(&automatic, "infra-1", &host_units(&automatic)["infra-1"]);
        assert!(!automatic_infra.contains("devices:"), "{automatic_infra}");
        assert!(
            !automatic_infra.contains("/dev/watchdog"),
            "{automatic_infra}"
        );

        for (mode, expected) in [("required", true), ("off", false)] {
            let mut value: serde_json::Value =
                serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
                    .expect("example");
            value["stateful"]["postgres"]["watchdog"] = serde_json::json!(mode);
            let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
            let text = host_yaml(&descriptor, "infra-1", &host_units(&descriptor)["infra-1"]);
            assert_eq!(text.contains("devices:"), expected, "{mode}: {text}");
            assert_eq!(
                text.contains("/dev/watchdog:/dev/watchdog"),
                expected,
                "{mode}: {text}"
            );
        }
    }

    #[test]
    fn the_minio_drive_counts_are_rendered() {
        let units = host_units(&descriptor());
        for host in ["infra-1", "infra-2", "infra-3", "infra-4"] {
            let text = host_yaml(&descriptor(), host, &units[host]);
            assert!(text.contains("/data1"), "{host}: {text}");
            assert!(!text.contains("/data2"), "{host}: {text}");
        }
    }

    #[test]
    fn a_host_with_no_unit_is_not_rendered() {
        let units = host_units(&descriptor());
        assert!(units.contains_key("infra-1"));
        assert!(!units.contains_key("core-1"));
    }
}
