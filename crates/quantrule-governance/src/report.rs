//! Deterministic governance diagnostics.
//!
//! A checker returns diagnostics; the binary prints them in one stable order and
//! chooses the exit code. The report carries no timestamps or absolute paths.

use std::fmt::Write as _;
use std::path::Path;

/// One checker finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Checker name.
    pub checker: String,
    /// Repository-relative path.
    pub path: String,
    /// One-based line, when the finding names one.
    pub line: Option<usize>,
    /// Human-readable finding.
    pub message: String,
}

impl Diagnostic {
    /// Build a path-level diagnostic.
    #[must_use]
    pub fn path(checker: &str, path: &Path, root: &Path, message: impl Into<String>) -> Self {
        Self {
            checker: checker.to_owned(),
            path: relative(path, root),
            line: None,
            message: message.into(),
        }
    }

    /// Build a line-level diagnostic.
    #[must_use]
    pub fn line(
        checker: &str,
        path: &Path,
        root: &Path,
        line: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            checker: checker.to_owned(),
            path: relative(path, root),
            line: Some(line),
            message: message.into(),
        }
    }
}

/// A stable, sorted diagnostic set.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Report {
    /// Findings, sorted by checker, path, line, and message.
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    /// Whether the report is empty.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Sort the findings into the one deterministic order.
    pub fn sort(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            (
                &left.checker,
                &left.path,
                left.line.unwrap_or(0),
                &left.message,
            )
                .cmp(&(
                    &right.checker,
                    &right.path,
                    right.line.unwrap_or(0),
                    &right.message,
                ))
        });
    }

    /// Render one diagnostic per line.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = String::new();
        for diagnostic in &self.diagnostics {
            let location = match diagnostic.line {
                Some(line) => format!("{}:{}", diagnostic.path, line),
                None => diagnostic.path.clone(),
            };
            let _ = writeln!(
                text,
                "error: {}: {}: {}",
                diagnostic.checker, location, diagnostic.message
            );
        }
        text
    }
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
