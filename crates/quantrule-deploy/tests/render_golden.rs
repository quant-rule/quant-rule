//! Golden output for the v3 renderer.
//!
//! The fixture descriptor is the checked-in example with secret and CA paths
//! rewritten to a temporary directory. The rendered YAML, scripts, manifest,
//! and public CA copies are compared byte for byte. The secret values are fake
//! fixtures and are compared with their source copies, not stored twice.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "qr-render-golden-{}-{}",
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
        "-----BEGIN CERTIFICATE-----\ngolden fixture CA\n-----END CERTIFICATE-----\n",
        0o644,
    );
    for endpoint in ["database", "queue", "object_store"] {
        value["endpoints"][endpoint]["tls_ca"] = serde_json::json!(ca.display().to_string());
    }
    let path = directory.join("deploy.json");
    std::fs::write(&path, value.to_string()).expect("descriptor");
    path
}

#[test]
fn the_render_matches_its_checked_in_golden_files() {
    let directory = temp_dir();
    let descriptor = local_descriptor(&directory);
    let out = directory.join("out");
    let output = Command::new(env!("CARGO_BIN_EXE_qr-deploy"))
        .args([
            "render",
            descriptor.to_str().expect("path"),
            "--out",
            out.to_str().expect("path"),
            "--target",
            "all",
        ])
        .output()
        .expect("render");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let expected = repo("crates/quantrule-deploy/tests/fixtures/render-v3/expected");
    let mut files = Vec::new();
    walk(&expected, &mut files);
    assert!(!files.is_empty());
    for path in files {
        let relative = path.strip_prefix(&expected).expect("relative");
        let actual = out.join(relative);
        assert!(actual.is_file(), "missing {}", relative.display());
        assert_eq!(
            std::fs::read(&actual).expect("actual"),
            std::fs::read(&path).expect("expected"),
            "{}",
            relative.display()
        );
    }

    for name in [
        "gateway_token",
        "gateway_database_url",
        "gateway_owner_database_url",
        "data_database_url",
        "data_owner_database_url",
        "evidence_database_url",
        "evidence_owner_database_url",
        "simulation_database_url",
        "simulation_owner_database_url",
        "queue_url",
        "object_store_access_key",
        "object_store_secret_key",
    ] {
        let source = std::fs::read_to_string(repo(&format!("deploy/v3/secrets/{name}")))
            .expect("source secret");
        let rendered =
            std::fs::read_to_string(out.join("secrets").join(name)).expect("rendered secret");
        assert_eq!(rendered, format!("{}\n", source.trim_end()), "{name}");
    }

    let manifest = std::fs::read_to_string(out.join("render.json")).expect("render manifest");
    assert!(
        manifest.contains("\"descriptor_hash\": \"sha256:"),
        "{manifest}"
    );
    assert!(
        !manifest.contains(directory.to_str().expect("temp")),
        "{manifest}"
    );
    let _ = std::fs::remove_dir_all(directory);
}

fn walk(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            walk(&path, files);
        } else {
            files.push(path);
        }
    }
}
