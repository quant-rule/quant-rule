//! Patroni, etcd, and HAProxy configuration rows.
//!
//! Each row checks the text of every node config (or the rendered tree) against
//! one contract fact; a missing fact names the host. No secret value is read.

use std::collections::BTreeSet;
use std::path::Path;

use crate::descriptor::v3::{Descriptor, Watchdog};
use crate::render::infra;
use crate::render::stateful;

use super::super::report::CheckRow;
use super::read_text;

fn patroni_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("patroni-{host}.yml")),
                "Patroni config",
            )?,
            None => stateful::patroni_yaml(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn etcd_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.etcd.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("etcd-{host}.conf.yml")),
                "etcd config",
            )?,
            None => stateful::etcd_yaml(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn haproxy_texts(
    descriptor: &Descriptor,
    out: Option<&Path>,
) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for host in &descriptor.stateful.haproxy.nodes {
        let text = match out {
            Some(directory) => read_text(
                &directory.join(format!("haproxy-{host}.cfg")),
                "HAProxy config",
            )?,
            None => stateful::haproxy_cfg(descriptor, host).map_err(|error| error.to_string())?,
        };
        texts.push((host.clone(), text));
    }
    Ok(texts)
}

fn unit_text(descriptor: &Descriptor, out: Option<&Path>, host: &str) -> Result<String, String> {
    match out {
        Some(directory) => read_text(
            &directory.join("infra").join(format!("{host}.yaml")),
            "host unit",
        ),
        None => infra::unit_yaml(descriptor, host)
            .ok_or_else(|| format!("the renderer produced no unit for {host}")),
    }
}

fn missing_needles(texts: &[(String, String)], needles: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for (host, text) in texts {
        for needle in needles {
            if !text.contains(needle) {
                missing.push(format!("{host} lacks {needle:?}"));
            }
        }
    }
    missing
}

fn push_requirement(
    rows: &mut Vec<CheckRow>,
    subject: &str,
    state: &str,
    texts: &[(String, String)],
    needles: &[&str],
    detail: &str,
) {
    let missing = missing_needles(texts, needles);
    if missing.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            subject,
            state,
            format!("{} node config(s); {detail}", texts.len()),
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            subject,
            "configuration",
            missing.join("; "),
        ));
    }
}

pub(super) fn patroni_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match patroni_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("stateful", "patroni", "unreadable", error)],
    };
    let mut rows = Vec::new();
    push_requirement(
        &mut rows,
        "patroni",
        "synchronous-mode",
        &texts,
        &["synchronous_mode: true"],
        "synchronous_mode: true",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "synchronous-mode-strict",
        &texts,
        &["synchronous_mode_strict: true"],
        "strict mode refuses an asynchronous fallback",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "synchronous-node-count",
        &texts,
        &["synchronous_node_count: 1"],
        "one synchronous standby",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "synchronous-commit",
        &texts,
        &["synchronous_commit: \"on\""],
        "synchronous_commit: on",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "rewind",
        &texts,
        &["use_pg_rewind: true", "use_slots: true"],
        "pg_rewind and replication slots enabled",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "wal-log-hints",
        &texts,
        &["wal_log_hints: \"on\""],
        "wal_log_hints: on",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "archive",
        &texts,
        &[
            "archive_mode: \"on\"",
            "archive_command: \"",
            "restore_command: \"",
        ],
        "archive/recovery commands are configured placeholders for HA-013",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "tls",
        &texts,
        &[
            "ssl: \"on\"",
            "ssl_cert_file: \"/etc/quantrule/pki/postgres/server.crt\"",
            "ssl_key_file: \"/etc/quantrule/pki/postgres/server.key\"",
            "ssl_ca_file: \"/etc/quantrule/pki/ca.crt\"",
        ],
        "TLS uses the Postgres leaf and the internal CA",
    );
    push_requirement(
        &mut rows,
        "patroni",
        "pg-hba",
        &texts,
        &[
            "hostssl all all 0.0.0.0/0 scram-sha-256",
            "hostssl replication replicator 0.0.0.0/0 scram-sha-256",
        ],
        "hostssl with scram-sha-256; no plaintext host rule",
    );
    let trusted: Vec<String> = texts
        .iter()
        .filter(|(_, text)| text.contains("trust"))
        .map(|(host, _)| format!("{host} contains a trust rule"))
        .collect();
    if trusted.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "pg-hba-no-trust",
            "no trust line in any node config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "pg-hba-no-trust",
            trusted.join("; "),
        ));
    }

    let mut alias_issues = Vec::new();
    for (host, text) in &texts {
        for expected in [
            format!("connect_address: \"patroni-{host}:5432\""),
            format!("connect_address: \"patroni-{host}:8008\""),
        ] {
            if !text.contains(&expected) {
                alias_issues.push(format!("{host} lacks {expected}"));
            }
        }
        for etcd_host in &descriptor.stateful.etcd.nodes {
            let expected = format!("https://etcd-{etcd_host}:2379");
            if !text.contains(&expected) {
                alias_issues.push(format!("{host} lacks {expected}"));
            }
        }
    }
    if alias_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "aliases",
            "connect_address and etcd3 use patroni-<host> and etcd-<host>",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "configuration",
            alias_issues.join("; "),
        ));
    }

    let mut auth_issues = Vec::new();
    for (host, text) in &texts {
        if !text.contains("username: \"patroni\"") {
            auth_issues.push(format!("{host} lacks the patroni REST username"));
        }
        if text.contains("password:") || text.contains("__PATRONI") {
            auth_issues.push(format!("{host} carries a REST password or placeholder"));
        }
    }
    if auth_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "rest-auth",
            "basic-auth username only; the password stays in the mounted file",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "rest-auth",
            auth_issues.join("; "),
        ));
    }

    let expected_mode = descriptor.stateful.postgres.watchdog.as_str();
    let mut watchdog_issues = Vec::new();
    for (host, text) in &texts {
        if !text.contains(&format!("mode: \"{expected_mode}\"")) {
            watchdog_issues.push(format!("{host} lacks watchdog mode {expected_mode}"));
        }
        match descriptor.stateful.postgres.watchdog {
            Watchdog::Off if text.contains("/dev/watchdog") => {
                watchdog_issues.push(format!("{host} names a watchdog device while off"));
            }
            Watchdog::Automatic | Watchdog::Required
                if !text.contains("device: \"/dev/watchdog\"") =>
            {
                watchdog_issues.push(format!("{host} lacks the watchdog device path"));
            }
            _ => {}
        }
    }
    if watchdog_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "watchdog",
            format!("mode {expected_mode}; the desktop device is gated by the mode"),
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "watchdog",
            watchdog_issues.join("; "),
        ));
    }

    let mut unit_issues = Vec::new();
    for host in &descriptor.stateful.postgres.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                unit_issues.push(error);
                continue;
            }
        };
        if !text.contains(&format!("\"patroni-{host}\"")) {
            unit_issues.push(format!("infra/{host}.yaml lacks the patroni-{host} alias"));
        }
        if text.contains("ports:") {
            unit_issues.push(format!("infra/{host}.yaml publishes a port"));
        }
        match descriptor.stateful.postgres.watchdog {
            Watchdog::Required if !text.contains("/dev/watchdog:/dev/watchdog") => {
                unit_issues.push(format!(
                    "infra/{host}.yaml lacks the required watchdog device"
                ));
            }
            Watchdog::Off | Watchdog::Automatic if text.contains("/dev/watchdog") => {
                unit_issues.push(format!(
                    "infra/{host}.yaml maps /dev/watchdog for mode {expected_mode}"
                ));
            }
            _ => {}
        }
    }
    if unit_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "unit",
            "unique alias, no published port, and the watchdog device only for required",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "unit",
            unit_issues.join("; "),
        ));
    }

    let addresses: BTreeSet<&str> = descriptor
        .hosts
        .values()
        .map(|host| host.address.as_str())
        .collect();
    let mut underlay = Vec::new();
    for (host, text) in &texts {
        for address in &addresses {
            if text.contains(*address) {
                underlay.push(format!("{host} contains {address}"));
            }
        }
    }
    if underlay.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "patroni",
            "overlay-only",
            "no host underlay address in any node config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "patroni",
            "overlay-only",
            underlay.join("; "),
        ));
    }
    rows
}

pub(super) fn etcd_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match etcd_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("stateful", "etcd", "unreadable", error)],
    };
    let mut rows = Vec::new();

    let mut member_issues = Vec::new();
    for (host, text) in &texts {
        if !text.contains(&format!("name: \"etcd-{host}\"")) {
            member_issues.push(format!("{host} lacks name etcd-{host}"));
        }
        let members = text.matches("=https://etcd-").count();
        if members != 3 {
            member_issues.push(format!(
                "{host} has {members} initial-cluster members; expected 3"
            ));
        }
        for member in &descriptor.stateful.etcd.nodes {
            let expected = format!("etcd-{member}=https://etcd-{member}:2380");
            if !text.contains(&expected) {
                member_issues.push(format!("{host} lacks {expected}"));
            }
        }
    }
    if member_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "etcd",
            "members",
            "three-member initial cluster with etcd-<host> peer URLs",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "etcd",
            "configuration",
            member_issues.join("; "),
        ));
    }

    push_requirement(
        &mut rows,
        "etcd",
        "client-tls",
        &texts,
        &[
            "cert-file: \"/etc/quantrule/pki/etcd/server.crt\"",
            "key-file: \"/etc/quantrule/pki/etcd/server.key\"",
            "trusted-ca-file: \"/etc/quantrule/pki/ca.crt\"",
            "client-cert-auth: true",
        ],
        "client listener requires a client certificate",
    );
    push_requirement(
        &mut rows,
        "etcd",
        "peer-tls",
        &texts,
        &[
            "cert-file: \"/etc/quantrule/pki/etcd/peer.crt\"",
            "key-file: \"/etc/quantrule/pki/etcd/peer.key\"",
            "trusted-ca-file: \"/etc/quantrule/pki/ca.crt\"",
            "client-cert-auth: true",
        ],
        "peer listener requires mTLS",
    );

    let mut url_issues = Vec::new();
    for (host, text) in &texts {
        for expected in [
            format!("advertise-client-urls: \"https://etcd-{host}:2379\""),
            format!("initial-advertise-peer-urls: \"https://etcd-{host}:2380\""),
        ] {
            if !text.contains(&expected) {
                url_issues.push(format!("{host} lacks {expected}"));
            }
        }
    }
    if url_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "etcd",
            "urls",
            "advertised client and peer URLs use etcd-<host>",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "etcd",
            "configuration",
            url_issues.join("; "),
        ));
    }

    push_requirement(
        &mut rows,
        "etcd",
        "metrics",
        &texts,
        &[
            "metrics: \"prometheus\"",
            "listen-metrics-urls: \"http://0.0.0.0:2381\"",
        ],
        "Prometheus metrics on the overlay; PLAT-006 owns scraping",
    );

    let addresses: BTreeSet<&str> = descriptor
        .hosts
        .values()
        .map(|host| host.address.as_str())
        .collect();
    let mut underlay = Vec::new();
    for (host, text) in &texts {
        for address in &addresses {
            if text.contains(*address) {
                underlay.push(format!("{host} contains {address}"));
            }
        }
    }
    if underlay.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "etcd",
            "overlay-only",
            "no host underlay address in any member config",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "etcd",
            "overlay-only",
            underlay.join("; "),
        ));
    }
    rows
}

pub(super) fn haproxy_rows(descriptor: &Descriptor, out: Option<&Path>) -> Vec<CheckRow> {
    let texts = match haproxy_texts(descriptor, out) {
        Ok(texts) => texts,
        Err(error) => return vec![CheckRow::failed("stateful", "haproxy", "unreadable", error)],
    };
    let mut rows = Vec::new();
    push_requirement(
        &mut rows,
        "haproxy",
        "health-check",
        &texts,
        &[
            "option httpchk GET /primary",
            "http-check expect status 200",
            "http-check send hdr Authorization",
            "check port 8008",
        ],
        "primary-only /primary check on the Patroni REST port",
    );
    let mut server_issues = Vec::new();
    for (host, text) in &texts {
        for member in &descriptor.stateful.postgres.nodes {
            let expected = format!("server patroni-{member} patroni-{member}:5432 check port 8008");
            if !text.contains(&expected) {
                server_issues.push(format!("{host} lacks {expected}"));
            }
        }
    }
    if server_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "haproxy",
            "servers",
            "every Patroni member is a backend server",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "haproxy",
            "configuration",
            server_issues.join("; "),
        ));
    }
    push_requirement(
        &mut rows,
        "haproxy",
        "shutdown-sessions",
        &texts,
        &["on-marked-down shutdown-sessions"],
        "a failed primary check closes its sessions",
    );
    let mut stats_issues = Vec::new();
    for (host, text) in &texts {
        if !text.contains("listen stats") || !text.contains("bind 0.0.0.0:8404") {
            stats_issues.push(format!("{host} lacks the overlay stats listener"));
        }
        if text.contains("bind *:443") || text.contains("bind :443") {
            stats_issues.push(format!("{host} binds 443"));
        }
    }
    if stats_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "haproxy",
            "stats-overlay",
            "stats on 0.0.0.0:8404; no 443 bind and no published port",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "haproxy",
            "stats-overlay",
            stats_issues.join("; "),
        ));
    }

    let mut unit_issues = Vec::new();
    for host in &descriptor.stateful.haproxy.nodes {
        let text = match unit_text(descriptor, out, host) {
            Ok(text) => text,
            Err(error) => {
                unit_issues.push(error);
                continue;
            }
        };
        for alias in [
            format!("\"haproxy-{host}\""),
            format!("\"{}\"", descriptor.endpoints.database.host),
        ] {
            if !text.contains(&alias) {
                unit_issues.push(format!("infra/{host}.yaml lacks the {alias} alias"));
            }
        }
        if text.contains("ports:") {
            unit_issues.push(format!("infra/{host}.yaml publishes a port"));
        }
    }
    if unit_issues.is_empty() {
        rows.push(CheckRow::ok(
            "stateful",
            "haproxy",
            "unit",
            "unique and postgres aliases; no published port",
        ));
    } else {
        rows.push(CheckRow::failed(
            "stateful",
            "haproxy",
            "unit",
            unit_issues.join("; "),
        ));
    }
    rows
}
