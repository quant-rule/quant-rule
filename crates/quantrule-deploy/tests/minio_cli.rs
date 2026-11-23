//! CLI tests for the MinIO check rows.
//!
//! `tests/cli.rs` is at its size limit; this file keeps the MinIO surface
//! separate. The tests run the real binary with a recorded probe and a fake
//! `openssl`, so the fast tier needs no network, Docker, or real secret.

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
        "qr-minio-cli-{}-{}",
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

fn probe_fixture(directory: &Path, digest: &str) -> PathBuf {
    let responses = serde_json::json!({
        "https://quantrule.dev/health": {
            "status": 200,
            "headers": {
                "strict-transport-security": "max-age=31536000; includeSubDomains",
                "x-content-type-options": "nosniff",
                "referrer-policy": "no-referrer"
            },
            "body": "{\"state\":\"healthy\"}"
        },
        "https://quantrule.dev/ready": {"status": 200, "body": "{\"state\":\"healthy\"}"},
        "https://quantrule.dev/version": {"status": 200, "body": format!("{{\"image_digest\":\"{digest}\"}}")},
        "http://quantrule.dev/": {"error": "connection refused"}
    });
    let path = directory.join("probe.json");
    std::fs::write(&path, responses.to_string()).expect("fixture");
    path
}

fn fake_openssl(directory: &Path) -> PathBuf {
    let path = directory.join("openssl");
    write_mode(
        &path,
        "#!/bin/sh\nprintf 'Certificate will not expire\\nX509v3 Subject Alternative Name:\\n    DNS:quantrule.dev, DNS:postgres, DNS:patroni-infra-1, DNS:patroni-infra-2, DNS:patroni-infra-3, DNS:etcd-infra-1, DNS:etcd-infra-2, DNS:etcd-infra-3, DNS:rabbitmq, DNS:rabbitmq-infra-1, DNS:rabbitmq-infra-2, DNS:rabbitmq-infra-3, DNS:minio, DNS:minio-infra-1, DNS:minio-infra-2, DNS:minio-infra-3, DNS:minio-infra-4\\n'\nexit 0\n",
        0o755,
    );
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn the_minio_rows_appear_in_the_default_check_and_hide_secrets() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor: Value =
        serde_json::from_str(&std::fs::read_to_string(&descriptor_path).expect("descriptor"))
            .expect("JSON");
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, digest);
    let openssl = fake_openssl(&directory);
    let output = run_envs(
        &["check", descriptor_path.to_str().expect("path")],
        &[
            ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
            ("QR_DEPLOY_OPENSSL", openssl.to_str().expect("openssl")),
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
        "ok\tminio\tminio\tconf",
        "ok\tminio\tminio\tquorum",
        "ok\tminio\tminio\tunit",
        "ok\tminio\thaproxy\tfrontend",
        "ok\tminio\tmanifest\tsans",
        "ok\tminio\tminio certificate\tsan",
        "ok\tminio\tapp\tcredentials",
        "qr_app",
    ] {
        assert!(printed.contains(expected), "{expected}: {printed}");
    }
    for name in [
        "minio_root_password",
        "minio_root_user",
        "object_store_secret_key",
    ] {
        let value =
            std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}"))).expect("secret");
        assert!(!printed.contains(value.trim()), "the check printed {name}");
    }
    assert!(!printed.contains("Real HA"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_check_reads_a_rendered_tree_and_reports_a_mutated_minio_config() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor: Value =
        serde_json::from_str(&std::fs::read_to_string(&descriptor_path).expect("descriptor"))
            .expect("JSON");
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let probe = probe_fixture(&directory, digest);
    let openssl = fake_openssl(&directory);
    let out = directory.join("out");
    let render = Command::new(binary())
        .args([
            "render",
            descriptor_path.to_str().expect("path"),
            "--out",
            out.to_str().expect("out"),
            "--target",
            "all",
        ])
        .output()
        .expect("render");
    assert!(render.status.success(), "{}", stderr(&render));

    let output = run_envs(
        &[
            "check",
            descriptor_path.to_str().expect("path"),
            "--out",
            out.to_str().expect("out"),
        ],
        &[
            ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
            ("QR_DEPLOY_OPENSSL", openssl.to_str().expect("openssl")),
        ],
    );
    assert!(
        output.status.success(),
        "{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(
        stdout(&output).contains("ok\tminio\tminio\tconf"),
        "{}",
        stdout(&output)
    );

    let path = out.join("minio-infra-1.conf");
    let text = std::fs::read_to_string(&path).expect("conf");
    std::fs::write(
        &path,
        text.replace("MINIO_BROWSER=\"off\"", "MINIO_BROWSER=\"on\""),
    )
    .expect("mutate");
    let mutated = run_envs(
        &["check", descriptor_path.to_str().expect("path")],
        &[
            ("QR_DEPLOY_PROBE_FIXTURE", probe.to_str().expect("probe")),
            ("QR_DEPLOY_OPENSSL", openssl.to_str().expect("openssl")),
            ("QR_DEPLOY_OUT", out.to_str().expect("out")),
        ],
    );
    assert_eq!(mutated.status.code(), Some(1));
    assert!(
        stdout(&mutated).contains("MINIO_BROWSER"),
        "{}",
        stdout(&mutated)
    );
    let _ = std::fs::remove_dir_all(directory);
}
