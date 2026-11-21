//! Check the append-only board rule and the board automation.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

const RULE: &str = "The board is append-only history.";
const FORBIDDEN: [&str; 4] = [
    "deleteProjectV2Item",
    "archiveProjectV2Item",
    "gh project item-delete",
    "gh project item-archive",
];

/// Check `AGENTS.md`, the project-sync script, and every workflow.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let agents = root.join("AGENTS.md");
    match fs::read_to_string(&agents) {
        Ok(text) if !text.contains(RULE) => diagnostics.push(Diagnostic::path(
            "board-permanence",
            &agents,
            root,
            format!("missing the board permanence rule: {RULE:?}"),
        )),
        Ok(_) => {}
        Err(_) => diagnostics.push(Diagnostic::path(
            "board-permanence",
            &agents,
            root,
            "AGENTS.md is missing",
        )),
    }
    let mut paths = vec![root.join(".github/scripts/project-sync.sh")];
    let workflows = root.join(".github/workflows");
    if let Ok(entries) = fs::read_dir(&workflows) {
        let mut found: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "yml"))
            .collect();
        found.sort();
        paths.extend(found);
    }
    for path in paths {
        match fs::read_to_string(&path) {
            Ok(text) => {
                for forbidden in FORBIDDEN {
                    if text.contains(forbidden) {
                        diagnostics.push(Diagnostic::path(
                            "board-permanence",
                            &path,
                            root,
                            format!("contains forbidden board mutation {forbidden:?}"),
                        ));
                    }
                }
            }
            Err(_) => diagnostics.push(Diagnostic::path(
                "board-permanence",
                &path,
                root,
                "file is missing",
            )),
        }
    }
    diagnostics
}
