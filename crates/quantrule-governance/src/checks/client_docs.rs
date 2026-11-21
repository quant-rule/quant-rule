//! Check the client document and model index against the tree.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::checks::markdown;
use crate::report::Diagnostic;

const SCREEN_HEADER: &str = "| Screen | Component | Shows | Empty state |";
const STORE_HEADER: &str = "| Store | Holds | Polling |";

/// Check screens, stores, navigation labels, commands, and registry counts.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let client_path = root.join("docs/architecture/client.md");
    let client = fs::read_to_string(&client_path).unwrap_or_default();
    let library = root.join("apps/web/src/lib");
    let mut diagnostics = Vec::new();
    let screens = markdown::rows(&client, SCREEN_HEADER);
    if screens.is_empty() {
        diagnostics.push(Diagnostic::path(
            "client-docs",
            &client_path,
            root,
            format!("no '{SCREEN_HEADER}' table"),
        ));
    }
    let mut screen_names = Vec::new();
    for row in &screens {
        let screen = row.first().cloned().unwrap_or_default();
        let component = row
            .get(1)
            .map(|cell| markdown::unquote(cell))
            .unwrap_or_default();
        screen_names.push(screen.clone());
        let path = if component.starts_with("apps/") {
            root.join(&component)
        } else {
            library.join(&component)
        };
        if component.is_empty() || !path.is_file() {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &client_path,
                root,
                format!("screen {screen:?} names missing component {component:?}"),
            ));
        }
    }
    let stores = markdown::rows(&client, STORE_HEADER);
    if stores.is_empty() {
        diagnostics.push(Diagnostic::path(
            "client-docs",
            &client_path,
            root,
            format!("no '{STORE_HEADER}' table"),
        ));
    }
    for row in &stores {
        let store = row
            .first()
            .map(|cell| markdown::unquote(cell))
            .unwrap_or_default();
        if !store.is_empty() && !library.join(&store).is_file() {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &client_path,
                root,
                format!("store {store:?} does not exist"),
            ));
        }
    }
    let capabilities_path = library.join("capabilities.ts");
    if !capabilities_path.is_file() {
        diagnostics.push(Diagnostic::path(
            "client-docs",
            &capabilities_path,
            root,
            "the browser command map is missing",
        ));
    } else {
        let text = fs::read_to_string(&capabilities_path).unwrap_or_default();
        let labels = section_labels(&text);
        if labels.is_empty() {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &capabilities_path,
                root,
                "the SECTIONS table is missing",
            ));
        }
        for label in labels {
            if !screen_names.contains(&label) {
                diagnostics.push(Diagnostic::path(
                    "client-docs",
                    &client_path,
                    root,
                    format!("navigation label {label:?} has no screen row"),
                ));
            }
        }
        let entries = command_entries(&text);
        if entries.is_empty() {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &capabilities_path,
                root,
                "the COMMANDS table is missing",
            ));
        }
        for entry in entries {
            let missing: Vec<_> = ["command", "section", "control", "component", "testId"]
                .into_iter()
                .filter(|field| entry.get(*field).is_none_or(String::is_empty))
                .collect();
            if !missing.is_empty() {
                diagnostics.push(Diagnostic::path(
                    "client-docs",
                    &capabilities_path,
                    root,
                    format!(
                        "{} is missing {}",
                        entry.get("command").map_or("?", String::as_str),
                        missing.join(", ")
                    ),
                ));
                continue;
            }
            let component = entry.get("component").cloned().unwrap_or_default();
            let path = library.join(&component);
            if !path.is_file() {
                diagnostics.push(Diagnostic::path(
                    "client-docs",
                    &capabilities_path,
                    root,
                    format!(
                        "{} names missing component {component}",
                        entry.get("command").map_or("?", String::as_str)
                    ),
                ));
                continue;
            }
            let test_id = entry.get("testId").cloned().unwrap_or_default();
            let source = fs::read_to_string(&path).unwrap_or_default();
            if !source.contains(&format!("data-testid=\"{test_id}\"")) {
                diagnostics.push(Diagnostic::path(
                    "client-docs",
                    &path,
                    root,
                    format!(
                        "{} control {test_id:?} has no data-testid",
                        entry.get("command").map_or("?", String::as_str)
                    ),
                ));
            }
        }
    }
    let registry_path = root.join("docs/registry/indicators.json");
    let readme_path = root.join("docs/model/README.md");
    let Ok(registry_text) = fs::read_to_string(&registry_path) else {
        diagnostics.push(Diagnostic::path(
            "client-docs",
            &registry_path,
            root,
            "the registry is missing",
        ));
        return diagnostics;
    };
    let Ok(registry) = serde_json::from_str::<Value>(&registry_text) else {
        return diagnostics;
    };
    let Ok(readme) = fs::read_to_string(&readme_path) else {
        diagnostics.push(Diagnostic::path(
            "client-docs",
            &readme_path,
            root,
            "the model index is missing",
        ));
        return diagnostics;
    };
    let indicators = registry
        .get("indicators")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let events = registry
        .get("events")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    for (kind, expected) in [("indicator", indicators), ("event", events)] {
        let needle = format!("| {} verification matrix | Current; ", title_case(kind));
        let Some(line) = readme.lines().find(|line| line.starts_with(&needle)) else {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &readme_path,
                root,
                format!("the {kind} verification row is missing"),
            ));
            continue;
        };
        let observed = line
            .strip_prefix(&needle)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse::<usize>().ok());
        if observed != Some(expected) {
            diagnostics.push(Diagnostic::path(
                "client-docs",
                &readme_path,
                root,
                format!(
                    "{} {kind} rows, registry has {expected}",
                    observed.map_or_else(|| "no".to_owned(), |value| value.to_string())
                ),
            ));
        }
    }
    diagnostics
}

fn section_labels(text: &str) -> Vec<String> {
    let Some(start) = text.find("export const SECTIONS = [") else {
        return Vec::new();
    };
    let end = text[start..]
        .find("] as const;")
        .map_or(text.len(), |offset| start + offset);
    labels(&text[start..end])
}

fn labels(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(index) = rest.find("label:") {
        let after = &rest[index + 6..];
        if let Some(value) = quoted(after) {
            found.push(value);
        }
        rest = &after[1..];
    }
    found
}

fn command_entries(text: &str) -> Vec<std::collections::BTreeMap<String, String>> {
    let Some(start) = text.find("export const COMMANDS") else {
        return Vec::new();
    };
    let end = text[start..]
        .find("];")
        .map_or(text.len(), |offset| start + offset);
    let mut entries = Vec::new();
    for line in text[start..end].lines() {
        if !line.contains("command:") {
            continue;
        }
        let mut entry = std::collections::BTreeMap::new();
        for field in ["command", "section", "control", "component", "testId"] {
            if let Some(index) = line.find(&format!("{field}:"))
                && let Some(value) = quoted(&line[index + field.len() + 1..])
            {
                entry.insert(field.to_owned(), value);
            }
        }
        if !entry.is_empty() {
            entries.push(entry);
        }
    }
    entries
}

fn quoted(text: &str) -> Option<String> {
    let start = text.find('"')? + 1;
    let end = text[start..].find('"')? + start;
    Some(text[start..end].to_owned())
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_screen_component_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-client-docs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("docs/architecture")).expect("tree");
        fs::write(
            root.join("docs/architecture/client.md"),
            "| Screen | Component | Shows | Empty state |\n|---|---|---|---|\n| Missing | `Nope.svelte` | x | y |\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("missing component"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
