//! The Rust checker registry.
//!
//! Each checker is a pure read-only function of `--root`. The registry is the
//! one name-to-function map; `check --all` is the gate once every active checker
//! has a port.

pub mod audit_backlog;
pub mod board_permanence;
pub mod claims;
pub mod doc_length;
pub mod file_size;

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
        name: "claims",
        run: claims::check,
    },
    Checker {
        name: "doc-length",
        run: doc_length::check,
    },
    Checker {
        name: "file-size",
        run: file_size::check,
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
