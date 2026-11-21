//! Fail when an audit document leaves a Critical or Major finding Open.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

/// Check every audit document's findings table.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let directory = root.join("docs/audits");
    let mut paths = Vec::new();
    collect_markdown(&directory, &mut paths);
    paths.sort();
    let mut diagnostics = Vec::new();
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let cells: Vec<_> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            if cells.len() < 3 || !cells[0].starts_with("AUD-") {
                continue;
            }
            if matches!(cells[1], "Critical" | "Major")
                && cells.iter().skip(2).any(|cell| *cell == "Open")
            {
                diagnostics.push(Diagnostic::line(
                    "audit-backlog",
                    &path,
                    root,
                    index + 1,
                    format!("{} ({}) is Open", cells[0], cells[1]),
                ));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture(name: &str, status: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-audit-backlog-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let directory = root.join("docs/audits/2026-01-01");
        fs::create_dir_all(&directory).expect("tree");
        fs::write(
            directory.join("sample-audit.md"),
            format!(
                "# Sample\n\n| ID | Severity | Area | Finding | Status | Links |\n|---|---|---|---|---|---|\n| AUD-2026-01-01-01 | Major | test | finding | {status} | - |\n"
            ),
        )
        .expect("audit");
        root
    }

    #[test]
    fn an_open_major_is_reported() {
        let root = fixture("open", "Open");
        assert!(check(&root).iter().any(|d| d.message.contains("is Open")));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_fixed_major_passes() {
        let root = fixture("fixed", "Fixed");
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
