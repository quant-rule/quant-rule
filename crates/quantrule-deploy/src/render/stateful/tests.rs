use super::*;

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
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
        serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.lab.json"))
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
            "minio",
            "minio-infra-1",
            "minio-infra-2",
            "minio-infra-3",
            "minio-infra-4",
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
    for host in &descriptor.stateful.minio.nodes {
        assert!(
            haproxy.contains(&format!("server minio-{host} minio-{host}:9000 check")),
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
        "frontend minio",
        "bind *:9000",
        "backend minio_nodes",
        "server minio-infra-1 minio-infra-1:9000 check",
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
        "minio-infra-1.conf",
        "/etc/quantrule/minio/minio.conf",
        "minio-policy.json",
        "/etc/quantrule/minio/quantrule-app-policy.json",
        "secrets/minio_root_user",
        "secrets/minio_root_password",
        "secrets/object_store_access_key",
        "secrets/patroni_rest_password",
        "/etc/quantrule/secrets/patroni_rest_password",
        "secrets/rabbitmq_erlang_cookie",
        "/etc/quantrule/rabbitmq/erlang.cookie",
        "pki/postgres/server.key",
        "pki/etcd/client.key",
        "pki/rabbitmq/server.key",
        "pki/minio/public.crt",
        "pki/minio/private.key",
        "pki/minio/CAs/ca.crt",
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
    assert_eq!(hosts("minio-infra-1.conf"), ["infra-1"]);
    assert_eq!(
        hosts("minio-policy.json"),
        ["infra-1", "infra-2", "infra-3", "infra-4"]
    );
    assert_eq!(
        hosts("secrets/minio_root_user"),
        ["infra-1", "infra-2", "infra-3", "infra-4"]
    );
    assert_eq!(
        hosts("secrets/minio_root_password"),
        ["infra-1", "infra-2", "infra-3", "infra-4"]
    );
    assert_eq!(
        hosts("secrets/object_store_secret_key"),
        ["infra-1", "infra-2", "infra-3", "infra-4"]
    );
    assert_eq!(
        hosts("pki/minio/private.key"),
        ["infra-1", "infra-2", "infra-3", "infra-4"]
    );
    assert!(
        !hosts("secrets/minio_root_password")
            .iter()
            .any(|host| host == "edge-1"),
        "the MinIO root password reached an edge host"
    );
    assert!(
        !hosts("pki/minio/private.key")
            .iter()
            .any(|host| host == "edge-1"),
        "the MinIO key reached an edge host"
    );
    assert!(
        !hosts("pki/rabbitmq/server.key")
            .iter()
            .any(|host| host == "edge-1"),
        "the broker key reached an edge host"
    );
}
