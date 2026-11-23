//! CLI tests for the v2/v3 boundary and the v3 render/check surface.
//!
//! These run the real binary. The v3 check tests inject a recorded probe
//! fixture through `QR_DEPLOY_PROBE_FIXTURE`, so the fast tier needs no
//! network, no Docker, and no real secret.

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

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("run qr-deploy")
}

fn run_env(args: &[&str], key: &str, value: &str) -> Output {
    run_envs(args, &[(key, value)])
}

fn run_envs(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(binary());
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    if !envs.iter().any(|(key, _)| *key == "QR_DEPLOY_OPENSSL") {
        command.env("QR_DEPLOY_OPENSSL", default_openssl());
    }
    command.output().expect("run qr-deploy")
}

/// One fake `openssl` per test process; the fixture certificate is not a real one.
fn default_openssl() -> &'static Path {
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("qr-fake-openssl-{}", std::process::id()));
        write_mode(
            &path,
            "#!/bin/sh\nprintf 'Certificate will not expire\\nX509v3 Subject Alternative Name:\\n    DNS:quantrule.dev\\n'\nexit 0\n",
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
    let ca = directory.join("ca.crt");
    write_mode(
        &ca,
        "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
        0o644,
    );
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

fn error_fixture(directory: &Path, error: &str) -> PathBuf {
    let path = directory.join("probe-error.json");
    let fixture = serde_json::json!({
        "https://quantrule.dev/health": {"error": error},
        "https://quantrule.dev/ready": {"error": error},
        "https://quantrule.dev/version": {"error": error},
        "http://quantrule.dev/": {"error": error},
    });
    std::fs::write(&path, fixture.to_string()).expect("fixture");
    path
}

#[test]
fn v3_validate_dispatches_and_reports_the_profile() {
    let sample = repo("deploy/v3/deploy.example.json");
    let output = run(&["validate", sample.to_str().expect("path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(
        printed.contains("ok: v3 swarm-ha, 11 hosts, 7 services"),
        "{printed}"
    );
    assert!(
        printed.contains("ingress quantrule.dev:443 (files not checked; pass --files)"),
        "{printed}"
    );
}

#[test]
fn v3_validate_refuses_an_even_or_undeclared_manager_set() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let mut value = descriptor_value(&descriptor_path);
    value["swarm"]["managers"] = serde_json::json!(["edge-1", "core-1"]);
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let output = run(&["validate", descriptor_path.to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("deploy.swarm.managers"),
        "{}",
        stderr(&output)
    );

    let mut value = descriptor_value(&descriptor_path);
    value["swarm"]["managers"] = serde_json::json!(["edge-1", "core-1", "missing"]);
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let output = run(&["validate", descriptor_path.to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("not declared in hosts"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v2_validate_output_is_unchanged() {
    let directory = temp_dir();
    let mut value: Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/multi-host/deploy.example.json"))
            .expect("v2 example"),
    )
    .expect("v2 JSON");
    let digest = "3f5a9c1e7b2d4f608a1c3e5d7b9f0a2c4e6d8f1b3a5c7e9d0f2a4c6e8b1d3f5a";
    value["image"]["digest"] = serde_json::json!(format!("sha256:{digest}"));
    let descriptor = directory.join("deploy.json");
    std::fs::write(&descriptor, value.to_string()).expect("descriptor");

    let output = run(&["validate", descriptor.to_str().expect("path")]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        format!(
            "ok: 4 hosts, 6 services, image sha256:{digest}, gate wireguard (secrets not checked; pass --secrets)\n"
        )
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_validate_files_checks_the_staged_secrets() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let output = run(&["validate", descriptor.to_str().expect("path"), "--files"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("files checked"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_render_all_writes_the_stack_infra_jobs_and_manifest() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let out = directory.join("out");
    let output = run(&[
        "render",
        descriptor.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
        "--target",
        "all",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    for path in [
        "stack.yaml",
        "infra/infra-1.yaml",
        "infra/edge-1.yaml",
        "bootstrap.sh",
        "jobs/db-bootstrap.sh",
        "jobs/migrate.sh",
        "secrets/manifest.json",
        "pki/database-ca.crt",
        "pki/ingress.crt",
        "pki/ingress.key",
        "quantrule_traefik.yml",
        "traefik-dynamic.yml",
        "keepalived-edge-1.conf",
        "keepalived-edge-2.conf",
        "render.json",
    ] {
        assert!(out.join(path).is_file(), "missing {path}");
    }
    let stack = std::fs::read_to_string(out.join("stack.yaml")).expect("stack");
    assert!(stack.contains("published: 443"), "{stack}");
    assert!(stack.contains("mode: \"host\""), "{stack}");
    assert_eq!(stack.matches("published:").count(), 1, "{stack}");
    assert!(!stack.contains("QR_DATABASE_URL="), "{stack}");
    assert!(!stack.contains("postgres://"), "{stack}");
    assert!(stack.contains("quantrule_traefik_dynamic"), "{stack}");
    assert!(
        stack.contains("target: \"/etc/traefik/dynamic/dynamic.yml\""),
        "{stack}"
    );
    assert!(
        stack.contains("target: \"/etc/traefik/certs/ingress.key\""),
        "{stack}"
    );
    assert_eq!(
        stack.matches("order: \"stop-first\"").count(),
        16,
        "{stack}"
    );
    assert_eq!(stack.matches("healthcheck:").count(), 7, "{stack}");
    let dynamic = std::fs::read_to_string(out.join("traefik-dynamic.yml")).expect("dynamic");
    assert!(dynamic.contains("Host(`quantrule.dev`)"), "{dynamic}");
    assert!(
        dynamic.contains("http://quantrule-ha_gateway:8080"),
        "{dynamic}"
    );
    let static_config = std::fs::read_to_string(out.join("quantrule_traefik.yml")).expect("static");
    assert!(
        static_config.contains("address: \":443\""),
        "{static_config}"
    );
    assert!(
        !static_config.contains("address: \":80\""),
        "{static_config}"
    );
    let keepalived =
        std::fs::read_to_string(out.join("keepalived-edge-1.conf")).expect("keepalived");
    assert!(
        keepalived.contains("203.0.113.20/32 dev eth0"),
        "{keepalived}"
    );
    for script in ["bootstrap.sh", "jobs/db-bootstrap.sh", "jobs/migrate.sh"] {
        let checked = Command::new("sh")
            .args(["-n", out.join(script).to_str().expect("path")])
            .output()
            .expect("sh -n");
        assert!(
            checked.status.success(),
            "{script}: {}",
            String::from_utf8_lossy(&checked.stderr)
        );
    }
    let manifest = std::fs::read_to_string(out.join("render.json")).expect("manifest");
    assert!(manifest.contains("descriptor_hash"), "{manifest}");
    assert!(!manifest.contains("timestamp"), "{manifest}");
    for path in [
        "quantrule_traefik.yml",
        "traefik-dynamic.yml",
        "keepalived-edge-1.conf",
        "keepalived-edge-2.conf",
    ] {
        assert!(manifest.contains(path), "{path}: {manifest}");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, expected) in [
            (out.clone(), 0o700),
            (out.join("secrets"), 0o700),
            (out.join("secrets/gateway_token"), 0o600),
            (out.join("pki/database-ca.crt"), 0o644),
            (out.join("pki/ingress.crt"), 0o644),
            (out.join("pki/ingress.key"), 0o600),
            (out.join("quantrule_traefik.yml"), 0o644),
            (out.join("traefik-dynamic.yml"), 0o644),
            (out.join("keepalived-edge-1.conf"), 0o644),
            (out.join("bootstrap.sh"), 0o700),
            (out.join("jobs/migrate.sh"), 0o700),
            (out.join("render.json"), 0o644),
        ] {
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, expected, "{}", path.display());
        }
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn two_renders_are_byte_identical() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let first = directory.join("first");
    let second = directory.join("second");
    for out in [&first, &second] {
        let output = run(&[
            "render",
            descriptor.to_str().expect("path"),
            "--out",
            out.to_str().expect("path"),
            "--target",
            "all",
        ]);
        assert!(output.status.success(), "{}", stderr(&output));
    }
    let mut first_files: Vec<PathBuf> = walk(&first);
    let mut second_files: Vec<PathBuf> = walk(&second);
    first_files.sort();
    second_files.sort();
    assert_eq!(first_files.len(), second_files.len());
    for (a, b) in first_files.iter().zip(&second_files) {
        let relative = a.strip_prefix(&first).expect("relative");
        assert_eq!(relative, b.strip_prefix(&second).expect("relative"));
        assert_eq!(
            std::fs::read(a).expect("first bytes"),
            std::fs::read(b).expect("second bytes"),
            "{}",
            relative.display()
        );
    }
    let _ = std::fs::remove_dir_all(directory);
}

fn walk(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(directory).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
fn the_target_selects_the_expected_half() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let swarm = directory.join("swarm");
    let output = run(&[
        "render",
        descriptor.to_str().expect("path"),
        "--out",
        swarm.to_str().expect("path"),
        "--target",
        "swarm",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(swarm.join("stack.yaml").is_file());
    assert!(!swarm.join("infra").exists());

    let infra = directory.join("infra");
    let output = run(&[
        "render",
        descriptor.to_str().expect("path"),
        "--out",
        infra.to_str().expect("path"),
        "--target",
        "infra",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(infra.join("infra/infra-1.yaml").is_file());
    assert!(!infra.join("stack.yaml").exists());
    assert!(!infra.join("bootstrap.sh").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_render_cert_mode_keeps_the_ingress_key_private() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let mut value = descriptor_value(&descriptor_path);
    let certificate = directory.join("ingress.crt");
    let key = directory.join("ingress.key");
    write_mode(
        &certificate,
        "-----BEGIN CERTIFICATE-----\nfixture\n-----END CERTIFICATE-----\n",
        0o644,
    );
    write_mode(&key, "key-fixture\n", 0o600);
    value["ingress"]["tls"] = serde_json::json!("cert");
    value["ingress"]["files"] = serde_json::json!({
        "certificate": certificate.display().to_string(),
        "key": key.display().to_string(),
    });
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let out = directory.join("out");
    let output = run(&[
        "render",
        descriptor_path.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
        "--target",
        "swarm",
    ]);
    assert!(output.status.success(), "{}", stderr(&output));
    let stack = std::fs::read_to_string(out.join("stack.yaml")).expect("stack");
    assert!(stack.contains("quantrule_ingress_cert"), "{stack}");
    assert!(stack.contains("quantrule_ingress_key"), "{stack}");
    assert!(
        stack.contains("target: \"/etc/traefik/certs/ingress.crt\""),
        "{stack}"
    );
    assert!(
        stack.contains("target: \"/etc/traefik/certs/ingress.key\""),
        "{stack}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for (path, expected) in [
            (out.join("pki/ingress.crt"), 0o644),
            (out.join("pki/ingress.key"), 0o600),
        ] {
            let mode = std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, expected, "{}", path.display());
        }
    }
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_render_requires_a_target_and_v2_refuses_one() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let out = directory.join("out");
    let output = run(&[
        "render",
        descriptor.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("--target"), "{}", stderr(&output));

    let v2 = write_v2(&directory);
    let output = run(&[
        "render",
        v2.to_str().expect("path"),
        "--out",
        out.to_str().expect("path"),
        "--target",
        "all",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("only valid for quantrule.deploy/v3"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

fn write_v2(directory: &Path) -> PathBuf {
    let mut value: Value = serde_json::from_str(
        &std::fs::read_to_string(repo("deploy/multi-host/deploy.example.json"))
            .expect("v2 example"),
    )
    .expect("v2 JSON");
    value["image"]["digest"] = serde_json::json!(format!("sha256:{}", "3".repeat(64)));
    let path = directory.join("v2.json");
    std::fs::write(&path, value.to_string()).expect("v2 descriptor");
    path
}

#[test]
fn v3_check_passes_with_a_healthy_recorded_probe() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest, "healthy");
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(
        printed.contains("evidence\tHA profile (unverified)"),
        "{printed}"
    );
    assert!(printed.contains(digest), "{printed}");
    assert!(printed.contains("ingress"), "{printed}");
    assert!(printed.contains("secret\tgateway_token"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_fails_on_a_digest_mismatch() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let fixture = probe_fixture(
        &directory,
        &descriptor,
        &format!("sha256:{}", "b".repeat(64)),
        "healthy",
    );
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("does not match"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_fails_on_an_unhealthy_ingress() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest, "down");
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("state is down"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_reports_an_unreachable_probe() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let fixture = error_fixture(&directory, "connection refused");
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stdout(&output).contains("connection refused"),
        "{}",
        stdout(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_reports_a_wrong_tls_or_san() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let fixture = error_fixture(
        &directory,
        "invalid peer certificate: certificate is not valid for the requested name",
    );
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(
        printed.contains("not valid for the requested name"),
        "{printed}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_files_reports_a_missing_secret() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let mut value = descriptor_value(&descriptor_path);
    let missing = directory.join("missing-token");
    value["secrets"]["files"]["gateway_token"] = serde_json::json!(missing.display().to_string());
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest, "healthy");
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert_eq!(output.status.code(), Some(1));
    let printed = stdout(&output);
    assert!(printed.contains("deploy.file.unreadable"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_refuses_a_second_public_port() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let mut value = descriptor_value(&descriptor_path);
    value["ingress"]["port"] = serde_json::json!(8443);
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let output = run(&["check", descriptor_path.to_str().expect("path")]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("deploy.ingress.port"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_reports_a_missing_openssl_as_limited() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest, "healthy");
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
        printed.contains("FAIL\tingress\tcertificate\tlimited"),
        "{printed}"
    );
    assert!(printed.contains("cannot run"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn v3_check_labels_a_lab_as_simulated_ha() {
    let directory = temp_dir();
    let descriptor_path = local_descriptor(&directory);
    let mut value = descriptor_value(&descriptor_path);
    value["profile"] = serde_json::json!("lab-single-host");
    std::fs::write(&descriptor_path, value.to_string()).expect("descriptor");
    let descriptor = descriptor_value(&descriptor_path);
    let digest = descriptor["image"]["digest"].as_str().expect("digest");
    let fixture = probe_fixture(&directory, &descriptor, digest, "healthy");
    let output = run_env(
        &["check", descriptor_path.to_str().expect("path")],
        "QR_DEPLOY_PROBE_FIXTURE",
        fixture.to_str().expect("fixture"),
    );
    assert!(output.status.success(), "{}", stderr(&output));
    let printed = stdout(&output);
    assert!(printed.contains("Simulated HA"), "{printed}");
    assert!(!printed.contains("Real HA"), "{printed}");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn the_old_and_new_flag_surfaces_do_not_cross() {
    let directory = temp_dir();
    let v2 = write_v2(&directory);
    let output = run(&["validate", v2.to_str().expect("path"), "--files"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("only valid for quantrule.deploy/v3"));

    let v3 = repo("deploy/v3/deploy.example.json");
    let output = run(&[
        "validate",
        v3.to_str().expect("path"),
        "--secrets",
        "secrets.env",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("use --files"));

    let output = run(&["check", v2.to_str().expect("path"), "--remote", "docker"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("only valid for quantrule.deploy/v3"),
        "{}",
        stderr(&output)
    );
    let _ = std::fs::remove_dir_all(directory);
}
