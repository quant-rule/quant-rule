//! Check the workflow index, document status, and required sections.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

const ALLOWED: [&str; 6] = [
    "Current",
    "Target",
    "Historical",
    "Deprecated",
    "Proposed",
    "Superseded",
];
const HISTORY_WORDS: [&str; 7] = [
    "historical",
    "retired",
    "superseded",
    "no longer",
    "not current",
    "history",
    "context",
];

/// Check every workflow document and the workflow index.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let workflows = root.join("docs/workflows");
    let index = workflows.join("README.md");
    let mut diagnostics = Vec::new();
    if !workflows.is_dir() {
        return vec![Diagnostic::path(
            "workflows",
            &workflows,
            root,
            "docs/workflows is missing",
        )];
    }
    if !index.is_file() {
        diagnostics.push(Diagnostic::path(
            "workflows",
            &index,
            root,
            "docs/workflows/README.md is missing",
        ));
    }
    if !root.join("docs/README.md").is_file() {
        diagnostics.push(Diagnostic::path(
            "workflows",
            &root.join("docs/README.md"),
            root,
            "docs/README.md is missing",
        ));
    }
    let index_text = fs::read_to_string(&index).unwrap_or_default();
    let mut files: Vec<PathBuf> = fs::read_dir(&workflows)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "md")
                && path.file_name().is_some_and(|name| name != "README.md")
        })
        .collect();
    files.sort();
    if files.is_empty() {
        diagnostics.push(Diagnostic::path(
            "workflows",
            &workflows,
            root,
            "no workflow documents found",
        ));
    }
    let mut statuses = BTreeMap::new();
    for path in &files {
        let text = fs::read_to_string(path).unwrap_or_default();
        let status = status_line(&text);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        if status.is_none() {
            diagnostics.push(Diagnostic::path(
                "workflows",
                path,
                root,
                "missing '- Status:' line",
            ));
        } else if !ALLOWED
            .iter()
            .any(|allowed| status.as_deref().is_some_and(|s| s.starts_with(allowed)))
        {
            diagnostics.push(Diagnostic::path(
                "workflows",
                path,
                root,
                "status must start with one of the allowed values",
            ));
        }
        if !has_heading(&text, "Purpose") {
            diagnostics.push(Diagnostic::path(
                "workflows",
                path,
                root,
                "missing a Purpose heading",
            ));
        }
        if !has_heading(&text, "Related documents") {
            diagnostics.push(Diagnostic::path(
                "workflows",
                path,
                root,
                "missing a Related documents heading",
            ));
        }
        if !index_text.contains(&name) {
            diagnostics.push(Diagnostic::path(
                "workflows",
                path,
                root,
                "not listed in docs/workflows/README.md",
            ));
        }
        statuses.insert(name, status);
    }
    for path in &files {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_owned();
        let Some(Some(status)) = statuses.get(&name) else {
            continue;
        };
        if !status.starts_with("Current") {
            continue;
        }
        let text = fs::read_to_string(path).unwrap_or_default();
        for (index, line) in text.lines().enumerate() {
            if HISTORY_WORDS
                .iter()
                .any(|word| line.to_lowercase().contains(word))
            {
                continue;
            }
            for target in link_targets(line) {
                if !target.ends_with(".md") {
                    continue;
                }
                let target_name = Path::new(target)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                if let Some(Some(target_status)) = statuses.get(target_name)
                    && (target_status.starts_with("Historical")
                        || target_status.starts_with("Deprecated")
                        || target_status.starts_with("Superseded"))
                {
                    diagnostics.push(Diagnostic::line(
                        "workflows",
                        path,
                        root,
                        index + 1,
                        format!("current workflow links historical workflow {target_name}"),
                    ));
                }
            }
        }
    }
    let templates = workflows.join("templates");
    if let Ok(entries) = fs::read_dir(&templates) {
        let mut paths: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        paths.sort();
        for path in paths {
            let text = fs::read_to_string(&path).unwrap_or_default();
            if !text.trim_start().starts_with("# ") {
                diagnostics.push(Diagnostic::path(
                    "workflows",
                    &path,
                    root,
                    "must start with a level-1 heading",
                ));
            }
        }
    }
    diagnostics
}

fn status_line(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        line.strip_prefix("- Status:")
            .map(|value| value.trim().to_owned())
    })
}

fn has_heading(text: &str, needle: &str) -> bool {
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with('#')
            && trimmed
                .trim_start_matches('#')
                .trim_start()
                .to_lowercase()
                .contains(&needle.to_lowercase())
    })
}

fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].split_whitespace().next().unwrap_or("").trim();
        if !target.is_empty() {
            targets.push(target);
        }
        rest = &after[end + 1..];
    }
    targets
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_workflow_without_status_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-workflows-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/workflows/templates")).expect("tree");
        fs::write(root.join("docs/README.md"), "# Docs\n").expect("fixture");
        fs::write(root.join("docs/workflows/README.md"), "| a.md |\n").expect("fixture");
        fs::write(
            root.join("docs/workflows/a.md"),
            "# A\n\n## Purpose\n\n## Related documents\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("Status")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
