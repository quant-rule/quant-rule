//! Alert rule and Alertmanager rows.
//!
//! Every rule carries a `severity` and one bounded `failure_row` label
//! `F01`..`F17`. The matrix below is the guard: a row without a rule fails the
//! check. The expressions use the documented Patroni 4 and MinIO v2 families.

use std::collections::BTreeSet;
use std::path::Path;

use crate::render::observability;

use super::super::report::CheckRow;
use super::{rendered, valid_row};

/// The alert names the contract requires.
const REQUIRED_ALERTS: [&str; 38] = [
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
];

/// The HA failure matrix: every row must still have at least one rule.
const HA_MATRIX: [(&str, &[&str]); 17] = [
    (
        "F01",
        &[
            "GatewayReplicaDown",
            "GatewaySyntheticDown",
            "GatewaySyntheticSlow",
            "IngressCertificateExpiring",
        ],
    ),
    ("F02", &["ServiceNotReady"]),
    ("F03", &["ServiceTargetDown"]),
    ("F04", &["DrainStuck"]),
    ("F05", &["ConsumerReconnectStorm"]),
    (
        "F06",
        &["EvidenceLeaderMissing", "EvidenceLeaderTermFlapping"],
    ),
    ("F07", &["RunsMissingDigest"]),
    (
        "F08",
        &[
            "PatroniNoLeader",
            "PatroniMultipleLeaders",
            "PostgresNoPrimaryBackend",
        ],
    ),
    ("F09", &["PatroniReplicaLagHigh"]),
    ("F10", &["PatroniSyncStandbyMissing"]),
    ("F11", &["EtcdNoLeader", "EtcdNodeDown", "EtcdQuorumAtRisk"]),
    ("F12", &["RabbitMqNodeDown", "RabbitMqPartition"]),
    (
        "F13",
        &[
            "RabbitMqMemoryAlarm",
            "RabbitMqDiskAlarm",
            "RabbitMqBacklogHigh",
        ],
    ),
    (
        "F14",
        &[
            "MinioNodeOffline",
            "MinioDriveOffline",
            "MinioClusterUnhealthy",
        ],
    ),
    ("F15", &["MinioHealing", "MinioCapacityLow"]),
    (
        "F16",
        &[
            "BackupAgeHigh",
            "VerifyAgeHigh",
            "ObservabilityTargetDown",
            "GatewayTokenOld",
        ],
    ),
    (
        "F17",
        &[
            "SwarmManagerQuorumAtRisk",
            "SwarmManagerQuorumLost",
            "SwarmManagerTextfileMissing",
            "NodeFilesystemAlmostFull",
            "NodeMemoryHigh",
        ],
    ),
];

/// Receiver keys that would leave the host or add an external integration.
const EXTERNAL_RECEIVERS: [&str; 13] = [
    "webhook_configs",
    "email_configs",
    "slack_configs",
    "pagerduty_configs",
    "opsgenie_configs",
    "victorops_configs",
    "wechat_configs",
    "pushover_configs",
    "sns_configs",
    "telegram_configs",
    "webex_configs",
    "msteams_configs",
    "discord_configs",
];

#[derive(Debug, Clone)]
struct Rule {
    name: String,
    expr: String,
    duration: String,
    severity: String,
    row: String,
    block: String,
}

/// The alert-rule rows in check order.
pub(super) fn rule_rows(out: Option<&Path>) -> Vec<CheckRow> {
    let text = match rendered(out, "alert-rules.yml", || Ok(observability::alert_rules())) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed(
                "observability",
                "alert-rules.yml",
                "unreadable",
                error,
            )];
        }
    };
    let rules = parse_rules(&text);
    vec![
        coverage_row(&rules),
        label_row(&rules),
        metric_row(&text, &rules),
        ha_row(&rules),
    ]
}

fn coverage_row(rules: &[Rule]) -> CheckRow {
    let names: BTreeSet<&str> = rules.iter().map(|rule| rule.name.as_str()).collect();
    let missing_alerts: Vec<&str> = REQUIRED_ALERTS
        .iter()
        .copied()
        .filter(|name| !names.contains(name))
        .collect();
    if missing_alerts.is_empty() {
        CheckRow::ok(
            "observability",
            "alert-rules.yml",
            "coverage",
            format!(
                "{} rules cover the {} required alert names; F01..F17 each have a rule",
                rules.len(),
                REQUIRED_ALERTS.len()
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "alert-rules.yml",
            "coverage",
            format!("missing alerts: {}", missing_alerts.join(", ")),
        )
    }
}

fn label_row(rules: &[Rule]) -> CheckRow {
    let mut issues = Vec::new();
    let mut seen_rows = BTreeSet::new();
    let mut seen_names = BTreeSet::new();
    for rule in rules {
        if !seen_names.insert(rule.name.as_str()) {
            issues.push(format!("duplicate alert {}", rule.name));
        }
        if !matches!(rule.severity.as_str(), "critical" | "warning") {
            issues.push(format!("{} has severity {:?}", rule.name, rule.severity));
        }
        if !valid_row(&rule.row) {
            issues.push(format!("{} has failure_row {:?}", rule.name, rule.row));
        } else {
            seen_rows.insert(rule.row.clone());
        }
        for forbidden in ["run_id:", "object_key:", "profile_id:", "token:"] {
            if rule.block.contains(forbidden) {
                issues.push(format!("{} carries the label {forbidden}", rule.name));
            }
        }
    }
    for row in (1..=17).map(|row| format!("F{row:02}")) {
        if !seen_rows.contains(&row) {
            issues.push(format!("{row} has no rule"));
        }
    }
    for (row, expected) in HA_MATRIX {
        let present: BTreeSet<&str> = rules
            .iter()
            .filter(|rule| rule.row == row)
            .map(|rule| rule.name.as_str())
            .collect();
        if present.is_empty() {
            issues.push(format!("the HA matrix row {row} has no alert"));
        }
        for alert in expected {
            if !present.contains(alert) {
                issues.push(format!("{row} lacks {alert}"));
            }
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "alert-rules.yml",
            "labels",
            format!(
                "every rule has severity critical/warning and one bounded F01..F17 row; the {}-row HA matrix has an alert",
                HA_MATRIX.len()
            ),
        )
    } else {
        CheckRow::failed(
            "observability",
            "alert-rules.yml",
            "labels",
            issues.join("; "),
        )
    }
}

fn metric_row(text: &str, rules: &[Rule]) -> CheckRow {
    let mut issues = Vec::new();
    for (name, expr) in [
        ("PatroniNoLeader", "sum(patroni_primary) < 1"),
        ("PatroniMultipleLeaders", "sum(patroni_primary) > 1"),
        (
            "PatroniReplicaLagHigh",
            "max(patroni_xlog_received_location - patroni_xlog_replayed_location) > 67108864",
        ),
        ("PatroniSyncStandbyMissing", "sum(patroni_sync_standby) < 1"),
        (
            "MinioDriveOffline",
            "max(minio_cluster_drive_offline_total) > 0",
        ),
        (
            "MinioHealing",
            "max(minio_cluster_health_erasure_set_healing_drives) > 0",
        ),
        ("MinioClusterUnhealthy", "minio_cluster_health_status < 1"),
    ] {
        match rules.iter().find(|rule| rule.name == name) {
            Some(rule) if rule.expr == expr => {}
            Some(rule) => issues.push(format!("{name} is {:?}; expected {:?}", rule.expr, expr)),
            None => issues.push(format!("missing {name}")),
        }
    }
    for forbidden in [
        "patroni_master",
        "patroni_postgres_streaming",
        "minio_cluster_disk_offline_total",
        "minio_cluster_heal_status",
    ] {
        if text.contains(forbidden) {
            issues.push(format!(
                "the rules still use the retired metric {forbidden}"
            ));
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "alert-rules.yml",
            "metrics",
            "Patroni 4 uses patroni_primary/patroni_sync_standby and received-minus-replayed lag; MinIO v2 uses drive-offline, erasure-set healing, and cluster health",
        )
    } else {
        CheckRow::failed(
            "observability",
            "alert-rules.yml",
            "metrics",
            issues.join("; "),
        )
    }
}

fn ha_row(rules: &[Rule]) -> CheckRow {
    let mut issues = Vec::new();
    for (name, expr, duration) in [
        (
            "GatewayReplicaDown",
            "up{job=\"app\", service=\"gateway\"} == 0",
            "5m",
        ),
        ("EtcdNodeDown", "sum(up{job=\"etcd\"}) < 3", "2m"),
        (
            "PostgresNoPrimaryBackend",
            "haproxy_backend_active_servers{proxy=\"postgres_primary\"} < 1",
            "5m",
        ),
        (
            "MinioClusterUnhealthy",
            "minio_cluster_health_status < 1",
            "5m",
        ),
        (
            "SwarmManagerQuorumAtRisk",
            "quantrule_swarm_managers_ready < 3",
            "10m",
        ),
        (
            "SwarmManagerQuorumLost",
            "quantrule_swarm_managers_ready < 2",
            "5m",
        ),
    ] {
        match rules.iter().find(|rule| rule.name == name) {
            Some(rule) if rule.expr == expr && rule.duration == duration => {}
            Some(rule) => issues.push(format!(
                "{name} is {:?} for {}; expected {:?} for {duration}",
                rule.expr, rule.duration, expr
            )),
            None => issues.push(format!("missing {name}")),
        }
    }
    if issues.is_empty() {
        CheckRow::ok(
            "observability",
            "alert-rules.yml",
            "ha-coverage",
            "gateway replica, etcd node, HAProxy primary backend, MinIO health, and manager quorum have bounded HA rules",
        )
    } else {
        CheckRow::failed(
            "observability",
            "alert-rules.yml",
            "ha-coverage",
            issues.join("; "),
        )
    }
}

/// The Alertmanager receiver and routing rows.
pub(super) fn alertmanager_rows(out: Option<&Path>) -> Vec<CheckRow> {
    let text = match rendered(out, "alertmanager.yml", || {
        Ok(observability::alertmanager_config())
    }) {
        Ok(text) => text,
        Err(error) => {
            return vec![CheckRow::failed(
                "observability",
                "alertmanager.yml",
                "unreadable",
                error,
            )];
        }
    };
    let mut receiver = Vec::new();
    if text.matches("name: \"quantrule-null\"").count() != 1 {
        receiver.push("expected exactly one quantrule-null receiver".to_owned());
    }
    if !text.contains("receiver: \"quantrule-null\"") {
        receiver.push("the route does not name quantrule-null".to_owned());
    }
    for key in EXTERNAL_RECEIVERS {
        if text.contains(key) {
            receiver.push(format!("found external receiver key {key}"));
        }
    }
    vec![
        if receiver.is_empty() {
            CheckRow::ok(
                "observability",
                "alertmanager.yml",
                "receiver",
                "one quantrule-null receiver; no external receiver configuration",
            )
        } else {
            CheckRow::failed(
                "observability",
                "alertmanager.yml",
                "receiver",
                receiver.join("; "),
            )
        },
        if text.contains("group_by:\n    -\n      \"alertname\"\n    -\n      \"failure_row\"")
            && text.contains("inhibit_rules:")
            && text.contains("source_matchers:")
            && text.contains("target_matchers:")
            && text.contains(
                "equal:\n      -\n        \"alertname\"\n      -\n        \"failure_row\"",
            )
        {
            CheckRow::ok(
                "observability",
                "alertmanager.yml",
                "routing",
                "group_by alertname and failure_row; a critical-over-warning inhibit rule shares both labels",
            )
        } else {
            CheckRow::failed(
                "observability",
                "alertmanager.yml",
                "routing",
                "group_by or the inhibit rule does not carry alertname and failure_row",
            )
        },
    ]
}

fn parse_rules(text: &str) -> Vec<Rule> {
    text.split("\n        alert: \"")
        .skip(1)
        .map(|chunk| {
            let (name, _) = chunk.split_once('"').unwrap_or((chunk, ""));
            Rule {
                name: name.to_owned(),
                expr: field(chunk, "expr"),
                duration: field(chunk, "for"),
                severity: field(chunk, "severity"),
                row: field(chunk, "failure_row"),
                block: chunk.to_owned(),
            }
        })
        .collect()
}

fn field(block: &str, key: &str) -> String {
    let prefix = format!("{key}: \"");
    block
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix(&prefix)
                .and_then(|value| value.strip_suffix('"'))
                .map(|value| value.replace("\\\"", "\""))
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rules() -> String {
        crate::render::observability::alert_rules()
    }

    #[test]
    fn the_rendered_rules_parse_into_every_required_alert() {
        let rules = parse_rules(&sample_rules());
        assert_eq!(rules.len(), REQUIRED_ALERTS.len());
        for name in REQUIRED_ALERTS {
            assert!(rules.iter().any(|rule| rule.name == name), "{name}");
        }
        for rule in &rules {
            assert!(valid_row(&rule.row), "{}: {}", rule.name, rule.row);
            assert!(!rule.expr.is_empty(), "{}", rule.name);
            assert!(!rule.duration.is_empty(), "{}", rule.name);
        }
    }

    #[test]
    fn the_ha_matrix_maps_every_row_to_a_rule() {
        let rules = parse_rules(&sample_rules());
        for (row, expected) in HA_MATRIX {
            for alert in expected {
                assert!(
                    rules
                        .iter()
                        .any(|rule| rule.row == row && rule.name == *alert),
                    "{row} lacks {alert}"
                );
            }
        }
    }

    #[test]
    fn the_rule_unit_fixture_names_gateway_synthetic_down() {
        let fixture = include_str!(
            "../../../../../deploy/v3/observability/promtool/gateway-synthetic-down.yml"
        );
        assert!(
            fixture.contains("alertname: GatewaySyntheticDown"),
            "{fixture}"
        );
        assert!(fixture.contains("failure_row: F01"), "{fixture}");
        assert!(fixture.contains("check=\"ready\""), "{fixture}");
    }
}
