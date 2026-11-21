//! Check the structure of current execution plans.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::report::Diagnostic;

const BASELINE: &str = "2026-11-11";
const EXEMPT: [&str; 1] = ["docs/execution-plans/2026-11-11/simulation-data-recovery.md"];
const PLAN_STATUSES: [&str; 6] = [
    "Proposed",
    "In Progress",
    "In Review",
    "Verified",
    "Superseded",
    "Approved",
];
const CLAIM_TYPES: [&str; 4] = [
    "Problem Claim",
    "Behaviour Claim",
    "Outcome Claim",
    "ADR (Architecture Decision)",
];
const COLUMNS: [&str; 10] = [
    "id",
    "plan item",
    "acceptance criterion",
    "evidence / command",
    "expected",
    "observed",
    "status",
    "reviewer",
    "date",
    "notes",
];
const ROW_STATUSES: [&str; 5] = ["Pass", "Fail", "Blocked", "Pending", "Accepted risk"];

/// Check every dated execution plan.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let plans = root.join("docs/execution-plans");
    if !plans.is_dir() {
        return vec![Diagnostic::path(
            "execution-plans",
            &plans,
            root,
            "docs/execution-plans is missing",
        )];
    }
    let mut paths = Vec::new();
    collect_markdown(&plans, &mut paths);
    paths.sort();
    let mut diagnostics = Vec::new();
    for path in paths {
        let Some(directory) = path.parent().and_then(|parent| parent.file_name()) else {
            continue;
        };
        let directory = directory.to_string_lossy();
        if !is_date(&directory) {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "plan directory must be YYYY-MM-DD",
            ));
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if directory.as_ref() < BASELINE {
            continue;
        }
        if EXEMPT.contains(&relative.as_str()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        let log = root
            .join("docs/ai-transparency-log")
            .join(directory.as_ref())
            .join(format!("{stem}.md"));
        if !log.is_file() && !text.contains("AI log:") {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "no same-name AI log and no recorded '- AI log:' link",
            ));
        }
        let status = line_value(&text, "- Status:");
        let plan_status = match status {
            Some(status) => normalize_status(status),
            None => {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    "missing '- Status:' line",
                ));
                None
            }
        };
        if status.is_some() && plan_status.is_none() {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                format!("unknown plan status {:?}", status.unwrap_or("")),
            ));
        }
        let Some(claim) = line_value(&text, "- Claim type:") else {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "missing '- Claim type:' line",
            ));
            continue;
        };
        if !CLAIM_TYPES
            .iter()
            .any(|claim_type| claim.contains(claim_type))
        {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "claim type is not one of the four approved values",
            ));
        }
        let marker = "## Verification table";
        let Some(section) = text.split_once(marker).map(|(_, section)| section) else {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "missing '## Verification table' section",
            ));
            continue;
        };
        let section = section.split("\n## ").next().unwrap_or(section);
        let Some(header) = section.lines().find(|line| {
            line.to_lowercase().starts_with("| id |")
                && line.to_lowercase().contains("acceptance criterion")
        }) else {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "verification table header is missing or incomplete",
            ));
            continue;
        };
        let lower = header.to_lowercase();
        let missing: Vec<_> = COLUMNS
            .iter()
            .filter(|column| !lower.contains(**column))
            .collect();
        if !missing.is_empty() {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                format!(
                    "verification table is missing columns {}",
                    missing
                        .iter()
                        .map(|column| **column)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
        let rows: Vec<&str> = section
            .lines()
            .filter(|line| {
                line.starts_with('|') && !line.chars().all(|ch| matches!(ch, '|' | '-' | ' ' | ':'))
            })
            .collect();
        if rows.len() <= 1 {
            diagnostics.push(Diagnostic::path(
                "execution-plans",
                &path,
                root,
                "verification table has no data rows",
            ));
            continue;
        }
        let mut ids = BTreeSet::new();
        for row in rows.iter().skip(1) {
            let cells = row_cells(row);
            if cells.len() < COLUMNS.len() {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    format!(
                        "verification row has fewer than {} cells: {}",
                        COLUMNS.len(),
                        row.trim()
                    ),
                ));
                continue;
            }
            let row_id = &cells[0];
            if row_id.is_empty() {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    "verification row has no ID",
                ));
                continue;
            }
            if !ids.insert(row_id.clone()) {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    format!("duplicate verification row ID {row_id:?}"),
                ));
            }
            for (index, label) in [
                (1, "Plan item"),
                (2, "Acceptance criterion"),
                (3, "Evidence / command"),
                (4, "Expected"),
            ] {
                if cells.get(index).is_none_or(String::is_empty) {
                    diagnostics.push(Diagnostic::path(
                        "execution-plans",
                        &path,
                        root,
                        format!("{row_id} has an empty {label}"),
                    ));
                }
            }
            let row_status = &cells[6];
            if !row_status.is_empty() && !ROW_STATUSES.contains(&row_status.as_str()) {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    format!("{row_id} has unknown status {row_status:?}"),
                ));
            }
            if plan_status.as_deref() == Some("Verified")
                && row_status != "Pass"
                && row_status != "Accepted risk"
            {
                diagnostics.push(Diagnostic::path(
                    "execution-plans",
                    &path,
                    root,
                    format!(
                        "Verified plan row {row_id} is {}",
                        if row_status.is_empty() {
                            "empty"
                        } else {
                            row_status
                        }
                    ),
                ));
            }
        }
    }
    diagnostics
}

fn line_value<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::trim))
}

fn normalize_status(raw: &str) -> Option<String> {
    let text = raw
        .trim()
        .trim_matches(|ch| matches!(ch, '*' | '_' | '`'))
        .trim();
    for status in PLAN_STATUSES {
        if text == status {
            return Some(status.to_owned());
        }
        if text.starts_with(status)
            && text[status.len()..]
                .chars()
                .next()
                .is_some_and(|ch| !ch.is_alphanumeric())
        {
            return Some(status.to_owned());
        }
    }
    None
}

fn row_cells(line: &str) -> Vec<String> {
    let protected = line.replace("\\|", "\u{0}");
    let parts: Vec<&str> = protected.split('|').collect();
    if parts.len() < 2 {
        return Vec::new();
    }
    parts[1..parts.len() - 1]
        .iter()
        .map(|cell| cell.trim().replace('\u{0}', "|"))
        .collect()
}

fn collect_markdown(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
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
    fn a_plan_without_status_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-plans-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/execution-plans/2026-11-21")).expect("tree");
        fs::write(
            root.join("docs/execution-plans/2026-11-21/test.md"),
            "# Test\n\n- Claim type: Behaviour Claim\n\n## Verification table\n\n| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |\n|---|---|---|---|---|---|---|---|---|---|\n| V01 | P | C | E | X |  | Pass |  |  |  |\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("Status")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
