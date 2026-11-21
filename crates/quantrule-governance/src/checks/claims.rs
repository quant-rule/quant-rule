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
