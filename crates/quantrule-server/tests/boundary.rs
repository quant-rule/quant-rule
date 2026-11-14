//! The adapter boundary.
//!
//! A service calls a trait, never a client library. This test reads the server
//! source and fails when a driver is named outside its own adapter module. The
//! runtime's own boundary test covers S3 and RabbitMQ; the backend clients moved
//! there in P1.

use std::path::{Path, PathBuf};

const CLIENT_TO_ADAPTER: [(&str, &str); 1] = [("sqlx", "adapters/postgres")];

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
            if relative.starts_with(&base) {
                continue;
            }
            if text.contains(client) {
                violations.push(format!("{relative} names {client}"));
            }
        }
    }
    assert!(violations.is_empty(), "{violations:#?}");
}
