//! Shell contract for `tools/release.sh`.
//!
//! These tests never use the network, Docker, a registry, a real signing key,
//! cosign, syft, or Grype. They prove `--plan`, argument validation, descriptor
//! pinning, the allowlist gate, the secret-free sign path, and the fail-closed
//! verification path with small fake tools.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-release-tool-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("temp dir");
    path
}

fn write_mode(path: &Path, content: &str, mode: u32) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent");
    }
    std::fs::write(path, content).expect("write file");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("mode");
    }
    #[cfg(not(unix))]
    let _ = mode;
}

fn run(args: &[&str], environment: &[(&str, &str)]) -> Output {
    let mut command = Command::new("bash");
    command.arg(repo("tools/release.sh")).args(args);
    for (key, value) in environment {
        command.env(key, value);
    }
    command.output().expect("run tools/release.sh")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fake_tools(directory: &Path) -> PathBuf {
    let bin = directory.join("bin");
    write_mode(
        &bin.join("grype"),
        r#"#!/bin/sh
set -eu
out=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --file) out=$2; shift ;;
  esac
  shift
done
[ -n "$out" ] || exit 1
cp "$FAKE_GRYPE_JSON" "$out"
"#,
        0o755,
    );
    write_mode(
        &bin.join("cargo"),
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = "audit" ]; then
  cat "$FAKE_CARGO_JSON"
  exit "${FAKE_CARGO_EXIT:-0}"
fi
exit 0
"#,
        0o755,
    );
    write_mode(
        &bin.join("cosign"),
        r#"#!/bin/sh
set -eu
printf 'cosign %s\n' "$*" >> "$FAKE_COSIGN_LOG"
sub=${1:-}
shift || true
bundle=""
directory=""
local_image=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --bundle) bundle=${2:-}; shift ;;
    --dir) directory=${2:-}; shift ;;
    --local-image) local_image=${2:-}; shift ;;
  esac
  shift || true
done
case "$sub" in
  sign|attest)
    [ -n "$bundle" ] || exit 1
    printf '{"fake":"%s"}\n' "$sub" > "$bundle"
    ;;
  save)
    [ -n "$directory" ] || exit 1
    mkdir -p "$directory"
    printf '{"schemaVersion":2}\n' > "$directory/index.json"
    printf '{"imageLayoutVersion":"1.0.0"}\n' > "$directory/oci-layout"
    ;;
  verify)
    if [ -n "$local_image" ] && [ ! -d "$local_image" ]; then
      exit 1
    fi
    ;;
esac
exit 0
"#,
        0o755,
    );
    write_mode(
        &bin.join("skopeo"),
        r#"#!/bin/sh
set -eu
[ "${1:-}" = "copy" ] || exit 2
destination=${3:-}
case "$destination" in
  oci:*)
    directory=${destination#oci:}
    directory=${directory%:latest}
    mkdir -p "$directory/blobs/sha256"
    printf '{"schemaVersion":2}\n' > "$directory/index.json"
    printf 'fixture layer\n' > "$directory/blobs/sha256/fixture"
    ;;
  *) exit 2 ;;
esac
"#,
        0o755,
    );
    write_mode(
        &bin.join("syft"),
        r#"#!/bin/sh
set -eu
out=""
for argument in "$@"; do
  case "$argument" in
    spdx-json=*) out=${argument#spdx-json=} ;;
  esac
done
[ -n "$out" ] || exit 1
printf '{"spdxVersion":"SPDX-2.3"}\n' > "$out"
"#,
        0o755,
    );
    bin
}

fn with_path(bin: &Path, environment: &mut Vec<(String, String)>) {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_owned())
    );
    environment.push(("PATH".to_owned(), path));
}

#[test]
fn the_tool_is_valid_shell() {
    let output = Command::new("bash")
        .args(["-n", repo("tools/release.sh").to_str().expect("path")])
        .output()
        .expect("bash -n");
    assert_success(&output);
}

#[test]
fn the_plan_is_offline_and_names_the_pins_and_manual_signing_step() {
    let output = run(&["--plan"], &[]);
    assert_success(&output);
    let text = String::from_utf8_lossy(&output.stdout);
    for expected in [
        "no Docker, network, registry, signing key, or secret is read",
        "staging-<commit>",
        "syft 1.57.2",
        "Grype 0.122.1",
        "cargo-audit 0.23.0",
        "cosign=v3.1.4",
        "skopeo image=quay.io/skopeo/stable@sha256:",
        "registry image=registry:2@sha256:",
        "manual: after the version image is pushed",
        "tools/release.sh sign",
        "tools/release.sh verify",
        "cosign verify --key cosign.pub --local-image",
        "signed-image",
        "16 infrastructure image pins are fixtures",
        "Unverified",
        "Real HA is HA-014",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    for secret in [
        "gateway_token",
        "postgres_superuser_password",
        "minio_root_password",
    ] {
        let value = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{secret}")))
            .expect("fixture secret")
            .trim()
            .to_owned();
        assert!(!text.contains(&value), "{secret} leaked into the plan");
    }
}

#[test]
fn the_tool_has_no_destructive_or_unverified_download_path() {
    let text = std::fs::read_to_string(repo("tools/release.sh")).expect("tool");
    assert!(!text.contains("set -x"), "{text}");
    for forbidden in [
        "wget | sh",
        "docker push",
        "rm -rf /",
        "cosign generate-key-pair",
    ] {
        assert!(!text.contains(forbidden), "{forbidden}: {text}");
    }
    for expected in [
        "sha256sum -c",
        "QR_RELEASE_PRIVATE_KEY",
        "COSIGN_PASSWORD",
        "attest --type spdxjson",
        "save --dir",
        "--local-image",
        "REGISTRY_IMAGE=",
        "skopeo",
    ] {
        assert!(text.contains(expected), "{expected}");
    }
}

#[test]
fn an_unknown_command_or_missing_argument_is_a_usage_error() {
    assert_eq!(run(&["explode"], &[]).status.code(), Some(2));
    assert_eq!(run(&["sbom", DIGEST], &[]).status.code(), Some(2));
    assert_eq!(
        run(&["pin", "--reference", "ghcr.io/a/b"], &[])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        run(
            &[
                "pin",
                "--descriptor",
                "x",
                "--reference",
                "ghcr.io/a/b",
                "--digest",
                "sha256:abc"
            ],
            &[]
        )
        .status
        .code(),
        Some(1)
    );
}

#[test]
fn pin_rewrites_only_the_app_image_and_validates() {
    let directory = temp_dir();
    let descriptor = directory.join("deploy.json");
    std::fs::copy(repo("deploy/v3/deploy.example.json"), &descriptor).expect("descriptor");
    let before: Value =
        serde_json::from_str(&std::fs::read_to_string(&descriptor).expect("read")).expect("json");
    let output = run(
        &[
            "pin",
            "--descriptor",
            descriptor.to_str().expect("path"),
            "--reference",
            "ghcr.io/quant-rule/quantrule-platform",
            "--digest",
            DIGEST,
        ],
        &[("QR_DEPLOY_BIN", env!("CARGO_BIN_EXE_qr-deploy"))],
    );
    assert_success(&output);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains(&format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}")),
        "{text}"
    );
    let after: Value =
        serde_json::from_str(&std::fs::read_to_string(&descriptor).expect("read")).expect("json");
    assert_eq!(
        after["image"]["reference"],
        "ghcr.io/quant-rule/quantrule-platform"
    );
    assert_eq!(after["image"]["digest"], DIGEST);
    assert_eq!(after["images"], before["images"], "an infra pin changed");
    let _ = std::fs::remove_dir_all(directory);
}

fn allowlist(directory: &Path, entries: &str) -> PathBuf {
    let path = directory.join("allowlist.json");
    std::fs::write(
        &path,
        format!("{{\"v\":\"quantrule.cve-allowlist/v1\",\"entries\":[{entries}]}}\n"),
    )
    .expect("allowlist");
    path
}

fn scan(
    directory: &Path,
    grype: &str,
    cargo: &str,
    cargo_exit: &str,
    allowlist_path: &Path,
) -> Output {
    let bin = fake_tools(directory);
    let grype_path = directory.join("grype.json");
    let cargo_path = directory.join("cargo.json");
    std::fs::write(&grype_path, grype).expect("grype fixture");
    std::fs::write(&cargo_path, cargo).expect("cargo fixture");
    let sbom = directory.join("sbom.spdx.json");
    std::fs::write(&sbom, "{}\n").expect("sbom");
    let output_dir = directory.join("cve");
    let mut environment = vec![
        (
            "FAKE_GRYPE_JSON".to_owned(),
            grype_path.display().to_string(),
        ),
        (
            "FAKE_CARGO_JSON".to_owned(),
            cargo_path.display().to_string(),
        ),
        ("FAKE_CARGO_EXIT".to_owned(), cargo_exit.to_owned()),
        (
            "GRYPE_BIN".to_owned(),
            bin.join("grype").display().to_string(),
        ),
        (
            "QR_RELEASE_ALLOWLIST".to_owned(),
            allowlist_path.display().to_string(),
        ),
        ("QR_RELEASE_TODAY".to_owned(), "2026-11-24".to_owned()),
    ];
    with_path(&bin, &mut environment);
    let refs: Vec<(&str, &str)> = environment
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    run(
        &[
            "scan",
            sbom.to_str().expect("sbom path"),
            output_dir.to_str().expect("out path"),
        ],
        &refs,
    )
}

fn summary(directory: &Path) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(directory.join("cve/summary.json")).expect("summary"),
    )
    .expect("summary json")
}

const EMPTY_GRYPE: &str = "{\"matches\":[]}";
const EMPTY_CARGO: &str = "{\"vulnerabilities\":{\"list\":[]}}";

#[test]
fn a_clean_scan_passes_and_writes_a_summary() {
    let directory = temp_dir();
    let allow = allowlist(&directory, "");
    let output = scan(&directory, EMPTY_GRYPE, EMPTY_CARGO, "0", &allow);
    assert_success(&output);
    let report = summary(&directory);
    assert_eq!(report["gate"], "pass");
    assert_eq!(report["counts"]["critical"], 0);
    assert_eq!(report["counts"]["violations"], 0);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn critical_and_fixable_high_fail_the_gate() {
    let directory = temp_dir();
    let allow = allowlist(&directory, "");
    let critical = r#"{"matches":[{"vulnerability":{"id":"CVE-C","severity":"Critical","fix":{"state":"fixed","versions":["1"]}},"artifact":{"name":"openssl","version":"3"}}]}"#;
    let output = scan(&directory, critical, EMPTY_CARGO, "0", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("FAIL Critical"));
    assert_eq!(summary(&directory)["gate"], "fail");

    let directory = temp_dir();
    let allow = allowlist(&directory, "");
    let fixed = r#"{"matches":[{"vulnerability":{"id":"CVE-H","severity":"High","fix":{"state":"fixed","versions":["1"]}},"artifact":{"name":"openssl","version":"3"}}]}"#;
    let output = scan(&directory, fixed, EMPTY_CARGO, "0", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixable High"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn an_unfixed_high_needs_a_valid_unexpired_allowlist_entry() {
    let directory = temp_dir();
    let unfixed = r#"{"matches":[{"vulnerability":{"id":"CVE-H","severity":"High","fix":{"state":"not-fixed","versions":[]}},"artifact":{"name":"openssl","version":"3"}}]}"#;
    let allow = allowlist(&directory, "");
    let output = scan(&directory, unfixed, EMPTY_CARGO, "0", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("without allowlist"));

    let directory = temp_dir();
    let entry = r#"{"id":"CVE-H","package":"openssl","version":"3","severity":"High","reason":"no upstream fix","owner":"release","expires":"2099-01-01","ticket":"ISSUE-1"}"#;
    let allow = allowlist(&directory, entry);
    let output = scan(&directory, unfixed, EMPTY_CARGO, "0", &allow);
    assert_success(&output);
    assert_eq!(summary(&directory)["counts"]["allowlisted"], 1);

    let directory = temp_dir();
    let expired = r#"{"id":"CVE-H","package":"openssl","version":"3","severity":"High","reason":"no upstream fix","owner":"release","expires":"2020-01-01","ticket":"ISSUE-1"}"#;
    let allow = allowlist(&directory, expired);
    let output = scan(&directory, unfixed, EMPTY_CARGO, "0", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("expired"));

    let directory = temp_dir();
    let malformed = r#"{"id":"CVE-H","package":"openssl","version":"3","severity":"High","expires":"2099-01-01","ticket":"ISSUE-1"}"#;
    let allow = allowlist(&directory, malformed);
    let output = scan(&directory, unfixed, EMPTY_CARGO, "0", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("malformed"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_fixed_cargo_audit_vulnerability_fails_the_gate() {
    let directory = temp_dir();
    let allow = allowlist(&directory, "");
    let cargo = r#"{"vulnerabilities":{"list":[{"advisory":{"id":"RUSTSEC-1","cvss":[{"score":5.0}]},"package":{"name":"demo","version":"1.0.0"},"versions":{"patched":["1.0.1"]}}]}}"#;
    let output = scan(&directory, EMPTY_GRYPE, cargo, "1", &allow);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixable High"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn sbom_prints_the_subject_digest_and_hash() {
    let directory = temp_dir();
    let bin = fake_tools(&directory);
    let output = directory.join("sbom.spdx.json");
    let mut environment = Vec::new();
    with_path(&bin, &mut environment);
    let refs: Vec<(&str, &str)> = environment
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let output = run(
        &[
            "sbom",
            &format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}"),
            output.to_str().expect("path"),
        ],
        &refs,
    );
    assert_success(&output);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains(&format!(
        "subject\tghcr.io/quant-rule/quantrule-platform@{DIGEST}"
    )));
    let hash = text
        .lines()
        .find_map(|line| line.strip_prefix("sha256\t"))
        .expect("hash");
    assert_eq!(hash.len(), 64);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn sign_does_not_echo_the_key_or_password_and_writes_a_bundle() {
    let directory = temp_dir();
    let bin = fake_tools(&directory);
    let public_key = directory.join("cosign.pub");
    let private_key = directory.join("cosign.key");
    let sbom = directory.join("sbom.spdx.json");
    std::fs::write(&public_key, "fixture public key\n").expect("public key");
    std::fs::write(&private_key, "fixture private key material\n").expect("private key");
    std::fs::write(&sbom, "{\"spdxVersion\":\"SPDX-2.3\"}\n").expect("sbom");
    let output_dir = directory.join("release");
    let cosign_log = directory.join("cosign.log");
    let mut environment = vec![
        (
            "QR_RELEASE_PUBLIC_KEY".to_owned(),
            public_key.display().to_string(),
        ),
        (
            "QR_RELEASE_PRIVATE_KEY".to_owned(),
            private_key.display().to_string(),
        ),
        ("COSIGN_PASSWORD".to_owned(), "fixture-password".to_owned()),
        (
            "FAKE_COSIGN_LOG".to_owned(),
            cosign_log.display().to_string(),
        ),
    ];
    with_path(&bin, &mut environment);
    let refs: Vec<(&str, &str)> = environment
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let output = run(
        &[
            "sign",
            &format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}"),
            output_dir.to_str().expect("path"),
            sbom.to_str().expect("sbom path"),
        ],
        &refs,
    );
    assert_success(&output);
    let printed = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!printed.contains("fixture-password"), "{printed}");
    assert!(
        !printed.contains("fixture private key material"),
        "{printed}"
    );
    assert!(printed.contains("signature bundle"), "{printed}");
    assert!(
        printed.contains(&format!(
            "signed image {}",
            output_dir.join("signed-image").display()
        )),
        "{printed}"
    );
    let signature_bundle = output_dir.join("cosign.bundle");
    let attestation_bundle = output_dir.join("attestation.bundle");
    assert!(signature_bundle.is_file(), "signature bundle missing");
    assert!(attestation_bundle.is_file(), "attestation bundle missing");
    assert!(
        output_dir.join("signed-image/index.json").is_file(),
        "signed-image layout missing"
    );
    let bundle = output_dir.join("signature.json");
    let value: Value =
        serde_json::from_str(&std::fs::read_to_string(&bundle).expect("bundle")).expect("json");
    assert_eq!(value["digest"], DIGEST);
    assert_eq!(value["keyless"], false);
    assert_eq!(value["signed_image"], "signed-image");
    for field in ["signature_bundle_sha256", "attestation_bundle_sha256"] {
        assert!(
            value[field]
                .as_str()
                .is_some_and(|hash| hash.starts_with("sha256:")),
            "{field}: {value}"
        );
    }
    assert!(
        value["public_key_fingerprint"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("sha256:")),
        "{value}"
    );
    let cosign = std::fs::read_to_string(&cosign_log).expect("cosign log");
    assert!(cosign.contains("sign --key"), "{cosign}");
    assert!(cosign.contains("--bundle"), "{cosign}");
    assert!(cosign.contains("attest --type spdxjson"), "{cosign}");
    assert!(cosign.contains("save --dir"), "{cosign}");
    let _ = std::fs::remove_dir_all(directory);
}

fn run_env(args: &[&str], environment: &[(String, String)]) -> Output {
    let refs: Vec<(&str, &str)> = environment
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    run(args, &refs)
}

fn sign_fixture(directory: &Path) -> (PathBuf, PathBuf, PathBuf, Vec<(String, String)>) {
    let bin = fake_tools(directory);
    let public_key = directory.join("cosign.pub");
    let private_key = directory.join("cosign.key");
    let sbom = directory.join("sbom.spdx.json");
    std::fs::write(&public_key, "fixture public key\n").expect("public key");
    std::fs::write(&private_key, "fixture private key material\n").expect("private key");
    std::fs::write(&sbom, "{\"spdxVersion\":\"SPDX-2.3\"}\n").expect("sbom");
    let output_dir = directory.join("release");
    let cosign_log = directory.join("cosign.log");
    let mut environment = vec![
        (
            "QR_RELEASE_PUBLIC_KEY".to_owned(),
            public_key.display().to_string(),
        ),
        (
            "QR_RELEASE_PRIVATE_KEY".to_owned(),
            private_key.display().to_string(),
        ),
        ("COSIGN_PASSWORD".to_owned(), "fixture-password".to_owned()),
        (
            "FAKE_COSIGN_LOG".to_owned(),
            cosign_log.display().to_string(),
        ),
    ];
    with_path(&bin, &mut environment);
    (output_dir, public_key, sbom, environment)
}

#[test]
fn bundle_copies_the_signed_image_and_prints_the_offline_verify() {
    let directory = temp_dir();
    let (release, public_key, sbom, environment) = sign_fixture(&directory);
    let target = format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}");
    let signed = run_env(
        &[
            "sign",
            &target,
            release.to_str().expect("release"),
            sbom.to_str().expect("sbom"),
        ],
        &environment,
    );
    assert_success(&signed);
    let bundle = directory.join("offline-bundle");
    let output = run_env(
        &[
            "bundle",
            &target,
            bundle.to_str().expect("bundle"),
            release.to_str().expect("release"),
        ],
        &environment,
    );
    assert_success(&output);
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("signed_image\tpresent"), "{printed}");
    assert!(printed.contains("cosign verify --key"), "{printed}");
    assert!(printed.contains("--local-image"), "{printed}");
    assert!(
        printed.contains(
            "registry:2@sha256:a3d8aaa63ed8681a604f1dea0aa03f100d5895b6a58ace528858a7b332415373"
        ),
        "{printed}"
    );
    for path in [
        "signature.json",
        "cosign.bundle",
        "attestation.bundle",
        "signed-image/index.json",
    ] {
        assert!(bundle.join(path).is_file(), "missing {path}");
    }
    let value: Value = serde_json::from_str(
        &std::fs::read_to_string(bundle.join("bundle.json")).expect("bundle.json"),
    )
    .expect("bundle JSON");
    assert_eq!(value["digest"], DIGEST);
    assert_eq!(value["signed_image"], "present");
    let files = value["files"].as_array().expect("files");
    for path in [
        "cosign.bundle",
        "attestation.bundle",
        "signature.json",
        "signed-image/index.json",
    ] {
        assert!(
            files.iter().any(|entry| entry["path"] == path),
            "bundle.json does not hash {path}: {value}"
        );
    }
    let verify = run_env(
        &["verify", &target, bundle.to_str().expect("bundle")],
        &environment,
    );
    assert_success(&verify);
    let printed = String::from_utf8_lossy(&verify.stdout);
    assert!(printed.contains("signed-image proof"), "{printed}");
    let cosign = std::fs::read_to_string(directory.join("cosign.log")).expect("cosign log");
    assert!(
        cosign.contains(&format!(
            "verify --key {} --local-image {}",
            public_key.display(),
            bundle.join("signed-image").display()
        )),
        "{cosign}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn a_bundle_without_evidence_labels_the_signed_image_absent() {
    let directory = temp_dir();
    let (release, _, _, environment) = sign_fixture(&directory);
    let target = format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}");
    assert_success(&run_env(
        &[
            "sign",
            &target,
            release.to_str().expect("release"),
            directory.join("sbom.spdx.json").to_str().expect("sbom"),
        ],
        &environment,
    ));
    let bundle = directory.join("offline-bundle");
    let output = run_env(
        &["bundle", &target, bundle.to_str().expect("bundle")],
        &environment,
    );
    assert_success(&output);
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(printed.contains("signed_image\tabsent"), "{printed}");
    assert!(printed.contains("signed-image proof absent"), "{printed}");
    assert!(printed.contains("registry:2@sha256:"), "{printed}");
    assert!(!printed.contains("registry:2 ("), "{printed}");
    let value: Value = serde_json::from_str(
        &std::fs::read_to_string(bundle.join("bundle.json")).expect("bundle.json"),
    )
    .expect("bundle JSON");
    assert_eq!(value["signed_image"], "absent");
    assert_eq!(value["offline_verify"], "");
    assert!(!bundle.join("signed-image").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn verify_rejects_a_tampered_signature_bundle() {
    let directory = temp_dir();
    let (release, _, sbom, environment) = sign_fixture(&directory);
    let target = format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}");
    assert_success(&run_env(
        &[
            "sign",
            &target,
            release.to_str().expect("release"),
            sbom.to_str().expect("sbom"),
        ],
        &environment,
    ));
    std::fs::write(release.join("cosign.bundle"), "tampered\n").expect("tamper");
    let output = run_env(
        &["verify", &target, release.to_str().expect("release")],
        &environment,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("signature bundle digest mismatch"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn verify_fails_closed_without_the_public_key() {
    let directory = temp_dir();
    let bin = fake_tools(&directory);
    let mut environment = vec![(
        "QR_RELEASE_PUBLIC_KEY".to_owned(),
        directory.join("missing.pub").display().to_string(),
    )];
    with_path(&bin, &mut environment);
    let refs: Vec<(&str, &str)> = environment
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let output = run(
        &[
            "verify",
            &format!("ghcr.io/quant-rule/quantrule-platform@{DIGEST}"),
        ],
        &refs,
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("fails closed"));
    let _ = std::fs::remove_dir_all(directory);
}
