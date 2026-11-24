//! Guard the release image reference, the seven-service pin, release mode, and
//! every labelled fixture or blocker digest.
//!
//! The app, MinIO, and MinIO-client digests are labelled examples, never a
//! published image. The rendered goldens prove every service uses the same
//! `reference@digest` and `QR_IMAGE_DIGEST`; release mode refuses the app
//! fixture, the MinIO blockers, and every retired infrastructure fixture at the
//! exact field, so a reverted pin cannot pass silently.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const REFERENCE: &str = "ghcr.io/quant-rule/quantrule-platform";
const FIXTURE: &str = "sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe";
const MINIO_FIXTURE: &str =
    "sha256:bdf0781acceb93983c15eec9e56bfbd59dcadc98ea57467022d61aeee074a2cc";
const MINIO_CLIENT_FIXTURE: &str =
    "sha256:5c5b1f8f2a4e3d6c9b7a1e0f8d2c4b6a9e7f1d3c5b8a0e2f4d6c8b1a3e5f7d9c";
const REAL_STYLE: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SAMPLE: &str = "deploy/v3/deploy.example.json";
const LAB: &str = "deploy/v3/deploy.lab.json";
const THREE_AZ: &str = "deploy/v3/reference/three-az.json";
const SINGLE_AZ: &str = "deploy/v3/reference/single-az.json";
const SAMPLE_STACK: &str = "crates/quantrule-deploy/tests/fixtures/render-v3/expected/stack.yaml";
const LAB_STACK: &str = "crates/quantrule-deploy/tests/fixtures/render-v3-lab/expected/stack.yaml";

/// The retired infrastructure fixture digests, by image key. A revert to any of
/// them must fail release mode at `images.<key>.digest`.
const OLD_INFRA: [(&str, &str); 16] = [
    (
        "traefik",
        "sha256:8330e8d28853641532609f63676dc2eec8d19e2bc7fadd5505a1d0c8c254d042",
    ),
    (
        "postgres",
        "sha256:dc184ed86a2b6421fd5ea1bf2ffa3d6e9f1df86796bf0830a7476eb064a186ff",
    ),
    (
        "patroni",
        "sha256:e78141e6fe0807c466f17d8343f99249c7480a94d603bc03f91b10e79e7b94fd",
    ),
    (
        "etcd",
        "sha256:a6cec115044c30857d8458d120facbaa2251f21fd4ad39296a99017468d482dc",
    ),
    (
        "haproxy",
        "sha256:2a88fcd69de33b07362a9b98f822f2b31f7607932356829cee9a5d44d851a2db",
    ),
    (
        "keepalived",
        "sha256:67a5f8c8ca46f83c69a30376f47eaa4704f29a0495675433812d5216b0171682",
    ),
    (
        "rabbitmq",
        "sha256:605a57a95a166325381f132867163eafc20cb0ece6039504e45681136550e6b7",
    ),
    ("minio", MINIO_FIXTURE),
    ("minio_client", MINIO_CLIENT_FIXTURE),
    (
        "prometheus",
        "sha256:c4bfb1e3ef91709eca17ae08b3676ac008939765ad9cf6c2b4617f363b1354be",
    ),
    (
        "grafana",
        "sha256:2e28e87d9b98ca68880a966df73ad63f90db6b4df6ae86637e130f52f48264ba",
    ),
    (
        "loki",
        "sha256:e13e67451cba9b2074bf13220303cb12e4ef2316709cd6f0fbd6f8f76b59e2d5",
    ),
    (
        "alertmanager",
        "sha256:62187e72213c4a4a0d1e1634c2350b73f3a1320777fb2274827a1f8c524f0ff2",
    ),
    (
        "node_exporter",
        "sha256:c26a6b6c79c6a303387b01b3dcfc34824e4cf03fe9c2e9899a762b601e46a680",
    ),
    (
        "promtail",
        "sha256:60b24aa368eea19a4fe8d35e294777963ba4a21f908a293de6f9e90d409ca919",
    ),
    (
        "blackbox_exporter",
        "sha256:a4e65b27bff6ae4a3491998b70df242bafffc87b0ddacbd7f9d0a4132395732a",
    ),
];

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-release-image-{}-{}",
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

fn write_descriptor(directory: &Path, value: &Value) -> PathBuf {
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    path
}

/// Replace the three remaining blockers with format-valid non-fixture digests.
fn unblock_minio(value: &mut Value) {
    value["image"]["digest"] = json!(REAL_STYLE);
    value["images"]["minio"]["digest"] = json!(REAL_STYLE);
    value["images"]["minio_client"]["digest"] = json!(REAL_STYLE);
}

#[test]
fn the_sample_and_lab_name_the_public_ghcr_package_and_fixture_digest() {
    for path in [SAMPLE, LAB] {
        let value = descriptor(path);
        assert_eq!(value["image"]["reference"], REFERENCE, "{path}");
        assert_eq!(value["image"]["digest"], FIXTURE, "{path}");
        assert!(
            value["images"]
                .as_object()
                .is_some_and(|images| images.len() == 16),
            "{path} must keep the 16 infrastructure pins"
        );
    }
}

#[test]
fn every_checked_in_infrastructure_pin_is_resolved_except_the_two_blockers() {
    for path in [SAMPLE, LAB, THREE_AZ, SINGLE_AZ] {
        let value = descriptor(path);
        let images = value["images"].as_object().expect("images");
        assert_eq!(images.len(), 16, "{path}");
        for (key, pin) in images {
            let digest = pin["digest"].as_str().expect("digest");
            match key.as_str() {
                "minio" => assert_eq!(digest, MINIO_FIXTURE, "{path}"),
                "minio_client" => assert_eq!(digest, MINIO_CLIENT_FIXTURE, "{path}"),
                _ => assert!(
                    !OLD_INFRA.iter().any(|(_, old)| *old == digest),
                    "{path}: images.{key}.digest is still a retired fixture"
                ),
            }
        }
    }
}

#[test]
fn every_rendered_service_uses_the_same_reference_digest_and_qr_image_digest() {
    for path in [SAMPLE_STACK, LAB_STACK] {
        let text = std::fs::read_to_string(repo(path)).expect("golden stack");
        let image = format!("image: \"{REFERENCE}@{FIXTURE}\"");
        let digest = format!("QR_IMAGE_DIGEST: \"{FIXTURE}\"");
        assert_eq!(
            text.matches(&image).count(),
            7,
            "{path} does not pin seven service images"
        );
        assert_eq!(
            text.matches(&digest).count(),
            7,
            "{path} does not set seven QR_IMAGE_DIGEST values"
        );
        assert!(
            !text.contains("registry.quantrule.dev"),
            "{path} still names the retired fixture registry"
        );
    }
}

#[test]
fn the_fixture_digest_is_labelled_in_the_deploy_readme() {
    let readme = std::fs::read_to_string(repo("deploy/v3/README.md")).expect("README");
    assert!(readme.contains(FIXTURE), "the fixture digest is not named");
    assert!(
        readme.to_lowercase().contains("fixture"),
        "the digest is not labelled as a fixture"
    );
    assert!(
        !repo("deploy/v3/cosign.pub").exists(),
        "a real public key was committed"
    );
    assert!(repo("deploy/v3/cosign.pub.example").is_file());
}

fn validate(path: &str, extra: &[&str], environment: &[(&str, &str)]) -> std::process::Output {
    let resolved = if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        repo(path)
    };
    let mut command = Command::new(env!("CARGO_BIN_EXE_qr-deploy"));
    command
        .args(["validate", resolved.to_str().expect("path")])
        .args(extra);
    for (key, value) in environment {
        command.env(key, value);
    }
    command.output().expect("qr-deploy validate")
}

#[test]
fn release_mode_refuses_the_fixture_app_digest_and_the_default_check_stays_green() {
    for path in [SAMPLE, LAB] {
        let output = validate(path, &[], &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = validate(path, &["--release"], &[]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("deploy.image.fixture"), "{stderr}");
        assert!(stderr.contains("image.digest"), "{stderr}");
        let output = validate(path, &[], &[("QR_DEPLOY_RELEASE", "1")]);
        assert_eq!(output.status.code(), Some(1), "{path}");
    }
}

#[test]
fn release_mode_names_the_minio_and_minio_client_blockers() {
    for path in [SAMPLE, LAB] {
        let directory = temp_dir();
        let mut value = descriptor(path);
        value["image"]["digest"] = json!(REAL_STYLE);
        let descriptor = write_descriptor(&directory, &value);

        let output = validate(descriptor.to_str().expect("path"), &["--release"], &[]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("images.minio.digest"), "{stderr}");
        assert!(stderr.contains("MinIO blocker"), "{stderr}");

        value["images"]["minio"]["digest"] = json!(REAL_STYLE);
        let descriptor = write_descriptor(&directory, &value);
        let output = validate(descriptor.to_str().expect("path"), &["--release"], &[]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("images.minio_client.digest"), "{stderr}");
        assert!(stderr.contains("MinIO client blocker"), "{stderr}");

        value["images"]["minio_client"]["digest"] = json!(REAL_STYLE);
        let descriptor = write_descriptor(&directory, &value);
        let output = validate(descriptor.to_str().expect("path"), &["--release"], &[]);
        assert!(
            output.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn release_mode_refuses_every_retired_infrastructure_fixture_at_its_field() {
    for (key, digest) in OLD_INFRA {
        let directory = temp_dir();
        let mut value = descriptor(SAMPLE);
        unblock_minio(&mut value);
        value["images"][key]["digest"] = json!(digest);
        let descriptor = write_descriptor(&directory, &value);
        let output = validate(descriptor.to_str().expect("path"), &["--release"], &[]);
        assert_eq!(output.status.code(), Some(1), "{key} was accepted");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("images.{key}.digest")),
            "{key}: {stderr}"
        );
        assert!(stderr.contains("deploy.image.fixture"), "{key}: {stderr}");
        let _ = std::fs::remove_dir_all(directory);
    }
}

#[test]
fn release_mode_accepts_a_fully_pinned_descriptor() {
    let directory = temp_dir();
    let mut value = descriptor(SAMPLE);
    unblock_minio(&mut value);
    let path = write_descriptor(&directory, &value);
    let output = validate(path.to_str().expect("path"), &["--release"], &[]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn release_mode_check_refuses_the_fixture_before_opening_files() {
    let output = Command::new(env!("CARGO_BIN_EXE_qr-deploy"))
        .args(["check", repo(SAMPLE).to_str().expect("path"), "--release"])
        .output()
        .expect("qr-deploy check");
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("deploy.image.fixture"), "{stderr}");
    assert!(stderr.contains("image.digest"), "{stderr}");
}

#[test]
fn a_bare_app_image_reference_is_refused_in_default_mode() {
    let directory = temp_dir();
    let mut value = descriptor(SAMPLE);
    value["image"]["reference"] = Value::String("quantrule-server".to_owned());
    let path = write_descriptor(&directory, &value);
    let output = validate(path.to_str().expect("path"), &[], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("deploy.image.reference"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}
