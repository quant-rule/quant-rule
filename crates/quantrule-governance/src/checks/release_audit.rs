//! Require the latest release note to name a verified audit status.

use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

/// Check the latest `docs/releases/*/v*.md` note.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let Some(note) = latest_note(root) else {
        return vec![Diagnostic::path(
            "release-audit",
            &root.join("docs/releases"),
            root,
            "no release note is present",
        )];
    };
    let Ok(text) = fs::read_to_string(&note) else {
        return vec![Diagnostic::path(
            "release-audit",
            &note,
            root,
            "cannot read the latest release note",
        )];
    };
    let statuses: Vec<&str> = text
        .lines()
        .filter_map(|line| line.strip_prefix("- Audit status:").map(str::trim))
        .collect();
    if statuses.is_empty() {
        return vec![Diagnostic::path(
            "release-audit",
            &note,
            root,
            "missing '- Audit status:'",
        )];
    }
    let mut diagnostics = Vec::new();
    let residual = text.to_ascii_lowercase().contains("residual risk accepted");
    for status in statuses {
        if audit_id(status).is_none() {
            diagnostics.push(Diagnostic::path(
                "release-audit",
                &note,
                root,
                "the audit status names no audit ID",
            ));
        }
        let lower = status.to_ascii_lowercase();
        let closed = lower.contains("verified")
            || lower.contains("won't fix")
            || lower.contains("residual risk accepted");
        if !closed {
            diagnostics.push(Diagnostic::path(
                "release-audit",
                &note,
                root,
                "the audit status is not Verified, Won't fix, or an accepted risk",
            ));
        }
        let severe = lower.contains("critical") || lower.contains("major");
        let open = lower.contains("fixed") || lower.contains("open");
        if severe && open && !residual {
            diagnostics.push(Diagnostic::path(
                "release-audit",
                &note,
                root,
                "a Critical/Major finding is Fixed or Open without a residual-risk acceptance",
            ));
        }
    }
    diagnostics
}

fn latest_note(root: &Path) -> Option<PathBuf> {
    let releases = root.join("docs/releases");
    let mut notes = Vec::new();
    for entry in fs::read_dir(&releases).ok()?.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        for file in fs::read_dir(directory).ok()?.flatten() {
            let path = file.path();
            let name = path.file_name()?.to_string_lossy();
            if name.starts_with('v') && name.ends_with(".md") {
                notes.push(path);
            }
        }
    }
    notes.sort();
    notes.pop()
}

fn audit_id(text: &str) -> Option<&str> {
    let start = text.find("AUDIT-")?;
    let rest = &text[start..];
    let bytes = rest.as_bytes();
    if bytes.get(5) != Some(&b'-')
        || bytes.get(10) != Some(&b'-')
        || bytes.get(13) != Some(&b'-')
        || bytes.get(16) != Some(&b'-')
    {
        return None;
    }
    if !(6..10)
        .chain(11..13)
        .chain(14..16)
        .all(|index| bytes.get(index).is_some_and(u8::is_ascii_digit))
    {
        return None;
    }
    let slug = rest.get(17..)?;
    let end = slug
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '-'))
        .map_or(slug.len(), |(index, _)| index);
    if end == 0 {
        return None;
    }
    Some(&rest[..17 + end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_status_without_an_audit_id_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-audit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/releases/2026-01-01")).expect("tree");
        fs::write(
            root.join("docs/releases/2026-01-01/v1.0.0.md"),
            "- Audit status: Verified\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("audit ID")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
