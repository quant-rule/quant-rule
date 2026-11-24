//! Prometheus, Alertmanager, Loki, promtail, blackbox, and Grafana for v3.
//!
//! The descriptor places the four observability units on declared hosts; this
//! module writes the scrape and alert contract, the internal null receiver, the
//! log pipeline, the synthetic probes, the Grafana provisioning and four
//! checked-in dashboards, and the per-host manifest the operator tool copies.
//! Every unit exposes its port on the encrypted overlay only. No rendered file
//! carries a secret value: the Patroni REST password is a mounted file, and the
//! MinIO scrape relies on overlay-only public metrics auth.
//!
//! Alert rules are bounded: every rule carries `severity` and `failure_row`
//! (`F01`..`F17`). A label never carries a run id, a path, a token, or an
//! unbounded upstream value.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::descriptor::v3::Descriptor;
use crate::error::Error;

use super::{create_dir, safe_component, write_public};

mod configs;
mod rules;

pub use configs::*;
pub use rules::*;

/// The Prometheus HTTP port on the overlay.
pub const PROMETHEUS_PORT: u16 = 9090;
/// The Alertmanager HTTP port on the overlay.
pub const ALERTMANAGER_PORT: u16 = 9093;
/// The Loki HTTP port on the overlay.
pub const LOKI_PORT: u16 = 3100;
/// The Grafana HTTP port on the overlay.
pub const GRAFANA_PORT: u16 = 3000;
/// The blackbox exporter HTTP port on the overlay.
pub const BLACKBOX_PORT: u16 = 9115;
/// The node_exporter HTTP port on the overlay.
pub const NODE_EXPORTER_PORT: u16 = 9100;
/// The promtail HTTP (readiness) port on the overlay.
pub const PROMTAIL_PORT: u16 = 9080;
/// The internal Traefik metrics port scraped through `tasks.ingress`.
pub const TRAEFIK_METRICS_PORT: u16 = 8082;
/// The HAProxy stats/native-metrics port.
pub const HAPROXY_METRICS_PORT: u16 = 8404;
/// The RabbitMQ Prometheus plugin port.
pub const RABBITMQ_METRICS_PORT: u16 = 15692;
/// The etcd metrics port.
pub const ETCD_METRICS_PORT: u16 = 2381;
/// The Patroni REST metrics port.
pub const PATRONI_REST_PORT: u16 = 8008;

/// The in-container Prometheus configuration path.
pub const PROMETHEUS_CONFIG_TARGET: &str = "/etc/prometheus/prometheus.yml";
/// The in-container alert-rules path.
pub const PROMETHEUS_RULES_TARGET: &str = "/etc/prometheus/alert-rules.yml";
/// The in-container Alertmanager configuration path.
pub const ALERTMANAGER_CONFIG_TARGET: &str = "/etc/alertmanager/alertmanager.yml";
/// The in-container Loki configuration path.
pub const LOKI_CONFIG_TARGET: &str = "/etc/loki/loki.yml";
/// The in-container promtail configuration path.
pub const PROMTAIL_CONFIG_TARGET: &str = "/etc/promtail/promtail.yml";
/// The in-container blackbox configuration path.
pub const BLACKBOX_CONFIG_TARGET: &str = "/etc/blackbox/blackbox.yml";
/// The in-container Grafana provisioning directory.
pub const GRAFANA_PROVISIONING_TARGET: &str = "/etc/grafana/provisioning";
/// The in-container Grafana dashboard directory.
pub const GRAFANA_DASHBOARDS_TARGET: &str = "/etc/grafana/dashboards";
/// The in-container CA bundle used for the MinIO and synthetic TLS probes.
pub const OBSERVABILITY_CA_TARGET: &str = "/etc/quantrule/pki/ca.crt";
/// The in-container Patroni REST password file.
pub const PATRONI_PASSWORD_TARGET: &str = "/etc/prometheus/secrets/patroni_rest_password";

/// The host path of the Prometheus configuration.
pub const PROMETHEUS_CONFIG_HOST: &str = "/etc/quantrule/prometheus/prometheus.yml";
/// The host path of the alert rules.
pub const PROMETHEUS_RULES_HOST: &str = "/etc/quantrule/prometheus/alert-rules.yml";
/// The host path of the Alertmanager configuration.
pub const ALERTMANAGER_CONFIG_HOST: &str = "/etc/quantrule/alertmanager/alertmanager.yml";
/// The host path of the Loki configuration.
pub const LOKI_CONFIG_HOST: &str = "/etc/quantrule/loki/loki.yml";
/// The host path of one host's promtail configuration.
pub const PROMTAIL_CONFIG_HOST: &str = "/etc/quantrule/promtail/promtail-{host}.yml";
/// The host path of the blackbox configuration.
pub const BLACKBOX_CONFIG_HOST: &str = "/etc/quantrule/blackbox/blackbox.yml";
/// The host path of the Grafana provisioning tree.
pub const GRAFANA_PROVISIONING_HOST: &str = "/etc/quantrule/grafana/provisioning";
/// The host path of the Grafana dashboards.
pub const GRAFANA_DASHBOARDS_HOST: &str = "/etc/quantrule/grafana/dashboards";

/// The four checked-in dashboards and their fixed Grafana UIDs.
pub const DASHBOARDS: [(&str, &str); 4] = [
    ("overview.json", "qr-overview"),
    ("ha-matrix.json", "qr-ha-matrix"),
    ("capacity.json", "qr-capacity"),
    ("runs-evidence.json", "qr-runs-evidence"),
];

const OVERVIEW_DASHBOARD: &str =
    include_str!("../../../../deploy/v3/observability/dashboards/overview.json");
const HA_MATRIX_DASHBOARD: &str =
    include_str!("../../../../deploy/v3/observability/dashboards/ha-matrix.json");
const CAPACITY_DASHBOARD: &str =
    include_str!("../../../../deploy/v3/observability/dashboards/capacity.json");
const RUNS_DASHBOARD: &str =
    include_str!("../../../../deploy/v3/observability/dashboards/runs-evidence.json");

/// The checked-in dashboard text for one file name.
#[must_use]
pub(crate) fn dashboard(name: &str) -> Option<&'static str> {
    match name {
        "overview.json" => Some(OVERVIEW_DASHBOARD),
        "ha-matrix.json" => Some(HA_MATRIX_DASHBOARD),
        "capacity.json" => Some(CAPACITY_DASHBOARD),
        "runs-evidence.json" => Some(RUNS_DASHBOARD),
        _ => None,
    }
}

/// One bind mount a unit needs.
#[derive(Debug, Clone)]
pub(crate) struct VolumeSpec {
    /// Host path.
    pub source: String,
    /// Container path.
    pub target: String,
    /// Read-only mount.
    pub read_only: bool,
    /// Optional bind propagation (`rslave` for the host root).
    pub propagation: Option<&'static str>,
}

/// One health check.
#[derive(Debug, Clone)]
pub(crate) struct HealthcheckSpec {
    /// The test command and arguments.
    pub test: Vec<String>,
    /// Interval.
    pub interval: &'static str,
    /// Timeout.
    pub timeout: &'static str,
    /// Retries.
    pub retries: u64,
    /// Start period.
    pub start_period: &'static str,
}

/// One observability unit's non-YAML specification.
#[derive(Debug, Clone)]
pub(crate) struct UnitSpec {
    /// Compose service key.
    pub name: String,
    /// Descriptor image key.
    pub image_key: &'static str,
    /// Container name.
    pub container_name: String,
    /// Optional hostname.
    pub hostname: Option<String>,
    /// Environment entries.
    pub environment: Vec<(String, String)>,
    /// Optional `user`.
    pub user: Option<&'static str>,
    /// Mounts.
    pub volumes: Vec<VolumeSpec>,
    /// Network aliases.
    pub aliases: Vec<String>,
    /// Command arguments.
    pub command: Vec<String>,
    /// Optional health check.
    pub healthcheck: Option<HealthcheckSpec>,
}

fn volume(source: impl Into<String>, target: impl Into<String>, read_only: bool) -> VolumeSpec {
    VolumeSpec {
        source: source.into(),
        target: target.into(),
        read_only,
        propagation: None,
    }
}

fn root_volume() -> VolumeSpec {
    VolumeSpec {
        source: "/".to_owned(),
        target: "/host".to_owned(),
        read_only: true,
        propagation: Some("rslave"),
    }
}

fn healthcheck(path: &str, port: u16) -> HealthcheckSpec {
    HealthcheckSpec {
        test: vec![
            "CMD-SHELL".to_owned(),
            format!("wget -q -O /dev/null http://127.0.0.1:{port}{path}"),
        ],
        interval: "15s",
        timeout: "5s",
        retries: 5,
        start_period: "60s",
    }
}

/// Write every observability configuration, dashboard, and manifest.
///
/// # Errors
///
/// Returns a render path or write error. Validation has already refused an
/// unsafe host or placement; this is the defensive boundary.
pub fn render(
    descriptor: &Descriptor,
    out: &Path,
    written: &mut Vec<PathBuf>,
) -> Result<(), Error> {
    for (name, text) in [
        ("prometheus.yml", prometheus_config(descriptor)),
        ("alert-rules.yml", alert_rules()),
        ("alertmanager.yml", alertmanager_config()),
        ("loki.yml", loki_config()),
        ("blackbox.yml", blackbox_config(descriptor)),
    ] {
        let path = out.join(name);
        write_public(&path, text.as_bytes())?;
        written.push(path);
    }
    for host in descriptor.hosts.keys() {
        safe_component(host, "hosts")?;
        let path = out.join(format!("promtail-{host}.yml"));
        write_public(&path, promtail_config(descriptor, host)?.as_bytes())?;
        written.push(path);
    }
    let grafana = out.join("grafana");
    create_dir(&grafana)?;
    for (relative, text) in [
        (
            "provisioning/datasources/datasources.yml",
            grafana_datasources(),
        ),
        ("provisioning/dashboards/dashboards.yml", grafana_provider()),
        ("dashboards/overview.json", OVERVIEW_DASHBOARD.to_owned()),
        ("dashboards/ha-matrix.json", HA_MATRIX_DASHBOARD.to_owned()),
        ("dashboards/capacity.json", CAPACITY_DASHBOARD.to_owned()),
        ("dashboards/runs-evidence.json", RUNS_DASHBOARD.to_owned()),
    ] {
        let path = grafana.join(relative);
        write_public(&path, text.as_bytes())?;
        written.push(path);
    }
    let path = out.join("observability-manifest.json");
    write_public(&path, observability_manifest(descriptor)?.as_bytes())?;
    written.push(path);
    Ok(())
}

/// The Prometheus unit on the first observability host.
pub(crate) fn prometheus_spec() -> UnitSpec {
    UnitSpec {
        name: "prometheus".to_owned(),
        image_key: "prometheus",
        container_name: "quantrule-prometheus".to_owned(),
        hostname: None,
        environment: Vec::new(),
        // The mounted Patroni password file is mode 600; the Prometheus image
        // runs as nobody by default, so the unit keeps root to read it.
        user: Some("0:0"),
        volumes: vec![
            volume("/srv/quantrule/prometheus/data", "/prometheus", false),
            volume(PROMETHEUS_CONFIG_HOST, PROMETHEUS_CONFIG_TARGET, true),
            volume(PROMETHEUS_RULES_HOST, PROMETHEUS_RULES_TARGET, true),
            volume("/etc/quantrule/pki/ca.crt", OBSERVABILITY_CA_TARGET, true),
            volume(
                "/etc/quantrule/secrets/patroni_rest_password",
                PATRONI_PASSWORD_TARGET,
                true,
            ),
        ],
        aliases: vec!["prometheus".to_owned()],
        command: vec![
            format!("--config.file={PROMETHEUS_CONFIG_TARGET}"),
            "--storage.tsdb.path=/prometheus".to_owned(),
            "--storage.tsdb.retention.time=30d".to_owned(),
            "--storage.tsdb.retention.size=20GB".to_owned(),
            format!("--web.listen-address=0.0.0.0:{PROMETHEUS_PORT}"),
        ],
        healthcheck: Some(healthcheck("/-/ready", PROMETHEUS_PORT)),
    }
}

/// The Alertmanager unit on its host.
pub(crate) fn alertmanager_spec() -> UnitSpec {
    UnitSpec {
        name: "alertmanager".to_owned(),
        image_key: "alertmanager",
        container_name: "quantrule-alertmanager".to_owned(),
        hostname: None,
        environment: Vec::new(),
        user: None,
        volumes: vec![
            volume("/srv/quantrule/alertmanager/data", "/alertmanager", false),
            volume(ALERTMANAGER_CONFIG_HOST, ALERTMANAGER_CONFIG_TARGET, true),
        ],
        aliases: vec!["alertmanager".to_owned()],
        command: vec![
            format!("--config.file={ALERTMANAGER_CONFIG_TARGET}"),
            "--storage.path=/alertmanager".to_owned(),
            "--web.listen-address=0.0.0.0:9093".to_owned(),
            "--data.retention=120h".to_owned(),
        ],
        healthcheck: Some(healthcheck("/-/ready", ALERTMANAGER_PORT)),
    }
}

/// The Loki unit on its host.
pub(crate) fn loki_spec() -> UnitSpec {
    UnitSpec {
        name: "loki".to_owned(),
        image_key: "loki",
        container_name: "quantrule-loki".to_owned(),
        hostname: None,
        environment: Vec::new(),
        user: None,
        volumes: vec![
            volume("/srv/quantrule/loki/data", "/loki", false),
            volume(LOKI_CONFIG_HOST, LOKI_CONFIG_TARGET, true),
        ],
        aliases: vec!["loki".to_owned()],
        command: vec![format!("-config.file={LOKI_CONFIG_TARGET}")],
        healthcheck: Some(healthcheck("/-/ready", LOKI_PORT)),
    }
}

/// The Grafana unit on its host.
pub(crate) fn grafana_spec() -> UnitSpec {
    UnitSpec {
        name: "grafana".to_owned(),
        image_key: "grafana",
        container_name: "quantrule-grafana".to_owned(),
        hostname: None,
        environment: vec![
            (
                "GF_ANALYTICS_REPORTING_ENABLED".to_owned(),
                "false".to_owned(),
            ),
            (
                "GF_ANALYTICS_CHECK_FOR_UPDATES".to_owned(),
                "false".to_owned(),
            ),
            ("GF_AUTH_ANONYMOUS_ENABLED".to_owned(), "true".to_owned()),
            ("GF_AUTH_ANONYMOUS_ORG_ROLE".to_owned(), "Viewer".to_owned()),
            ("GF_AUTH_BASIC_ENABLED".to_owned(), "false".to_owned()),
            ("GF_AUTH_DISABLE_LOGIN_FORM".to_owned(), "true".to_owned()),
            ("GF_AUTH_DISABLE_SIGNOUT_MENU".to_owned(), "true".to_owned()),
            ("GF_SECURITY_ALLOW_EMBEDDING".to_owned(), "false".to_owned()),
            ("GF_SECURITY_COOKIE_SECURE".to_owned(), "false".to_owned()),
            ("GF_USERS_ALLOW_SIGN_UP".to_owned(), "false".to_owned()),
            ("GF_SERVER_HTTP_PORT".to_owned(), GRAFANA_PORT.to_string()),
        ],
        user: None,
        volumes: vec![
            volume("/srv/quantrule/grafana/data", "/var/lib/grafana", false),
            volume(GRAFANA_PROVISIONING_HOST, GRAFANA_PROVISIONING_TARGET, true),
            volume(GRAFANA_DASHBOARDS_HOST, GRAFANA_DASHBOARDS_TARGET, true),
        ],
        aliases: vec!["grafana".to_owned()],
        command: Vec::new(),
        healthcheck: Some(healthcheck("/api/health", GRAFANA_PORT)),
    }
}

/// The blackbox exporter unit on the Prometheus host.
pub(crate) fn blackbox_spec() -> UnitSpec {
    UnitSpec {
        name: "blackbox".to_owned(),
        image_key: "blackbox_exporter",
        container_name: "quantrule-blackbox".to_owned(),
        hostname: None,
        environment: Vec::new(),
        user: None,
        volumes: vec![
            volume(BLACKBOX_CONFIG_HOST, BLACKBOX_CONFIG_TARGET, true),
            volume("/etc/quantrule/pki/ca.crt", OBSERVABILITY_CA_TARGET, true),
        ],
        aliases: vec!["blackbox".to_owned()],
        command: vec![format!("--config.file={BLACKBOX_CONFIG_TARGET}")],
        healthcheck: Some(healthcheck("/", BLACKBOX_PORT)),
    }
}

/// The node_exporter unit on every host.
pub(crate) fn node_exporter_spec(host: &str) -> UnitSpec {
    UnitSpec {
        name: format!("node-exporter-{host}"),
        image_key: "node_exporter",
        container_name: format!("quantrule-node-exporter-{host}"),
        hostname: None,
        environment: Vec::new(),
        user: None,
        volumes: vec![
            root_volume(),
            volume(
                "/srv/quantrule/node_exporter/textfile",
                "/var/lib/node_exporter/textfile",
                true,
            ),
        ],
        aliases: vec![format!("node-{host}")],
        command: vec![
            "--path.rootfs=/host".to_owned(),
            "--collector.textfile.directory=/var/lib/node_exporter/textfile".to_owned(),
            format!("--web.listen-address=0.0.0.0:{NODE_EXPORTER_PORT}"),
        ],
        healthcheck: Some(healthcheck("/metrics", NODE_EXPORTER_PORT)),
    }
}

/// The promtail unit on every host.
pub(crate) fn promtail_spec(host: &str) -> UnitSpec {
    UnitSpec {
        name: format!("promtail-{host}"),
        image_key: "promtail",
        container_name: format!("quantrule-promtail-{host}"),
        hostname: None,
        environment: Vec::new(),
        user: Some("0:0"),
        volumes: vec![
            volume(
                format!("/etc/quantrule/promtail/promtail-{host}.yml"),
                PROMTAIL_CONFIG_TARGET,
                true,
            ),
            volume("/var/run/docker.sock", "/var/run/docker.sock", true),
            volume("/srv/quantrule/promtail", "/var/lib/promtail", false),
        ],
        aliases: vec![format!("promtail-{host}")],
        command: vec![format!("-config.file={PROMTAIL_CONFIG_TARGET}")],
        healthcheck: Some(healthcheck("/ready", PROMTAIL_PORT)),
    }
}

/// The observability file manifest.
///
/// # Errors
///
/// Returns [`Error::Invalid`] when the fixed manifest cannot be serialized.
pub fn observability_manifest(descriptor: &Descriptor) -> Result<String, Error> {
    let prometheus_host = descriptor.observability.prometheus.host.clone();
    let grafana_host = descriptor.observability.grafana.host.clone();
    let loki_host = descriptor.observability.loki.host.clone();
    let alertmanager_host = descriptor.observability.alertmanager.host.clone();
    let all_hosts: Vec<String> = descriptor.hosts.keys().cloned().collect();
    let mut entries = vec![
        entry(
            "prometheus.yml",
            PROMETHEUS_CONFIG_HOST,
            "644",
            "config",
            vec![prometheus_host.clone()],
        ),
        entry(
            "alert-rules.yml",
            PROMETHEUS_RULES_HOST,
            "644",
            "config",
            vec![prometheus_host.clone()],
        ),
        entry(
            "alertmanager.yml",
            ALERTMANAGER_CONFIG_HOST,
            "644",
            "config",
            vec![alertmanager_host],
        ),
        entry(
            "loki.yml",
            LOKI_CONFIG_HOST,
            "644",
            "config",
            vec![loki_host],
        ),
        entry(
            "blackbox.yml",
            BLACKBOX_CONFIG_HOST,
            "644",
            "config",
            vec![prometheus_host.clone()],
        ),
        entry(
            "grafana/provisioning/datasources/datasources.yml",
            format!("{GRAFANA_PROVISIONING_HOST}/datasources/datasources.yml"),
            "644",
            "config",
            vec![grafana_host.clone()],
        ),
        entry(
            "grafana/provisioning/dashboards/dashboards.yml",
            format!("{GRAFANA_PROVISIONING_HOST}/dashboards/dashboards.yml"),
            "644",
            "config",
            vec![grafana_host.clone()],
        ),
    ];
    for (name, _) in DASHBOARDS {
        entries.push(entry(
            format!("grafana/dashboards/{name}"),
            format!("{GRAFANA_DASHBOARDS_HOST}/{name}"),
            "644",
            "config",
            vec![grafana_host.clone()],
        ));
    }
    for host in &all_hosts {
        entries.push(entry(
            format!("promtail-{host}.yml"),
            PROMTAIL_CONFIG_HOST.replace("{host}", host),
            "644",
            "config",
            vec![host.clone()],
        ));
    }
    entries.push(entry(
        "pki/database-ca.crt",
        "/etc/quantrule/pki/ca.crt",
        "644",
        "pki",
        vec![prometheus_host.clone()],
    ));
    entries.push(entry(
        "secrets/patroni_rest_password",
        "/etc/quantrule/secrets/patroni_rest_password",
        "600",
        "secret",
        vec![prometheus_host],
    ));
    let manifest = ObservabilityManifest {
        v: "quantrule.deploy/v3",
        note: "copy each source to its absolute target on the listed hosts; no secret value or fingerprint is listed",
        files: entries,
    };
    let text = serde_json::to_string_pretty(&manifest).map_err(|error| Error::Invalid {
        field: "observability-manifest.json".to_owned(),
        message: error.to_string(),
    })?;
    Ok(text + "\n")
}

/// The `F01`..`F17` row labels carried by the rules.
#[cfg(test)]
#[must_use]
pub fn failure_rows() -> Vec<String> {
    (1..=17).map(|row| format!("F{row:02}")).collect()
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

#[derive(Serialize)]
struct ObservabilityManifest<'a> {
    v: &'a str,
    note: &'a str,
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

#[cfg(test)]
mod tests;
