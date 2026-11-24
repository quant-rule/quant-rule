//! The 17-row alert rules and the internal Alertmanager receiver.
//!
//! Every rule carries `severity` and a bounded `failure_row`; Alertmanager
//! routes to one internal null receiver and has no external configuration.

use super::super::yaml::Yaml;

/// The 38 rules across the 17 bounded failure rows.
#[must_use]
pub fn alert_rules() -> String {
    let rules: [(&str, &str, &str, &str, &str, &str); 38] = [
        (
            "GatewaySyntheticDown",
            "probe_success{job=\"blackbox\"} == 0",
            "2m",
            "critical",
            "F01",
            "the synthetic ingress probe failed",
        ),
        (
            "GatewaySyntheticSlow",
            "probe_duration_seconds{job=\"blackbox\"} > 2",
            "10m",
            "warning",
            "F01",
            "the synthetic ingress probe is slow",
        ),
        (
            "GatewayReplicaDown",
            "up{job=\"app\", service=\"gateway\"} == 0",
            "5m",
            "critical",
            "F01",
            "a gateway replica is not scraped",
        ),
        (
            "IngressCertificateExpiring",
            "probe_ssl_earliest_cert_expiry{job=\"blackbox\"} - time() < 14 * 24 * 3600",
            "1h",
            "warning",
            "F01",
            "the ingress certificate expires within 14 days",
        ),
        (
            "ServiceNotReady",
            "quantrule_ready{job=\"app\"} == 0 unless on(instance) quantrule_draining == 1",
            "5m",
            "critical",
            "F02",
            "a service replica is not ready and is not draining",
        ),
        (
            "ServiceTargetDown",
            "up{job=\"app\"} == 0",
            "5m",
            "warning",
            "F03",
            "a service replica is not scraped",
        ),
        (
            "DrainStuck",
            "quantrule_draining == 1",
            "15m",
            "warning",
            "F04",
            "a drain did not complete",
        ),
        (
            "ConsumerReconnectStorm",
            "increase(quantrule_consumer_reconnects_total[15m]) > 5",
            "15m",
            "warning",
            "F05",
            "a consumer reconnected more than five times in fifteen minutes",
        ),
        (
            "EvidenceLeaderMissing",
            "sum(quantrule_leader{service=\"evidence\"}) < 1",
            "2m",
            "critical",
            "F06",
            "no evidence replica holds the leader lease",
        ),
        (
            "EvidenceLeaderTermFlapping",
            "changes(quantrule_leader_term{service=\"evidence\"}[30m]) > 3",
            "5m",
            "warning",
            "F06",
            "the evidence leader term changed more than three times in thirty minutes",
        ),
        (
            "RunsMissingDigest",
            "quantrule_runs_missing_digest > 0",
            "15m",
            "critical",
            "F07",
            "a succeeded run is missing its digest",
        ),
        (
            "PatroniNoLeader",
            "sum(patroni_primary) < 1",
            "2m",
            "critical",
            "F08",
            "no Patroni member reports the primary role",
        ),
        (
            "PatroniMultipleLeaders",
            "sum(patroni_primary) > 1",
            "2m",
            "critical",
            "F08",
            "more than one Patroni member reports the primary role",
        ),
        (
            "PostgresNoPrimaryBackend",
            "haproxy_backend_active_servers{proxy=\"postgres_primary\"} < 1",
            "5m",
            "critical",
            "F08",
            "HAProxy has no active backend for the Patroni primary",
        ),
        (
            "PatroniReplicaLagHigh",
            "max(patroni_xlog_received_location - patroni_xlog_replayed_location) > 67108864",
            "10m",
            "warning",
            "F09",
            "a Patroni replica lags by more than 64 MiB",
        ),
        (
            "PatroniSyncStandbyMissing",
            "sum(patroni_sync_standby) < 1",
            "5m",
            "critical",
            "F10",
            "no synchronous standby is registered",
        ),
        (
            "EtcdNoLeader",
            "max(etcd_server_has_leader) == 0",
            "1m",
            "critical",
            "F11",
            "etcd reports no leader",
        ),
        (
            "EtcdNodeDown",
            "sum(up{job=\"etcd\"}) < 3",
            "2m",
            "warning",
            "F11",
            "fewer than three etcd members are scraped",
        ),
        (
            "EtcdQuorumAtRisk",
            "sum(up{job=\"etcd\"}) < 2",
            "2m",
            "critical",
            "F11",
            "fewer than two etcd members are scraped",
        ),
        (
            "RabbitMqNodeDown",
            "sum(rabbitmq_running) < 3",
            "3m",
            "critical",
            "F12",
            "fewer than three RabbitMQ nodes are running",
        ),
        (
            "RabbitMqPartition",
            "max(rabbitmq_partitions) > 0",
            "3m",
            "critical",
            "F12",
            "RabbitMQ reports a network partition",
        ),
        (
            "RabbitMqMemoryAlarm",
            "max(rabbitmq_alarms_memory_used_watermark) > 0",
            "5m",
            "warning",
            "F13",
            "a RabbitMQ memory alarm is active",
        ),
        (
            "RabbitMqDiskAlarm",
            "max(rabbitmq_alarms_free_disk_space_watermark) > 0",
            "5m",
            "warning",
            "F13",
            "a RabbitMQ disk-free alarm is active",
        ),
        (
            "RabbitMqBacklogHigh",
            "sum(rabbitmq_queue_messages_ready{queue=~\"runs[.].*|stages[.].*\"}) > 1000",
            "15m",
            "warning",
            "F13",
            "the run or stage queue backlog exceeds 1000 messages",
        ),
        (
            "MinioNodeOffline",
            "max(minio_cluster_nodes_offline_total) > 0",
            "3m",
            "critical",
            "F14",
            "a MinIO node is offline",
        ),
        (
            "MinioDriveOffline",
            "max(minio_cluster_drive_offline_total) > 0",
            "3m",
            "critical",
            "F14",
            "a MinIO drive is offline",
        ),
        (
            "MinioClusterUnhealthy",
            "minio_cluster_health_status < 1",
            "5m",
            "critical",
            "F14",
            "the MinIO cluster health check is failing",
        ),
        (
            "MinioHealing",
            "max(minio_cluster_health_erasure_set_healing_drives) > 0",
            "30m",
            "warning",
            "F15",
            "MinIO is healing an erasure set",
        ),
        (
            "MinioCapacityLow",
            "minio_cluster_capacity_usable_free_bytes / minio_cluster_capacity_usable_total_bytes < 0.15",
            "30m",
            "warning",
            "F15",
            "MinIO usable capacity is below 15 percent",
        ),
        (
            "SwarmManagerQuorumAtRisk",
            "quantrule_swarm_managers_ready < 3",
            "10m",
            "warning",
            "F17",
            "fewer than three Swarm managers are ready",
        ),
        (
            "SwarmManagerQuorumLost",
            "quantrule_swarm_managers_ready < 2",
            "5m",
            "critical",
            "F17",
            "fewer than two Swarm managers are ready",
        ),
        (
            "SwarmManagerTextfileMissing",
            "absent(quantrule_swarm_managers_ready)",
            "15m",
            "warning",
            "F17",
            "the manager-quorum textfile is missing",
        ),
        (
            "NodeFilesystemAlmostFull",
            "100 - (node_filesystem_avail_bytes{fstype!~\"tmpfs|overlay\"} / node_filesystem_size_bytes{fstype!~\"tmpfs|overlay\"} * 100) > 90",
            "15m",
            "warning",
            "F17",
            "a host filesystem is more than 90 percent full",
        ),
        (
            "NodeMemoryHigh",
            "100 * (1 - node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes) > 90",
            "15m",
            "warning",
            "F17",
            "a host uses more than 90 percent of memory",
        ),
        (
            "BackupAgeHigh",
            "absent(quantrule_backup_last_success_timestamp_seconds) or time() - quantrule_backup_last_success_timestamp_seconds > 8 * 24 * 3600",
            "1h",
            "critical",
            "F16",
            "the last successful backup is more than eight days old, or the backup-age textfile is missing",
        ),
        (
            "VerifyAgeHigh",
            "time() - quantrule_verify_last_success_timestamp_seconds > 8 * 24 * 3600",
            "1h",
            "warning",
            "F16",
            "the last successful verification is more than eight days old",
        ),
        (
            "ObservabilityTargetDown",
            "up{job=~\"prometheus|alertmanager|loki|grafana|promtail|blackbox|node\"} == 0",
            "5m",
            "warning",
            "F16",
            "an observability target is down",
        ),
        (
            "GatewayTokenOld",
            "quantrule_auth_token_age_seconds > 90 * 24 * 3600",
            "1h",
            "warning",
            "F16",
            "the gateway bearer token is more than 90 days old",
        ),
    ];
    let rendered: Vec<Yaml> = rules
        .iter()
        .map(|(alert, expr, duration, severity, row, summary)| {
            Yaml::map()
                .with("alert", Yaml::string(*alert))
                .with("expr", Yaml::string(*expr))
                .with("for", Yaml::string(*duration))
                .with(
                    "labels",
                    Yaml::map()
                        .with("severity", Yaml::string(*severity))
                        .with("failure_row", Yaml::string(*row)),
                )
                .with(
                    "annotations",
                    Yaml::map()
                        .with("summary", Yaml::string(*summary))
                        .with(
                            "runbook",
                            Yaml::string(
                                "https://github.com/quant-rule/quant-rule/blob/main/docs/manual/16-observability.md",
                            ),
                        ),
                )
        })
        .collect();
    Yaml::map()
        .with(
            "groups",
            Yaml::list([Yaml::map()
                .with("name", Yaml::string("quantrule-ha"))
                .with("rules", Yaml::list(rendered))]),
        )
        .render()
}

/// The Alertmanager configuration with one internal null receiver.
#[must_use]
pub fn alertmanager_config() -> String {
    Yaml::map()
        .with(
            "global",
            Yaml::map().with("resolve_timeout", Yaml::string("5m")),
        )
        .with(
            "route",
            Yaml::map()
                .with(
                    "group_by",
                    Yaml::list([Yaml::string("alertname"), Yaml::string("failure_row")]),
                )
                .with("group_interval", Yaml::string("5m"))
                .with("group_wait", Yaml::string("30s"))
                .with("receiver", Yaml::string("quantrule-null"))
                .with("repeat_interval", Yaml::string("4h")),
        )
        .with(
            "receivers",
            Yaml::list([Yaml::map().with("name", Yaml::string("quantrule-null"))]),
        )
        .with(
            "inhibit_rules",
            Yaml::list([Yaml::map()
                .with(
                    "source_matchers",
                    Yaml::list([Yaml::string("severity=\"critical\"")]),
                )
                .with(
                    "target_matchers",
                    Yaml::list([Yaml::string("severity=\"warning\"")]),
                )
                .with(
                    "equal",
                    Yaml::list([Yaml::string("alertname"), Yaml::string("failure_row")]),
                )]),
        )
        .render()
}
