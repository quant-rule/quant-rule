//! Prometheus scrape, Loki/promtail, blackbox, and Grafana configuration.
//!
//! The parent owns placement and the unit specs; this module owns the config
//! bodies. Every value is deterministic and every port stays on the overlay.

use super::super::stack_name;
use super::super::yaml::Yaml;
use super::*;

/// The Prometheus scrape and alerting configuration.
#[must_use]
pub fn prometheus_config(descriptor: &Descriptor) -> String {
    let stack = stack_name(descriptor);
    let app_names = [
        "gateway",
        "data",
        "model",
        "strategy",
        "backtest",
        "evidence",
        "simulation",
    ];
    let app = Yaml::map()
        .with("job_name", Yaml::string("app"))
        .with(
            "dns_sd_configs",
            Yaml::list([Yaml::map()
                .with(
                    "names",
                    Yaml::list(
                        app_names
                            .iter()
                            .map(|name| Yaml::string(format!("tasks.{name}"))),
                    ),
                )
                .with("type", Yaml::string("A"))
                .with("port", Yaml::number(u64::from(NODE_EXPORTER_PORT)))
                .with("refresh_interval", Yaml::string("30s"))]),
        )
        .with(
            "relabel_configs",
            Yaml::list([
                Yaml::map()
                    .with(
                        "source_labels",
                        Yaml::list([Yaml::string("__meta_dns_name")]),
                    )
                    .with("regex", Yaml::string("tasks\\.(.+)"))
                    .with("target_label", Yaml::string("service"))
                    .with("replacement", Yaml::string("$1")),
                Yaml::map()
                    .with("target_label", Yaml::string("stack"))
                    .with("replacement", Yaml::string(stack)),
            ]),
        );
    let ingress = Yaml::map()
        .with("job_name", Yaml::string("ingress"))
        .with(
            "dns_sd_configs",
            Yaml::list([Yaml::map()
                .with("names", Yaml::list([Yaml::string("tasks.ingress")]))
                .with("type", Yaml::string("A"))
                .with("port", Yaml::number(u64::from(TRAEFIK_METRICS_PORT)))
                .with("refresh_interval", Yaml::string("30s"))]),
        )
        .with(
            "relabel_configs",
            Yaml::list([Yaml::map()
                .with("target_label", Yaml::string("service"))
                .with("replacement", Yaml::string("ingress"))]),
        );
    let patroni = static_http_job(
        "patroni",
        "/metrics",
        descriptor
            .stateful
            .postgres
            .nodes
            .iter()
            .map(|host| (format!("patroni-{host}:{PATRONI_REST_PORT}"), host.clone()))
            .collect(),
    )
    .with(
        "basic_auth",
        Yaml::map()
            .with("username", Yaml::string("patroni"))
            .with("password_file", Yaml::string(PATRONI_PASSWORD_TARGET)),
    );
    let etcd = static_http_job(
        "etcd",
        "/metrics",
        descriptor
            .stateful
            .etcd
            .nodes
            .iter()
            .map(|host| (format!("etcd-{host}:{ETCD_METRICS_PORT}"), host.clone()))
            .collect(),
    );
    let haproxy = static_http_job(
        "haproxy",
        "/metrics",
        descriptor
            .stateful
            .haproxy
            .nodes
            .iter()
            .map(|host| {
                (
                    format!("haproxy-{host}:{HAPROXY_METRICS_PORT}"),
                    host.clone(),
                )
            })
            .collect(),
    );
    let rabbitmq = static_http_job(
        "rabbitmq",
        "/metrics",
        descriptor
            .stateful
            .rabbitmq
            .nodes
            .iter()
            .map(|host| {
                (
                    format!("rabbitmq-{host}:{RABBITMQ_METRICS_PORT}"),
                    host.clone(),
                )
            })
            .collect(),
    );
    let minio = Yaml::map()
        .with("job_name", Yaml::string("minio"))
        .with("scheme", Yaml::string("https"))
        .with("metrics_path", Yaml::string("/minio/v2/metrics/cluster"))
        .with(
            "tls_config",
            Yaml::map().with("ca_file", Yaml::string(OBSERVABILITY_CA_TARGET)),
        )
        .with(
            "static_configs",
            Yaml::list([Yaml::map()
                .with(
                    "targets",
                    Yaml::list(
                        descriptor
                            .stateful
                            .minio
                            .nodes
                            .iter()
                            .map(|host| Yaml::string(format!("minio-{host}:9000"))),
                    ),
                )
                .with(
                    "labels",
                    Yaml::mapping(
                        descriptor
                            .stateful
                            .minio
                            .nodes
                            .iter()
                            .map(|host| ("host".to_owned(), Yaml::string(host))),
                    ),
                )]),
        );
    let node = static_http_job(
        "node",
        "/metrics",
        descriptor
            .hosts
            .keys()
            .map(|host| (format!("node-{host}:{NODE_EXPORTER_PORT}"), host.clone()))
            .collect(),
    );
    let promtail = static_http_job(
        "promtail",
        "/metrics",
        descriptor
            .hosts
            .keys()
            .map(|host| (format!("promtail-{host}:{PROMTAIL_PORT}"), host.clone()))
            .collect(),
    );
    let blackbox_targets = [
        (
            format!("https://{}/health", descriptor.ingress.dns),
            "health",
        ),
        (format!("https://{}/ready", descriptor.ingress.dns), "ready"),
    ];
    let blackbox = Yaml::map()
        .with("job_name", Yaml::string("blackbox"))
        .with("metrics_path", Yaml::string("/probe"))
        .with(
            "params",
            Yaml::map().with("module", Yaml::list([Yaml::string("https_2xx")])),
        )
        .with(
            "static_configs",
            Yaml::list(blackbox_targets.iter().map(|(target, check)| {
                Yaml::map()
                    .with("targets", Yaml::list([Yaml::string(target)]))
                    .with("labels", Yaml::map().with("check", Yaml::string(*check)))
            })),
        )
        .with(
            "relabel_configs",
            Yaml::list([
                Yaml::map()
                    .with("source_labels", Yaml::list([Yaml::string("__address__")]))
                    .with("target_label", Yaml::string("__param_target")),
                Yaml::map()
                    .with(
                        "source_labels",
                        Yaml::list([Yaml::string("__param_target")]),
                    )
                    .with("target_label", Yaml::string("instance")),
                Yaml::map()
                    .with("target_label", Yaml::string("__address__"))
                    .with(
                        "replacement",
                        Yaml::string(format!("blackbox:{BLACKBOX_PORT}")),
                    ),
            ]),
        );
    let self_job = static_http_job(
        "prometheus",
        "/metrics",
        vec![(
            format!("localhost:{PROMETHEUS_PORT}"),
            "prometheus".to_owned(),
        )],
    );
    let alertmanager = static_http_job(
        "alertmanager",
        "/metrics",
        vec![(
            format!("alertmanager:{ALERTMANAGER_PORT}"),
            "alertmanager".to_owned(),
        )],
    );
    let loki = static_http_job(
        "loki",
        "/metrics",
        vec![(format!("loki:{LOKI_PORT}"), "loki".to_owned())],
    );
    let grafana = static_http_job(
        "grafana",
        "/metrics",
        vec![(format!("grafana:{GRAFANA_PORT}"), "grafana".to_owned())],
    );
    Yaml::map()
        .with(
            "global",
            Yaml::map()
                .with("evaluation_interval", Yaml::string("15s"))
                .with(
                    "external_labels",
                    Yaml::map().with("stack", Yaml::string(stack)),
                )
                .with("scrape_interval", Yaml::string("15s")),
        )
        .with(
            "rule_files",
            Yaml::list([Yaml::string(PROMETHEUS_RULES_TARGET)]),
        )
        .with(
            "alerting",
            Yaml::map().with(
                "alertmanagers",
                Yaml::list([Yaml::map().with(
                    "static_configs",
                    Yaml::list([Yaml::map().with(
                        "targets",
                        Yaml::list([Yaml::string(format!("alertmanager:{ALERTMANAGER_PORT}"))]),
                    )]),
                )]),
            ),
        )
        .with(
            "scrape_configs",
            Yaml::list([
                app,
                ingress,
                patroni,
                etcd,
                haproxy,
                rabbitmq,
                minio,
                node,
                promtail,
                blackbox,
                self_job,
                alertmanager,
                loki,
                grafana,
            ]),
        )
        .render()
}

fn static_http_job(job: &str, path: &str, targets: Vec<(String, String)>) -> Yaml {
    let configs: Vec<Yaml> = targets
        .into_iter()
        .map(|(address, host)| {
            Yaml::map()
                .with("targets", Yaml::list([Yaml::string(address)]))
                .with("labels", Yaml::map().with("host", Yaml::string(host)))
        })
        .collect();
    Yaml::map()
        .with("job_name", Yaml::string(job))
        .with("metrics_path", Yaml::string(path))
        .with("static_configs", Yaml::list(configs))
}

/// The Loki single-binary configuration with 30-day retention.
#[must_use]
pub fn loki_config() -> String {
    Yaml::map()
        .with("auth_enabled", Yaml::Bool(false))
        .with(
            "server",
            Yaml::map()
                .with("grpc_listen_port", Yaml::number(9096_u64))
                .with("http_listen_port", Yaml::number(u64::from(LOKI_PORT))),
        )
        .with(
            "common",
            Yaml::map()
                .with("path_prefix", Yaml::string("/loki"))
                .with("replication_factor", Yaml::number(1_u64))
                .with(
                    "ring",
                    Yaml::map()
                        .with("instance_addr", Yaml::string("127.0.0.1"))
                        .with(
                            "kvstore",
                            Yaml::map().with("store", Yaml::string("inmemory")),
                        ),
                )
                .with(
                    "storage",
                    Yaml::map().with(
                        "filesystem",
                        Yaml::map()
                            .with("chunks_directory", Yaml::string("/loki/chunks"))
                            .with("rules_directory", Yaml::string("/loki/rules")),
                    ),
                ),
        )
        .with(
            "schema_config",
            Yaml::map().with(
                "configs",
                Yaml::list([Yaml::map()
                    .with("from", Yaml::string("2020-10-24"))
                    .with(
                        "index",
                        Yaml::map()
                            .with("period", Yaml::string("24h"))
                            .with("prefix", Yaml::string("index_")),
                    )
                    .with("object_store", Yaml::string("filesystem"))
                    .with("schema", Yaml::string("v13"))
                    .with("store", Yaml::string("tsdb"))]),
            ),
        )
        .with(
            "limits_config",
            Yaml::map()
                .with("reject_old_samples", Yaml::Bool(true))
                .with("reject_old_samples_max_age", Yaml::string("168h"))
                .with("retention_period", Yaml::string("720h")),
        )
        .with(
            "compactor",
            Yaml::map()
                .with("compaction_interval", Yaml::string("10m"))
                .with("delete_request_store", Yaml::string("filesystem"))
                .with("retention_enabled", Yaml::Bool(true))
                .with("working_directory", Yaml::string("/loki/compactor")),
        )
        .with(
            "ruler",
            Yaml::map().with(
                "storage",
                Yaml::map().with("type", Yaml::string("local")).with(
                    "local",
                    Yaml::map().with("directory", Yaml::string("/loki/rules")),
                ),
            ),
        )
        .render()
}

/// One host's promtail configuration.
///
/// # Errors
///
/// Returns a rule when the host is not declared; validation refuses that first.
pub fn promtail_config(descriptor: &Descriptor, host: &str) -> Result<String, Error> {
    if !descriptor.hosts.contains_key(host) {
        return Err(Error::rule(
            "deploy.host.undeclared",
            "hosts",
            format!("{host} is not declared in hosts"),
        ));
    }
    let root = Yaml::map()
        .with(
            "server",
            Yaml::map()
                .with("grpc_listen_port", Yaml::number(0_u64))
                .with("http_listen_port", Yaml::number(u64::from(PROMTAIL_PORT))),
        )
        .with(
            "positions",
            Yaml::map().with("filename", Yaml::string("/var/lib/promtail/positions.yaml")),
        )
        .with(
            "clients",
            Yaml::list([Yaml::map().with(
                "url",
                Yaml::string(format!("http://loki:{LOKI_PORT}/loki/api/v1/push")),
            )]),
        )
        .with(
            "scrape_configs",
            Yaml::list([Yaml::map()
                .with("job_name", Yaml::string("docker"))
                .with(
                    "docker_sd_configs",
                    Yaml::list([Yaml::map()
                        .with("host", Yaml::string("unix:///var/run/docker.sock"))
                        .with("refresh_interval", Yaml::string("30s"))]),
                )
                .with(
                    "relabel_configs",
                    Yaml::list([
                        // Only containers whose name starts with `quantrule` are
                        // kept; the label whitelist below drops every Docker
                        // discovery label, so no arbitrary container label reaches
                        // Loki.
                        Yaml::map()
                            .with("action", Yaml::string("keep"))
                            .with(
                                "source_labels",
                                Yaml::list([Yaml::string("__meta_docker_container_name")]),
                            )
                            .with("regex", Yaml::string("/(quantrule.*)")),
                        Yaml::map()
                            .with(
                                "source_labels",
                                Yaml::list([Yaml::string("__meta_docker_container_name")]),
                            )
                            .with("regex", Yaml::string("/(quantrule.*)"))
                            .with("target_label", Yaml::string("container")),
                        Yaml::map()
                            .with(
                                "source_labels",
                                Yaml::list([Yaml::string(
                                    "__meta_docker_container_label_com_docker_swarm_service_name",
                                )]),
                            )
                            .with("target_label", Yaml::string("service")),
                        Yaml::map()
                            .with(
                                "source_labels",
                                Yaml::list([Yaml::string(
                                    "__meta_docker_container_label_com_docker_compose_service",
                                )]),
                            )
                            .with("target_label", Yaml::string("service")),
                        Yaml::map()
                            .with("target_label", Yaml::string("host"))
                            .with("replacement", Yaml::string(host)),
                        Yaml::map()
                            .with("action", Yaml::string("labeldrop"))
                            .with("regex", Yaml::string("__meta_docker_.*")),
                    ]),
                )
                .with(
                    "pipeline_stages",
                    Yaml::list([Yaml::map().with("docker", Yaml::map())]),
                )]),
        );
    Ok(root.render())
}

/// The blackbox exporter modules.
#[must_use]
pub fn blackbox_config(descriptor: &Descriptor) -> String {
    Yaml::map()
        .with(
            "modules",
            Yaml::map().with(
                "https_2xx",
                Yaml::map()
                    .with("prober", Yaml::string("http"))
                    .with("timeout", Yaml::string("5s"))
                    .with(
                        "http",
                        Yaml::map()
                            .with("preferred_ip_protocol", Yaml::string("ip4"))
                            .with("valid_status_codes", Yaml::list([Yaml::number(200_u64)]))
                            .with(
                                "tls_config",
                                Yaml::map()
                                    .with("ca_file", Yaml::string(OBSERVABILITY_CA_TARGET))
                                    .with(
                                        "server_name",
                                        Yaml::string(descriptor.ingress.dns.clone()),
                                    ),
                            ),
                    ),
            ),
        )
        .render()
}

/// Grafana datasource provisioning with fixed UIDs.
#[must_use]
pub fn grafana_datasources() -> String {
    Yaml::map()
        .with("apiVersion", Yaml::number(1_u64))
        .with(
            "datasources",
            Yaml::list([
                Yaml::map()
                    .with("access", Yaml::string("proxy"))
                    .with("editable", Yaml::Bool(false))
                    .with("isDefault", Yaml::Bool(true))
                    .with("name", Yaml::string("Prometheus"))
                    .with("type", Yaml::string("prometheus"))
                    .with("uid", Yaml::string("qr-prometheus"))
                    .with(
                        "url",
                        Yaml::string(format!("http://prometheus:{PROMETHEUS_PORT}")),
                    ),
                Yaml::map()
                    .with("access", Yaml::string("proxy"))
                    .with("editable", Yaml::Bool(false))
                    .with("name", Yaml::string("Loki"))
                    .with("type", Yaml::string("loki"))
                    .with("uid", Yaml::string("qr-loki"))
                    .with("url", Yaml::string(format!("http://loki:{LOKI_PORT}"))),
            ]),
        )
        .render()
}

/// Grafana dashboard-provider provisioning.
#[must_use]
pub fn grafana_provider() -> String {
    Yaml::map()
        .with("apiVersion", Yaml::number(1_u64))
        .with(
            "providers",
            Yaml::list([Yaml::map()
                .with("allowUiUpdates", Yaml::Bool(false))
                .with("disableDeletion", Yaml::Bool(true))
                .with("folder", Yaml::string("QuantRule"))
                .with("name", Yaml::string("quantrule"))
                .with("orgId", Yaml::number(1_u64))
                .with(
                    "options",
                    Yaml::map().with("path", Yaml::string(GRAFANA_DASHBOARDS_TARGET)),
                )
                .with("type", Yaml::string("file"))
                .with("updateIntervalSeconds", Yaml::number(30_u64))]),
        )
        .render()
}
