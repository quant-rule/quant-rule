//! Check that the checked-in issue and pull-request templates carry the four
//! approved claim types, and that `AGENTS.md` still defines them.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

const CLAIM_TYPES: [&str; 4] = [
    "Problem Claim",
    "Behaviour Claim",
    "Outcome Claim",
    "ADR (Architecture Decision)",
];

/// Check the templates and the normative claim-type section.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let templates = root.join(".github/ISSUE_TEMPLATE");
    if templates.is_dir() {
        let mut paths: Vec<_> = fs::read_dir(&templates)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
            .collect();
        paths.sort();
        for path in paths {
            check_template(root, &path, &mut diagnostics);
        }
    } else {
        diagnostics.push(Diagnostic::path(
            "claims",
            &templates,
            root,
            ".github/ISSUE_TEMPLATE is missing",
        ));
    }
    check_template(
        root,
        &root.join(".github/pull_request_template.md"),
        &mut diagnostics,
    );
    let agents = root.join("AGENTS.md");
    match fs::read_to_string(&agents) {
        Ok(text) => {
            if !text.contains("### Claim type") {
                diagnostics.push(Diagnostic::path(
                    "claims",
                    &agents,
                    root,
                    "missing '### Claim type' section",
                ));
            }
            for claim in CLAIM_TYPES {
                if !text.contains(claim) {
                    diagnostics.push(Diagnostic::path(
                        "claims",
                        &agents,
                        root,
                        format!("missing claim type {claim:?}"),
                    ));
                }
            }
        }
        Err(_) => diagnostics.push(Diagnostic::path(
            "claims",
            &agents,
            root,
            "AGENTS.md is missing",
        )),
    }
    diagnostics
}

fn check_template(root: &Path, path: &Path, diagnostics: &mut Vec<Diagnostic>) {
    match fs::read_to_string(path) {
        Ok(text) => {
            if !text.contains("Claim type:") {
                diagnostics.push(Diagnostic::path(
                    "claims",
                    path,
                    root,
                    "missing 'Claim type:' line",
                ));
            } else if !CLAIM_TYPES.iter().all(|claim| text.contains(claim)) {
                diagnostics.push(Diagnostic::path(
                    "claims",
                    path,
                    root,
                    "claim-type line does not name all four types",
                ));
            }
        }
        Err(_) => diagnostics.push(Diagnostic::path(
            "claims",
            path,
            root,
            "template is missing",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const CLAIM_LINE: &str = "Claim type: one of Problem Claim, Behaviour Claim, Outcome Claim, or ADR (Architecture Decision)\n";

    fn fixture(name: &str, template: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-claims-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".github/ISSUE_TEMPLATE")).expect("templates");
        fs::write(root.join(".github/ISSUE_TEMPLATE/task.md"), template).expect("issue template");
        fs::write(root.join(".github/pull_request_template.md"), CLAIM_LINE)
            .expect("pull request template");
        fs::write(
            root.join("AGENTS.md"),
            "### Claim type\n\nProblem Claim\nBehaviour Claim\nOutcome Claim\nADR (Architecture Decision)\n",
        )
        .expect("agents");
        root
    }

    #[test]
    fn a_valid_tree_passes() {
        let root = fixture("ok", CLAIM_LINE);
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_claim_line_is_reported() {
        let root = fixture("missing", "# Task\n");
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("missing 'Claim type:'"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
