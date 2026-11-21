//! Find documented command invocations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use super::collect;
use super::lines::fenced_blocks;

pub(super) fn invocations(
    root: &Path,
    commands: &BTreeSet<String>,
) -> BTreeMap<String, Vec<PathBuf>> {
    let mut found: BTreeMap<String, Vec<PathBuf>> = commands
        .iter()
        .map(|name| (name.clone(), Vec::new()))
        .collect();
    let directory = root.join("docs/tutorials");
    if !directory.is_dir() {
        return found;
    }
    let mut files = Vec::new();
    collect(&directory, "md", &mut files);
    files.sort();
    for path in files {
        let Ok(blocks) = fenced_blocks(root, &path) else {
            continue;
        };
        for block in blocks {
            for command in commands {
                if contains_invocation(&block, command) {
                    found.entry(command.clone()).or_default().push(path.clone());
                }
            }
        }
    }
    found
}

pub(super) fn contains_invocation(block: &str, command: &str) -> bool {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    let needle = "quantrule";
    let mut search = 0;
    while let Some(index) = block[search..].find(needle) {
        let absolute = search + index;
        let before = block[..absolute].chars().next_back();
        if before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-') {
            search = absolute + needle.len();
            continue;
        }
        let mut rest = &block[absolute + needle.len()..];
        let Some(trimmed) = strip_whitespace(rest) else {
            search = absolute + needle.len();
            continue;
        };
        if trimmed.len() == rest.len() {
            search = absolute + needle.len();
            continue;
        }
        rest = trimmed;
        let mut matched = true;
        for (position, token) in tokens.iter().enumerate() {
            if !rest.starts_with(token) {
                matched = false;
                break;
            }
            rest = &rest[token.len()..];
            let after = if position + 1 == tokens.len() {
                rest.is_empty() || rest.chars().next().is_some_and(|ch| ch.is_whitespace())
            } else {
                let Some(next) = strip_whitespace(rest) else {
                    matched = false;
                    break;
                };
                if next.len() == rest.len() {
                    matched = false;
                    break;
                }
                rest = next;
                true
            };
            if !after {
                matched = false;
                break;
            }
        }
        if matched {
            return true;
        }
        search = absolute + needle.len();
    }
    false
}

pub(super) fn invocations_text(block: &str) -> Vec<String> {
    let mut found = Vec::new();
    let needle = "quantrule";
    let mut search = 0;
    while let Some(index) = block[search..].find(needle) {
        let absolute = search + index;
        let before = block[..absolute].chars().next_back();
        if before.is_some_and(|ch| ch.is_alphanumeric() || ch == '_' || ch == '-') {
            search = absolute + needle.len();
            continue;
        }
        let rest = &block[absolute + needle.len()..];
        let Some(trimmed) = strip_whitespace(rest) else {
            search = absolute + needle.len();
            continue;
        };
        if trimmed.len() == rest.len() {
            search = absolute + needle.len();
            continue;
        }
        let end = trimmed.find('\n').unwrap_or(trimmed.len());
        let value = trimmed[..end].trim();
        if !value.is_empty() {
            found.push(value.to_owned());
        }
        search = absolute + needle.len();
    }
    found
}

fn strip_whitespace(text: &str) -> Option<&str> {
    let trimmed = text.trim_start();
    (trimmed.len() != text.len()).then_some(trimmed)
}
