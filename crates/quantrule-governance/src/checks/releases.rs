//! Check release notes, their current-tag map, and changelog alignment.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::report::Diagnostic;

const BASELINE: &str = "2026-11-07";
const SUPPLY_CHAIN_BASELINE: &str = "1.5.10";
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
        if version_at_least(version, SUPPLY_CHAIN_BASELINE) {
            check_supply_chain(&path, root, &text, &mut diagnostics);
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

fn check_supply_chain(path: &Path, root: &Path, text: &str, diagnostics: &mut Vec<Diagnostic>) {
    let final_note = text
        .lines()
        .find_map(|line| line.strip_prefix("- Status:").map(str::trim))
        .is_none_or(|status| status.to_ascii_lowercase().contains("released"));
    let image = line_value(text, "- Image:");
    match image {
        None => diagnostics.push(Diagnostic::path(
            "releases",
            path,
            root,
            "missing '- Image:' line for a release at or after v1.5.10",
        )),
        Some(value) if !value.contains('/') || !(value.contains(':') || value.contains('@')) => {
            diagnostics.push(Diagnostic::path(
                "releases",
                path,
                root,
                "'- Image:' must name a registry/repository and tag or digest",
            ));
        }
        Some(_) => {}
    }
    let digest = line_value(text, "- Image digest:");
    match digest {
        None => diagnostics.push(Diagnostic::path(
            "releases",
            path,
            root,
            "missing '- Image digest: sha256:<64 lowercase hex>' line",
        )),
        Some(value) if !is_digest(value) => diagnostics.push(Diagnostic::path(
            "releases",
            path,
            root,
            "'- Image digest:' must be sha256:<64 lowercase hex>",
        )),
        Some(_) => {}
    }
    for (field, kind) in [
        ("- Image signature:", "cosign signature"),
        ("- SBOM:", "SBOM"),
        ("- CVE scan:", "CVE scan"),
    ] {
        let Some(value) = line_value(text, field) else {
            diagnostics.push(Diagnostic::path(
                "releases",
                path,
                root,
                format!("missing '{field}' line for a release at or after v1.5.10"),
            ));
            continue;
        };
        if is_pending(value) {
            if final_note {
                diagnostics.push(Diagnostic::path(
                    "releases",
                    path,
                    root,
                    format!("{kind} is pending in a Status: Released note"),
                ));
            }
            continue;
        }
        match field {
            "- Image signature:" if !value.contains("cosign") || find_digest(value).is_none() => {
                diagnostics.push(Diagnostic::path(
                    "releases",
                    path,
                    root,
                    "the image signature must name cosign and sha256:<64 lowercase hex>",
                ));
            }
            "- SBOM:" if !value.contains(".spdx.json") || find_digest(value).is_none() => {
                diagnostics.push(Diagnostic::path(
                    "releases",
                    path,
                    root,
                    "the SBOM must name a .spdx.json asset and sha256:<64 lowercase hex>",
                ));
            }
            "- CVE scan:" if find_digest(value).is_none() => {
                diagnostics.push(Diagnostic::path(
                    "releases",
                    path,
                    root,
                    "the CVE scan must name sha256:<64 lowercase hex>",
                ));
            }
            _ => {}
        }
    }
}

fn line_value<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.strip_prefix(prefix).map(str::trim))
}

fn version_at_least(value: &str, baseline: &str) -> bool {
    version_parts(value)
        .zip(version_parts(baseline))
        .is_some_and(|(actual, expected)| actual >= expected)
}

fn version_parts(value: &str) -> Option<(u64, u64, u64)> {
    let mut parts = value.split('.').map(|part| part.parse::<u64>().ok());
    Some((parts.next()??, parts.next()??, parts.next()??))
}

fn is_digest(value: &str) -> bool {
    let value = value.trim().trim_matches('`').trim();
    find_digest(value).is_some_and(|digest| digest == value)
}

fn find_digest(value: &str) -> Option<&str> {
    let start = value.find("sha256:")?;
    let rest = &value[start + 7..];
    if rest.len() < 64 || !rest.as_bytes()[..64].iter().all(u8::is_ascii_hexdigit) {
        return None;
    }
    if rest.as_bytes()[..64]
        .iter()
        .any(|byte| byte.is_ascii_uppercase())
    {
        return None;
    }
    if rest.as_bytes().get(64).is_some_and(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(&value[start..start + 7 + 64])
}

fn is_pending(value: &str) -> bool {
    value.to_ascii_lowercase().starts_with("pending")
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
            if name.ends_with(".md") && name != "README.md" {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn fixture(name: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-releases-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        root
    }

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(path, text).expect("write");
    }

    fn note(path: &Path, tag: &str, date: &str) {
        write(
            path,
            &format!(
                "# {tag} - Test\n\n- Date: {date} (UTC)\n- Tag: `{tag}`\n- Commit: `209530b`\n- Status: Released\n"
            ),
        );
    }

    fn changelog(release: &str) -> String {
        format!("# Changelog\n\n## [Unreleased]\n\n### Added\n\n- Pending change.\n\n{release}")
    }

    fn readme(mapping: &str, table: &[&str]) -> String {
        let rows = table
            .iter()
            .map(|tag| format!("| `{tag}` | `abc1234` | `{tag}` |"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "# Releases and Tags\n\n### Renumbering (2026-11-15)\n\n| Current tag | Commit | Was |\n|---|---|---|\n{rows}\n\n<!-- current-tags:start -->\n```json\n{mapping}\n```\n<!-- current-tags:end -->\n\n## 4. Historical tags\n"
        )
    }

    #[test]
    fn a_post_baseline_release_with_changelog_passes() {
        let root = fixture("pass");
        note(
            &root.join("docs/releases/2026-11-07/v0.6.0.md"),
            "v0.6.0",
            "2026-11-07",
        );
        write(
            &root.join("CHANGELOG.md"),
            &changelog("## [0.6.0] - 2026-11-07\n\n### Added\n\n- Release.\n"),
        );
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_historical_release_is_exempt_from_the_changelog() {
        let root = fixture("historical");
        note(
            &root.join("docs/releases/2026-10-28/v0.2.0.md"),
            "v0.2.0",
            "2026-10-28",
        );
        write(&root.join("CHANGELOG.md"), &changelog(""));
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_changelog_section_is_reported() {
        let root = fixture("changelog");
        note(
            &root.join("docs/releases/2026-11-07/v0.6.0.md"),
            "v0.6.0",
            "2026-11-07",
        );
        write(&root.join("CHANGELOG.md"), &changelog(""));
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("no changelog section"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_release_note_is_reported() {
        let root = fixture("note");
        write(
            &root.join("CHANGELOG.md"),
            &changelog("## [0.6.0] - 2026-11-07\n\n### Added\n\n- Release.\n"),
        );
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("no release note"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_tag_mismatch_is_reported() {
        let root = fixture("tag");
        note(
            &root.join("docs/releases/2026-11-07/v0.6.0.md"),
            "v0.6.1",
            "2026-11-07",
        );
        write(&root.join("CHANGELOG.md"), &changelog(""));
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("expected v0.6.0"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_bad_release_file_name_is_reported() {
        let root = fixture("name");
        note(
            &root.join("docs/releases/2026-11-07/release.md"),
            "v0.6.0",
            "2026-11-07",
        );
        write(&root.join("CHANGELOG.md"), &changelog(""));
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("vMAJOR.MINOR.PATCH"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_current_tag_resolves_through_the_map() {
        let root = fixture("resolve");
        let note_path = root.join("docs/releases/2026-11-15/v1.1.0.md");
        note(&note_path, "v1.1.0", "2026-11-15");
        write(
            &root.join("docs/releases/README.md"),
            &readme(r#"{"v1.0.0": "2026-11-15/v1.1.0.md"}"#, &["v1.0.0"]),
        );
        assert_eq!(resolve(&root, "v1.0.0").expect("resolved"), note_path);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_invalid_map_is_reported() {
        let root = fixture("map");
        write(
            &root.join("docs/releases/README.md"),
            &readme(r#"{"v1.0.0": }"#, &[]),
        );
        let mut diagnostics = Vec::new();
        let tags = current_tags(&root, &mut diagnostics);
        assert!(tags.is_empty());
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("not valid JSON"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    const DIGEST: &str = "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn supply_note(tag: &str, date: &str, status: &str, signature: &str) -> String {
        format!(
            "# {tag} - Test\n\n- Date: {date} (UTC)\n- Tag: `{tag}`\n- Status: {status}\n\
             - Image: `ghcr.io/quant-rule/quantrule-platform:{tag}`\n\
             - Image digest: `{DIGEST}`\n\
             - Image signature: {signature}\n\
             - SBOM: `sbom-{tag}.spdx.json sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`\n\
             - CVE scan: `cve-report-{tag}.json sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef` (0 Critical, 0 High)\n"
        )
    }

    fn supply_changelog(tag: &str, date: &str) -> String {
        let version = &tag[1..];
        changelog(&format!(
            "## [{version}] - {date}\n\n### Added\n\n- Supply chain.\n"
        ))
    }

    #[test]
    fn a_post_supply_chain_release_with_evidence_passes() {
        let root = fixture("supply-pass");
        write(
            &root.join("docs/releases/2026-11-24/v1.5.10.md"),
            &supply_note(
                "v1.5.10",
                "2026-11-24",
                "Released",
                "cosign sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            ),
        );
        write(
            &root.join("CHANGELOG.md"),
            &supply_changelog("v1.5.10", "2026-11-24"),
        );
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_supply_chain_release_may_start_pending() {
        let root = fixture("supply-pending");
        write(
            &root.join("docs/releases/2026-11-24/v1.5.10.md"),
            &supply_note("v1.5.10", "2026-11-24", "Prepared", "pending"),
        );
        write(
            &root.join("CHANGELOG.md"),
            &supply_changelog("v1.5.10", "2026-11-24"),
        );
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_pending_signature_in_a_released_note_is_reported() {
        let root = fixture("supply-pending-final");
        write(
            &root.join("docs/releases/2026-11-24/v1.5.10.md"),
            &supply_note("v1.5.10", "2026-11-24", "Released", "pending"),
        );
        write(
            &root.join("CHANGELOG.md"),
            &supply_changelog("v1.5.10", "2026-11-24"),
        );
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("pending in a Status: Released"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_supply_chain_field_is_reported() {
        for field in ["- Image signature:", "- SBOM:", "- CVE scan:"] {
            let root = fixture("supply-missing");
            let note = supply_note(
                "v1.5.10",
                "2026-11-24",
                "Released",
                "cosign sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            )
            .lines()
            .filter(|line| !line.starts_with(field))
            .collect::<Vec<_>>()
            .join("\n")
                + "\n";
            write(&root.join("docs/releases/2026-11-24/v1.5.10.md"), &note);
            write(
                &root.join("CHANGELOG.md"),
                &supply_changelog("v1.5.10", "2026-11-24"),
            );
            assert!(
                check(&root)
                    .iter()
                    .any(|d| d.message.contains(&format!("missing '{field}'"))),
                "{field}"
            );
            fs::remove_dir_all(&root).expect("cleanup");
        }
    }

    #[test]
    fn a_malformed_supply_chain_digest_is_reported() {
        let root = fixture("supply-digest");
        let note = supply_note("v1.5.10", "2026-11-24", "Released", "cosign sha256:abc");
        write(&root.join("docs/releases/2026-11-24/v1.5.10.md"), &note);
        write(
            &root.join("CHANGELOG.md"),
            &supply_changelog("v1.5.10", "2026-11-24"),
        );
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("image signature must name cosign"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_historical_note_without_supply_chain_fields_passes() {
        let root = fixture("supply-historical");
        write(
            &root.join("docs/releases/2026-11-21/v1.5.2.md"),
            &format!(
                "# v1.5.2 - Historical\n\n- Date: 2026-11-21 (UTC)\n- Tag: `v1.5.2`\n\
                 - Status: Released\n- Image: `ghcr.io/quant-rule/quantrule-platform`, digest `{DIGEST}`\n"
            ),
        );
        write(
            &root.join("CHANGELOG.md"),
            &supply_changelog("v1.5.2", "2026-11-21"),
        );
        assert!(check(&root).is_empty(), "{:?}", check(&root));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
