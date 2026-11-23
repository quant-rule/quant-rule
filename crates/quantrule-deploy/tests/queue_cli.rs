//! CLI tests for the v3 RabbitMQ queue check rows.
//!
//! The real binary runs with a recorded probe fixture, a fake `openssl`, and a
//! rendered tree. No network, Docker daemon, broker, or real secret is used.

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
        "qr-queue-cli-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn run_envs(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("run qr-deploy")
}

/// One fake `openssl` per test process; the fixture certificate is not real.
fn default_openssl() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("qr-fake-openssl-queue-{}", std::process::id()));
        write_mode(
            &path,
            "#!/bin/sh\nprintf 'Certificate will not expire\\nX509v3 Subject Alternative Name:\\n    DNS:quantrule.dev, DNS:postgres, DNS:patroni-infra-1, DNS:patroni-infra-2, DNS:patroni-infra-3, DNS:etcd-infra-1, DNS:etcd-infra-2, DNS:etcd-infra-3, DNS:rabbitmq, DNS:rabbitmq-infra-1, DNS:rabbitmq-infra-2, DNS:rabbitmq-infra-3\\n'\nexit 0\n",
            0o755,
        );
        path
    })
}

fn stage_pki(directory: &Path) -> PathBuf {
    let root = directory.join("pki");
    std::fs::create_dir_all(root.join("postgres")).expect("postgres pki");
    std::fs::create_dir_all(root.join("etcd")).expect("etcd pki");
    std::fs::create_dir_all(root.join("rabbitmq")).expect("rabbitmq pki");
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
    ca
}

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

fn probe_fixture(directory: &Path, descriptor: &Value, digest: &str) -> PathBuf {
    let dns = descriptor["ingress"]["dns"].as_str().expect("dns");
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
    std::fs::write(&path, serde_json::Value::Object(responses).to_string()).expect("fixture");
    path
}

fn render(descriptor: &Path, out: &Path) -> Output {
    Command::new(binary())
        .args([
            "render",
            descriptor.to_str().expect("path"),
            "--out",
            out.to_str().expect("out"),
            "--target",
            "all",
        ])
        .output()
        .expect("render")
}

#[test]
fn the_default_check_reports_the_queue_rows_with_a_fake_certificate() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest);
    let output = run_envs(
        &["check", descriptor_path.to_str().expect("path")],
        &[
            (
                "QR_DEPLOY_PROBE_FIXTURE",
                fixture.to_str().expect("fixture"),
            ),
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
    let printed = stdout(&output);
    for expected in [
        "ok\tqueue\trabbitmq\ttls-only",
        "ok\tqueue\trabbitmq\tcluster",
        "ok\tqueue\thaproxy\tfrontend",
        "ok\tqueue\thaproxy\tbackend",
        "ok\tqueue\trabbitmq\tunit",
        "ok\tqueue\tmanifest\thosts",
        "ok\tqueue\tmanifest\tbroker-modes",
        "ok\tqueue\tmanifest\tsans",
        "ok\tqueue\trabbitmq certificate\tsan",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    assert!(!printed.contains("FAIL"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn check_out_reads_the_rendered_queue_tree() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest);
    let out = directory.join("out");
    let rendered = render(&descriptor_path, &out);
    assert!(rendered.status.success(), "{}", stderr(&rendered));
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--out",
            out.to_str().expect("out"),
        ],
        &[
            (
                "QR_DEPLOY_PROBE_FIXTURE",
                fixture.to_str().expect("fixture"),
            ),
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
    let printed = stdout(&output);
    assert!(printed.contains("ok\tqueue\trabbitmq\tunit"), "{printed}");
    assert!(
        printed.contains("ok\tqueue\trabbitmq certificate\tsan"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_rendered_plaintext_listener_fails_the_queue_row() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest);
    let out = directory.join("out");
    let rendered = render(&descriptor_path, &out);
    assert!(rendered.status.success(), "{}", stderr(&rendered));
    let path = out.join("rabbitmq-infra-1.conf");
    let text = std::fs::read_to_string(&path).expect("broker config");
    std::fs::write(
        &path,
        text.replace("listeners.tcp = none", "listeners.tcp = 5672"),
    )
    .expect("mutate");
    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--out",
            out.to_str().expect("out"),
        ],
        &[
            (
                "QR_DEPLOY_PROBE_FIXTURE",
                fixture.to_str().expect("fixture"),
            ),
            (
                "QR_DEPLOY_OPENSSL",
                default_openssl().to_str().expect("openssl"),
            ),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tqueue\trabbitmq\tconfiguration"),
        "{printed}"
    );
    assert!(printed.contains("5672"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_missing_openssl_reports_the_queue_certificate_as_limited() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest);
    let missing = directory.join("no-openssl-here");
    let output = run_envs(
        &["check", descriptor_path.to_str().expect("path")],
        &[
            (
                "QR_DEPLOY_PROBE_FIXTURE",
                fixture.to_str().expect("fixture"),
            ),
            (
                "QR_DEPLOY_OPENSSL",
                missing.to_str().expect("missing openssl"),
            ),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("FAIL\tqueue\trabbitmq certificate\tlimited"),
        "{printed}"
    );
    assert!(!printed.contains("FAIL\tqueue\trabbitmq\t"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}
