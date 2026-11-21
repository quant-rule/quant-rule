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
            if matches!(cells[1], "Critical" | "Major") && cells.last() == Some(&"Open") {
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
