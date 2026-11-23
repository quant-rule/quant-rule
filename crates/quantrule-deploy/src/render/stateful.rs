//! Patroni, etcd, and HAProxy configuration for the v3 stateful tier.
//!
//! The renderer writes one Patroni YAML per PostgreSQL node, one etcd YAML per
//! etcd node, one HAProxy configuration per edge host, the Postgres and etcd
//! PKI leaves, and a `stateful-manifest.json` that names every file the operator
//! copies to a host. No rendered file carries a password: the units read the
//! four mounted secret files into Patroni's `PATRONI_*` environment variables or
//! a HAProxy runtime configuration before exec. HAProxy forwards TCP; it never
//! terminates database TLS.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::descriptor::v3::{Descriptor, Watchdog};
use crate::error::Error;
use crate::validate::v3::LoadedFiles;

use super::yaml::Yaml;
use super::{create_dir, safe_component, stack_name, write_private, write_public};

pub(crate) use crate::validate::secrets::STATEFUL_SECRETS;

/// The HAProxy health-check placeholder the unit command replaces from the file.
pub(crate) const REST_AUTH_PLACEHOLDER: &str = "__PATRONI_REST_AUTH__";

/// Write the stateful configuration, PKI leaves, and manifest for an infra render.
pub fn render(
    descriptor: &Descriptor,
    files: &LoadedFiles,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    let pki = out.join("pki");
    create_dir(&pki)?;
    for host in &descriptor.stateful.postgres.nodes {
        safe_component(host, "stateful.postgres.nodes")?;
        let path = out.join(format!("patroni-{host}.yml"));
        write_private(&path, patroni_yaml(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    for host in &descriptor.stateful.etcd.nodes {
        safe_component(host, "stateful.etcd.nodes")?;
        let path = out.join(format!("etcd-{host}.conf.yml"));
        write_private(&path, etcd_yaml(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    for host in &descriptor.stateful.haproxy.nodes {
        safe_component(host, "stateful.haproxy.nodes")?;
        let path = out.join(format!("haproxy-{host}.cfg"));
        write_private(&path, haproxy_cfg(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    super::queue::render(descriptor, files, out, written)?;

    let postgres = pki.join("postgres");
    create_dir(&postgres)?;
    for (path, content) in [
        (
            postgres.join("server.crt"),
            files.stateful_pki.postgres.certificate.as_str(),
        ),
        (
            postgres.join("server.key"),
            files.stateful_pki.postgres.key.as_str(),
        ),
    ] {
        if path.extension().is_some_and(|extension| extension == "key") {
            write_private(&path, content.as_bytes())?;
        } else {
            write_public(&path, content.as_bytes())?;
        }
        written.push(path);
    }
    let etcd = pki.join("etcd");
    create_dir(&etcd)?;
    for (name, leaf) in [
        ("server", &files.stateful_pki.etcd.server),
        ("peer", &files.stateful_pki.etcd.peer),
        ("client", &files.stateful_pki.etcd.client),
    ] {
        let certificate = etcd.join(format!("{name}.crt"));
        write_public(&certificate, leaf.certificate.as_bytes())?;
        written.push(certificate);
        let key = etcd.join(format!("{name}.key"));
        write_private(&key, leaf.key.as_bytes())?;
        written.push(key);
    }

    let path = out.join("stateful-manifest.json");
    write_public(&path, stateful_manifest(descriptor)?.as_bytes())?;
    written.push(path);
    Ok(())
}

/// The in-cluster DNS names the stateful PKI must cover.
///
/// `postgres` is the application name on HAProxy; every Patroni member also
/// carries `patroni-<host>` for replication/rewind, and every etcd member
/// carries `etcd-<host>` for client and peer TLS. `tools/pki.sh` receives this
/// set through `PKI_EXTRA_SANS` (the `postgres` and `etcd` base names are
/// already in the leaf).
#[must_use]
pub fn stateful_sans(descriptor: &Descriptor) -> Vec<String> {
    let mut names = BTreeSet::from(["postgres".to_owned()]);
    for host in &descriptor.stateful.postgres.nodes {
        names.insert(format!("patroni-{host}"));
    }
    for host in &descriptor.stateful.etcd.nodes {
        names.insert(format!("etcd-{host}"));
    }
    names.extend(super::queue::queue_sans(descriptor));
    names.into_iter().collect()
}

/// Build one Patroni node's YAML.
///
/// # Errors
///
/// Returns a rule when the node host is not declared. Validation refuses that
/// before rendering; this is the defensive boundary. In-cluster addresses are
/// overlay aliases, never the host underlay address.
pub fn patroni_yaml(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    host_by_name(descriptor, host, "stateful.postgres.nodes")?;
    let mut etcd_hosts = Vec::new();
    for etcd_host in &descriptor.stateful.etcd.nodes {
        host_by_name(descriptor, etcd_host, "stateful.etcd.nodes")?;
        etcd_hosts.push(Yaml::string(format!("https://etcd-{etcd_host}:2379")));
    }
    let parameters = Yaml::map()
        .with("archive_command", Yaml::string(archive_command()))
        .with("archive_mode", Yaml::string("on"))
        .with("archive_timeout", Yaml::number(60_u64))
        .with("hot_standby", Yaml::string("on"))
        .with("restore_command", Yaml::string(restore_command()))
        .with("ssl", Yaml::string("on"))
        .with("ssl_ca_file", Yaml::string("/etc/quantrule/pki/ca.crt"))
        .with(
            "ssl_cert_file",
            Yaml::string("/etc/quantrule/pki/postgres/server.crt"),
        )
        .with(
            "ssl_key_file",
            Yaml::string("/etc/quantrule/pki/postgres/server.key"),
        )
        .with("ssl_min_protocol_version", Yaml::string("TLSv1.2"))
        .with("synchronous_commit", Yaml::string("on"))
        .with("unix_socket_directories", Yaml::string("/run/postgresql"))
        .with("wal_level", Yaml::string("replica"))
        .with("wal_log_hints", Yaml::string("on"));
    let dcs_parameters = Yaml::map()
        .with("archive_command", Yaml::string(archive_command()))
        .with("archive_mode", Yaml::string("on"))
        .with("archive_timeout", Yaml::number(60_u64))
        .with("hot_standby", Yaml::string("on"))
        .with("synchronous_commit", Yaml::string("on"))
        .with("wal_level", Yaml::string("replica"))
        .with("wal_log_hints", Yaml::string("on"));

    let authentication = Yaml::map()
        .with(
            "replication",
            Yaml::map()
                .with("sslmode", Yaml::string("verify-full"))
                .with("sslrootcert", Yaml::string("/etc/quantrule/pki/ca.crt"))
                .with("username", Yaml::string("replicator")),
        )
        .with(
            "rewind",
            Yaml::map()
                .with("sslmode", Yaml::string("verify-full"))
                .with("sslrootcert", Yaml::string("/etc/quantrule/pki/ca.crt"))
                .with("username", Yaml::string("rewind")),
        )
        .with(
            "superuser",
            Yaml::map()
                .with("sslmode", Yaml::string("verify-full"))
                .with("sslrootcert", Yaml::string("/etc/quantrule/pki/ca.crt"))
                .with("username", Yaml::string("postgres")),
        );

    let pg_hba = Yaml::list([
        Yaml::string("local all all peer"),
        Yaml::string("hostssl all all 0.0.0.0/0 scram-sha-256"),
        Yaml::string("hostssl replication replicator 0.0.0.0/0 scram-sha-256"),
        Yaml::string("hostssl all all ::/0 scram-sha-256"),
    ]);

    let postgresql = Yaml::map()
        .with("authentication", authentication)
        .with(
            "connect_address",
            Yaml::string(format!("patroni-{host}:5432")),
        )
        .with("data_dir", Yaml::string("/var/lib/postgresql/data"))
        .with("listen", Yaml::string("0.0.0.0:5432"))
        .with("parameters", parameters)
        .with("pg_hba", pg_hba)
        .with("pgpass", Yaml::string("/tmp/pgpass"))
        .with(
            "recovery_conf",
            Yaml::map().with("restore_command", Yaml::string(restore_command())),
        );

    let dcs_postgresql = Yaml::map()
        .with("parameters", dcs_parameters)
        .with("use_pg_rewind", Yaml::Bool(true))
        .with("use_slots", Yaml::Bool(true));
    let dcs = Yaml::map()
        .with("loop_wait", Yaml::number(10_u64))
        .with("maximum_lag_on_failover", Yaml::number(1_048_576_u64))
        .with("postgresql", dcs_postgresql)
        .with("retry_timeout", Yaml::number(10_u64))
        .with("synchronous_mode", Yaml::Bool(true))
        .with("synchronous_mode_strict", Yaml::Bool(true))
        .with(
            "synchronous_node_count",
            Yaml::number(u64::from(descriptor.stateful.postgres.synchronous_standby)),
        )
        .with("ttl", Yaml::number(30_u64));
    let initdb = Yaml::list([
        Yaml::string("data-checksums"),
        Yaml::map()
            .with("auth-host", Yaml::string("scram-sha-256"))
            .with("auth-local", Yaml::string("peer"))
            .with("encoding", Yaml::string("UTF8")),
    ]);
    let bootstrap = Yaml::map().with("dcs", dcs).with("initdb", initdb);

    let etcd3 = Yaml::map()
        .with("cacert", Yaml::string("/etc/quantrule/pki/ca.crt"))
        .with("cert", Yaml::string("/etc/quantrule/pki/etcd/client.crt"))
        .with("hosts", Yaml::list(etcd_hosts))
        .with("key", Yaml::string("/etc/quantrule/pki/etcd/client.key"))
        .with("protocol", Yaml::string("https"));

    let restapi = Yaml::map()
        .with(
            "authentication",
            Yaml::map().with("username", Yaml::string("patroni")),
        )
        .with(
            "connect_address",
            Yaml::string(format!("patroni-{host}:8008")),
        )
        .with("listen", Yaml::string("0.0.0.0:8008"));

    let watchdog = match descriptor.stateful.postgres.watchdog {
        Watchdog::Off => Yaml::map().with("mode", Yaml::string("off")),
        mode => Yaml::map()
            .with("device", Yaml::string("/dev/watchdog"))
            .with("mode", Yaml::string(mode.as_str())),
    };

    let root = Yaml::map()
        .with("bootstrap", bootstrap)
        .with("etcd3", etcd3)
        .with("name", Yaml::string(format!("patroni-{host}")))
        .with("namespace", Yaml::string("/service/"))
        .with("postgresql", postgresql)
        .with("restapi", restapi)
        .with("scope", Yaml::string(stack_name(descriptor)))
        .with("watchdog", watchdog);
    Ok(root.render())
}

/// Build one etcd node's YAML.
///
/// # Errors
///
/// Returns a rule when the node host is not declared. In-cluster addresses are
/// overlay aliases, never the host underlay address.
pub fn etcd_yaml(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    host_by_name(descriptor, host, "stateful.etcd.nodes")?;
    let mut initial_cluster = Vec::new();
    for etcd_host in &descriptor.stateful.etcd.nodes {
        host_by_name(descriptor, etcd_host, "stateful.etcd.nodes")?;
        initial_cluster.push(format!("etcd-{etcd_host}=https://etcd-{etcd_host}:2380"));
    }
    let client_security = Yaml::map()
        .with(
            "cert-file",
            Yaml::string("/etc/quantrule/pki/etcd/server.crt"),
        )
        .with("client-cert-auth", Yaml::Bool(true))
        .with(
            "key-file",
            Yaml::string("/etc/quantrule/pki/etcd/server.key"),
        )
        .with("trusted-ca-file", Yaml::string("/etc/quantrule/pki/ca.crt"));
    let peer_security = Yaml::map()
        .with(
            "cert-file",
            Yaml::string("/etc/quantrule/pki/etcd/peer.crt"),
        )
        .with("client-cert-auth", Yaml::Bool(true))
        .with("key-file", Yaml::string("/etc/quantrule/pki/etcd/peer.key"))
        .with("trusted-ca-file", Yaml::string("/etc/quantrule/pki/ca.crt"));
    let root = Yaml::map()
        .with(
            "advertise-client-urls",
            Yaml::string(format!("https://etcd-{host}:2379")),
        )
        .with("client-transport-security", client_security)
        .with("data-dir", Yaml::string("/etcd-data"))
        .with(
            "initial-advertise-peer-urls",
            Yaml::string(format!("https://etcd-{host}:2380")),
        )
        .with("initial-cluster", Yaml::string(initial_cluster.join(",")))
        .with(
            "initial-cluster-token",
            Yaml::string(format!("{}-etcd", stack_name(descriptor))),
        )
        .with("initial-cluster-state", Yaml::string("new"))
        .with("listen-client-urls", Yaml::string("https://0.0.0.0:2379"))
        .with("listen-metrics-urls", Yaml::string("http://0.0.0.0:2381"))
        .with("listen-peer-urls", Yaml::string("https://0.0.0.0:2380"))
        .with("metrics", Yaml::string("prometheus"))
        .with("name", Yaml::string(format!("etcd-{host}")))
        .with("peer-transport-security", peer_security);
    Ok(root.render())
}

/// Build one HAProxy edge's configuration.
///
/// # Errors
///
/// Returns a rule when a PostgreSQL member host is not declared. In-cluster
/// addresses are overlay aliases, never the host underlay address.
pub fn haproxy_cfg(descriptor: &Descriptor, _host: &str) -> Result<String, Error> {
    let mut servers = String::new();
    for member in &descriptor.stateful.postgres.nodes {
        host_by_name(descriptor, member, "stateful.postgres.nodes")?;
        servers.push_str(&format!(
            "  server patroni-{member} patroni-{member}:5432 check port 8008\n"
        ));
    }
    let mut queue_servers = String::new();
    for broker in &descriptor.stateful.rabbitmq.nodes {
        host_by_name(descriptor, broker, "stateful.rabbitmq.nodes")?;
        queue_servers.push_str(&format!(
            "  server rabbitmq-{broker} rabbitmq-{broker}:5671 check\n"
        ));
    }
    Ok(format!(
        "# Generated by qr-deploy; copy to /etc/quantrule/haproxy/haproxy.cfg.\n\
         # TCP passthrough: HAProxy never terminates the database or broker TLS session.\n\
         # The unit reads patroni_rest_password and replaces the check header.\n\
         global\n\
         \x20 log stdout format raw local0\n\
         \x20 stats socket /tmp/haproxy.sock mode 600 level admin\n\
         \x20 maxconn 2048\n\
         \n\
         defaults\n\
         \x20 mode tcp\n\
         \x20 log global\n\
         \x20 option tcplog\n\
         \x20 timeout connect 5s\n\
         \x20 timeout client 30m\n\
         \x20 timeout server 30m\n\
         \x20 timeout check 5s\n\
         \x20 default-server inter 2s fastinter 1s downinter 2s fall 3 rise 2\n\
         \n\
         frontend postgres\n\
         \x20 bind *:5432\n\
         \x20 mode tcp\n\
         \x20 option tcplog\n\
         \x20 default_backend postgres_primary\n\
         \n\
         backend postgres_primary\n\
         \x20 mode tcp\n\
         \x20 balance roundrobin\n\
         \x20 option httpchk GET /primary\n\
         \x20 http-check expect status 200\n\
         \x20 http-check send hdr Authorization \"Basic {REST_AUTH_PLACEHOLDER}\"\n\
         {servers}\
         \x20 on-marked-down shutdown-sessions\n\
         \n\
         frontend rabbitmq\n\
         \x20 bind *:5671\n\
         \x20 mode tcp\n\
         \x20 option tcplog\n\
         \x20 default_backend rabbitmq_nodes\n\
         \n\
         backend rabbitmq_nodes\n\
         \x20 mode tcp\n\
         \x20 balance roundrobin\n\
         \x20 option tcp-check\n\
         \x20 tcp-check connect\n\
         {queue_servers}\
         \x20 on-marked-down shutdown-sessions\n\
         \n\
         listen stats\n\
         \x20 mode http\n\
         \x20 bind 0.0.0.0:8404\n\
         \x20 stats enable\n\
         \x20 stats uri /stats\n\
         \x20 stats refresh 10s\n"
    ))
}

/// Build the stateful file manifest.
///
/// Every entry names the hosts it belongs to. A broker file is never listed for
/// a Patroni or etcd host, and a database secret is never listed for a broker.
/// The copy step in `tools/stateful.sh` uses this selector.
///
/// # Errors
///
/// Returns [`Error::Invalid`] if the fixed manifest cannot be serialized.
pub fn stateful_manifest(descriptor: &Descriptor) -> Result<String, Error> {
    let postgres = &descriptor.stateful.postgres.nodes;
    let etcd = &descriptor.stateful.etcd.nodes;
    let haproxy = &descriptor.stateful.haproxy.nodes;
    let brokers = &descriptor.stateful.rabbitmq.nodes;
    let database_hosts = union(postgres, etcd);
    let rest_hosts = union(postgres, haproxy);
    let mut entries = Vec::new();
    for host in postgres {
        entries.push(entry(
            format!("patroni-{host}.yml"),
            "/etc/quantrule/patroni/patroni.yml",
            "600",
            "patroni",
            vec![host.clone()],
        ));
    }
    for host in etcd {
        entries.push(entry(
            format!("etcd-{host}.conf.yml"),
            "/etc/quantrule/etcd/etcd.conf.yml",
            "600",
            "etcd",
            vec![host.clone()],
        ));
    }
    for host in haproxy {
        entries.push(entry(
            format!("haproxy-{host}.cfg"),
            "/etc/quantrule/haproxy/haproxy.cfg",
            "600",
            "haproxy",
            vec![host.clone()],
        ));
    }
    for host in brokers {
        entries.push(entry(
            format!("rabbitmq-{host}.conf"),
            super::queue::RABBITMQ_CONFIG_TARGET,
            "600",
            "config",
            vec![host.clone()],
        ));
    }
    entries.push(entry(
        "rabbitmq-enabled_plugins",
        super::queue::RABBITMQ_PLUGINS_TARGET,
        "600",
        "config",
        brokers.clone(),
    ));
    for name in STATEFUL_SECRETS {
        let (target, hosts) = match name {
            "rabbitmq_erlang_cookie" => (
                super::queue::RABBITMQ_COOKIE_TARGET.to_owned(),
                brokers.clone(),
            ),
            "patroni_rest_password" => {
                (format!("/etc/quantrule/secrets/{name}"), rest_hosts.clone())
            }
            _ => (format!("/etc/quantrule/secrets/{name}"), postgres.clone()),
        };
        entries.push(entry(
            format!("secrets/{name}"),
            target,
            "600",
            "secret",
            hosts,
        ));
    }
    entries.push(entry(
        "pki/database-ca.crt",
        "/etc/quantrule/pki/ca.crt",
        "644",
        "pki",
        database_hosts,
    ));
    for (source, target, mode, hosts) in [
        (
            "pki/postgres/server.crt",
            "/etc/quantrule/pki/postgres/server.crt",
            "644",
            postgres.clone(),
        ),
        (
            "pki/postgres/server.key",
            "/etc/quantrule/pki/postgres/server.key",
            "600",
            postgres.clone(),
        ),
        (
            "pki/etcd/server.crt",
            "/etc/quantrule/pki/etcd/server.crt",
            "644",
            etcd.clone(),
        ),
        (
            "pki/etcd/server.key",
            "/etc/quantrule/pki/etcd/server.key",
            "600",
            etcd.clone(),
        ),
        (
            "pki/etcd/peer.crt",
            "/etc/quantrule/pki/etcd/peer.crt",
            "644",
            etcd.clone(),
        ),
        (
            "pki/etcd/peer.key",
            "/etc/quantrule/pki/etcd/peer.key",
            "600",
            etcd.clone(),
        ),
        (
            "pki/etcd/client.crt",
            "/etc/quantrule/pki/etcd/client.crt",
            "644",
            etcd.clone(),
        ),
        (
            "pki/etcd/client.key",
            "/etc/quantrule/pki/etcd/client.key",
            "600",
            etcd.clone(),
        ),
        (
            "pki/rabbitmq/ca.crt",
            "/etc/quantrule/pki/rabbitmq/ca.crt",
            "644",
            brokers.clone(),
        ),
        (
            "pki/rabbitmq/server.crt",
            "/etc/quantrule/pki/rabbitmq/server.crt",
            "644",
            brokers.clone(),
        ),
        (
            "pki/rabbitmq/server.key",
            "/etc/quantrule/pki/rabbitmq/server.key",
            "600",
            brokers.clone(),
        ),
    ] {
        entries.push(entry(source, target, mode, "pki", hosts));
    }
    let manifest = StatefulManifest {
        v: "quantrule.deploy/v3",
        note: "copy each source to its absolute target on the listed hosts; no secret value or fingerprint is listed",
        sans: stateful_sans(descriptor),
        files: entries,
    };
    let text = serde_json::to_string_pretty(&manifest).map_err(|error| Error::Invalid {
        field: "stateful-manifest.json".to_owned(),
        message: error.to_string(),
    })?;
    Ok(text + "\n")
}

/// The union of two host lists, preserving first-seen order.
fn union(first: &[String], second: &[String]) -> Vec<String> {
    let mut hosts = first.to_vec();
    for host in second {
        if !hosts.contains(host) {
            hosts.push(host.clone());
        }
    }
    hosts
}

fn entry(
    source: impl Into<String>,
    target: impl Into<String>,
    mode: &str,
    scope: &str,
    hosts: Vec<String>,
) -> Entry {
    Entry {
        source: source.into(),
        target: target.into(),
        mode: mode.to_owned(),
        owner: "root".to_owned(),
        scope: scope.to_owned(),
        hosts,
    }
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

fn archive_command() -> &'static str {
    "test ! -f /srv/quantrule/wal-archive/%f && cp %p /srv/quantrule/wal-archive/%f"
}

fn restore_command() -> &'static str {
    "cp /srv/quantrule/wal-archive/%f %p"
}

#[derive(Serialize)]
struct StatefulManifest<'a> {
    v: &'a str,
    note: &'a str,
    sans: Vec<String>,
    files: Vec<Entry>,
}

#[derive(Serialize)]
struct Entry {
    source: String,
    target: String,
    mode: String,
    owner: String,
    scope: String,
    hosts: Vec<String>,
}

/// The in-container path of the Patroni config.
pub(crate) const PATRONI_CONFIG_TARGET: &str = "/etc/patroni/patroni.yml";
/// The in-container path of the etcd config.
pub(crate) const ETCD_CONFIG_TARGET: &str = "/etc/etcd/etcd.conf.yml";
/// The in-container path of the HAProxy config.
pub(crate) const HAPROXY_CONFIG_TARGET: &str = "/usr/local/etc/haproxy/haproxy.cfg";

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor() -> Descriptor {
        serde_json::from_str(include_str!("../../../../deploy/v3/deploy.example.json"))
            .expect("v3 example")
    }

    #[test]
    fn patroni_carries_the_sync_tls_archive_and_watchdog_contract() {
        let text = patroni_yaml(&descriptor(), "infra-1").expect("patroni");
        for expected in [
            "scope: \"quantrule-ha\"",
            "namespace: \"/service/\"",
            "name: \"patroni-infra-1\"",
            "connect_address: \"patroni-infra-1:5432\"",
            "connect_address: \"patroni-infra-1:8008\"",
            "https://etcd-infra-1:2379",
            "synchronous_mode: true",
            "synchronous_mode_strict: true",
            "synchronous_node_count: 1",
            "synchronous_commit: \"on\"",
            "use_pg_rewind: true",
            "use_slots: true",
            "wal_level: \"replica\"",
            "hot_standby: \"on\"",
            "wal_log_hints: \"on\"",
            "archive_mode: \"on\"",
            "archive_command:",
            "restore_command:",
            "archive_timeout: 60",
            "ssl_cert_file: \"/etc/quantrule/pki/postgres/server.crt\"",
            "ssl_key_file: \"/etc/quantrule/pki/postgres/server.key\"",
            "ssl_ca_file: \"/etc/quantrule/pki/ca.crt\"",
            "data-checksums",
            "auth-host: \"scram-sha-256\"",
            "hostssl all all 0.0.0.0/0 scram-sha-256",
            "hostssl replication replicator 0.0.0.0/0 scram-sha-256",
            "data_dir: \"/var/lib/postgresql/data\"",
            "device: \"/dev/watchdog\"",
            "mode: \"automatic\"",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("trust"), "{text}");
        assert!(!text.contains("password:"), "{text}");
        assert!(!text.contains("__PATRONI"), "{text}");
    }

    #[test]
    fn the_lab_patroni_watchdog_is_off() {
        let lab: Descriptor =
            serde_json::from_str(include_str!("../../../../deploy/v3/deploy.lab.json"))
                .expect("lab");
        let text = patroni_yaml(&lab, "lab-1").expect("patroni");
        assert!(text.contains("mode: \"off\""), "{text}");
        assert!(!text.contains("/dev/watchdog"), "{text}");
    }

    #[test]
    fn etcd_carries_the_three_member_mtls_cluster() {
        let text = etcd_yaml(&descriptor(), "infra-1").expect("etcd");
        for expected in [
            "name: \"etcd-infra-1\"",
            "data-dir: \"/etcd-data\"",
            "listen-client-urls: \"https://0.0.0.0:2379\"",
            "advertise-client-urls: \"https://etcd-infra-1:2379\"",
            "listen-peer-urls: \"https://0.0.0.0:2380\"",
            "etcd-infra-1=https://etcd-infra-1:2380,etcd-infra-2=https://etcd-infra-2:2380,etcd-infra-3=https://etcd-infra-3:2380",
            "initial-cluster-token: \"quantrule-ha-etcd\"",
            "initial-cluster-state: \"new\"",
            "cert-file: \"/etc/quantrule/pki/etcd/server.crt\"",
            "cert-file: \"/etc/quantrule/pki/etcd/peer.crt\"",
            "trusted-ca-file: \"/etc/quantrule/pki/ca.crt\"",
            "client-cert-auth: true",
            "metrics: \"prometheus\"",
            "listen-metrics-urls: \"http://0.0.0.0:2381\"",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
    }

    #[test]
    fn the_in_cluster_addresses_match_the_documented_pki_sans() {
        let descriptor = descriptor();
        let sans = stateful_sans(&descriptor);
        assert_eq!(
            sans,
            vec![
                "etcd-infra-1",
                "etcd-infra-2",
                "etcd-infra-3",
                "patroni-infra-1",
                "patroni-infra-2",
                "patroni-infra-3",
                "postgres",
                "rabbitmq",
                "rabbitmq-infra-1",
                "rabbitmq-infra-2",
                "rabbitmq-infra-3",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        );

        let patroni = patroni_yaml(&descriptor, "infra-1").expect("patroni");
        assert!(patroni.contains("connect_address: \"patroni-infra-1:5432\""));
        assert!(patroni.contains("connect_address: \"patroni-infra-1:8008\""));
        for host in &descriptor.stateful.etcd.nodes {
            assert!(
                patroni.contains(&format!("https://etcd-{host}:2379")),
                "{host}: {patroni}"
            );
        }

        let etcd = etcd_yaml(&descriptor, "infra-1").expect("etcd");
        assert!(etcd.contains("advertise-client-urls: \"https://etcd-infra-1:2379\""));
        assert!(etcd.contains("initial-advertise-peer-urls: \"https://etcd-infra-1:2380\""));
        for host in &descriptor.stateful.etcd.nodes {
            assert!(
                etcd.contains(&format!("etcd-{host}=https://etcd-{host}:2380")),
                "{host}: {etcd}"
            );
        }

        let haproxy = haproxy_cfg(&descriptor, "edge-1").expect("haproxy");
        for host in &descriptor.stateful.postgres.nodes {
            assert!(
                haproxy.contains(&format!(
                    "server patroni-{host} patroni-{host}:5432 check port 8008"
                )),
                "{host}: {haproxy}"
            );
        }
        for host in &descriptor.stateful.rabbitmq.nodes {
            assert!(
                haproxy.contains(&format!(
                    "server rabbitmq-{host} rabbitmq-{host}:5671 check"
                )),
                "{host}: {haproxy}"
            );
        }

        for host in descriptor.hosts.values() {
            assert!(!patroni.contains(&host.address), "{}", host.address);
            assert!(!etcd.contains(&host.address), "{}", host.address);
            assert!(!haproxy.contains(&host.address), "{}", host.address);
        }
    }

    #[test]
    fn haproxy_routes_only_the_primary_over_tcp() {
        let text = haproxy_cfg(&descriptor(), "edge-1").expect("haproxy");
        for expected in [
            "mode tcp",
            "bind *:5432",
            "option httpchk GET /primary",
            "http-check expect status 200",
            "http-check send hdr Authorization",
            "check port 8008",
            "server patroni-infra-1 patroni-infra-1:5432 check port 8008",
            "frontend rabbitmq",
            "bind *:5671",
            "backend rabbitmq_nodes",
            "option tcp-check",
            "server rabbitmq-infra-1 rabbitmq-infra-1:5671 check",
            "on-marked-down shutdown-sessions",
            "listen stats",
            "bind 0.0.0.0:8404",
            "stats socket /tmp/haproxy.sock",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("bind *:443"), "{text}");
        assert!(!text.contains("mode http\n  bind *:5432"), "{text}");
    }

    #[test]
    fn the_stateful_manifest_lists_files_and_no_values() {
        let text = stateful_manifest(&descriptor()).expect("manifest");
        for expected in [
            "patroni-infra-1.yml",
            "/etc/quantrule/patroni/patroni.yml",
            "etcd-infra-1.conf.yml",
            "/etc/quantrule/etcd/etcd.conf.yml",
            "haproxy-edge-1.cfg",
            "/etc/quantrule/haproxy/haproxy.cfg",
            "rabbitmq-infra-1.conf",
            "/etc/rabbitmq/rabbitmq.conf",
            "rabbitmq-enabled_plugins",
            "/etc/rabbitmq/enabled_plugins",
            "secrets/patroni_rest_password",
            "/etc/quantrule/secrets/patroni_rest_password",
            "secrets/rabbitmq_erlang_cookie",
            "/etc/quantrule/rabbitmq/erlang.cookie",
            "pki/postgres/server.key",
            "pki/etcd/client.key",
            "pki/rabbitmq/server.key",
            "\"mode\": \"600\"",
            "\"owner\": \"root\"",
            "\"scope\": \"secret\"",
            "\"scope\": \"pki\"",
            "\"sans\": [",
            "\"hosts\": [",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains(&format!("{}:", "postgres")), "{text}");
        let value: serde_json::Value = serde_json::from_str(&text).expect("JSON");
        let files = value["files"].as_array().expect("files");
        assert!(files.len() > 20, "{}", files.len());
        let entry = |source: &str| {
            files
                .iter()
                .find(|entry| entry["source"] == source)
                .unwrap_or_else(|| panic!("{source} is listed"))
        };
        let hosts = |source: &str| {
            entry(source)["hosts"]
                .as_array()
                .expect("hosts")
                .iter()
                .map(|host| host.as_str().expect("host").to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            hosts("secrets/rabbitmq_erlang_cookie"),
            ["infra-1", "infra-2", "infra-3"]
        );
        assert_eq!(
            hosts("secrets/postgres_superuser_password"),
            ["infra-1", "infra-2", "infra-3"]
        );
        assert_eq!(
            hosts("secrets/patroni_rest_password"),
            ["infra-1", "infra-2", "infra-3", "edge-1", "edge-2"]
        );
        assert_eq!(hosts("rabbitmq-infra-1.conf"), ["infra-1"]);
        assert_eq!(
            hosts("rabbitmq-enabled_plugins"),
            ["infra-1", "infra-2", "infra-3"]
        );
        assert_eq!(
            hosts("pki/rabbitmq/server.key"),
            ["infra-1", "infra-2", "infra-3"]
        );
        assert!(
            !hosts("pki/rabbitmq/server.key")
                .iter()
                .any(|host| host == "edge-1"),
            "the broker key reached an edge host"
        );
    }
}
