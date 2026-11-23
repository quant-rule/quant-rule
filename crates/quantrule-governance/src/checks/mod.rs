//! The Rust checker registry.
//!
//! Each checker is a pure read-only function of `--root`. The registry is the
//! one name-to-function map; `check --all` is the gate once every active checker
//! has a port.

pub mod audit_backlog;
pub mod board_permanence;
pub mod changelog;
pub mod claims;
pub mod client_docs;
pub mod command_surface;
pub mod doc_length;
pub mod event_emitters;
pub mod execution_plans;
pub mod file_size;
pub mod labels;
pub mod legacy;
pub mod links;
pub mod manual;
pub mod markdown;
pub mod partitions;
pub mod port_parity;
pub mod private_keys;
pub mod py_guard;
pub mod registry;
pub mod release_audit;
pub mod releases;
pub mod rust_tests;
pub mod structure;
pub mod terms;
pub mod validation;
pub mod verification;
pub mod verification_matrix;
pub mod workflows;

use std::path::Path;

use crate::report::Report;

/// One registered checker.
pub struct Checker {
    /// Stable CLI name.
    pub name: &'static str,
    /// Checker function.
    pub run: fn(&Path) -> Vec<crate::report::Diagnostic>,
}

/// Every checker implemented in Rust so far.
pub const CHECKERS: &[Checker] = &[
    Checker {
        name: "audit-backlog",
        run: audit_backlog::check,
    },
    Checker {
        name: "board-permanence",
        run: board_permanence::check,
    },
    Checker {
        name: "changelog",
        run: changelog::check,
    },
    Checker {
        name: "claims",
        run: claims::check,
    },
    Checker {
        name: "client-docs",
        run: client_docs::check,
    },
    Checker {
        name: "command-surface",
        run: command_surface::check,
    },
    Checker {
        name: "doc-length",
        run: doc_length::check,
    },
    Checker {
        name: "file-size",
        run: file_size::check,
    },
    Checker {
        name: "event-emitters",
        run: event_emitters::check,
    },
    Checker {
        name: "execution-plans",
        run: execution_plans::check,
    },
    Checker {
        name: "labels",
        run: labels::check,
    },
    Checker {
        name: "legacy",
        run: legacy::check,
    },
    Checker {
        name: "links",
        run: links::check,
    },
    Checker {
        name: "manual",
        run: manual::check,
    },
    Checker {
        name: "partitions",
        run: partitions::check,
    },
    Checker {
        name: "port-parity",
        run: port_parity::check,
    },
    Checker {
        name: "private-keys",
        run: private_keys::check,
    },
    Checker {
        name: "py-guard",
        run: py_guard::check,
    },
    Checker {
        name: "structure",
        run: structure::check,
    },
    Checker {
        name: "terms",
        run: terms::check,
    },
    Checker {
        name: "registry",
        run: registry::check,
    },
    Checker {
        name: "release-audit",
        run: release_audit::check,
    },
    Checker {
        name: "releases",
        run: releases::check,
    },
    Checker {
        name: "validation",
        run: validation::check,
    },
    Checker {
        name: "verification",
        run: verification::check,
    },
    Checker {
        name: "verification-matrix",
        run: verification_matrix::check,
    },
    Checker {
        name: "workflows",
        run: workflows::check,
    },
];

/// Run one checker, or every checker.
///
/// # Errors
///
/// Returns the unknown checker name.
pub fn run(root: &Path, only: Option<&str>) -> Result<Report, String> {
    let mut report = Report::default();
    match only {
        Some(name) => {
            let checker = CHECKERS
                .iter()
                .find(|checker| checker.name == name)
                .ok_or_else(|| format!("unknown checker {name}"))?;
            report.diagnostics.extend((checker.run)(root));
        }
        None => {
            for checker in CHECKERS {
                report.diagnostics.extend((checker.run)(root));
            }
        }
    }
    report.sort();
    Ok(report)
}

/// The registered names, one per line.
#[must_use]
pub fn names() -> String {
    let mut text = String::new();
    for checker in CHECKERS {
        text.push_str(checker.name);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn the_registry_has_one_unique_name_per_checker() {
        assert_eq!(CHECKERS.len(), 27);
        let names: BTreeSet<_> = CHECKERS.iter().map(|checker| checker.name).collect();
        assert_eq!(names.len(), CHECKERS.len());
    }

    #[test]
    fn every_checker_runs_on_an_empty_root() {
        let root = std::env::temp_dir().join(format!("qr-checks-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("tree");
        for checker in CHECKERS {
            let _ = (checker.run)(&root);
        }
        std::fs::remove_dir_all(&root).expect("cleanup");
    }
}
