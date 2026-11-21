//! Check the length caps for current living documents.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

const INDEX_CAP: usize = 330;
const DEFAULT_ARCHITECTURE: usize = 170;
const ADR_CAP: usize = 110;
const TUTORIAL_CAP: usize = 200;
const CURRENT_FROM: &str = "2026-11-12";

/// Check the architecture set, the docs index, and current ADRs/tutorials.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut targets = Vec::new();
    let index = root.join("docs/README.md");
    if index.is_file() {
        targets.push((index, INDEX_CAP));
    }
    let architecture = root.join("docs/architecture");
    if let Ok(entries) = fs::read_dir(&architecture) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        paths.sort();
        for path in paths {
            let cap = match path.file_name().and_then(|name| name.to_str()) {
                Some("command-surface.md") => 330,
                Some("http-api.md") => 230,
                _ => DEFAULT_ARCHITECTURE,
            };
            targets.push((path, cap));
        }
    }
    for (kind, cap) in [("adr", ADR_CAP), ("tutorials", TUTORIAL_CAP)] {
        let base = root.join("docs").join(kind);
        let mut paths = Vec::new();
        collect_markdown(&base, &mut paths);
        paths.sort();
        for path in paths {
            if current_since(&path, root) {
                targets.push((path, cap));
            }
        }
    }
    let mut diagnostics = Vec::new();
    for (path, cap) in targets {
        let Ok(text) = fs::read_to_string(&path) else {
            diagnostics.push(Diagnostic::path("doc-length", &path, root, "cannot read"));
            continue;
        };
        let lines = text.lines().count();
        if lines > cap {
            diagnostics.push(Diagnostic::path(
                "doc-length",
                &path,
                root,
                format!("{lines} lines, cap {cap}"),
            ));
        }
    }
    diagnostics
}

fn collect_markdown(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
}

fn current_since(path: &Path, root: &Path) -> bool {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|part| {
            part.len() == 10
                && part.as_bytes().get(4) == Some(&b'-')
                && part.as_bytes().get(7) == Some(&b'-')
                && part >= CURRENT_FROM
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_architecture_file_over_the_cap_is_reported() {
        let root =
            std::env::temp_dir().join(format!("qr-governance-doc-length-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/architecture")).expect("tree");
        fs::write(
            root.join("docs/architecture/long.md"),
            "line\n".repeat(DEFAULT_ARCHITECTURE + 1),
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("cap 170"));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_historical_adr_is_exempt() {
        let root = std::env::temp_dir().join(format!(
            "qr-governance-doc-length-old-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/adr/2026-01-01")).expect("tree");
        fs::write(
            root.join("docs/adr/2026-01-01/old.md"),
            "line\n".repeat(ADR_CAP + 1),
        )
        .expect("fixture");
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
