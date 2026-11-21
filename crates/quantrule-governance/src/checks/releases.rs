//! Check release notes, their current-tag map, and changelog alignment.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::report::Diagnostic;

const BASELINE: &str = "2026-11-07";
const LIVING: [&str; 5] = [
    "AGENTS.md",
    "README.md",
    "docs/README.md",
    "docs/architecture",
    "docs/workflows",
];
const CONTEXT: [&str; 10] = [
    "prepared as",
    "renumbered",
    "historical",
    "retired",
    "ready →",
    "connected ·",
    "product ",
    "release note keeps",
    "keeps its",
    "docs/releases/",
];
const MAP_START: &str = "<!-- current-tags:start -->";
const MAP_END: &str = "<!-- current-tags:end -->";

/// Check release notes, the current-tag map, and changelog alignment.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let tags = current_tags(root, &mut diagnostics);
    if tags.is_empty() {
        // The map function already reported the reason.
    } else {
        living_tag_claims(root, &tags, &mut diagnostics);
    }
    let changelog = changelog_releases(root);
    let mut seen = BTreeSet::new();
    for path in release_notes(root) {
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(directory) = path.parent().and_then(|parent| parent.file_name()) else {
            continue;
        };
        let directory = directory.to_string_lossy();
        if !is_date(&directory) {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                "release directory must be YYYY-MM-DD",
            ));
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if !is_tag(stem) {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                "release file must be vMAJOR.MINOR.PATCH.md",
            ));
            continue;
        }
        if !seen.insert(stem.to_owned()) {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!("release {stem} appears twice"),
            ));
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let tag_line = text
            .lines()
            .find_map(|line| line.strip_prefix("- Tag:").map(str::trim));
        match tag_line {
            None => diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                "missing '- Tag:' line",
            )),
            Some(value) => match find_tag(value) {
                None => diagnostics.push(Diagnostic::path(
                    "releases",
                    &path,
                    root,
                    "'- Tag:' must name a vMAJOR.MINOR.PATCH tag",
                )),
                Some(found) if found != stem => diagnostics.push(Diagnostic::path(
                    "releases",
                    &path,
                    root,
                    format!("'- Tag:' names {found}, expected {stem}"),
                )),
                Some(_) => {}
            },
        }
        let date_line = text
            .lines()
            .find_map(|line| line.strip_prefix("- Date:").map(str::trim))
            .and_then(|value| value.split_whitespace().next());
        let Some(note_date) = date_line else {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                "missing '- Date: YYYY-MM-DD' line",
            ));
            continue;
        };
        if note_date != directory.as_ref() {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!(
                    "'- Date:' {note_date} does not match the directory {}",
                    directory
                ),
            ));
        }
        let version = &stem[1..];
        if note_date >= BASELINE {
            match changelog.get(version) {
                None => diagnostics.push(Diagnostic::path(
                    "releases",
                    &path,
                    root,
                    format!(
                        "release {stem} has no changelog section '## [{version}] - {note_date}'"
                    ),
                )),
                Some(date) if date != note_date => diagnostics.push(Diagnostic::path(
                    "releases",
                    &path,
                    root,
                    format!("changelog date {date} does not match release date {note_date}"),
                )),
                Some(_) => {}
            }
        }
        let _ = relative;
    }
    for (version, date) in &changelog {
        if date.as_str() < BASELINE {
            continue;
        }
        let tag = format!("v{version}");
        let expected = root
            .join("docs/releases")
            .join(date)
            .join(format!("{tag}.md"));
        if !expected.is_file() {
            diagnostics.push(Diagnostic::path(
                "releases",
                &expected,
                root,
                format!(
                    "CHANGELOG.md: release {tag} has no release note at {}",
                    relative(&expected, root)
                ),
            ));
        }
    }
    diagnostics
}

/// Resolve a current tag to its release note.
///
/// # Errors
///
/// Returns the map or missing-note reason.
pub fn resolve(root: &Path, tag: &str) -> Result<PathBuf, String> {
    let mut diagnostics = Vec::new();
    let tags = current_tags(root, &mut diagnostics);
    if !diagnostics.is_empty() {
        return Err(diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
            .join("; "));
    }
    let relative = tags
        .get(tag)
        .ok_or_else(|| format!("{tag} is not in the current-tag map"))?;
    let note = root.join("docs/releases").join(relative);
    if !note.is_file() {
        return Err(format!("{tag} resolves to missing note {relative}"));
    }
    Ok(note)
}

fn current_tags(root: &Path, diagnostics: &mut Vec<Diagnostic>) -> BTreeMap<String, String> {
    let path = root.join("docs/releases/README.md");
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Some(start) = text.find(MAP_START) else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            format!("missing the {MAP_START} current-tag map"),
        ));
        return BTreeMap::new();
    };
    let Some(end) = text[start..].find(MAP_END).map(|offset| start + offset) else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            format!("missing the {MAP_END} current-tag map end"),
        ));
        return BTreeMap::new();
    };
    let block = &text[start..end];
    let Some(json_start) = block.find("```json") else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            "the current-tag map must be a fenced json block",
        ));
        return BTreeMap::new();
    };
    let json_text = &block[json_start + 7..];
    let Some(json_end) = json_text.find("```") else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            "the current-tag map must be a fenced json block",
        ));
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(json_text[..json_end].trim()) else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            "the current-tag map is not valid JSON",
        ));
        return BTreeMap::new();
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(Diagnostic::path(
            "releases",
            &path,
            root,
            "the current-tag map must be a JSON object",
        ));
        return BTreeMap::new();
    };
    let mut tags = BTreeMap::new();
    for (key, value) in object {
        let Some(relative) = value.as_str() else {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                "current-tag map keys and values must be strings",
            ));
            continue;
        };
        if !is_tag(key) {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!("current tag {key} is not vMAJOR.MINOR.PATCH"),
            ));
            continue;
        }
        let note = root.join("docs/releases").join(relative);
        if !note.is_file() {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!("current tag {key} points to missing note {relative}"),
            ));
            continue;
        }
        if note
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_none_or(|stem| !is_tag(stem))
        {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!("current tag {key} points to {relative}, not a vMAJOR.MINOR.PATCH note"),
            ));
            continue;
        }
        tags.insert(key.clone(), relative.to_owned());
    }
    let table_start = text.find("### Renumbering (2026-11-15)");
    let table_end = table_start.and_then(|start| {
        text[start..]
            .find("## 4. Historical tags")
            .map(|offset| start + offset)
    });
    if let (Some(table_start), Some(table_end)) = (table_start, table_end) {
        let table: BTreeSet<String> = text[table_start..table_end]
            .lines()
            .filter_map(|line| {
                let rest = line.strip_prefix("| `")?;
                let tag = rest.split('`').next()?;
                is_tag(tag).then(|| tag.to_owned())
            })
            .collect();
        let missing: Vec<_> = table
            .difference(&tags.keys().cloned().collect())
            .cloned()
            .collect();
        let extra: Vec<_> = tags
            .keys()
            .filter(|key| !table.contains(*key))
            .cloned()
            .collect();
        if !missing.is_empty() {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!("current-tag map is missing {}", missing.join(", ")),
            ));
        }
        if !extra.is_empty() {
            diagnostics.push(Diagnostic::path(
                "releases",
                &path,
                root,
                format!(
                    "current-tag map names {}, which the renumbering table does not",
                    extra.join(", ")
                ),
            ));
        }
    }
    tags
}

fn living_tag_claims(
    root: &Path,
    tags: &BTreeMap<String, String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut files = Vec::new();
    for entry in LIVING {
        let path = root.join(entry);
        if path.is_file() {
            files.push(path);
        } else if path.is_dir() {
            collect_markdown(&path, &mut files);
        }
    }
    files.sort();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if CONTEXT.iter().any(|context| lower.contains(context)) {
                continue;
            }
            for token in tag_tokens(line) {
                if !tags.contains_key(&token) {
                    diagnostics.push(Diagnostic::line(
                        "releases",
                        &path,
                        root,
                        index + 1,
                        format!("living document names {token}, which is not a current tag"),
                    ));
                }
            }
        }
    }
}

fn changelog_releases(root: &Path) -> BTreeMap<String, String> {
    let Ok(text) = fs::read_to_string(root.join("CHANGELOG.md")) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("## [")?;
            let close = rest.find(']')?;
            let version = &rest[..close];
            let date = rest[close + 1..].strip_prefix(" - ")?.trim();
            is_date(date).then(|| (version.to_owned(), date.to_owned()))
        })
        .collect()
}

fn release_notes(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root.join("docs/releases")) else {
        return found;
    };
    for entry in entries.flatten() {
        let directory = entry.path();
        if !directory.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(directory) else {
            continue;
        };
        for file in files.flatten() {
            let path = file.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name.starts_with('v') && name.ends_with(".md") {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn collect_markdown(directory: &Path, found: &mut Vec<PathBuf>) {
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

fn tag_tokens(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut found = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'v' && index + 1 < bytes.len() && bytes[index + 1].is_ascii_digit() {
            let start = index;
            index += 1;
            let mut components = 0;
            while index < bytes.len() && (bytes[index].is_ascii_digit() || bytes[index] == b'.') {
                if bytes[index] == b'.' {
                    components += 1;
                }
                index += 1;
            }
            if components == 2 {
                found.push(line[start..index].to_owned());
            }
        } else {
            index += 1;
        }
    }
    found
}

fn find_tag(text: &str) -> Option<String> {
    tag_tokens(text).into_iter().next()
}

fn is_tag(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('v') else {
        return false;
    };
    let parts: Vec<&str> = rest.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
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

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
