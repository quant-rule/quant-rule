//! Guard the release image reference, the seven-service pin, and release mode.
//!
//! The fixture digest is a labelled example, never a published image. The
//! rendered goldens prove every service uses the same `reference@digest` and
//! `QR_IMAGE_DIGEST`; release mode refuses the fixture while the default check
//! stays green.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const REFERENCE: &str = "ghcr.io/quant-rule/quantrule-platform";
const FIXTURE: &str = "sha256:cdace4a996c06cd6142fb63eb23a52136c071d9450fe0e07f5556fc26b9edabe";
const SAMPLE: &str = "deploy/v3/deploy.example.json";
const LAB: &str = "deploy/v3/deploy.lab.json";
const SAMPLE_STACK: &str = "crates/quantrule-deploy/tests/fixtures/render-v3/expected/stack.yaml";
const LAB_STACK: &str = "crates/quantrule-deploy/tests/fixtures/render-v3-lab/expected/stack.yaml";

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
fn release_mode_refuses_the_fixture_digest_and_the_default_check_stays_green() {
    for path in [SAMPLE, LAB] {
        let output = validate(path, &[], &[]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = validate(path, &["--release"], &[]);
        assert_eq!(output.status.code(), Some(1), "{path}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("deploy.image.fixture"),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = validate(path, &[], &[("QR_DEPLOY_RELEASE", "1")]);
        assert_eq!(output.status.code(), Some(1), "{path}");
    }
}

#[test]
fn release_mode_accepts_a_descriptor_with_a_real_style_digest() {
    let directory = temp_dir();
    let mut value = descriptor(SAMPLE);
    value["image"]["digest"] = Value::String(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
    );
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
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
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("deploy.image.fixture"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_bare_app_image_reference_is_refused_in_default_mode() {
    let directory = temp_dir();
    let mut value = descriptor(SAMPLE);
    value["image"]["reference"] = Value::String("quantrule-server".to_owned());
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    let output = validate(path.to_str().expect("path"), &[], &[]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("deploy.image.reference"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}
