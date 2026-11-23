//! Structural guards for the secret sources.
//!
//! The runtime has exactly one production secret reader: `Config::from_env`
//! through `quantrule_runtime::config`. A second reader would bypass the file
//! contract, the plain-variable refusal and the redacted error path. These
//! tests read the checked-in sources, so a future adapter that quietly reads
//! `QR_DATABASE_URL` or restores the AWS default chain fails the fast gate.

use std::path::{Path, PathBuf};

const PLAIN_SECRETS: [&str; 6] = [
    "QR_DATABASE_URL",
    "QR_DATABASE_OWNER_URL",
    "QR_QUEUE_URL",
    "QR_BEARER_TOKEN",
    "QR_OBJECT_STORE_ACCESS_KEY",
    "QR_OBJECT_STORE_SECRET_KEY",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the runtime manifest has two ancestors")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

fn production_part(source: &str) -> &str {
    source.split("#[cfg(test)]").next().unwrap_or(source)
}

#[test]
fn the_s3_adapter_has_no_second_credential_chain() {
    let source = read("crates/quantrule-runtime/src/adapters/s3.rs");
    let production = production_part(&source);
    assert!(
        !production.contains("aws_config::defaults("),
        "the S3 adapter restored the AWS default credential chain"
    );
    for name in ["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_PROFILE"] {
        assert!(
            !production.contains(name),
            "the S3 adapter reads {name} outside the secret files"
        );
    }
    let manifest = read("crates/quantrule-runtime/Cargo.toml");
    assert!(
        !manifest.contains("aws-config"),
        "aws-config would make the default chain available again"
    );
}

#[test]
fn no_secret_is_read_from_the_environment_outside_config_resolution() {
    let root = workspace_root();
    let mut files = Vec::new();
    rust_files(&root.join("crates"), &mut files);
    let config = root.join("crates/quantrule-runtime/src/config.rs");
    let mut violations = Vec::new();
    for file in files {
        if file == config {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&file) else {
            continue;
        };
        for (line_number, line) in source.lines().enumerate() {
            if !line.contains("env::var") {
                continue;
            }
            for name in PLAIN_SECRETS {
                if line.contains(name) {
                    violations.push(format!(
                        "{}:{} reads {name}",
                        file.strip_prefix(&root).unwrap_or(&file).display(),
                        line_number + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "a secret is read outside config resolution: {violations:?}"
    );
}

#[test]
fn the_image_stages_file_secrets_before_dropping_privileges() {
    let entrypoint = read("docker/entrypoint.sh");
    assert!(
        entrypoint.contains("/run/quantrule-secrets"),
        "the entrypoint does not stage the file secrets"
    );
    assert!(
        entrypoint.contains("setpriv --reuid=10001"),
        "the entrypoint does not drop privileges"
    );
    assert!(
        entrypoint.contains("chmod 400"),
        "the staged files are not owner-only"
    );
    let dockerfile = read("Dockerfile");
    assert!(
        dockerfile.contains("USER 0:0"),
        "the entrypoint cannot stage a root-owned mount"
    );
    assert!(
        dockerfile.contains("COPY --chmod=0755 docker/entrypoint.sh"),
        "the entrypoint is not executable in the image"
    );
}

#[test]
fn the_gateway_reads_the_token_only_from_its_file() {
    let source = read("crates/quantrule-gateway/src/token.rs");
    assert!(
        !source.contains("env::var"),
        "the gateway token source reads the environment"
    );
    assert!(
        source.contains("SecretFileReader"),
        "the gateway token source must use the runtime secret reader"
    );
}
