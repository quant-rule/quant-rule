use super::*;
use serde_json::Value;

use crate::descriptor::v3::Profile;

fn descriptor() -> Descriptor {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/v3/deploy.example.json"
    )))
    .expect("v3 example")
}

#[derive(Default, Clone)]
struct FakeDocker {
    swarm: Option<Result<String, String>>,
    node_ids: Option<Result<Vec<String>, String>>,
    nodes: BTreeMap<String, Result<String, String>>,
    networks: BTreeMap<String, Result<String, String>>,
    services: BTreeMap<String, Result<String, String>>,
    tasks: BTreeMap<String, Result<String, String>>,
}

impl DockerQuery for FakeDocker {
    fn swarm_info(&self) -> Result<String, String> {
        self.swarm
            .clone()
            .unwrap_or_else(|| Err("no Swarm fixture".to_owned()))
    }

    fn node_ids(&self) -> Result<Vec<String>, String> {
        self.node_ids
            .clone()
            .unwrap_or_else(|| Err("no node fixture".to_owned()))
    }

    fn node_inspect(&self, id: &str) -> Result<String, String> {
        self.nodes
            .get(id)
            .cloned()
            .unwrap_or_else(|| Err(format!("no node fixture for {id}")))
    }

    fn network_inspect(&self, name: &str) -> Result<String, String> {
        self.networks
            .get(name)
            .cloned()
            .unwrap_or_else(|| Err(format!("no network fixture for {name}")))
    }

    fn service_inspect(&self, name: &str) -> Result<String, String> {
        self.services
            .get(name)
            .cloned()
            .unwrap_or_else(|| Err(format!("no service fixture for {name}")))
    }

    fn service_tasks(&self, name: &str) -> Result<String, String> {
        self.tasks
            .get(name)
            .cloned()
            .unwrap_or_else(|| Err(format!("no task fixture for {name}")))
    }

    fn runtime_probe(&self, _network: &str, _image: &str, _url: &str) -> Result<String, String> {
        Err("unused".to_owned())
    }
}

fn service_json(name: &str, image: &str, replicas: u32, ports: Vec<serde_json::Value>) -> String {
    serde_json::json!({
        "Spec": {
            "Name": name,
            "TaskTemplate": {"ContainerSpec": {"Image": image}},
            "Mode": {"Replicated": {"Replicas": replicas}}
        },
        "Endpoint": {"Ports": ports},
        "ServiceStatus": {"RunningTasks": replicas, "DesiredTasks": replicas},
        "UpdateStatus": {"State": "completed"}
    })
    .to_string()
}

fn task_lines(replicas: u32, image: &str) -> String {
    (1..=replicas)
        .map(|_| {
            serde_json::json!({
                "DesiredState": "Running",
                "CurrentState": "Running",
                "Image": image,
                "Error": ""
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn healthy(descriptor: &Descriptor) -> FakeDocker {
    let stack = stack_name(descriptor);
    let mut fake = FakeDocker {
        swarm: Some(Ok(
            serde_json::json!({"LocalNodeState": "active", "ControlAvailable": true}).to_string(),
        )),
        ..FakeDocker::default()
    };
    let mut ids = Vec::new();
    for (host, value) in &descriptor.hosts {
        let id = format!("node-{host}");
        ids.push(id.clone());
        let manager = descriptor.swarm.managers.iter().any(|value| value == host);
        let mut labels = serde_json::Map::new();
        labels.insert("quantrule.host".to_owned(), serde_json::json!(host));
        labels.insert(
            "quantrule.domain".to_owned(),
            serde_json::json!(value.failure_domain),
        );
        labels.insert(
            "quantrule.profile".to_owned(),
            serde_json::json!(descriptor.profile.as_str()),
        );
        labels.insert("quantrule.deploy".to_owned(), serde_json::json!(stack));
        for (service, _) in APP_SERVICES {
            if descriptor
                .services
                .get(service)
                .is_some_and(|service| service.hosts.iter().any(|value| value == host))
            {
                labels.insert(format!("quantrule.{service}"), serde_json::json!("true"));
            }
        }
        if descriptor.ingress.hosts.iter().any(|value| value == host) {
            labels.insert("quantrule.ingress".to_owned(), serde_json::json!("true"));
        }
        let node = serde_json::json!({
            "Spec": {
                "Labels": labels,
                "Role": if manager {"manager"} else {"worker"},
                "Availability": "active"
            },
            "Description": {"Hostname": host},
            "Status": {"State": "ready", "Addr": value.address}
        });
        fake.nodes.insert(id, Ok(node.to_string()));
    }
    fake.node_ids = Some(Ok(ids));
    fake.networks.insert(
        descriptor.network.overlay.clone(),
        Ok(serde_json::json!({
            "Name": descriptor.network.overlay,
            "Driver": "overlay",
            "Attachable": true,
            "Options": {"encrypted": "true"}
        })
        .to_string()),
    );
    for (service, _) in APP_SERVICES {
        let expected = descriptor.services.get(service).expect("service");
        let full = format!("{stack}_{service}");
        fake.services.insert(
            full.clone(),
            Ok(service_json(
                &full,
                &app_image(descriptor),
                expected.replicas,
                Vec::new(),
            )),
        );
        fake.tasks.insert(
            full,
            Ok(task_lines(expected.replicas, &app_image(descriptor))),
        );
    }
    let ingress = format!("{stack}_ingress");
    let ingress_image = pinned(&descriptor.images["traefik"]);
    fake.services.insert(
        ingress.clone(),
        Ok(service_json(
            &ingress,
            &ingress_image,
            descriptor.ingress.replicas,
            vec![serde_json::json!({
                "Protocol": "tcp",
                "TargetPort": 443,
                "PublishedPort": 443,
                "PublishMode": "host"
            })],
        )),
    );
    fake.tasks.insert(
        ingress,
        Ok(task_lines(descriptor.ingress.replicas, &ingress_image)),
    );
    fake
}

fn rows(fake: &FakeDocker) -> Vec<CheckRow> {
    inspect_docker(&descriptor(), fake).expect("inspect rows")
}

fn failures(rows: &[CheckRow]) -> Vec<&CheckRow> {
    rows.iter().filter(|row| row.is_failure()).collect()
}

fn text(rows: &[CheckRow]) -> String {
    rows.iter()
        .map(|row| {
            format!(
                "{} {} {} {} {}",
                row.status(),
                row.group,
                row.subject,
                row.state,
                row.error.as_deref().or(row.detail.as_deref()).unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn a_healthy_swarm_passes_every_remote_row() {
    let rows = rows(&healthy(&descriptor()));
    assert!(failures(&rows).is_empty(), "{}", text(&rows));
    let rendered = text(&rows);
    assert!(rendered.contains("3 managers"), "{rendered}");
    assert!(rendered.contains("labels match"), "{rendered}");
    assert!(rendered.contains("one published port"), "{rendered}");
    assert!(
        rendered.contains("no failed or rejected tasks"),
        "{rendered}"
    );
}

#[test]
fn a_missing_service_is_a_failed_row() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.services
        .remove(&format!("{}_gateway", stack_name(&descriptor)));
    let rows = rows(&fake);
    let failures = failures(&rows);
    assert!(!failures.is_empty());
    assert!(
        failures.iter().any(|row| row.subject == "service gateway"),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_wrong_replica_count_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    fake.services.insert(
        full.clone(),
        Ok(service_json(&full, &app_image(&descriptor), 1, Vec::new())),
    );
    fake.tasks
        .insert(full, Ok(task_lines(1, &app_image(&descriptor))));
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "service gateway"),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_wrong_image_digest_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    fake.services.insert(
        full.clone(),
        Ok(service_json(
            &full,
            "registry.quantrule.dev/quantrule/server@sha256:wrong",
            2,
            Vec::new(),
        )),
    );
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "service gateway"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("expected"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn every_running_task_image_must_equal_the_descriptor() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    let mut tasks: Vec<Value> = fake.tasks[&full]
        .as_ref()
        .expect("tasks")
        .lines()
        .map(|line| serde_json::from_str(line).expect("task"))
        .collect();
    tasks[0]["Image"] = serde_json::json!("ghcr.io/foreign/server@sha256:wrong");
    fake.tasks.insert(
        full,
        Ok(tasks
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")),
    );
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "running task image gateway"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("expected"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_running_task_with_no_image_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    fake.tasks.insert(
        full,
        Ok(serde_json::json!({
            "DesiredState": "Running",
            "CurrentState": "Running",
            "Error": ""
        })
        .to_string()),
    );
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "running task image gateway" && row.state == "mismatch"),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_stopped_task_image_is_ignored_by_the_image_row() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    let tasks = format!(
        "{}\n{}",
        serde_json::json!({
            "DesiredState": "Running",
            "CurrentState": "Running",
            "Image": app_image(&descriptor),
            "Error": ""
        }),
        serde_json::json!({
            "DesiredState": "Running",
            "CurrentState": "Failed 2 minutes ago",
            "Image": "ghcr.io/foreign/server@sha256:wrong",
            "Error": ""
        })
    );
    fake.tasks.insert(full, Ok(tasks));
    let rows = rows(&fake);
    let image = rows
        .iter()
        .find(|row| row.subject == "running task image gateway")
        .expect("image row");
    assert!(!image.is_failure(), "{}", text(&rows));
    assert!(
        failures(&rows).iter().any(|row| row.subject == "tasks"),
        "the aggregate task row still reports the failed task: {}",
        text(&rows)
    );
}

#[test]
fn a_second_published_port_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    fake.services.insert(
        full.clone(),
        Ok(service_json(
            &full,
            &app_image(&descriptor),
            2,
            vec![serde_json::json!({
                "Protocol": "tcp",
                "TargetPort": 8080,
                "PublishedPort": 8080,
                "PublishMode": "ingress"
            })],
        )),
    );
    let rows = rows(&fake);
    assert!(
        failures(&rows).iter().any(|row| row.subject == "ports"),
        "{}",
        text(&rows)
    );
}

#[test]
fn an_empty_encrypted_option_is_accepted() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.networks.insert(
        descriptor.network.overlay.clone(),
        Ok(serde_json::json!({
            "Name": descriptor.network.overlay,
            "Driver": "overlay",
            "Attachable": true,
            "Options": {"encrypted": ""}
        })
        .to_string()),
    );
    let rows = rows(&fake);
    assert!(
        !failures(&rows).iter().any(|row| row
            .error
            .as_deref()
            .is_some_and(|error| error.contains("not encrypted"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_zero_service_status_uses_the_task_list() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    let mut service: serde_json::Value =
        serde_json::from_str(fake.services[&full].as_ref().expect("service")).expect("service");
    service["ServiceStatus"] = serde_json::json!({"RunningTasks": 0, "DesiredTasks": 0});
    fake.services.insert(full, Ok(service.to_string()));
    let rows = rows(&fake);
    assert!(
        !failures(&rows)
            .iter()
            .any(|row| row.subject == "service gateway"),
        "{}",
        text(&rows)
    );
}

#[test]
fn an_unencrypted_overlay_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.networks.insert(
        descriptor.network.overlay.clone(),
        Ok(serde_json::json!({
            "Name": descriptor.network.overlay,
            "Driver": "overlay",
            "Attachable": true,
            "Options": {}
        })
        .to_string()),
    );
    let rows = rows(&fake);
    assert!(
        failures(&rows).iter().any(|row| row
            .error
            .as_deref()
            .is_some_and(|error| error.contains("not encrypted"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_missing_label_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let id = "node-edge-1";
    let mut node: serde_json::Value =
        serde_json::from_str(fake.nodes[id].as_ref().expect("node")).expect("node JSON");
    node["Spec"]["Labels"]
        .as_object_mut()
        .expect("labels")
        .remove("quantrule.domain");
    fake.nodes.insert(id.to_owned(), Ok(node.to_string()));
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "node edge-1"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("missing label quantrule.domain"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_manager_count_below_three_fails() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let id = "node-edge-1";
    let mut node: serde_json::Value =
        serde_json::from_str(fake.nodes[id].as_ref().expect("node")).expect("node JSON");
    node["Spec"]["Role"] = serde_json::json!("worker");
    fake.nodes.insert(id.to_owned(), Ok(node.to_string()));
    let rows = rows(&fake);
    assert!(
        failures(&rows)
            .iter()
            .any(|row| row.subject == "swarm managers"
                && row
                    .error
                    .as_deref()
                    .is_some_and(|error| error.contains("manager count 2"))),
        "{}",
        text(&rows)
    );
}

#[test]
fn a_command_error_is_a_failure_not_a_skip() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    fake.swarm = Some(Err("permission denied".to_owned()));
    let error = inspect_docker(&descriptor, &fake).expect_err("command error");
    assert!(error.contains("deploy.remote.docker"), "{error}");
    let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "img");
    append_docker(&mut report, &descriptor, &fake);
    assert!(report.failed());
    assert!(report.render().contains("permission denied"));
}

#[test]
fn the_overlay_inspector_reports_encryption_and_a_limited_dns_row() {
    let descriptor = descriptor();
    let fake = healthy(&descriptor);
    let rows = inspect_overlay(&descriptor, &fake).expect("overlay rows");
    assert!(failures(&rows).is_empty(), "{}", text(&rows));
    let rendered = text(&rows);
    assert!(rendered.contains("attachable; encrypted"), "{rendered}");
    assert!(rendered.contains("dns-route"), "{rendered}");
    assert!(rendered.contains("limited"), "{rendered}");
}

#[test]
fn no_row_prints_an_environment_value() {
    let descriptor = descriptor();
    let mut fake = healthy(&descriptor);
    let full = format!("{}_gateway", stack_name(&descriptor));
    let mut service: serde_json::Value =
        serde_json::from_str(fake.services[&full].as_ref().expect("service"))
            .expect("service JSON");
    service["Spec"]["TaskTemplate"]["ContainerSpec"]["Env"] =
        serde_json::json!(["GATEWAY_TOKEN=do-not-print-this"]);
    fake.services.insert(full, Ok(service.to_string()));
    let rendered = text(&rows(&fake));
    assert!(!rendered.contains("do-not-print-this"), "{rendered}");
}

#[test]
fn a_lab_profile_is_not_claimed_as_real_ha() {
    let mut value: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/v3/deploy.example.json"
    )))
    .expect("example");
    value["profile"] = serde_json::json!("lab-single-host");
    let descriptor: Descriptor = serde_json::from_value(value).expect("descriptor");
    let rows = rows(&healthy(&descriptor));
    let rendered = text(&rows);
    assert_eq!(descriptor.profile, Profile::LabSingleHost);
    assert!(!rendered.contains("Real HA"), "{rendered}");
}
