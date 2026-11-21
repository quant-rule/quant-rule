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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture(name: &str, rule: bool, script: &str, workflow: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-board-permanence-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/scripts")).expect("scripts");
        fs::create_dir_all(root.join(".github/workflows")).expect("workflows");
        let agents = if rule {
            format!("# Agents\n\n{RULE}\nNo deletion.\n")
        } else {
            "# Agents\n\nNo deletion.\n".to_owned()
        };
        fs::write(root.join("AGENTS.md"), agents).expect("agents");
        fs::write(root.join(".github/scripts/project-sync.sh"), script).expect("script");
        fs::write(root.join(".github/workflows/ci.yml"), workflow).expect("workflow");
        root
    }

    #[test]
    fn valid_board_automation_passes() {
        let root = fixture("ok", true, "set -euo pipefail\n", "name: CI\n");
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_rule_is_reported() {
        let root = fixture("rule", false, "set -euo pipefail\n", "name: CI\n");
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("board permanence rule"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_delete_mutation_in_the_script_is_reported() {
        let root = fixture(
            "delete",
            true,
            "deleteProjectV2Item(input: {})\n",
            "name: CI\n",
        );
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("deleteProjectV2Item"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_archive_command_in_a_workflow_is_reported() {
        let root = fixture(
            "archive",
            true,
            "set -euo pipefail\n",
            "run: gh project item-archive 1 --owner quant-rule\n",
        );
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("item-archive"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
