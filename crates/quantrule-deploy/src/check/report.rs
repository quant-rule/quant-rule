//! Check rows and the deterministic v3 report.
//!
//! A row never carries a secret value. It names the group, the subject (a
//! service, a secret name, or a URL), the observed state, and either a detail
//! or the stable failure.

use std::fmt::Write as _;

/// One v3 check row.
#[derive(Debug, Clone)]
pub struct CheckRow {
    /// The check family: descriptor, placement, image, secret, stateful, ingress, remote.
    pub group: &'static str,
    /// The service, file field, or URL that was checked.
    pub subject: String,
    /// The observed state.
    pub state: String,
    /// A positive detail, never a secret.
    pub detail: Option<String>,
    /// The reported image digest, when the row is a version probe.
    pub digest: Option<String>,
    /// A failure detail; `None` on success.
    pub error: Option<String>,
}

impl CheckRow {
    /// A passing row.
    #[must_use]
    pub fn ok(
        group: &'static str,
        subject: impl Into<String>,
        state: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            group,
            subject: subject.into(),
            state: state.into(),
            detail: Some(detail.into()),
            digest: None,
            error: None,
        }
    }

    /// A failing row.
    #[must_use]
    pub fn failed(
        group: &'static str,
        subject: impl Into<String>,
        state: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            group,
            subject: subject.into(),
            state: state.into(),
            detail: None,
            digest: None,
            error: Some(error.into()),
        }
    }

    /// Whether this row is a failure.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        self.error.is_some()
    }

    /// The printed status.
    #[must_use]
    pub fn status(&self) -> &'static str {
        if self.is_failure() { "FAIL" } else { "ok" }
    }
}

/// The complete v3 check result.
#[derive(Debug, Clone)]
pub struct CheckReport {
    /// The descriptor profile.
    pub profile: String,
    /// The evidence label; a lab result is `Simulated HA`.
    pub evidence: String,
    /// The pinned application image.
    pub image: String,
    rows: Vec<CheckRow>,
}

impl CheckReport {
    /// Create a report with the three identity lines.
    #[must_use]
    pub fn new(
        profile: impl Into<String>,
        evidence: impl Into<String>,
        image: impl Into<String>,
    ) -> Self {
        Self {
            profile: profile.into(),
            evidence: evidence.into(),
            image: image.into(),
            rows: Vec::new(),
        }
    }

    /// Append one row.
    pub fn push(&mut self, row: CheckRow) {
        self.rows.push(row);
    }

    /// The rows in check order, for tests.
    #[cfg(test)]
    #[must_use]
    pub fn rows(&self) -> &[CheckRow] {
        &self.rows
    }

    /// Whether any row failed.
    #[must_use]
    pub fn failed(&self) -> bool {
        self.rows.iter().any(CheckRow::is_failure)
    }

    /// Render the deterministic report text.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "profile\t{}", self.profile);
        let _ = writeln!(out, "evidence\t{}", self.evidence);
        let _ = writeln!(out, "image\t{}", self.image);
        for row in &self.rows {
            let tail = if let Some(error) = &row.error {
                error.clone()
            } else {
                match (&row.digest, &row.detail) {
                    (Some(digest), Some(detail)) => format!("{digest}\t{detail}"),
                    (Some(digest), None) => digest.clone(),
                    (None, Some(detail)) => detail.clone(),
                    (None, None) => String::new(),
                }
            };
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}\t{}",
                row.status(),
                row.group,
                row.subject,
                row.state,
                tail
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_row_is_not_ok() {
        let mut report = CheckReport::new("swarm-ha", "HA profile (unverified)", "img");
        report.push(CheckRow::ok(
            "secret",
            "token",
            "path",
            "/etc/quantrule/token",
        ));
        assert!(!report.failed());
        report.push(CheckRow::failed("secret", "token", "missing", "absent"));
        assert!(report.failed());
        assert_eq!(report.rows()[1].status(), "FAIL");
    }
}
