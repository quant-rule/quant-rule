//! Check the grouped release changelog.

use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

const TYPES: [&str; 6] = [
    "Added",
    "Changed",
    "Deprecated",
    "Removed",
    "Fixed",
    "Security",
];

struct Release {
    name: String,
    date: Option<String>,
    line: usize,
    types: Vec<(String, usize)>,
    entries: usize,
}

/// Check `CHANGELOG.md`.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join("CHANGELOG.md");
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Diagnostic::path(
            "changelog",
            &path,
            root,
            "CHANGELOG.md is missing",
        )];
    };
    parse(root, &path, &text)
}

fn parse(root: &Path, path: &Path, text: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    if !lines
        .first()
        .is_some_and(|line| line.starts_with("# Changelog"))
    {
        diagnostics.push(Diagnostic::path(
            "changelog",
            path,
            root,
            "must start with '# Changelog'",
        ));
    }
    let mut releases: Vec<Release> = Vec::new();
    let mut current: Option<Release> = None;
    let mut current_type: Option<String> = None;
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        if let Some(rest) = line.strip_prefix("## [") {
            let Some(close) = rest.find(']') else {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    "malformed release heading",
                ));
                continue;
            };
            let name = &rest[..close];
            let after = &rest[close + 1..];
            let date = after
                .strip_prefix(" - ")
                .map(str::trim)
                .filter(|date| !date.is_empty())
                .map(str::to_owned);
            if name == "Unreleased" && date.is_some() {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    "[Unreleased] must not carry a date",
                ));
            }
            if name != "Unreleased" && date.is_none() {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    format!("release [{name}] must be '## [{name}] - YYYY-MM-DD'"),
                ));
            }
            if let Some(date) = &date
                && !is_date(date)
            {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    format!("release date {date:?} must be YYYY-MM-DD"),
                ));
            }
            if let Some(release) = current.take() {
                releases.push(release);
            }
            current = Some(Release {
                name: name.to_owned(),
                date,
                line: number,
                types: Vec::new(),
                entries: 0,
            });
            current_type = None;
            continue;
        }
        if line.starts_with("## ") {
            if let Some(release) = current.take() {
                releases.push(release);
            }
            current_type = None;
            continue;
        }
        if let Some(kind) = line.strip_prefix("### ") {
            if let Some(release) = current.as_mut() {
                if TYPES.contains(&kind) {
                    release.types.push((kind.to_owned(), number));
                    current_type = Some(kind.to_owned());
                } else {
                    diagnostics.push(Diagnostic::line(
                        "changelog",
                        path,
                        root,
                        number,
                        format!("unknown changelog section {line:?}"),
                    ));
                }
            }
            continue;
        }
        if let Some(entry) = line.strip_prefix("- ") {
            let Some(release) = current.as_mut() else {
                continue;
            };
            let Some(kind) = current_type.as_ref() else {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    "entry is outside a release type section",
                ));
                continue;
            };
            if entry.trim().is_empty() {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    number,
                    "entry is empty",
                ));
            } else {
                release.entries += 1;
                let _ = kind;
            }
        }
    }
    if let Some(release) = current.take() {
        releases.push(release);
    }
    if releases.is_empty() {
        diagnostics.push(Diagnostic::path(
            "changelog",
            path,
            root,
            "has no release section",
        ));
    }
    if !releases.iter().any(|release| release.name == "Unreleased") {
        diagnostics.push(Diagnostic::path(
            "changelog",
            path,
            root,
            "has no [Unreleased] section",
        ));
    } else if releases
        .first()
        .is_some_and(|release| release.name != "Unreleased")
    {
        diagnostics.push(Diagnostic::path(
            "changelog",
            path,
            root,
            "[Unreleased] must be the first release section",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut previous_date: Option<&str> = None;
    for release in &releases {
        if !seen.insert(release.name.clone()) {
            diagnostics.push(Diagnostic::line(
                "changelog",
                path,
                root,
                release.line,
                format!("release [{}] appears twice", release.name),
            ));
        }
        if release.types.is_empty() {
            if release.name != "Unreleased" {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    release.line,
                    format!("release [{}] has no type section", release.name),
                ));
            }
            continue;
        }
        if release.entries == 0 && release.name != "Unreleased" {
            diagnostics.push(Diagnostic::line(
                "changelog",
                path,
                root,
                release.line,
                format!("release [{}] has no entries", release.name),
            ));
        }
        if release.name != "Unreleased"
            && let Some(date) = release.date.as_deref()
        {
            if let Some(previous) = previous_date
                && date > previous
            {
                diagnostics.push(Diagnostic::line(
                    "changelog",
                    path,
                    root,
                    release.line,
                    format!(
                        "release [{}] date {date} is newer than the previous release {previous}",
                        release.name
                    ),
                ));
            }
            previous_date = Some(date);
        }
    }
    diagnostics
}

fn is_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value
            .bytes()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_unreleased_section_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-changelog-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0] - 2026-01-01\n\n### Added\n\n- Entry.\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("Unreleased")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
