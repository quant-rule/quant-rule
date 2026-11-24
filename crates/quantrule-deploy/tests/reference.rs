//! The reference topologies, their sizing contract, and their rendered units.
//!
//! This test is Docker-free: it validates and renders the 3-AZ and single-AZ
//! reference descriptors against fixture secrets, then checks the rendered tree
//! with a recorded HTTPS probe. It also keeps the machine-readable sizing table
//! and the operator contract from drifting.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const THREE_AZ: &str = "deploy/v3/reference/three-az.json";
const SINGLE_AZ: &str = "deploy/v3/reference/single-az.json";
const SIZING: &str = "deploy/v3/reference/sizing.json";
const CONTRACT: &str = "docs/config/2026-11-24/reference-topologies.md";
const ROLES: [&str; 7] = [
    "edge",
    "core",
    "compute",
    "infra",
    "observability",
    "lab",
    "client",
];

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-reference-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn descriptor(path: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(repo(path)).expect("descriptor"))
        .expect("descriptor JSON")
}

fn sizing() -> Value {
    serde_json::from_str(&std::fs::read_to_string(repo(SIZING)).expect("sizing"))
        .expect("sizing JSON")
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
    write_mode(
        &ca,
        "-----BEGIN CERTIFICATE-----\nreference fixture CA\n-----END CERTIFICATE-----\n",
        0o644,
    );
    for (directory, name) in [
        ("postgres", "server"),
        ("etcd", "server"),
        ("etcd", "peer"),
        ("etcd", "client"),
        ("rabbitmq", "server"),
    ] {
        write_mode(
            &root.join(directory).join(format!("{name}.crt")),
            &format!(
                "-----BEGIN CERTIFICATE-----\n{name} reference fixture certificate\n-----END CERTIFICATE-----\n"
            ),
            0o644,
        );
        write_mode(
            &root.join(directory).join(format!("{name}.key")),
            &format!("{name}-reference-fixture-private-key\n"),
            0o600,
        );
    }
    write_mode(
        &root.join("minio/public.crt"),
        "-----BEGIN CERTIFICATE-----\nminio reference fixture certificate\n-----END CERTIFICATE-----\n",
        0o644,
    );
    write_mode(
        &root.join("minio/private.key"),
        "minio-reference-fixture-private-key\n",
        0o600,
    );
    write_mode(
        &root.join("minio/CAs/ca.crt"),
        "-----BEGIN CERTIFICATE-----\nreference fixture CA\n-----END CERTIFICATE-----\n",
        0o644,
    );
    ca
}

fn local_descriptor(descriptor_path: &str, directory: &Path) -> PathBuf {
    let mut value = descriptor(descriptor_path);
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

fn fake_openssl() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("qr-reference-openssl-{}", std::process::id()));
        write_mode(
            &path,
            "#!/bin/sh\nprintf 'Certificate will not expire\\nX509v3 Subject Alternative Name:\\n    DNS:quantrule.dev, DNS:postgres, DNS:patroni-infra-1, DNS:patroni-infra-2, DNS:patroni-infra-3, DNS:patroni-infra-4, DNS:etcd-infra-1, DNS:etcd-infra-2, DNS:etcd-infra-3, DNS:etcd-infra-4, DNS:rabbitmq, DNS:rabbitmq-infra-1, DNS:rabbitmq-infra-2, DNS:rabbitmq-infra-3, DNS:rabbitmq-infra-4, DNS:minio, DNS:minio-infra-1, DNS:minio-infra-2, DNS:minio-infra-3, DNS:minio-infra-4\\n'\nexit 0\n",
            0o755,
        );
        path
    })
}

fn write_probe(directory: &Path, dns: &str, digest: &str) -> PathBuf {
    let body = r#"{"state":"healthy"}"#;
    let version = format!(r#"{{"image_digest":"{digest}"}}"#);
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
    std::fs::write(&path, serde_json::Value::Object(responses).to_string()).expect("probe");
    path
}

fn run(args: &[&str], environment: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_qr-deploy"));
    command.args(args);
    for (key, value) in environment {
        command.env(key, value);
    }
    if !environment
        .iter()
        .any(|(key, _)| *key == "QR_DEPLOY_OPENSSL")
    {
        command.env("QR_DEPLOY_OPENSSL", fake_openssl());
    }
    command.output().expect("qr-deploy")
}

fn validate(path: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["validate", path.to_str().expect("path")];
    args.extend_from_slice(extra);
    run(&args, &[])
}

#[test]
fn every_sizing_role_is_present_and_min_never_exceeds_recommended() {
    let sizing = sizing();
    assert_eq!(sizing["v"], "quantrule.sizing/v1");
    for role in ROLES {
        let value = &sizing["roles"][role];
        assert!(value.is_object(), "{role} is missing from sizing.json");
        for field in ["vcpu", "memory_gib", "disk_gib"] {
            let min = value["min"][field]
                .as_u64()
                .unwrap_or_else(|| panic!("{role}.min.{field} is not a number"));
            let recommended = value["recommended"][field]
                .as_u64()
                .unwrap_or_else(|| panic!("{role}.recommended.{field} is not a number"));
            assert!(min > 0, "{role}.min.{field} must be positive");
            assert!(
                min <= recommended,
                "{role}.{field}: min {min} > recommended {recommended}"
            );
        }
    }
}

#[test]
fn every_reference_host_role_is_covered_by_the_sizing_table() {
    let sizing = sizing();
    let mut seen = std::collections::BTreeSet::new();
    for path in [THREE_AZ, SINGLE_AZ] {
        let value = descriptor(path);
        assert_eq!(
            value["hosts"].as_object().expect("hosts").len(),
            11,
            "{path}"
        );
        for host in value["hosts"].as_object().expect("hosts").keys() {
            let role = host.split('-').next().expect("host prefix");
            assert!(
                ROLES.contains(&role),
                "{path}: host {host} has no sizing role"
            );
            assert!(
                sizing["roles"].get(role).is_some(),
                "{path}: role {role} is absent from sizing.json"
            );
            seen.insert(role.to_owned());
        }
    }
    assert_eq!(
        seen,
        ["compute", "core", "edge", "infra"]
            .map(str::to_owned)
            .into_iter()
            .collect(),
        "the reference descriptors must use the four host roles"
    );
}

#[test]
fn the_contract_sizing_table_matches_the_machine_readable_values() {
    let sizing = sizing();
    let contract = std::fs::read_to_string(repo(CONTRACT)).expect("reference contract");
    for role in ROLES {
        let value = &sizing["roles"][role];
        let min = &value["min"];
        let recommended = &value["recommended"];
        let expected = format!(
            "| `{role}` | {} / {} | {} / {} | {} / {} |",
            min["vcpu"],
            recommended["vcpu"],
            min["memory_gib"],
            recommended["memory_gib"],
            min["disk_gib"],
            recommended["disk_gib"],
        );
        assert!(
            contract.contains(&expected),
            "the contract table does not contain `{expected}`"
        );
    }
    for unsupported in sizing["unsupported"].as_array().expect("unsupported") {
        let text = unsupported.as_str().expect("unsupported text");
        assert!(
            contract.contains(text),
            "the contract does not name the unsupported item {text:?}"
        );
    }
}

#[test]
fn three_az_has_four_racks_across_three_azs_and_one_edge_l2() {
    let value = descriptor(THREE_AZ);
    assert_eq!(value["profile"], "swarm-ha");
    let hosts = value["hosts"].as_object().expect("hosts");
    let domains: std::collections::BTreeSet<&str> = hosts
        .values()
        .map(|host| host["failure_domain"].as_str().expect("domain"))
        .collect();
    assert_eq!(domains.len(), 4, "four rack-level failure domains");
    let azs: std::collections::BTreeSet<&str> = domains
        .iter()
        .map(|domain| domain.split('-').next().expect("az prefix"))
        .collect();
    assert_eq!(azs, ["az1", "az2", "az3"].into_iter().collect());
    let edge_az: std::collections::BTreeSet<&str> = ["edge-1", "edge-2"]
        .iter()
        .map(|host| {
            hosts.get(*host).expect("edge host")["failure_domain"]
                .as_str()
                .expect("domain")
                .split('-')
                .next()
                .expect("az")
        })
        .collect();
    assert_eq!(
        edge_az,
        ["az1"].into_iter().collect(),
        "the VIP pair shares one AZ"
    );
    assert_ne!(
        hosts.get("edge-1").expect("edge-1")["failure_domain"],
        hosts.get("edge-2").expect("edge-2")["failure_domain"],
        "the edge pair still uses distinct racks"
    );
    let managers = value["swarm"]["managers"].as_array().expect("managers");
    assert_eq!(managers.len(), 3);
    let manager_azs: std::collections::BTreeSet<&str> = managers
        .iter()
        .map(|host| {
            hosts
                .get(host.as_str().expect("manager"))
                .expect("manager host")["failure_domain"]
                .as_str()
                .expect("domain")
                .split('-')
                .next()
                .expect("az")
        })
        .collect();
    assert_eq!(manager_azs.len(), 3, "managers span three AZs");
    for unit in ["postgres", "etcd", "rabbitmq"] {
        let nodes = value["stateful"][unit]["nodes"].as_array().expect("nodes");
        assert_eq!(nodes.len(), 3, "{unit}");
        let unit_azs: std::collections::BTreeSet<&str> = nodes
            .iter()
            .map(|host| {
                hosts.get(host.as_str().expect("node")).expect("unit host")["failure_domain"]
                    .as_str()
                    .expect("domain")
                    .split('-')
                    .next()
                    .expect("az")
            })
            .collect();
        assert_eq!(unit_azs.len(), 3, "{unit} spans three AZs");
    }
    let minio = &value["stateful"]["minio"];
    assert_eq!(minio["nodes"].as_array().expect("minio nodes").len(), 4);
    assert_eq!(minio["drives"], serde_json::json!([1, 1, 1, 1]));
    assert_eq!(minio["parity"], "EC:2");
    assert_eq!(
        value["stateful"]["haproxy"]["nodes"],
        serde_json::json!(["edge-1", "edge-2"])
    );
}

#[test]
fn single_az_has_four_racks_in_one_az_and_the_same_eleven_hosts() {
    let value = descriptor(SINGLE_AZ);
    assert_eq!(value["profile"], "single-az");
    let hosts = value["hosts"].as_object().expect("hosts");
    assert_eq!(hosts.len(), 11);
    let domains: std::collections::BTreeSet<&str> = hosts
        .values()
        .map(|host| host["failure_domain"].as_str().expect("domain"))
        .collect();
    assert_eq!(domains.len(), 4);
    assert!(
        domains.iter().all(|domain| domain.starts_with("rack-")),
        "the single-AZ descriptor names racks, not AZs: {domains:?}"
    );
    assert_eq!(
        hosts.keys().collect::<std::collections::BTreeSet<_>>(),
        descriptor(THREE_AZ)["hosts"]
            .as_object()
            .expect("hosts")
            .keys()
            .collect::<std::collections::BTreeSet<_>>(),
        "both reference descriptors use the same host names"
    );
    assert_eq!(
        value["swarm"]["managers"]
            .as_array()
            .expect("managers")
            .len(),
        3
    );
    let minio = &value["stateful"]["minio"];
    assert_eq!(minio["nodes"].as_array().expect("minio nodes").len(), 4);
    assert_eq!(minio["drives"], serde_json::json!([1, 1, 1, 1]));
    assert_eq!(minio["parity"], "EC:2");
}

#[test]
fn both_reference_descriptors_validate_and_release_names_the_remaining_blockers() {
    for path in [THREE_AZ, SINGLE_AZ] {
        let full = repo(path);
        let output = validate(&full, &[]);
        assert!(
            output.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let printed = String::from_utf8_lossy(&output.stdout);
        assert!(printed.contains("11 hosts, 7 services"), "{printed}");
        assert!(printed.contains("files not checked"), "{printed}");

        let output = validate(&full, &["--release"]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("image.digest"), "{stderr}");

        let directory = temp_dir();
        let mut value = descriptor(path);
        value["image"]["digest"] = serde_json::json!(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        let temporary = directory.join("deploy.json");
        std::fs::write(&temporary, value.to_string()).expect("descriptor");
        let output = validate(&temporary, &["--release"]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("images.minio.digest"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        value["images"]["minio"]["digest"] = serde_json::json!(
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        std::fs::write(&temporary, value.to_string()).expect("descriptor");
        let output = validate(&temporary, &["--release"]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("images.minio_client.digest"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn both_reference_descriptors_render_and_check() {
    for path in [THREE_AZ, SINGLE_AZ] {
        let directory = temp_dir();
        let descriptor_path = local_descriptor(path, &directory);
        let out = directory.join("out");
        let output = run(
            &[
                "render",
                descriptor_path.to_str().expect("path"),
                "--out",
                out.to_str().expect("path"),
                "--target",
                "all",
            ],
            &[],
        );
        assert!(
            output.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value = descriptor(path);
        let dns = value["ingress"]["dns"].as_str().expect("dns");
        let digest = value["image"]["digest"].as_str().expect("digest");
        let probe = write_probe(&directory, dns, digest);
        let output = run(
            &[
                "check",
                descriptor_path.to_str().expect("path"),
                "--out",
                out.to_str().expect("path"),
            ],
            &[("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe"))],
        );
        assert!(
            output.status.success(),
            "{path}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let printed = String::from_utf8_lossy(&output.stdout);
        assert!(printed.contains("descriptor\tcontract\tvalid"), "{printed}");
        assert!(printed.contains("placement\thosts\tspread"), "{printed}");
        assert!(printed.contains("ingress"), "{printed}");
        assert!(
            out.join("stack.yaml").is_file(),
            "{path} has no rendered stack"
        );
        assert!(
            out.join("infra/edge-1.yaml").is_file(),
            "{path} has no edge unit"
        );
        assert!(
            out.join("infra/infra-4.yaml").is_file(),
            "{path} has no fourth infra unit"
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}
