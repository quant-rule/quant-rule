//! Unit tests for the v3 observability render.

use super::*;
use serde_json::Value;

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.example.json"))
        .expect("v3 example")
}

fn lab() -> Descriptor {
    serde_json::from_str(include_str!("../../../../../deploy/v3/deploy.lab.json")).expect("v3 lab")
}

#[test]
fn prometheus_scrapes_every_approved_target_on_the_overlay() {
    let text = prometheus_config(&descriptor());
    for expected in [
        "tasks.gateway",
        "tasks.data",
        "tasks.model",
        "tasks.strategy",
        "tasks.backtest",
        "tasks.evidence",
        "tasks.simulation",
        "tasks.ingress",
        "port: 9100",
        "patroni-infra-1:8008",
        "password_file: \"/etc/prometheus/secrets/patroni_rest_password\"",
        "username: \"patroni\"",
        "etcd-infra-1:2381",
        "haproxy-edge-1:8404",
        "rabbitmq-infra-1:15692",
        "\"minio-infra-1:9000\"",
        "scheme: \"https\"",
        "/minio/v2/metrics/cluster",
        "ca_file: \"/etc/quantrule/pki/ca.crt\"",
        "node-infra-1:9100",
        "promtail-infra-1:9080",
        "blackbox:9115",
        "https://203.0.113.20/health",
        "https://203.0.113.20/ready",
        "check: \"health\"",
        "check: \"ready\"",
        "alertmanager:9093",
        "loki:3100",
        "grafana:3000",
        "stack: \"quantrule-ha\"",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    for forbidden in [
        "10.0.1.41",
        "password: \"",
        "10.0.1.11:8082",
        "9090:9090",
        "published",
        "\"metrics\"",
        "hostNetwork",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
}

#[test]
fn alert_rules_cover_f01_to_f17_and_bound_every_label() {
    let text = alert_rules();
    for row in failure_rows() {
        assert!(
            text.contains(&format!("failure_row: \"{row}\"")),
            "{row}: {text}"
        );
    }
    assert_eq!(
        text.matches("failure_row:").count(),
        text.matches("alert: \"").count()
    );
    assert_eq!(text.matches("alert: \"").count(), 38, "{text}");
    for expected in [
        "GatewaySyntheticDown",
        "GatewaySyntheticSlow",
        "GatewayReplicaDown",
        "IngressCertificateExpiring",
        "ServiceNotReady",
        "ServiceTargetDown",
        "DrainStuck",
        "ConsumerReconnectStorm",
        "EvidenceLeaderMissing",
        "EvidenceLeaderTermFlapping",
        "RunsMissingDigest",
        "PatroniNoLeader",
        "PatroniMultipleLeaders",
        "PostgresNoPrimaryBackend",
        "PatroniReplicaLagHigh",
        "PatroniSyncStandbyMissing",
        "EtcdNoLeader",
        "EtcdNodeDown",
        "EtcdQuorumAtRisk",
        "RabbitMqNodeDown",
        "RabbitMqPartition",
        "RabbitMqMemoryAlarm",
        "RabbitMqDiskAlarm",
        "RabbitMqBacklogHigh",
        "MinioNodeOffline",
        "MinioDriveOffline",
        "MinioClusterUnhealthy",
        "MinioHealing",
        "MinioCapacityLow",
        "SwarmManagerQuorumAtRisk",
        "SwarmManagerQuorumLost",
        "SwarmManagerTextfileMissing",
        "NodeFilesystemAlmostFull",
        "NodeMemoryHigh",
        "BackupAgeHigh",
        "VerifyAgeHigh",
        "ObservabilityTargetDown",
        "GatewayTokenOld",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    for forbidden in ["run_id", "object_key", "profile_id", "patroni_master"] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
    // Conservative windows: no critical alert fires immediately.
    assert!(!text.contains("for: \"0s\""), "{text}");
    // Patroni 4 exposes `patroni_primary` and `patroni_sync_standby`; the
    // replica lag is received minus replayed because the location is zero on a
    // replica. MinIO v2 names the drive and erasure-set metrics differently
    // from v1.
    for expected in [
        "sum(patroni_primary) < 1",
        "sum(patroni_primary) > 1",
        "patroni_xlog_received_location - patroni_xlog_replayed_location",
        "sum(patroni_sync_standby) < 1",
        "minio_cluster_drive_offline_total",
        "minio_cluster_health_erasure_set_healing_drives",
        "minio_cluster_health_status",
        "quantrule_swarm_managers_ready < 3",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    for forbidden in [
        "patroni_master",
        "patroni_postgres_streaming",
        "minio_cluster_disk_offline_total",
        "minio_cluster_heal_status",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
}

#[test]
fn alertmanager_has_one_internal_null_receiver_and_no_external_config() {
    let text = alertmanager_config();
    assert_eq!(
        text.matches("name: \"quantrule-null\"").count(),
        1,
        "{text}"
    );
    assert!(text.contains("receiver: \"quantrule-null\""), "{text}");
    for forbidden in [
        "webhook_configs",
        "email_configs",
        "slack_configs",
        "pagerduty_configs",
        "opsgenie_configs",
        "victorops_configs",
        "http://",
        "https://",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
}

#[test]
fn loki_keeps_thirty_days_and_promtail_reads_docker_json() {
    let loki = loki_config();
    for expected in [
        "retention_period: \"720h\"",
        "retention_enabled: true",
        "chunks_directory: \"/loki/chunks\"",
        "store: \"tsdb\"",
        "object_store: \"filesystem\"",
    ] {
        assert!(loki.contains(expected), "{expected}: {loki}");
    }
    for host in descriptor().hosts.keys() {
        let text = promtail_config(&descriptor(), host).expect("promtail");
        for expected in [
            "unix:///var/run/docker.sock",
            "http://loki:3100/loki/api/v1/push",
            "positions.yaml",
            "action: \"keep\"",
            "__meta_docker_container_name",
            "labeldrop",
            "__meta_docker_.*",
            "docker: {}",
        ] {
            assert!(text.contains(expected), "{host} {expected}: {text}");
        }
        assert!(
            text.contains(&format!("replacement: \"{host}\"")),
            "{host}: {text}"
        );
        for forbidden in ["com_docker_compose_project", "com_docker_compose_version"] {
            assert!(!text.contains(forbidden), "{host} {forbidden}: {text}");
        }
    }
}

#[test]
fn blackbox_probes_health_and_ready_through_the_private_ca() {
    let text = blackbox_config(&descriptor());
    for expected in [
        "https_2xx",
        "prober: \"http\"",
        "preferred_ip_protocol: \"ip4\"",
        "ca_file: \"/etc/quantrule/pki/ca.crt\"",
        "server_name: \"quantrule.dev\"",
        "Host: \"quantrule.dev\"",
        "valid_status_codes",
        "200",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
}

#[test]
fn grafana_provisioning_is_read_only_with_fixed_uids() {
    let datasources = grafana_datasources();
    for expected in [
        "uid: \"qr-prometheus\"",
        "uid: \"qr-loki\"",
        "url: \"http://prometheus:9090\"",
        "url: \"http://loki:3100\"",
        "editable: false",
    ] {
        assert!(datasources.contains(expected), "{expected}: {datasources}");
    }
    let provider = grafana_provider();
    for expected in [
        "allowUiUpdates: false",
        "disableDeletion: true",
        "path: \"/etc/grafana/dashboards\"",
        "folder: \"QuantRule\"",
    ] {
        assert!(provider.contains(expected), "{expected}: {provider}");
    }
    for (name, text) in [
        ("overview", OVERVIEW_DASHBOARD),
        ("ha", HA_MATRIX_DASHBOARD),
        ("capacity", CAPACITY_DASHBOARD),
        ("runs", RUNS_DASHBOARD),
    ] {
        let value: Value = serde_json::from_str(text).expect(name);
        assert_eq!(value["editable"], false, "{name}");
        assert_eq!(value["schemaVersion"], 39, "{name}");
        let uid = value["uid"].as_str().expect("uid");
        assert!(uid.starts_with("qr-"), "{name}");
        assert!(text.contains("qr-prometheus"), "{name}");
    }
    let ha: Value = serde_json::from_str(HA_MATRIX_DASHBOARD).expect("ha matrix");
    let backup = ha["panels"][6]["targets"]
        .as_array()
        .expect("backup targets");
    assert_eq!(backup.len(), 2, "{ha}");
    for expected in [
        "quantrule_backup_last_success_timestamp_seconds",
        "quantrule_verify_last_success_timestamp_seconds",
    ] {
        assert!(
            backup.iter().any(|target| target["expr"]
                .as_str()
                .is_some_and(|expr| expr.contains(expected))),
            "{expected}: {ha}"
        );
    }
    let targets = ha["panels"][7]["targets"].as_array().expect("targets");
    assert_eq!(targets.len(), 17, "{ha}");
    for row in failure_rows() {
        assert!(
            HA_MATRIX_DASHBOARD.contains(&format!("failure_row=\\\"{row}\\\"")),
            "{row}"
        );
    }
    for (name, uid) in DASHBOARDS {
        assert!(matches!(
            name,
            "overview.json" | "ha-matrix.json" | "capacity.json" | "runs-evidence.json"
        ));
        assert!(uid.starts_with("qr-"));
    }
}

#[test]
fn every_host_gets_node_exporter_and_promtail_and_the_prometheus_host_gets_blackbox() {
    let descriptor = descriptor();
    for host in descriptor.hosts.keys() {
        let exporter = node_exporter_spec(host);
        assert_eq!(exporter.name, format!("node-exporter-{host}"));
        assert_eq!(exporter.image_key, "node_exporter");
        assert_eq!(exporter.aliases, vec![format!("node-{host}")]);
        assert!(exporter.command.contains(
            &"--collector.textfile.directory=/var/lib/node_exporter/textfile".to_owned()
        ));
        assert!(exporter.volumes.iter().any(|volume| {
            volume.source == "/" && volume.target == "/host" && volume.propagation == Some("rslave")
        }));
        let promtail = promtail_spec(host);
        assert_eq!(promtail.name, format!("promtail-{host}"));
        assert_eq!(promtail.image_key, "promtail");
        assert_eq!(promtail.aliases, vec![format!("promtail-{host}")]);
        assert!(
            promtail
                .volumes
                .iter()
                .any(|volume| volume.source == "/var/run/docker.sock")
        );
    }
    assert_eq!(blackbox_spec().image_key, "blackbox_exporter");
    assert_eq!(prometheus_spec().image_key, "prometheus");
    assert_eq!(alertmanager_spec().image_key, "alertmanager");
    assert_eq!(loki_spec().image_key, "loki");
    assert_eq!(grafana_spec().image_key, "grafana");
}

#[test]
fn observability_units_are_pinned_health_checked_and_log_rotated() {
    let text = super::super::infra::unit_yaml(&descriptor(), "infra-1").expect("infra-1");
    for expected in [
        "\n  prometheus:\n",
        "\n  blackbox:\n",
        "\n  node-exporter-infra-1:\n",
        "\n  promtail-infra-1:\n",
        "healthcheck:",
        "logging:",
        "driver: \"json-file\"",
        "max-size: \"50m\"",
        "max-file: \"5\"",
        "restart: \"unless-stopped\"",
        "stop_grace_period: \"40s\"",
        "--storage.tsdb.retention.time=30d",
        "--storage.tsdb.retention.size=20GB",
        "/etc/quantrule/secrets/patroni_rest_password",
        "/etc/quantrule/pki/ca.crt",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert!(!text.contains("ports:"), "{text}");
    let alerting = super::super::infra::unit_yaml(&descriptor(), "infra-2").expect("infra-2");
    for expected in [
        "\n  alertmanager:\n",
        "\n  loki:\n",
        "--data.retention=120h",
        "/etc/quantrule/loki/loki.yml",
    ] {
        assert!(alerting.contains(expected), "{expected}: {alerting}");
    }
    let grafana = super::super::infra::unit_yaml(&descriptor(), "infra-1").expect("grafana");
    for expected in [
        "GF_AUTH_ANONYMOUS_ENABLED: \"true\"",
        "GF_AUTH_ANONYMOUS_ORG_ROLE: \"Viewer\"",
        "GF_AUTH_BASIC_ENABLED: \"false\"",
        "GF_AUTH_DISABLE_LOGIN_FORM: \"true\"",
        "GF_USERS_ALLOW_SIGN_UP: \"false\"",
        "GF_SECURITY_ADMIN_PASSWORD",
    ] {
        if expected.ends_with("GF_SECURITY_ADMIN_PASSWORD") {
            assert!(
                !grafana.contains(expected),
                "a default admin secret is rendered: {grafana}"
            );
        } else {
            assert!(grafana.contains(expected), "{expected}: {grafana}");
        }
    }
}

#[test]
fn the_observability_manifest_sends_material_only_to_the_right_hosts() {
    let text = observability_manifest(&descriptor()).expect("manifest");
    let value: Value = serde_json::from_str(&text).expect("manifest JSON");
    let files = value["files"].as_array().expect("files");
    let selector = |host: &str| {
        files
            .iter()
            .filter(|entry| {
                entry["hosts"]
                    .as_array()
                    .is_some_and(|hosts| hosts.iter().any(|value| value == host))
            })
            .filter_map(|entry| entry["source"].as_str())
            .collect::<Vec<_>>()
    };
    let prometheus = selector("infra-1");
    assert!(prometheus.contains(&"prometheus.yml"), "{prometheus:?}");
    assert!(prometheus.contains(&"alert-rules.yml"), "{prometheus:?}");
    assert!(prometheus.contains(&"blackbox.yml"), "{prometheus:?}");
    assert!(
        prometheus.contains(&"secrets/patroni_rest_password"),
        "{prometheus:?}"
    );
    assert!(
        prometheus.contains(&"pki/database-ca.crt"),
        "{prometheus:?}"
    );
    assert!(!prometheus.contains(&"alertmanager.yml"), "{prometheus:?}");
    assert_eq!(prometheus.len(), 12, "{prometheus:?}");

    let alertmanager = selector("infra-2");
    assert!(
        alertmanager.contains(&"alertmanager.yml"),
        "{alertmanager:?}"
    );
    assert!(
        !alertmanager.contains(&"prometheus.yml"),
        "{alertmanager:?}"
    );
    assert!(
        !alertmanager.contains(&"secrets/patroni_rest_password"),
        "{alertmanager:?}"
    );

    let grafana = selector("infra-1");
    assert!(
        grafana.contains(&"grafana/dashboards/ha-matrix.json"),
        "{grafana:?}"
    );
    for host in ["edge-1", "core-1", "compute-1"] {
        let selected = selector(host);
        assert!(
            selected.contains(&format!("promtail-{host}.yml").as_str()),
            "{host}: {selected:?}"
        );
    }
    assert!(!text.contains("10.0.1.41"), "{text}");
}

#[test]
fn metrics_exposure_is_internal_only() {
    assert!(crate::render::ingress::STATIC_CONFIG.contains("address: \":8082\""));
    assert!(crate::render::ingress::STATIC_CONFIG.contains("entryPoint: metrics"));
    assert!(!crate::render::ingress::STATIC_CONFIG.contains("address: \":80\""));
    let descriptor = descriptor();
    let haproxy = crate::render::stateful::haproxy_cfg(&descriptor, "edge-1").expect("haproxy");
    assert!(haproxy.contains("bind 0.0.0.0:8404"), "{haproxy}");
    assert!(
        haproxy.contains("prometheus-exporter if { path /metrics }"),
        "{haproxy}"
    );
    assert!(!haproxy.contains("bind *:9090"), "{haproxy}");
    let minio = crate::render::minio::minio_conf(&descriptor, "infra-1").expect("minio");
    assert!(
        minio.contains("MINIO_PROMETHEUS_AUTH_TYPE=\"public\""),
        "{minio}"
    );
    let unit = super::super::infra::unit_yaml(&descriptor, "infra-1").expect("unit");
    assert!(!unit.contains("ports:"), "{unit}");
    assert!(!unit.contains("8082"), "{unit}");
    assert!(!unit.contains("8404"), "{unit}");
}

#[test]
fn the_lab_uses_the_same_observability_contract() {
    let descriptor = lab();
    let prometheus = prometheus_config(&descriptor);
    assert!(prometheus.contains("tasks.gateway"), "{prometheus}");
    assert!(prometheus.contains("minio-lab-1:9000"), "{prometheus}");
    assert!(!prometheus.contains("minio-infra-1"), "{prometheus}");
    for host in descriptor.hosts.keys() {
        assert!(promtail_config(&descriptor, host).is_ok(), "{host}");
    }
    let manifest = observability_manifest(&descriptor).expect("manifest");
    assert!(manifest.contains("promtail-lab-3.yml"), "{manifest}");
}
