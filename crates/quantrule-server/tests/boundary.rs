//! The adapter boundary.
//!
//! A service calls a trait, never a client library. This test reads the source
//! tree and fails when a driver is named outside its own adapter module, which
//! is what keeps the embedded desktop profile additive rather than a rewrite.

use std::path::{Path, PathBuf};

const CLIENT_TO_ADAPTER: [(&str, &str); 3] = [
    ("aws_sdk_s3", "runtime/adapters/s3.rs"),
    ("sqlx", "runtime/adapters/postgres.rs"),
    ("lapin", "runtime/adapters/rabbitmq.rs"),
];

fn sources(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn a_client_library_is_named_only_in_its_adapter() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&manifest.join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");

    let mut violations = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix(&manifest)
            .expect("relative")
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&file).expect("read source");
        for (client, allowed) in CLIENT_TO_ADAPTER {
            let base = format!("src/{allowed}");
            if relative == base
                || (allowed.ends_with("postgres.rs")
                    && relative.starts_with("src/runtime/adapters/postgres/"))
            {
                continue;
            }
            if text.contains(client) {
                violations.push(format!("{relative} names {client}"));
            }
        }
    }
    assert!(violations.is_empty(), "{violations:#?}");
}
