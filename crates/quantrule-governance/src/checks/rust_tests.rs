//! Scan the Rust workspace for test function names.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Every function whose previous three lines carry `#[test]`.
#[must_use]
pub fn function_names(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for path in rust_files(root) {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some(name) = function_name(line) else {
                continue;
            };
            let prefix = lines[index.saturating_sub(3)..index].join("\n");
            if prefix.contains("#[test]") {
                names.insert(name.to_owned());
            }
        }
    }
    names
}

/// Test functions whose body constructs or reads an event batch.
#[must_use]
pub fn event_test_names(root: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for path in rust_files(root) {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().collect();
        let mut bodies: BTreeMap<String, String> = BTreeMap::new();
        for (index, line) in lines.iter().enumerate() {
            let Some(name) = function_name(line) else {
                continue;
            };
            let indent = line.len() - line.trim_start().len();
            let mut body = vec![*line];
            for later in lines.iter().skip(index + 1) {
                body.push(*later);
                if later.trim() == "}" && later.len() - later.trim_start().len() == indent {
                    break;
                }
            }
            bodies.insert(name.to_owned(), body.join("\n"));
        }
        for (index, line) in lines.iter().enumerate() {
            let Some(name) = function_name(line) else {
                continue;
            };
            let prefix = lines[index.saturating_sub(3)..index].join("\n");
            if !prefix.contains("#[test]") {
                continue;
            }
            let body = bodies.get(name).cloned().unwrap_or_default();
            let called = called_functions(&body);
            let mut expanded = body;
            for called_name in called {
                if let Some(called_body) = bodies.get(&called_name) {
                    expanded.push('\n');
                    expanded.push_str(called_body);
                }
            }
            if expanded.contains("EventRecord") || expanded.contains(".events()") {
                names.insert(name.to_owned());
            }
        }
    }
    names
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(&root.join("crates"), &mut files);
    files
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn function_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("fn ")?;
    let end = rest.find('(')?;
    let name = &rest[..end];
    (!name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'))
    .then_some(name)
}

fn called_functions(body: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    let mut rest = body;
    while let Some(index) = rest.find('(') {
        let before = &rest[..index];
        let name: String = before
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if !name.is_empty() && !name.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
            found.insert(name);
        }
        rest = &rest[index + 1..];
    }
    found
}
