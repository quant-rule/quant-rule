//! Check the user manual against the command and client registries.

use std::fs;
use std::path::Path;

use crate::checks::markdown;
use crate::report::Diagnostic;

const SECTIONS: [&str; 6] = [
    "Goal",
    "Before you start",
    "Steps",
    "What you see",
    "What it proves",
    "Failure",
];

/// Check chapters, command rows, GUI rows, journey sections, and evidence labels.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let manual = root.join("docs/manual");
    let readme_path = manual.join("README.md");
    let Ok(readme) = fs::read_to_string(&readme_path) else {
        return vec![Diagnostic::path(
            "manual",
            &readme_path,
            root,
            "the manual index is missing",
        )];
    };
    let mut diagnostics = Vec::new();
    let rows = chapter_rows(&readme);
    if rows.is_empty() {
        diagnostics.push(Diagnostic::path(
            "manual",
            &readme_path,
            root,
            "no chapter table with a Status column",
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for row in &rows {
        if row.len() < 5 {
            diagnostics.push(Diagnostic::path(
                "manual",
                &readme_path,
                root,
                "a chapter row has fewer than five columns",
            ));
            continue;
        }
        let number = row[0].trim();
        let file = chapter_file(&row[2]);
        if file.is_empty() {
            diagnostics.push(Diagnostic::path(
                "manual",
                &readme_path,
                root,
                format!("chapter {number} names no file"),
            ));
            continue;
        }
        if !seen.insert(file.clone()) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &readme_path,
                root,
                format!("chapter file {file} is listed twice"),
            ));
        }
        if !readme.contains(&format!("]({file})")) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &readme_path,
                root,
                format!("chapter {file} is not linked"),
            ));
        }
        let path = manual.join(&file);
        let Ok(text) = fs::read_to_string(&path) else {
            diagnostics.push(Diagnostic::path(
                "manual",
                &path,
                root,
                "the chapter file is missing",
            ));
            continue;
        };
        if !text.lines().any(|line| line.starts_with("- Status:")) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &path,
                root,
                "missing a '- Status:' line",
            ));
        }
        if row[4].trim().to_lowercase() != "current" {
            diagnostics.push(Diagnostic::path(
                "manual",
                &readme_path,
                root,
                format!("chapter {file} is not Current"),
            ));
        }
        if let Ok(chapter) = number.parse::<usize>()
            && chapter <= 8
        {
            for title in SECTIONS {
                if heading_body(&text, title).is_none() {
                    diagnostics.push(Diagnostic::path(
                        "manual",
                        &path,
                        root,
                        format!("missing the '{title}' section"),
                    ));
                }
            }
            if let Some(proves) = heading_body(&text, "What it proves") {
                let lower = proves.to_lowercase();
                if !(lower.contains("governed evidence")
                    || lower.contains("evidence")
                    || lower.contains("diagnostic")
                    || lower.contains("verification"))
                {
                    diagnostics.push(Diagnostic::path(
                        "manual",
                        &path,
                        root,
                        "'What it proves' names no evidence label",
                    ));
                }
            }
        }
    }
    let command_path = manual.join("09-command-reference.md");
    let command_text = fs::read_to_string(&command_path).unwrap_or_default();
    let commands = help_commands(root);
    let usages = help_usages(root);
    if commands.is_empty() {
        diagnostics.push(Diagnostic::path(
            "manual",
            &root.join("crates/quantrule-shell/src/command/help.rs"),
            root,
            "no commands",
        ));
    }
    for command in &commands {
        if !markdown::has_row(&command_text, command) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &command_path,
                root,
                format!("no row for '{command}'"),
            ));
            continue;
        }
        let documented =
            markdown::row_cell(&command_text, command, 1).map(|cell| markdown::unquote(&cell));
        let expected = usages.get(command.as_str()).cloned();
        match (documented, expected) {
            (Some(documented), Some(expected)) if documented == expected => {}
            (None, _) | (_, None) => diagnostics.push(Diagnostic::path(
                "manual",
                &command_path,
                root,
                format!("no usage for '{command}'"),
            )),
            (Some(documented), Some(expected)) => diagnostics.push(Diagnostic::path(
                "manual",
                &command_path,
                root,
                format!("usage for '{command}' is {documented:?}, registry has {expected:?}"),
            )),
        }
    }
    let gui_path = manual.join("10-gui-reference.md");
    let gui = fs::read_to_string(&gui_path).unwrap_or_default();
    let screens = screen_names(root);
    if screens.is_empty() {
        diagnostics.push(Diagnostic::path(
            "manual",
            &root.join("docs/architecture/client.md"),
            root,
            "no screen table rows",
        ));
    }
    for screen in screens {
        if !markdown::has_row(&gui, &screen) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &gui_path,
                root,
                format!("no row for screen '{screen}'"),
            ));
        }
    }
    let tabs = workbench_tabs(root);
    if tabs.is_empty() {
        diagnostics.push(Diagnostic::path(
            "manual",
            &root.join("apps/web/src/lib/WorkbenchPanel.svelte"),
            root,
            "no tabs",
        ));
    }
    for tab in tabs {
        let label = format!("Workbench — {tab}");
        if !markdown::has_row(&gui, &label) {
            diagnostics.push(Diagnostic::path(
                "manual",
                &gui_path,
                root,
                format!("no row for '{label}'"),
            ));
        }
    }
    diagnostics
}

fn chapter_rows(readme: &str) -> Vec<Vec<String>> {
    let Some(header) = readme
        .lines()
        .find(|line| line.trim_start().starts_with("| # | Chapter |") && line.contains("File"))
    else {
        return Vec::new();
    };
    markdown::rows(readme, header)
}

fn chapter_file(cell: &str) -> String {
    markdown::link_target(cell).unwrap_or_else(|| markdown::unquote(cell))
}

fn help_commands(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join("crates/quantrule-shell/src/command/help.rs"))
        .unwrap_or_default();
    let Some(start) = text.find("const COMMANDS") else {
        return Vec::new();
    };
    let end = text[start..]
        .find("];")
        .map_or(text.len(), |offset| start + offset);
    text[start..end]
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("name:")?;
            quoted(rest)
        })
        .collect()
}

fn help_usages(root: &Path) -> std::collections::BTreeMap<String, String> {
    let text = fs::read_to_string(root.join("crates/quantrule-shell/src/command/help.rs"))
        .unwrap_or_default();
    let Some(start) = text.find("const COMMANDS") else {
        return std::collections::BTreeMap::new();
    };
    let end = text[start..]
        .find("];")
        .map_or(text.len(), |offset| start + offset);
    let mut usages = std::collections::BTreeMap::new();
    for entry in text[start..end].split("CommandHelp {").skip(1) {
        let name = entry
            .lines()
            .find_map(|line| quoted(line.trim().strip_prefix("name:")?));
        let usage = entry
            .lines()
            .find_map(|line| quoted(line.trim().strip_prefix("usage:")?));
        if let (Some(name), Some(usage)) = (name, usage) {
            usages.insert(name, usage);
        }
    }
    usages
}

fn screen_names(root: &Path) -> Vec<String> {
    let text = fs::read_to_string(root.join("docs/architecture/client.md")).unwrap_or_default();
    markdown::rows(&text, "| Screen | Component | Shows | Empty state |")
        .into_iter()
        .filter_map(|row| row.first().cloned())
        .collect()
}

fn workbench_tabs(root: &Path) -> Vec<String> {
    let text =
        fs::read_to_string(root.join("apps/web/src/lib/WorkbenchPanel.svelte")).unwrap_or_default();
    let mut tabs = Vec::new();
    let mut rest: &str = &text;
    while let Some(index) = rest.find("role=\"tab\"") {
        let after = &rest[index..];
        let Some(close) = after.find("</button>") else {
            break;
        };
        let segment = &after[..close];
        let Some(open) = segment.rfind('>') else {
            break;
        };
        let label = segment[open + 1..].trim();
        if !label.is_empty() {
            tabs.push(label.to_owned());
        }
        rest = &after[close + 9..];
    }
    tabs
}

fn heading_body(text: &str, title: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|line| {
        line.strip_prefix("## ")
            .map(|heading| {
                heading
                    .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.')
                    .trim()
            })
            .is_some_and(|heading| heading.eq_ignore_ascii_case(title))
    })? + 1;
    let end = lines[start..]
        .iter()
        .position(|line| line.starts_with("## "))
        .map_or(lines.len(), |offset| start + offset);
    Some(lines[start..end].join("\n"))
}

fn quoted(text: &str) -> Option<String> {
    let start = text.find('"')? + 1;
    let end = text[start..].find('"')? + start;
    Some(text[start..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_chapter_file_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-manual-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/manual")).expect("tree");
        fs::write(
            root.join("docs/manual/README.md"),
            "| # | Chapter | File | Answers | Status |\n|---|---|---|---|---|\n| 1 | Missing | [01.md](01.md) | x | Current |\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("chapter file is missing"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
