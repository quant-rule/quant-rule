//! Opt-in remote and overlay inspectors for a v3 check.
//!
//! The default check is local plus one HTTPS ingress probe. `--remote docker`
//! and `--overlay` are explicit opt-ins. The inspectors read Docker's own
//! read-only inspect output through a [`DockerQuery`] seam; the production
//! adapter shells out to the Docker CLI, and tests inject a fixture. No
//! inspector prints an environment value, a secret or config payload, a token,
//! or a full inspect body.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::descriptor::v3::Descriptor;
use crate::render::{APP_SERVICES, app_image, pinned, stack_name};

use super::report::{CheckReport, CheckRow};

/// The read-only Docker queries the inspectors need.
///
/// Every method maps to one `docker ... inspect` or `docker ... ls` read. The
/// seam exists so the fast tier can prove every branch without a daemon.
pub trait DockerQuery {
    /// Read the local Swarm state (`docker info --format '{{json .Swarm}}'`).
    fn swarm_info(&self) -> Result<String, String>;
    /// List the Swarm node IDs (`docker node ls -q`).
    fn node_ids(&self) -> Result<Vec<String>, String>;
    /// Inspect one node as JSON (`docker node inspect --format '{{json .}}'`).
    fn node_inspect(&self, id: &str) -> Result<String, String>;
    /// Inspect one network as JSON (`docker network inspect --format ...`).
    fn network_inspect(&self, name: &str) -> Result<String, String>;
    /// Inspect one service as JSON (`docker service inspect --format ...`).
    fn service_inspect(&self, name: &str) -> Result<String, String>;
    /// List one service's desired-running tasks as JSON lines.
    fn service_tasks(&self, name: &str) -> Result<String, String>;
}

/// The production adapter: one read-only Docker CLI process per query.
#[derive(Debug, Clone)]
pub struct CommandDocker {
    executable: String,
}

impl CommandDocker {
    /// Use `QR_DEPLOY_DOCKER` when it names an executable, otherwise `docker`.
    ///
    /// The override exists for tests and for an operator whose Docker CLI is
    /// not on `PATH`; it is never a path inside a service container.
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            executable: std::env::var("QR_DEPLOY_DOCKER").unwrap_or_else(|_| "docker".to_owned()),
        }
    }

    fn output(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new(&self.executable)
            .args(args)
            .output()
            .map_err(|error| format!("cannot run {}: {error}", self.executable))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let detail: String = stderr
                .lines()
                .next()
                .unwrap_or("the command failed")
                .trim()
                .chars()
                .take(200)
                .collect();
            let verb = args.first().copied().unwrap_or("inspect");
            return Err(format!("docker {verb} failed: {detail}"));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

impl DockerQuery for CommandDocker {
    fn swarm_info(&self) -> Result<String, String> {
        self.output(&["info", "--format", "{{json .Swarm}}"])
    }

    fn node_ids(&self) -> Result<Vec<String>, String> {
        let text = self.output(&["node", "ls", "-q"])?;
        Ok(text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToOwned::to_owned)
            .collect())
    }

    fn node_inspect(&self, id: &str) -> Result<String, String> {
        self.output(&["node", "inspect", "--format", "{{json .}}", id])
    }

    fn network_inspect(&self, name: &str) -> Result<String, String> {
        self.output(&["network", "inspect", "--format", "{{json .}}", name])
    }

    fn service_inspect(&self, name: &str) -> Result<String, String> {
        self.output(&["service", "inspect", "--format", "{{json .}}", name])
    }

    fn service_tasks(&self, name: &str) -> Result<String, String> {
        self.output(&[
            "service",
            "ps",
            "--filter",
            "desired-state=running",
            "--format",
            "{{json .}}",
            name,
        ])
    }
}

/// One opt-in inspector.
pub trait Inspector {
    /// Inspect the deployment and return rows, or one stable failure.
    ///
    /// # Errors
    ///
    /// Returns a human-readable failure when the Docker read itself cannot be
    /// performed. A missing object or a wrong value is a failed row, not an
    /// error, so one bad service does not hide the others.
    fn inspect(&self) -> Result<Vec<CheckRow>, String>;
}

/// The Docker service, node, and placement inspector.
pub struct DockerInspector<'a> {
    descriptor: &'a Descriptor,
    docker: &'a dyn DockerQuery,
}

impl<'a> DockerInspector<'a> {
    /// Bind one descriptor to one Docker query source.
    #[must_use]
    pub fn new(descriptor: &'a Descriptor, docker: &'a dyn DockerQuery) -> Self {
        Self { descriptor, docker }
    }
}

impl Inspector for DockerInspector<'_> {
    fn inspect(&self) -> Result<Vec<CheckRow>, String> {
        inspect_docker(self.descriptor, self.docker)
    }
}

/// The encrypted-overlay inspector.
pub struct OverlayInspector<'a> {
    descriptor: &'a Descriptor,
    docker: &'a dyn DockerQuery,
}

impl<'a> OverlayInspector<'a> {
    /// Bind one descriptor to one Docker query source.
    #[must_use]
    pub fn new(descriptor: &'a Descriptor, docker: &'a dyn DockerQuery) -> Self {
        Self { descriptor, docker }
    }
}

impl Inspector for OverlayInspector<'_> {
    fn inspect(&self) -> Result<Vec<CheckRow>, String> {
        inspect_overlay(self.descriptor, self.docker)
    }
}

/// Append one inspector's rows, or its failure.
pub fn append(
    report: &mut CheckReport,
    group: &'static str,
    subject: &'static str,
    inspector: &dyn Inspector,
) {
    match inspector.inspect() {
        Ok(rows) => {
            for row in rows {
                report.push(row);
            }
        }
        Err(error) => report.push(CheckRow::failed(group, subject, "failed", error)),
    }
}

/// Append the remote Docker rows when the operator opted in.
pub fn append_docker(report: &mut CheckReport, descriptor: &Descriptor, docker: &dyn DockerQuery) {
    append(
        report,
        "remote",
        "docker",
        &DockerInspector::new(descriptor, docker),
    );
}

/// Append the overlay rows when the operator opted in.
pub fn append_overlay(report: &mut CheckReport, descriptor: &Descriptor, docker: &dyn DockerQuery) {
    append(
        report,
        "overlay",
        "network",
        &OverlayInspector::new(descriptor, docker),
    );
}

#[derive(Debug, Deserialize)]
struct SwarmInfo {
    #[serde(rename = "LocalNodeState", default)]
    local_node_state: String,
    #[serde(rename = "ControlAvailable", default)]
    control_available: bool,
}

#[derive(Debug, Deserialize, Default)]
struct NodeInspect {
    #[serde(rename = "Spec", default)]
    spec: NodeSpec,
    #[serde(rename = "Description", default)]
    description: NodeDescription,
    #[serde(rename = "Status", default)]
    status: NodeStatus,
}

#[derive(Debug, Deserialize, Default)]
struct NodeSpec {
    #[serde(rename = "Labels", default)]
    labels: BTreeMap<String, String>,
    #[serde(rename = "Role", default)]
    role: String,
    #[serde(rename = "Availability", default)]
    availability: String,
}

#[derive(Debug, Deserialize, Default)]
struct NodeDescription {
    #[serde(rename = "Hostname", default)]
    hostname: String,
}

#[derive(Debug, Deserialize, Default)]
struct NodeStatus {
    #[serde(rename = "State", default)]
    state: String,
    #[serde(rename = "Addr", default)]
    addr: String,
}

#[derive(Debug, Deserialize, Default)]
struct NetworkInspect {
    #[serde(rename = "Name", default)]
    name: String,
    #[serde(rename = "Driver", default)]
    driver: String,
    #[serde(rename = "Attachable", default)]
    attachable: bool,
    #[serde(rename = "Options", default)]
    options: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
struct ServiceInspect {
    #[serde(rename = "Spec", default)]
    spec: ServiceSpec,
    #[serde(rename = "Endpoint", default)]
    endpoint: ServiceEndpoint,
    #[serde(rename = "ServiceStatus", default)]
    service_status: ServiceStatus,
    #[serde(rename = "UpdateStatus", default)]
    update_status: Option<UpdateStatus>,
}

#[derive(Debug, Deserialize, Default)]
struct ServiceSpec {
    #[serde(rename = "TaskTemplate", default)]
    task_template: TaskTemplate,
    #[serde(rename = "Mode", default)]
    mode: ServiceMode,
}

#[derive(Debug, Deserialize, Default)]
struct TaskTemplate {
    #[serde(rename = "ContainerSpec", default)]
    container_spec: ContainerSpec,
}

#[derive(Debug, Deserialize, Default)]
struct ContainerSpec {
    #[serde(rename = "Image", default)]
    image: String,
}

#[derive(Debug, Deserialize, Default)]
struct ServiceMode {
    #[serde(rename = "Replicated", default)]
    replicated: Option<Replicated>,
}

#[derive(Debug, Deserialize, Default)]
struct Replicated {
    #[serde(rename = "Replicas", default)]
    replicas: u64,
}

#[derive(Debug, Deserialize, Default)]
struct ServiceEndpoint {
    #[serde(rename = "Ports", default)]
    ports: Vec<ServicePort>,
}

#[derive(Debug, Deserialize, Default)]
struct ServicePort {
    #[serde(rename = "Protocol", default)]
    protocol: String,
    #[serde(rename = "TargetPort", default)]
    target_port: u64,
    #[serde(rename = "PublishedPort", default)]
    published_port: u64,
    #[serde(rename = "PublishMode", default)]
    publish_mode: String,
}

#[derive(Debug, Deserialize, Default)]
struct ServiceStatus {
    #[serde(rename = "RunningTasks", default)]
    running_tasks: u64,
    #[serde(rename = "DesiredTasks", default)]
    desired_tasks: u64,
}

#[derive(Debug, Deserialize, Default)]
struct UpdateStatus {
    #[serde(rename = "State", default)]
    state: String,
}

#[derive(Debug, Deserialize, Default)]
struct TaskInspect {
    #[serde(rename = "DesiredState", default)]
    desired_state: String,
    #[serde(rename = "CurrentState", default)]
    current_state: String,
    #[serde(rename = "Error", default)]
    error: String,
}

fn inspect_docker(
    descriptor: &Descriptor,
    docker: &dyn DockerQuery,
) -> Result<Vec<CheckRow>, String> {
    let swarm_text = docker
        .swarm_info()
        .map_err(|error| format!("deploy.remote.docker: {error}"))?;
    let swarm: SwarmInfo = parse_single(&swarm_text, "Swarm")?;
    let ids = docker
        .node_ids()
        .map_err(|error| format!("deploy.remote.docker: {error}"))?;
    if ids.is_empty() {
        return Err("deploy.remote.docker: the Swarm reports no nodes".to_owned());
    }

    let mut rows = vec![swarm_state_row(&swarm)];
    let mut nodes = Vec::new();
    for id in &ids {
        match docker
            .node_inspect(id)
            .and_then(|text| parse_single::<NodeInspect>(&text, "node"))
        {
            Ok(node) => nodes.push(node),
            Err(error) => rows.push(CheckRow::failed(
                "remote",
                format!("node {id}"),
                "inspect",
                format!("deploy.remote.docker: {error}"),
            )),
        }
    }
    let manager_count = nodes
        .iter()
        .filter(|node| node.spec.role == "manager")
        .count();
    rows.push(manager_row(manager_count, &swarm));

    let manager_hosts: BTreeSet<&str> = descriptor
        .swarm
        .managers
        .iter()
        .map(String::as_str)
        .collect();
    for (host, value) in &descriptor.hosts {
        match resolve_host(host, &value.address, &nodes) {
            Ok(node) => {
                let issues = node_issues(descriptor, host, value, node, &manager_hosts);
                if issues.is_empty() {
                    rows.push(CheckRow::ok(
                        "remote",
                        format!("node {host}"),
                        node.spec.role.clone(),
                        format!(
                            "{}; {}; labels match",
                            node.spec.role, node.spec.availability
                        ),
                    ));
                } else {
                    rows.push(CheckRow::failed(
                        "remote",
                        format!("node {host}"),
                        "labels",
                        issues.join("; "),
                    ));
                }
            }
            Err(error) => rows.push(CheckRow::failed(
                "remote",
                format!("node {host}"),
                "missing",
                error,
            )),
        }
    }

    match docker.network_inspect(&descriptor.network.overlay) {
        Ok(text) => match parse_single::<NetworkInspect>(&text, "network") {
            Ok(network) => rows.push(network_row("remote", descriptor, &network)),
            Err(error) => rows.push(CheckRow::failed(
                "remote",
                format!("overlay {}", descriptor.network.overlay),
                "inspect",
                error,
            )),
        },
        Err(error) => rows.push(CheckRow::failed(
            "remote",
            format!("overlay {}", descriptor.network.overlay),
            "missing",
            format!("deploy.remote.docker: {error}"),
        )),
    }

    let mut inspected = BTreeMap::new();
    for (kind, full, expected_image, expected_replicas) in service_table(descriptor) {
        match docker
            .service_inspect(&full)
            .and_then(|text| parse_single::<ServiceInspect>(&text, "service"))
        {
            Ok(service) => {
                let (issues, running, desired) =
                    service_issues(&service, &expected_image, expected_replicas);
                if issues.is_empty() {
                    rows.push(CheckRow::ok(
                        "remote",
                        format!("service {kind}"),
                        "converged",
                        format!("{running}/{desired} replicas; image pinned"),
                    ));
                } else {
                    rows.push(CheckRow::failed(
                        "remote",
                        format!("service {kind}"),
                        "mismatch",
                        issues.join("; "),
                    ));
                }
                inspected.insert(full, service);
            }
            Err(error) => rows.push(CheckRow::failed(
                "remote",
                format!("service {kind}"),
                "missing",
                format!("deploy.remote.docker: {error}"),
            )),
        }
    }
    rows.push(ports_row(descriptor, &inspected));
    rows.push(tasks_row(descriptor, docker));
    rows.push(CheckRow::ok(
        "remote",
        "health",
        "limited",
        "service ps exposes task state, not container health; /health rows and the 180 s update monitor cover it",
    ));
    Ok(rows)
}

fn inspect_overlay(
    descriptor: &Descriptor,
    docker: &dyn DockerQuery,
) -> Result<Vec<CheckRow>, String> {
    let text = docker
        .network_inspect(&descriptor.network.overlay)
        .map_err(|error| format!("deploy.overlay.network: {error}"))?;
    let network: NetworkInspect = parse_single(&text, "network")?;
    Ok(vec![
        network_row("overlay", descriptor, &network),
        CheckRow::ok(
            "overlay",
            "dns-route",
            "limited",
            "no read-only DNS or route probe; the ingress /ready row and the ha tier cover service discovery",
        ),
    ])
}

fn swarm_state_row(swarm: &SwarmInfo) -> CheckRow {
    if swarm.local_node_state == "active" && swarm.control_available {
        CheckRow::ok("remote", "swarm local-state", "active", "control available")
    } else {
        CheckRow::failed(
            "remote",
            "swarm local-state",
            "inactive",
            format!(
                "local state {}; control available {}",
                swarm.local_node_state, swarm.control_available
            ),
        )
    }
}

fn manager_row(manager_count: usize, swarm: &SwarmInfo) -> CheckRow {
    if manager_count >= 3 && manager_count % 2 == 1 && swarm.control_available {
        CheckRow::ok(
            "remote",
            "swarm managers",
            "quorum",
            format!("{manager_count} managers (odd)"),
        )
    } else {
        CheckRow::failed(
            "remote",
            "swarm managers",
            "quorum",
            format!(
                "manager count {manager_count} is even, below three, or unreachable; three odd managers are required"
            ),
        )
    }
}

fn resolve_host<'a>(
    host: &str,
    address: &str,
    nodes: &'a [NodeInspect],
) -> Result<&'a NodeInspect, String> {
    let by_label: Vec<&NodeInspect> = nodes
        .iter()
        .filter(|node| {
            node.spec
                .labels
                .get("quantrule.host")
                .is_some_and(|value| value == host)
        })
        .collect();
    if by_label.len() > 1 {
        return Err(format!(
            "host {host} matches more than one node by quantrule.host"
        ));
    }
    if let Some(node) = by_label.first() {
        return Ok(node);
    }
    let by_hostname: Vec<&NodeInspect> = nodes
        .iter()
        .filter(|node| node.description.hostname == host)
        .collect();
    if by_hostname.len() > 1 {
        return Err(format!("host {host} matches more than one node hostname"));
    }
    if let Some(node) = by_hostname.first() {
        return Ok(node);
    }
    let by_address: Vec<&NodeInspect> = nodes
        .iter()
        .filter(|node| node.status.addr == address)
        .collect();
    if by_address.len() > 1 {
        return Err(format!(
            "address {address} matches more than one node for host {host}"
        ));
    }
    by_address
        .first()
        .copied()
        .ok_or_else(|| format!("cannot map host {host} ({address}) to a Swarm node"))
}

fn node_issues(
    descriptor: &Descriptor,
    host: &str,
    value: &crate::descriptor::v3::Host,
    node: &NodeInspect,
    manager_hosts: &BTreeSet<&str>,
) -> Vec<String> {
    let mut issues = Vec::new();
    if node.status.state != "ready" {
        issues.push(format!("node state {}", node.status.state));
    }
    if node.spec.availability != "active" {
        issues.push(format!("availability {}", node.spec.availability));
    }
    if manager_hosts.contains(host) && node.spec.role != "manager" {
        issues.push(format!(
            "role {} but swarm.managers names this host",
            node.spec.role
        ));
    }
    for (key, expected) in [
        ("quantrule.host", host),
        ("quantrule.domain", value.failure_domain.as_str()),
        ("quantrule.profile", descriptor.profile.as_str()),
        ("quantrule.deploy", stack_name(descriptor)),
    ] {
        match node.spec.labels.get(key) {
            Some(actual) if actual == expected => {}
            Some(actual) => issues.push(format!("{key}={actual}, expected {expected}")),
            None => issues.push(format!("missing label {key}")),
        }
    }
    for (service, _) in APP_SERVICES {
        let expected = descriptor
            .services
            .get(service)
            .is_some_and(|service| service.hosts.iter().any(|value| value == host));
        let key = format!("quantrule.{service}");
        let actual = node.spec.labels.get(&key).map(String::as_str);
        match (expected, actual) {
            (true, Some("true")) => {}
            (true, _) => issues.push(format!("missing label {key}")),
            (false, Some(_)) => issues.push(format!("unexpected label {key}")),
            (false, None) => {}
        }
    }
    let ingress_expected = descriptor.ingress.hosts.iter().any(|value| value == host);
    match (
        ingress_expected,
        node.spec
            .labels
            .get("quantrule.ingress")
            .map(String::as_str),
    ) {
        (true, Some("true")) => {}
        (true, _) => issues.push("missing label quantrule.ingress".to_owned()),
        (false, Some(_)) => issues.push("unexpected label quantrule.ingress".to_owned()),
        (false, None) => {}
    }
    issues
}

fn network_row(group: &'static str, descriptor: &Descriptor, network: &NetworkInspect) -> CheckRow {
    let mut issues = Vec::new();
    if network.name != descriptor.network.overlay {
        issues.push(format!(
            "network name {}, expected {}",
            network.name, descriptor.network.overlay
        ));
    }
    if network.driver != "overlay" {
        issues.push(format!("driver {}, expected overlay", network.driver));
    }
    if !network.attachable {
        issues.push("the network is not attachable".to_owned());
    }
    if network.options.get("encrypted").map(String::as_str) != Some("true") {
        issues.push("the network is not encrypted".to_owned());
    }
    if issues.is_empty() {
        CheckRow::ok(
            group,
            format!("network {}", descriptor.network.overlay),
            "overlay",
            "overlay; attachable; encrypted",
        )
    } else {
        CheckRow::failed(
            group,
            format!("network {}", descriptor.network.overlay),
            "network",
            issues.join("; "),
        )
    }
}

/// The app services in their fixed order, then the ingress.
fn service_table(descriptor: &Descriptor) -> Vec<(String, String, String, u32)> {
    let stack = stack_name(descriptor);
    let mut table = Vec::new();
    for (name, _) in APP_SERVICES {
        let replicas = descriptor
            .services
            .get(name)
            .map_or(0, |service| service.replicas);
        table.push((
            (*name).to_owned(),
            format!("{stack}_{name}"),
            app_image(descriptor),
            replicas,
        ));
    }
    table.push((
        "ingress".to_owned(),
        format!("{stack}_ingress"),
        descriptor
            .images
            .get("traefik")
            .map(pinned)
            .unwrap_or_default(),
        descriptor.ingress.replicas,
    ));
    table
}

fn service_issues(
    service: &ServiceInspect,
    expected_image: &str,
    expected_replicas: u32,
) -> (Vec<String>, u64, u64) {
    let mut issues = Vec::new();
    let image = &service.spec.task_template.container_spec.image;
    if image != expected_image {
        issues.push(format!("image {image}, expected {expected_image}"));
    }
    let spec_replicas = service
        .spec
        .mode
        .replicated
        .as_ref()
        .map_or(0, |replicated| replicated.replicas);
    if spec_replicas != u64::from(expected_replicas) {
        issues.push(format!(
            "spec replicas {spec_replicas}, expected {expected_replicas}"
        ));
    }
    let desired = if service.service_status.desired_tasks > 0 {
        service.service_status.desired_tasks
    } else {
        spec_replicas
    };
    let running = service.service_status.running_tasks;
    if desired != u64::from(expected_replicas) {
        issues.push(format!(
            "desired tasks {desired}, expected {expected_replicas}"
        ));
    }
    if running != desired {
        issues.push(format!("running tasks {running}/{desired}"));
    }
    if let Some(update) = &service.update_status
        && matches!(update.state.as_str(), "paused" | "rollback_paused")
    {
        issues.push(format!("update state {}", update.state));
    }
    (issues, running, desired)
}

fn ports_row(descriptor: &Descriptor, inspected: &BTreeMap<String, ServiceInspect>) -> CheckRow {
    let stack = stack_name(descriptor);
    let ingress = format!("{stack}_ingress");
    let mut published = Vec::new();
    let mut issues = Vec::new();
    for (full, service) in inspected {
        if *full != ingress {
            if !service.endpoint.ports.is_empty() {
                issues.push(format!(
                    "{full} exposes {} port entries",
                    service.endpoint.ports.len()
                ));
            }
            continue;
        }
        for port in &service.endpoint.ports {
            if port.published_port != 0 {
                published.push(port);
            }
        }
    }
    if published.len() != 1 {
        issues.push(format!("{} published ports; expected one", published.len()));
    }
    if let Some(port) = published.first()
        && (port.published_port != 443
            || port.target_port != 443
            || port.publish_mode != "host"
            || port.protocol != "tcp")
    {
        issues.push(format!(
            "published {}/{}/{} in {} mode; expected 443/443/tcp in host mode",
            port.published_port, port.target_port, port.protocol, port.publish_mode
        ));
    }
    if issues.is_empty() {
        CheckRow::ok(
            "remote",
            "ports",
            "single-public-port",
            "one published port (443/tcp, host mode); no internal published port",
        )
    } else {
        CheckRow::failed("remote", "ports", "published-port", issues.join("; "))
    }
}

fn tasks_row(descriptor: &Descriptor, docker: &dyn DockerQuery) -> CheckRow {
    let stack = stack_name(descriptor);
    let mut names: Vec<String> = APP_SERVICES
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    names.push("ingress".to_owned());
    let mut issues = Vec::new();
    for kind in names {
        let full = format!("{stack}_{kind}");
        match docker.service_tasks(&full) {
            Ok(text) => {
                let mut tasks = Vec::new();
                for line in text.lines().filter(|line| !line.trim().is_empty()) {
                    match serde_json::from_str::<TaskInspect>(line) {
                        Ok(task) => tasks.push(task),
                        Err(_) => issues.push(format!("{kind} has an unreadable task row")),
                    }
                }
                if tasks.is_empty() {
                    issues.push(format!("{kind} has no running task"));
                    continue;
                }
                for task in tasks {
                    if task.desired_state != "Running" {
                        issues.push(format!("{kind} desired state {}", task.desired_state));
                    }
                    let state = task
                        .current_state
                        .split_whitespace()
                        .next()
                        .unwrap_or("unknown");
                    if state != "Running" {
                        issues.push(format!("{kind} task state {state}"));
                    }
                    if !task.error.is_empty() {
                        issues.push(format!("{kind} has a task error"));
                    }
                }
            }
            Err(error) => issues.push(format!("{kind} tasks: {error}")),
        }
    }
    if issues.is_empty() {
        CheckRow::ok("remote", "tasks", "running", "no failed or rejected tasks")
    } else {
        CheckRow::failed("remote", "tasks", "failed", issues.join("; "))
    }
}

fn parse_single<T: DeserializeOwned>(text: &str, what: &str) -> Result<T, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("cannot parse the {what} inspect output: {error}"))?;
    let value = match value {
        serde_json::Value::Array(mut values) if values.len() == 1 => values.remove(0),
        other => other,
    };
    serde_json::from_value(value)
        .map_err(|error| format!("cannot read the {what} inspect fields: {error}"))
}

#[cfg(test)]
mod tests;
