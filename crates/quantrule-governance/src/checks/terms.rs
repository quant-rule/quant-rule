//! Report retired workflow terms in the active documentation.

use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

const TERMS: [&str; 3] = [
    "Continuous Improvement",
    "continuous improvement",
    "Governing / Experiment Control Swarm",
];
const ROOTS: [&str; 3] = ["docs", "README.md", "AGENTS.md"];
const EXCLUDES: [&str; 2] = [
    "docs/ai-transparency-log",
    "docs/improvement/INVALIDATIONS.md",
];
const SKIP: [&str; 8] = [
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "test-results",
];
const MAX_BYTES: u64 = 5 * 1024 * 1024;

/// Scan `docs/`, `README.md`, and `AGENTS.md` for banned workflow terms.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let excludes: Vec<PathBuf> = EXCLUDES.iter().map(|path| root.join(path)).collect();
    let mut files = Vec::new();
    for entry in ROOTS {
        let path = root.join(entry);
        if path.is_file() {
            if !inside(&path, &excludes) {
                files.push(path);
            }
        } else if path.is_dir() {
            collect(&path, &excludes, &mut files);
        }
    }
    files.sort();
    let mut diagnostics = Vec::new();
    for path in files {
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > MAX_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if bytes[..bytes.len().min(4096)].contains(&0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (index, line) in text.lines().enumerate() {
            for term in TERMS {
                if line.contains(term) {
                    diagnostics.push(Diagnostic::line(
                        "terms",
                        &path,
                        root,
                        index + 1,
                        format!("retired workflow term {term:?}"),
                    ));
                    break;
                }
            }
        }
    }
    diagnostics
}

fn collect(directory: &Path, excludes: &[PathBuf], found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if inside(&path, excludes) {
            continue;
        }
        if path.is_dir() {
            if !SKIP.contains(&name) {
                collect(&path, excludes, found);
            }
        } else {
            found.push(path);
        }
    }
}

fn inside(path: &Path, excludes: &[PathBuf]) -> bool {
    excludes
        .iter()
        .any(|exclude| path == exclude || path.starts_with(exclude))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_retired_term_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-terms-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs")).expect("tree");
        fs::write(root.join("docs/x.md"), "Continuous Improvement\n").expect("fixture");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
