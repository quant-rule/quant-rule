//! Pinned per-host Compose skeletons for the stateful and observability units.
//!
//! The descriptor places the units; this renderer writes one file per host and
//! never writes the unit configuration. Patroni, HAProxy, RabbitMQ, MinIO,
//! Prometheus, Grafana, Loki, and Alertmanager configuration belongs to
//! PLAT-003/004/005/006. Every unit is digest-pinned, attaches to the encrypted
//! overlay, publishes no port, and keeps its data on a host-local path.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::Descriptor;
use crate::error::Error;

use super::yaml::Yaml;
use super::{create_dir, evidence_tier, pinned, safe_component, write_public};

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
    name: &'static str,
    image: String,
    container_name: String,
    volumes: Vec<Volume>,
    aliases: Vec<String>,
    host_network: bool,
    cap_add: Vec<&'static str>,
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
    name: &'static str,
    image_key: &str,
    host: &str,
    volumes: Vec<Volume>,
    aliases: Vec<String>,
) -> Unit {
    Unit {
        name,
        image: pinned(&descriptor.images[image_key]),
        container_name: format!("quantrule-{name}-{host}"),
        volumes,
        aliases,
        host_network: false,
        cap_add: Vec::new(),
    }
}

/// A host-network unit with the two capabilities keepalived needs.
fn host_unit(
    descriptor: &Descriptor,
    name: &'static str,
    image_key: &str,
    host: &str,
    volumes: Vec<Volume>,
) -> Unit {
    Unit {
        name,
        image: pinned(&descriptor.images[image_key]),
        container_name: format!("quantrule-{name}-{host}"),
        volumes,
        aliases: Vec::new(),
        host_network: true,
        cap_add: vec!["NET_ADMIN", "NET_BROADCAST"],
    }
}

/// The units assigned to each host.
fn host_units(descriptor: &Descriptor) -> BTreeMap<String, Vec<Unit>> {
    let mut hosts: BTreeMap<String, Vec<Unit>> = BTreeMap::new();
    for host in &descriptor.stateful.postgres.nodes {
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            "patroni",
            "postgres",
            host,
            vec![
                volume(
                    "/srv/quantrule/patroni/data",
                    "/var/lib/postgresql/data",
                    false,
                ),
                volume(
                    "/etc/quantrule/patroni/patroni.yml",
                    "/etc/patroni/patroni.yml",
                    true,
                ),
                volume("/etc/quantrule/pki/postgres", "/etc/quantrule/pki", true),
            ],
            Vec::new(),
        ));
    }
    for host in &descriptor.stateful.etcd.nodes {
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            "etcd",
            "etcd",
            host,
            vec![
                volume("/srv/quantrule/etcd/data", "/etcd-data", false),
                volume(
                    "/etc/quantrule/etcd/etcd.conf.yml",
                    "/etc/etcd/etcd.conf.yml",
                    true,
                ),
                volume("/etc/quantrule/pki/etcd", "/etc/quantrule/pki", true),
            ],
            Vec::new(),
        ));
    }
    let database_host = descriptor.endpoints.database.host.clone();
    for host in &descriptor.stateful.haproxy.nodes {
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            "haproxy",
            "haproxy",
            host,
            vec![
                volume(
                    "/etc/quantrule/haproxy/haproxy.cfg",
                    "/usr/local/etc/haproxy/haproxy.cfg",
                    true,
                ),
                volume("/etc/quantrule/pki/haproxy", "/etc/quantrule/pki", true),
            ],
            vec![database_host.clone()],
        ));
    }
    let queue_host = descriptor.endpoints.queue.host.clone();
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
        hosts.entry(host.clone()).or_default().push(unit(
            descriptor,
            "rabbitmq",
            "rabbitmq",
            host,
            vec![
                volume("/srv/quantrule/rabbitmq/data", "/var/lib/rabbitmq", false),
                volume(
                    "/etc/quantrule/rabbitmq/rabbitmq.conf",
                    "/etc/rabbitmq/rabbitmq.conf",
                    true,
                ),
                volume("/etc/quantrule/pki/rabbitmq", "/etc/quantrule/pki", true),
            ],
            vec![queue_host.clone()],
        ));
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
        if value.host_network {
            service = service.with("network_mode", Yaml::string("host"));
        }
        if !value.cap_add.is_empty() {
            service = service.with(
                "cap_add",
                Yaml::list(value.cap_add.iter().map(|cap| Yaml::string(*cap))),
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
        services.insert(value.name.to_owned(), service);
    }
    let root = Yaml::map()
        .with(
            "name",
            Yaml::string(format!("quantrule-infra-{host}")),
        )
        .with(
            "x-quantrule",
            Yaml::map()
                .with("owner", Yaml::string("HA-012"))
                .with("profile", Yaml::string(descriptor.profile.as_str()))
                .with("evidence", Yaml::string(evidence_tier(descriptor.profile)))
                .with(
                    "network",
                    Yaml::string("overlay, attachable, encrypted; created by bootstrap.sh"),
                )
                .with(
                    "platform",
                    Yaml::string(
                        "keepalived is PLAT-002; the remaining unit configuration belongs to PLAT-003/004/005/006; this file only pins the image, host volumes, and overlay attachment",
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
