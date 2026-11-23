//! RabbitMQ configuration for the v3 stateful tier.
//!
//! The renderer writes one TLS-only broker configuration per RabbitMQ node, the
//! enabled-plugin list, the queue leaf, and the `rabbitmq-<host>` SAN set. The
//! application connects to the HAProxy pair through the shared `rabbitmq` alias;
//! brokers never carry that alias, so one broker loss cannot strand the client.
//!
//! Every broker uses a short node name (`rabbit@rabbitmq-<host>`), classic
//! peer discovery, and `pause_minority`. The only operator default is
//! `default_queue_type = quorum`: the application declares its own queue
//! arguments, bindings, delivery limit, and dead-letter exchange. No policy is
//! rendered on `runs.*` or `stages.*`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::descriptor::v3::Descriptor;
use crate::error::Error;
use crate::validate::secrets::read_public_file;
use crate::validate::v3::LoadedFiles;

use super::{create_dir, safe_component, stack_name, write_private, write_public};

/// The broker client port; TLS only.
pub const RABBITMQ_PORT: u16 = 5671;

/// The overlay-only Prometheus listener.
pub const RABBITMQ_METRICS_PORT: u16 = 15692;

/// The in-container configuration path.
pub const RABBITMQ_CONFIG_TARGET: &str = "/etc/rabbitmq/rabbitmq.conf";

/// The in-container enabled-plugin path.
pub const RABBITMQ_PLUGINS_TARGET: &str = "/etc/rabbitmq/enabled_plugins";

/// The host path of the installed Erlang cookie.
pub const RABBITMQ_COOKIE_TARGET: &str = "/etc/quantrule/rabbitmq/erlang.cookie";

/// The broker's data directory inside the container.
pub const RABBITMQ_DATA_TARGET: &str = "/var/lib/rabbitmq";

/// The broker's definitions directory inside the container.
pub const RABBITMQ_DEFINITIONS_TARGET: &str = "/srv/quantrule/rabbitmq/definitions";

/// The host PKI directory mounted into the broker.
pub const RABBITMQ_PKI_TARGET: &str = "/etc/quantrule/pki/rabbitmq";

/// The broker's copy of the CA bundle, certificate, and key.
pub const RABBITMQ_DATA_PKI: &str = "/var/lib/rabbitmq/pki";

/// The enabled-plugin list: Prometheus only; management stays off.
#[must_use]
pub fn enabled_plugins() -> &'static str {
    "[rabbitmq_prometheus].\n"
}

/// Write the broker configuration, plugin list, and queue PKI leaf.
pub fn render(
    descriptor: &Descriptor,
    files: &LoadedFiles,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    for host in &descriptor.stateful.rabbitmq.nodes {
        safe_component(host, "stateful.rabbitmq.nodes")?;
        let path = out.join(format!("rabbitmq-{host}.conf"));
        write_private(&path, rabbitmq_conf(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    let path = out.join("rabbitmq-enabled_plugins");
    write_private(&path, enabled_plugins().as_bytes())?;
    written.push(path);

    let pki = out.join("pki").join("rabbitmq");
    create_dir(&pki)?;
    let queue_ca = read_public_file(&descriptor.endpoints.queue.tls_ca, "endpoints.queue.tls_ca")
        .map_err(|error| {
        Error::rule(
            "deploy.endpoint.tls_ca",
            "endpoints.queue.tls_ca",
            format!("cannot use the queue CA bundle for the broker leaf: {error}"),
        )
    })?;
    for (name, content, private) in [
        ("ca.crt", queue_ca.as_str(), false),
        (
            "server.crt",
            files.stateful_pki.rabbitmq.certificate.as_str(),
            false,
        ),
        ("server.key", files.stateful_pki.rabbitmq.key.as_str(), true),
    ] {
        let path = pki.join(name);
        if private {
            write_private(&path, content.as_bytes())?;
        } else {
            write_public(&path, content.as_bytes())?;
        }
        written.push(path);
    }
    Ok(())
}

/// The DNS names the broker leaf must cover.
///
/// The shared `rabbitmq` name is the application front door on HAProxy; every
/// broker also needs its unique `rabbitmq-<host>` name for Erlang distribution
/// and direct cluster traffic.
#[must_use]
pub fn queue_sans(descriptor: &Descriptor) -> Vec<String> {
    let mut names = BTreeSet::from(["rabbitmq".to_owned()]);
    for host in &descriptor.stateful.rabbitmq.nodes {
        names.insert(format!("rabbitmq-{host}"));
    }
    names.into_iter().collect()
}

/// Build one broker's `rabbitmq.conf`.
///
/// # Errors
///
/// Returns a rule when the broker host is not declared. The in-cluster names
/// are overlay aliases, never the host underlay address.
pub fn rabbitmq_conf(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    host_by_name(descriptor, host, "stateful.rabbitmq.nodes")?;
    let mut peers = String::new();
    for (index, broker) in descriptor.stateful.rabbitmq.nodes.iter().enumerate() {
        host_by_name(descriptor, broker, "stateful.rabbitmq.nodes")?;
        peers.push_str(&format!(
            "cluster_formation.classic_config.nodes.{} = rabbit@rabbitmq-{broker}\n",
            index + 1
        ));
    }
    let watermark = format!(
        "0.{:02}",
        descriptor.stateful.rabbitmq.memory_high_watermark_percent
    );
    Ok(format!(
        "# Generated by qr-deploy; copy to {RABBITMQ_CONFIG_TARGET}.\n\
         # TLS-only client listener. There is no plaintext listener and no\n\
         # management listener. The certificate and key are installed under\n\
         # /var/lib/rabbitmq/pki by the unit command before the broker starts.\n\
         listeners.tcp = none\n\
         listeners.ssl.default = {RABBITMQ_PORT}\n\
         \n\
         ssl_options.cacertfile = /var/lib/rabbitmq/pki/ca.crt\n\
         ssl_options.certfile = /var/lib/rabbitmq/pki/server.crt\n\
         ssl_options.keyfile = /var/lib/rabbitmq/pki/server.key\n\
         ssl_options.verify = verify_none\n\
         ssl_options.fail_if_no_peer_cert = false\n\
         ssl_options.versions.1 = tlsv1.2\n\
         ssl_options.versions.2 = tlsv1.3\n\
         \n\
         cluster_formation.peer_discovery_backend = rabbit_peer_discovery_classic_config\n\
         {peers}\
         cluster_formation.node_type = disc\n\
         cluster_name = {cluster}\n\
         cluster_partition_handling = pause_minority\n\
         \n\
         default_queue_type = quorum\n\
         disk_free_limit.absolute = {disk}MiB\n\
         vm_memory_high_watermark.relative = {watermark}\n\
         vm_memory_high_watermark_paging_ratio = 0.5\n\
         loopback_users.guest = true\n\
         \n\
         prometheus.tcp.port = {RABBITMQ_METRICS_PORT}\n\
         log.console = true\n\
         log.console.formatter = json\n\
         log.file = false\n",
        cluster = stack_name(descriptor),
        disk = descriptor.stateful.rabbitmq.disk_free_limit_mib,
        watermark = watermark,
    ))
}

fn host_by_name<'a>(
    descriptor: &'a Descriptor,
    host: &str,
    field: &str,
) -> Result<&'a crate::descriptor::v3::Host, Error> {
    descriptor.hosts.get(host).ok_or_else(|| {
        Error::rule(
            "deploy.host.undeclared",
            field,
            format!("{host} is not declared in hosts"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    #[test]
    fn the_conf_is_tls_only_and_discovers_the_three_short_names() {
        let text = rabbitmq_conf(&descriptor(), "infra-1").expect("conf");
        for expected in [
            "listeners.tcp = none",
            "listeners.ssl.default = 5671",
            "ssl_options.cacertfile = /var/lib/rabbitmq/pki/ca.crt",
            "ssl_options.certfile = /var/lib/rabbitmq/pki/server.crt",
            "ssl_options.keyfile = /var/lib/rabbitmq/pki/server.key",
            "ssl_options.verify = verify_none",
            "ssl_options.fail_if_no_peer_cert = false",
            "ssl_options.versions.1 = tlsv1.2",
            "ssl_options.versions.2 = tlsv1.3",
            "cluster_formation.peer_discovery_backend = rabbit_peer_discovery_classic_config",
            "cluster_formation.classic_config.nodes.1 = rabbit@rabbitmq-infra-1",
            "cluster_formation.classic_config.nodes.2 = rabbit@rabbitmq-infra-2",
            "cluster_formation.classic_config.nodes.3 = rabbit@rabbitmq-infra-3",
            "cluster_formation.node_type = disc",
            "cluster_name = quantrule-ha",
            "cluster_partition_handling = pause_minority",
            "default_queue_type = quorum",
            "disk_free_limit.absolute = 5120MiB",
            "vm_memory_high_watermark.relative = 0.60",
            "vm_memory_high_watermark_paging_ratio = 0.5",
            "loopback_users.guest = true",
            "prometheus.tcp.port = 15692",
            "log.console = true",
            "log.console.formatter = json",
            "log.file = false",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        for forbidden in [
            "listeners.tcp.default",
            "5672",
            "management.tcp",
            "policy.",
            "runs.",
            "stages.",
            "guest = false",
        ] {
            assert!(!text.contains(forbidden), "{forbidden}: {text}");
        }
        assert!(!text.contains("localhost"), "{text}");
    }

    #[test]
    fn the_plugin_list_is_prometheus_only() {
        assert_eq!(enabled_plugins(), "[rabbitmq_prometheus].\n");
    }

    #[test]
    fn the_sans_cover_the_front_door_and_every_broker() {
        let sans = queue_sans(&descriptor());
        assert_eq!(
            sans,
            vec![
                "rabbitmq".to_owned(),
                "rabbitmq-infra-1".to_owned(),
                "rabbitmq-infra-2".to_owned(),
                "rabbitmq-infra-3".to_owned(),
            ]
        );
    }

    #[test]
    fn the_lab_conf_uses_the_lab_names_and_stack() {
        let lab: Descriptor =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.lab.json"))
                .expect("lab");
        let text = rabbitmq_conf(&lab, "lab-1").expect("conf");
        assert!(
            text.contains("cluster_formation.classic_config.nodes.1 = rabbit@rabbitmq-lab-1"),
            "{text}"
        );
        assert!(text.contains("cluster_name = quantrule-lab"), "{text}");
    }
}
