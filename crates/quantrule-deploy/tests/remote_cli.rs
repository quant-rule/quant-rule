//! CLI tests for the read-only Docker inspector.
//!
//! The real binary runs against a fake `docker` selected by
//! `QR_DEPLOY_DOCKER`; no daemon is required.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_qr-deploy")
}

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-deploy-cli-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn run_envs(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run qr-deploy")
}

const APP_SERVICES: [&str; 7] = [
    "gateway",
    "data",
    "model",
    "strategy",
    "backtest",
    "evidence",
    "simulation",
];

fn pinned_image(value: &Value) -> String {
    let reference = value["reference"].as_str().expect("image reference");
    let digest = value["digest"].as_str().expect("image digest");
    let reference = reference
        .split_once('@')
        .map_or(reference, |(name, _)| name);
    format!("{reference}@{digest}")
}

fn write_service(fixtures: &Path, name: &str, image: &str, replicas: u64, ports: Vec<Value>) {
    write_service_with_mounts(
        fixtures,
        name,
        image,
        replicas,
        ports,
        Value::Null,
        Value::Null,
    );
}

fn write_service_with_mounts(
    fixtures: &Path,
    name: &str,
    image: &str,
    replicas: u64,
    ports: Vec<Value>,
    configs: Value,
    secrets: Value,
) {
    let mut container = serde_json::json!({"Image": image});
    if !configs.is_null() {
        container["Configs"] = configs;
    }
    if !secrets.is_null() {
        container["Secrets"] = secrets;
    }
    let service = serde_json::json!({
        "Spec": {
            "Name": name,
            "TaskTemplate": {"ContainerSpec": container},
            "Mode": {"Replicated": {"Replicas": replicas}}
        },
        "Endpoint": {"Ports": ports},
        "ServiceStatus": {"RunningTasks": replicas, "DesiredTasks": replicas},
        "UpdateStatus": {"State": "completed"}
    });
    std::fs::write(
        fixtures.join(format!("service-{name}.json")),
        service.to_string(),
    )
    .expect("service fixture");
    let tasks = (0..replicas)
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
        .join("\n");
    std::fs::write(fixtures.join(format!("tasks-{name}.json")), tasks).expect("task fixture");
}

/// Write a fake `docker` and the read-only inspect fixtures it serves.
///
/// The fake never starts a daemon: it `cat`s the files the test wrote and fails
/// when a fixture is missing, exactly as the real CLI fails for a missing
/// service.
fn fake_docker(directory: &Path, descriptor: &Value) -> (PathBuf, PathBuf, PathBuf) {
    let fixtures = directory.join("docker-fixtures");
    std::fs::create_dir_all(&fixtures).expect("fixtures");
    std::fs::write(
        fixtures.join("swarm.json"),
        serde_json::json!({"LocalNodeState": "active", "ControlAvailable": true}).to_string(),
    )
    .expect("swarm fixture");
    let stack = descriptor["name"].as_str().unwrap_or("quantrule");
    let image = pinned_image(&descriptor["image"]);
    let hosts = descriptor["hosts"].as_object().expect("hosts");
    let managers = descriptor["swarm"]["managers"]
        .as_array()
        .expect("managers")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let ingress_hosts = descriptor["ingress"]["hosts"]
        .as_array()
        .expect("ingress hosts")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut ids = Vec::new();
    for (host, value) in hosts {
        let id = format!("node-{host}");
        ids.push(id.clone());
        let mut labels = serde_json::Map::new();
        labels.insert("quantrule.host".to_owned(), serde_json::json!(host));
        labels.insert(
            "quantrule.domain".to_owned(),
            serde_json::json!(value["failure_domain"]),
        );
        labels.insert(
            "quantrule.profile".to_owned(),
            serde_json::json!(descriptor["profile"]),
        );
        labels.insert("quantrule.deploy".to_owned(), serde_json::json!(stack));
        for service in APP_SERVICES {
            let expected = descriptor["services"][service]["hosts"]
                .as_array()
                .is_some_and(|hosts| hosts.iter().any(|entry| entry == host));
            if expected {
                labels.insert(format!("quantrule.{service}"), serde_json::json!("true"));
            }
        }
        if ingress_hosts.contains(&host.as_str()) {
            labels.insert("quantrule.ingress".to_owned(), serde_json::json!("true"));
        }
        let node = serde_json::json!({
            "Spec": {
                "Labels": labels,
                "Role": if managers.contains(&host.as_str()) {"manager"} else {"worker"},
                "Availability": "active"
            },
            "Description": {"Hostname": host},
            "Status": {"State": "ready", "Addr": value["address"]}
        });
        std::fs::write(fixtures.join(format!("{id}.json")), node.to_string())
            .expect("node fixture");
    }
    std::fs::write(fixtures.join("nodes.txt"), format!("{}\n", ids.join("\n"))).expect("node list");
    std::fs::write(
        fixtures.join(format!(
            "network-{}.json",
            descriptor["network"]["overlay"].as_str().expect("overlay")
        )),
        serde_json::json!({
            "Name": descriptor["network"]["overlay"],
            "Driver": "overlay",
            "Attachable": true,
            "Options": {"encrypted": "true"}
        })
        .to_string(),
    )
    .expect("network fixture");
    for service in APP_SERVICES {
        let replicas = descriptor["services"][service]["replicas"]
            .as_u64()
            .expect("replicas");
        write_service(
            &fixtures,
            &format!("{stack}_{service}"),
            &image,
            replicas,
            Vec::new(),
        );
    }
    let ingress_image = pinned_image(&descriptor["images"]["traefik"]);
    let ingress_replicas = descriptor["ingress"]["replicas"]
        .as_u64()
        .expect("ingress replicas");
    write_service_with_mounts(
        &fixtures,
        &format!("{stack}_ingress"),
        &ingress_image,
        ingress_replicas,
        vec![serde_json::json!({
            "Protocol": "tcp",
            "TargetPort": 443,
            "PublishedPort": 443,
            "PublishMode": "host"
        })],
        serde_json::json!([
            {"ConfigName": "quantrule_traefik", "File": {"Name": "/etc/traefik/traefik.yml", "Mode": 292}},
            {"ConfigName": "quantrule_traefik_dynamic", "File": {"Name": "/etc/traefik/dynamic/dynamic.yml", "Mode": 292}},
            {"ConfigName": "quantrule_ingress_cert", "File": {"Name": "/etc/traefik/certs/ingress.crt", "Mode": 292}}
        ]),
        serde_json::json!([
            {"SecretName": "quantrule_ingress_key", "File": {"Name": "/etc/traefik/certs/ingress.key", "Mode": 256}}
        ]),
    );
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    for service in [
        "gateway",
        "data",
        "model",
        "strategy",
        "backtest",
        "evidence",
        "simulation",
    ] {
        std::fs::write(
            fixtures.join(format!("runtime-{service}.json")),
            serde_json::json!({"image_digest": digest}).to_string(),
        )
        .expect("runtime fixture");
    }
    let script = directory.join("fake-docker");
    write_mode(
        &script,
        r#"#!/bin/sh
set -eu
dir=${QR_FAKE_DOCKER_DIR:?}
if [ -n "${QR_FAKE_DOCKER_LOG:-}" ]; then
    printf '%s\n' "$*" >> "$QR_FAKE_DOCKER_LOG"
fi
case "$1" in
    info) cat "$dir/swarm.json" ;;
    node)
        case "$2" in
            ls) cat "$dir/nodes.txt" ;;
            inspect) cat "$dir/$5.json" ;;
            *) echo "unsupported node command" >&2; exit 2 ;;
        esac
        ;;
    network) cat "$dir/network-$5.json" ;;
    service)
        case "$2" in
            inspect) cat "$dir/service-$5.json" ;;
            ps) cat "$dir/tasks-$7.json" ;;
            *) echo "unsupported service command" >&2; exit 2 ;;
        esac
        ;;
    run)
        url=""
        for argument in "$@"; do url=$argument; done
        name=${url#http://}
        name=${name%%:*}
        cat "$dir/runtime-$name.json"
        ;;
    *) echo "unsupported docker command" >&2; exit 2 ;;
esac
"#,
        0o755,
    );
    let log = directory.join("docker.log");
    (script, fixtures, log)
}

fn remote_envs<'a>(
    probe: &'a Path,
    script: &'a Path,
    fixtures: &'a Path,
    log: &'a Path,
) -> Vec<(&'static str, &'a str)> {
    vec![
        ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
        ("QR_DEPLOY_DOCKER", script.to_str().expect("script")),
        ("QR_FAKE_DOCKER_DIR", fixtures.to_str().expect("fixtures")),
        ("QR_FAKE_DOCKER_LOG", log.to_str().expect("log")),
        (
            "QR_DEPLOY_OPENSSL",
            default_openssl().to_str().expect("openssl"),
        ),
    ]
}

/// One fake `openssl` per test process; the fixture certificate is not real.
fn default_openssl() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let path = std::env::temp_dir().join(format!(
            "qr-fake-openssl-remote-{}",
            std::process::id()
        ));
        write_mode(
            &path,
            "#!/bin/sh\nprintf 'Certificate will not expire\\nX509v3 Subject Alternative Name:\\n    DNS:quantrule.dev, DNS:postgres, DNS:patroni-infra-1, DNS:patroni-infra-2, DNS:patroni-infra-3, DNS:etcd-infra-1, DNS:etcd-infra-2, DNS:etcd-infra-3, DNS:rabbitmq, DNS:rabbitmq-infra-1, DNS:rabbitmq-infra-2, DNS:rabbitmq-infra-3, DNS:minio, DNS:minio-infra-1, DNS:minio-infra-2, DNS:minio-infra-3, DNS:minio-infra-4, DNS:minio-lab-1, DNS:minio-lab-2, DNS:minio-lab-3\\n'\nexit 0\n",
            0o755,
        );
        path
    })
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn write_mode(path: &Path, content: &str, mode: u32) {
    std::fs::write(path, content).expect("write file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }
    #[cfg(not(unix))]
    let _ = mode;
}

fn stage_pki(directory: &Path) -> PathBuf {
    let root = directory.join("pki");
    std::fs::create_dir_all(root.join("postgres")).expect("postgres pki");
    std::fs::create_dir_all(root.join("etcd")).expect("etcd pki");
    std::fs::create_dir_all(root.join("rabbitmq")).expect("rabbitmq pki");
    std::fs::create_dir_all(root.join("minio/CAs")).expect("minio pki");
    let ca = root.join("ca.crt");
    write_mode(&ca, "fixture CA\n", 0o644);
    for (directory, name) in [
        ("postgres", "server"),
        ("etcd", "server"),
        ("etcd", "peer"),
        ("etcd", "client"),
        ("rabbitmq", "server"),
    ] {
        write_mode(
            &root.join(directory).join(format!("{name}.crt")),
            "fixture certificate\n",
            0o644,
        );
        write_mode(
            &root.join(directory).join(format!("{name}.key")),
            "fixture key\n",
            0o600,
        );
    }
    write_mode(
        &root.join("minio/public.crt"),
        "fixture certificate\n",
        0o644,
    );
    write_mode(&root.join("minio/private.key"), "fixture key\n", 0o600);
    write_mode(&root.join("minio/CAs/ca.crt"), "fixture CA\n", 0o644);
    ca
}

/// Create a descriptor whose secret and CA paths are local, mode-600 copies.
fn local_descriptor(directory: &Path) -> PathBuf {
    let mut value: Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/v3/deploy.example.json")).expect("v3 example"),
    )
    .expect("v3 JSON");
    let files = value["secrets"]["files"].as_object_mut().expect("files");
    for (name, path) in files.iter_mut() {
        let content = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}")))
            .expect("fixture secret");
        let destination = directory.join(name);
        write_mode(&destination, &content, 0o600);
        *path = serde_json::json!(destination.display().to_string());
    }
    let ca = stage_pki(directory);
    for endpoint in ["database", "queue", "object_store"] {
        value["endpoints"][endpoint]["tls_ca"] = serde_json::json!(ca.display().to_string());
    }
    let certificate = directory.join("ingress.crt");
    let key = directory.join("ingress.key");
    write_mode(
        &certificate,
        &std::fs::read_to_string(repo("deploy/v3/pki/ingress.crt")).expect("fixture cert"),
        0o644,
    );
    write_mode(
        &key,
        &std::fs::read_to_string(repo("deploy/v3/pki/ingress-key.pem")).expect("fixture key"),
        0o600,
    );
    value["ingress"]["files"] = serde_json::json!({
        "certificate": certificate.display().to_string(),
        "key": key.display().to_string(),
    });
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    path
}

fn descriptor_value(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("descriptor")).expect("JSON")
}

fn probe_fixture(
    directory: &Path,
    descriptor: &Value,
    version_digest: &str,
    state: &str,
) -> PathBuf {
    let dns = descriptor["ingress"]["dns"].as_str().expect("dns");
    let body = format!(r#"{{"state":"{state}"}}"#);
    let version = format!(r#"{{"image_digest":"{version_digest}"}}"#);
    let mut responses = serde_json::Map::new();
    for path in ["health", "ready"] {
        responses.insert(
            format!("https://{dns}/{path}"),
            serde_json::json!({
                "status": 200,
                "headers": {
                    "strict-transport-security": "max-age=31536000; includeSubDomains",
                    "x-content-type-options": "nosniff",
                    "referrer-policy": "no-referrer"
                },
                "body": body
            }),
        );
    }
    responses.insert(
        format!("https://{dns}/version"),
        serde_json::json!({"status": 200, "body": version}),
    );
    responses.insert(
        format!("http://{dns}/"),
        serde_json::json!({"error": "connection refused"}),
    );
    let path = directory.join("probe.json");
    std::fs::write(&path, serde_json::Value::Object(responses).to_string()).expect("fixture");
    path
}

#[test]
fn v3_check_remote_docker_passes_with_a_healthy_fixture() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &envs,
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    let printed = stdout(&output);
    assert!(
        printed.contains("ok\tremote\tswarm managers\tquorum\t3 managers"),
        "{printed}"
    );
    assert!(
        printed.contains("ok\tremote\tservice gateway\tconverged"),
        "{printed}"
    );
    assert!(
        printed.contains("ok\tremote\tports\tsingle-public-port"),
        "{printed}"
    );
    assert!(
        printed.contains(&format!(
            "ok\tremote\trunning task image gateway\tequals\t{}",
            pinned_image(&descriptor["image"])
        )),
        "{printed}"
    );
    assert!(
        printed.contains("ok\tremote\trunning task image ingress\tequals"),
        "{printed}"
    );
    assert!(!printed.contains("FAIL"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_remote_docker_fails_when_an_ingress_mount_is_missing() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    let service = fixtures.join("service-quantrule-ha_ingress.json");
    let mut value: Value =
        serde_json::from_str(&std::fs::read_to_string(&service).expect("ingress service"))
            .expect("service JSON");
    value["Spec"]["TaskTemplate"]["ContainerSpec"]["Configs"]
        .as_array_mut()
        .expect("configs")
        .pop();
    std::fs::write(&service, value.to_string()).expect("write service");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tingress\tservice mounts\tmount"),
        "{printed}"
    );
    assert!(printed.contains("quantrule_ingress_cert"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_overlay_reports_encryption_and_a_limited_dns_row() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--overlay",
        ],
        &envs,
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    let printed = stdout(&output);
    assert!(
        printed.contains("ok\toverlay\tnetwork quantrule\toverlay"),
        "{printed}"
    );
    assert!(
        printed.contains("ok\toverlay\tdns-route\tlimited"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_remote_docker_fails_when_a_service_is_missing() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    std::fs::remove_file(fixtures.join("service-quantrule-ha_gateway.json"))
        .expect("remove gateway fixture");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tremote\tservice gateway\tmissing"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_remote_docker_fails_on_a_wrong_running_task_image() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    let tasks = fixtures.join("tasks-quantrule-ha_gateway.json");
    let text = std::fs::read_to_string(&tasks).expect("gateway tasks");
    let mut rows: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("task row"))
        .collect();
    rows[0]["Image"] = serde_json::json!("ghcr.io/foreign/server@sha256:wrong");
    std::fs::write(
        &tasks,
        rows.iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .expect("write tasks");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tremote\trunning task image gateway\tmismatch"),
        "{printed}"
    );
    assert!(
        printed.contains("ghcr.io/foreign/server@sha256:wrong"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_remote_docker_fails_when_a_service_has_no_running_task() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    std::fs::write(fixtures.join("tasks-quantrule-ha_gateway.json"), "").expect("empty tasks");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tremote\trunning task image gateway\tmissing"),
        "{printed}"
    );
    assert!(printed.contains("FAIL\tremote\ttasks\tfailed"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_runtime_reports_all_seven_service_digests() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
            "--runtime",
        ],
        &envs,
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    let printed = stdout(&output);
    for service in [
        "gateway",
        "data",
        "model",
        "strategy",
        "backtest",
        "evidence",
        "simulation",
    ] {
        assert!(
            printed.contains(&format!("ok\tremote\truntime {service}\tdigest\t{digest}")),
            "{service}: {printed}"
        );
    }
    assert!(!printed.contains("FAIL"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_runtime_fails_closed_on_a_wrong_digest() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    std::fs::write(
        fixtures.join("runtime-data.json"),
        serde_json::json!({"image_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"})
            .to_string(),
    )
    .expect("wrong runtime digest");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
            "--runtime",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tremote\truntime data\tmismatch"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_runtime_requires_remote_docker() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--runtime",
        ],
        &[],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("needs --remote docker"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_remote_docker_fails_on_a_docker_command_error() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let script = directory.join("broken-docker");
    write_mode(
        &script,
        "#!/bin/sh\necho 'cannot connect to the Docker daemon' >&2\nexit 1\n",
        0o755,
    );
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--remote",
            "docker",
        ],
        &[
            ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
            ("QR_DEPLOY_DOCKER", script.to_str().expect("script")),
            (
                "QR_DEPLOY_OPENSSL",
                default_openssl().to_str().expect("openssl"),
            ),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("deploy.remote.docker"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_overlay_fails_when_the_network_is_not_encrypted() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let (script, fixtures, log) = fake_docker(&directory, &descriptor);
    std::fs::write(
        fixtures.join("network-quantrule.json"),
        serde_json::json!({
            "Name": "quantrule",
            "Driver": "overlay",
            "Attachable": true,
            "Options": {}
        })
        .to_string(),
    )
    .expect("unencrypted network");
    let envs = remote_envs(&probe, &script, &fixtures, &log);
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--overlay",
        ],
        &envs,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("not encrypted"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_default_check_never_invokes_docker() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, &descriptor, digest, "healthy");
    let marker = directory.join("docker-called");
    let script = directory.join("marker-docker");
    write_mode(
        &script,
        "#!/bin/sh\nprintf 'called\\n' > \"$QR_FAKE_DOCKER_MARKER\"\nexit 1\n",
        0o755,
    );
    let output = run_envs(
        &["check", descriptor_path.to_str().expect("path")],
        &[
            ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
            ("QR_DEPLOY_DOCKER", script.to_str().expect("script")),
            ("QR_FAKE_DOCKER_MARKER", marker.to_str().expect("marker")),
            (
                "QR_DEPLOY_OPENSSL",
                default_openssl().to_str().expect("openssl"),
            ),
        ],
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(!marker.exists(), "the default check invoked docker");
    let _ = std::fs::remove_dir_all(directory);
}
