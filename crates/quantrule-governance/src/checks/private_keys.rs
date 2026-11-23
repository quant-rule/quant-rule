//! Fail when a private key is committed.
//!
//! The internal PKI lives under the ignored `pki/` directory; the only key that
//! may be in a repository is a fixture generated in memory. A tracked `.key`,
//! PKCS#12/PFX/JKS bundle or PEM private-key block is therefore a defect, not a
//! test fixture.

use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

const SKIP: [&str; 8] = [
    ".git",
    "target",
    "node_modules",
    "dist",
    "test-results",
    "playwright-report",
    "blob-report",
    "__pycache__",
];

/// Scan the repository for private-key file types and PEM blocks.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut files = Vec::new();
    collect(root, &mut files);
    files.sort();
    let mut diagnostics = Vec::new();
    for path in files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if [".key", ".p12", ".pfx", ".jks"]
            .iter()
            .any(|extension| name.ends_with(extension))
        {
            diagnostics.push(Diagnostic::path(
                "private-keys",
                &path,
                root,
                "committed private-key file",
            ));
            continue;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > 2 * 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        if ["", "RSA ", "EC "]
            .iter()
            .any(|kind| text.contains(&format!("BEGIN {kind}PRIVATE KEY")))
        {
            diagnostics.push(Diagnostic::path(
                "private-keys",
                &path,
                root,
                "PEM private-key block",
            ));
        }
    }
    diagnostics
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            if !SKIP.contains(&name) {
                collect(&path, found);
            }
        } else {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_committed_key_or_pem_block_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-private-keys-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("pki")).expect("tree");
        fs::write(root.join("pki/server.key"), "not a real key").expect("key");
        fs::write(
            root.join("notes.txt"),
            format!("-----BEGIN {}PRIVATE KEY-----\nfixture\n", ""),
        )
        .expect("pem");
        let messages: Vec<_> = check(&root)
            .into_iter()
            .map(|diagnostic| diagnostic.message)
            .collect();
        assert!(
            messages
                .iter()
                .any(|message| message.contains("private-key file"))
        );
        assert!(messages.iter().any(|message| message.contains("PEM")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
