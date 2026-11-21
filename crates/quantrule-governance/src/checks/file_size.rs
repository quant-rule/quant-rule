//! Fail when a Rust source file exceeds the 1,000-line rule.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

/// The project limit from `AGENTS.md` section 3.
const LIMIT: usize = 1_000;

/// Scan every `*.rs` file under `crates/`.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut files = Vec::new();
    collect_rust_files(&root.join("crates"), &mut files);
    files.sort();
    files
        .into_iter()
        .filter_map(|path| {
            let text = fs::read_to_string(&path).ok()?;
            let count = text.lines().count();
            (count > LIMIT).then(|| {
                Diagnostic::path(
                    "file-size",
                    &path,
                    root,
                    format!("has {count} lines; the limit is {LIMIT}"),
                )
            })
        })
        .collect()
}

fn collect_rust_files(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_over_the_limit_is_reported() {
        let root =
            std::env::temp_dir().join(format!("qr-governance-file-size-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/demo/src")).expect("tree");
        let path = root.join("crates/demo/src/lib.rs");
        fs::write(&path, "line\n".repeat(LIMIT + 1)).expect("fixture");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("1001"));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_file_at_the_limit_passes() {
        let root =
            std::env::temp_dir().join(format!("qr-governance-file-size-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/demo/src")).expect("tree");
        fs::write(root.join("crates/demo/src/lib.rs"), "line\n".repeat(LIMIT)).expect("fixture");
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
