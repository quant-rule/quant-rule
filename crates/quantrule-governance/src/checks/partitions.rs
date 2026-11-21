//! Check the partition file, instrument aliases, and config references.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::report::Diagnostic;

/// Check `data/partitions.json` and every JSON config instrument reference.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let path = root.join("data/partitions.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Diagnostic::path(
            "partitions",
            &path,
            root,
            "data/partitions.json is missing",
        )];
    };
    let Ok(document) = serde_json::from_str::<Value>(&text) else {
        return vec![Diagnostic::path(
            "partitions",
            &path,
            root,
            "data/partitions.json is not valid JSON",
        )];
    };
    let mut diagnostics = Vec::new();
    let instruments = document
        .get("instruments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let canonical: Vec<String> = instruments
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if canonical.is_empty() {
        diagnostics.push(Diagnostic::path(
            "partitions",
            &path,
            root,
            "instruments must be a non-empty list",
        ));
    }
    let canonical_set: BTreeSet<_> = canonical.iter().cloned().collect();
    if canonical_set.len() != canonical.len() {
        diagnostics.push(Diagnostic::path(
            "partitions",
            &path,
            root,
            "instruments must be unique",
        ));
    }
    let aliases = document
        .get("aliases")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if !document.get("aliases").is_some_and(Value::is_object) {
        diagnostics.push(Diagnostic::path(
            "partitions",
            &path,
            root,
            "aliases must be an object",
        ));
    }
    for (alias, target) in &aliases {
        if !target
            .as_str()
            .is_some_and(|target| canonical_set.contains(target))
        {
            diagnostics.push(Diagnostic::path(
                "partitions",
                &path,
                root,
                format!("alias {alias:?} does not resolve to a canonical instrument"),
            ));
        }
    }
    let excluded = excluded_windows(&document, root, &path, &mut diagnostics);
    let partitions = document
        .get("partitions")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if partitions.is_empty() {
        diagnostics.push(Diagnostic::path(
            "partitions",
            &path,
            root,
            "partitions must be a non-empty object",
        ));
    }
    for (name, partition) in &partitions {
        let Some(partition) = partition.as_object() else {
            diagnostics.push(Diagnostic::path(
                "partitions",
                &path,
                root,
                format!("partition {name:?} must be an object"),
            ));
            continue;
        };
        if let Some(window) = partition.get("window").and_then(Value::as_object) {
            let start = window.get("from").and_then(Value::as_str).unwrap_or("");
            let end = window.get("to").and_then(Value::as_str).unwrap_or("");
            if !is_date(start) {
                diagnostics.push(Diagnostic::path(
                    "partitions",
                    &path,
                    root,
                    format!("partition {name:?} window.from must be YYYY-MM-DD"),
                ));
            }
            if !is_date(end) {
                diagnostics.push(Diagnostic::path(
                    "partitions",
                    &path,
                    root,
                    format!("partition {name:?} window.to must be YYYY-MM-DD"),
                ));
            }
            if is_date(start) && is_date(end) && start > end {
                diagnostics.push(Diagnostic::path(
                    "partitions",
                    &path,
                    root,
                    format!("partition {name:?} window.from is after window.to"),
                ));
            }
            for (excluded_start, excluded_end) in &excluded {
                if start <= excluded_end.as_str() && excluded_start.as_str() <= end {
                    diagnostics.push(Diagnostic::path(
                        "partitions",
                        &path,
                        root,
                        format!(
                            "partition {name:?} overlaps excluded window {excluded_start}..{excluded_end}"
                        ),
                    ));
                }
            }
        } else {
            diagnostics.push(Diagnostic::path(
                "partitions",
                &path,
                root,
                format!("partition {name:?} has no window"),
            ));
        }
        if !partition
            .get("use")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.is_empty())
        {
            diagnostics.push(Diagnostic::path(
                "partitions",
                &path,
                root,
                format!("partition {name:?} has no use"),
            ));
        }
        let declared = partition
            .get("instruments")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if declared.is_empty() {
            diagnostics.push(Diagnostic::path(
                "partitions",
                &path,
                root,
                format!("partition {name:?} has no instruments"),
            ));
        }
        for instrument in declared.iter().filter_map(Value::as_str) {
            if !canonical_set.contains(instrument) {
                diagnostics.push(Diagnostic::path(
                    "partitions",
                    &path,
                    root,
                    format!("partition {name:?} cites undeclared instrument {instrument:?}"),
                ));
            }
        }
    }
    for (config_path, instrument) in config_instruments(root) {
        if !canonical_set.contains(&instrument) && !aliases.contains_key(&instrument) {
            diagnostics.push(Diagnostic::path(
                "partitions",
                Path::new(&config_path),
                root,
                format!("instrument {instrument:?} is not declared"),
            ));
        }
    }
    diagnostics
}

fn excluded_windows(
    document: &Value,
    root: &Path,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<(String, String)> {
    let Some(excluded) = document.get("excluded_windows").and_then(Value::as_array) else {
        diagnostics.push(Diagnostic::path(
            "partitions",
            path,
            root,
            "excluded_windows must be a list",
        ));
        return Vec::new();
    };
    let mut windows = Vec::new();
    for (index, window) in excluded.iter().enumerate() {
        let Some(window) = window.as_object() else {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}] must be an object"),
            ));
            continue;
        };
        let start = window.get("from").and_then(Value::as_str).unwrap_or("");
        let end = window.get("to").and_then(Value::as_str).unwrap_or("");
        if !is_date(start) {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}].from must be YYYY-MM-DD"),
            ));
        }
        if !is_date(end) {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}].to must be YYYY-MM-DD"),
            ));
        }
        if !window
            .get("reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| !reason.trim().is_empty())
        {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}] has no reason"),
            ));
        }
        if !is_date(start) || !is_date(end) {
            continue;
        }
        if start > end {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}].from is after .to"),
            ));
            continue;
        }
        if windows
            .last()
            .is_some_and(|(previous, _): &(String, String)| start < previous)
        {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!("excluded_windows[{index}] is out of order"),
            ));
        }
        windows.push((start.to_owned(), end.to_owned()));
    }
    for pair in windows.windows(2) {
        if pair[1].0 <= pair[0].1 {
            diagnostics.push(Diagnostic::path(
                "partitions",
                path,
                root,
                format!(
                    "excluded_windows overlap: {}..{} and {}..{}",
                    pair[0].0, pair[0].1, pair[1].0, pair[1].1
                ),
            ));
        }
    }
    windows
}

fn config_instruments(root: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let config = root.join("config");
    let mut files = Vec::new();
    collect_json(&config, &mut files);
    files.sort();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(document) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        walk_instruments(&document, "", &relative, &mut found);
    }
    found
}

fn walk_instruments(
    document: &Value,
    trail: &str,
    relative: &str,
    found: &mut Vec<(String, String)>,
) {
    match document {
        Value::Object(map) => {
            for (key, value) in map {
                let path = if trail.is_empty() {
                    key.clone()
                } else {
                    format!("{trail}.{key}")
                };
                if key == "instrument"
                    && let Some(instrument) = value.as_str().filter(|value| !value.is_empty())
                {
                    found.push((format!("{relative}:{path}"), instrument.to_owned()));
                }
                if key == "instruments"
                    && let Some(items) = value.as_array()
                {
                    for (index, item) in items.iter().enumerate() {
                        if let Some(instrument) = item.as_str().filter(|value| !value.is_empty()) {
                            found.push((
                                format!("{relative}:{path}[{index}]"),
                                instrument.to_owned(),
                            ));
                        }
                    }
                }
                walk_instruments(value, &path, relative, found);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                walk_instruments(item, &format!("{trail}[{index}]"), relative, found);
            }
        }
        _ => {}
    }
}

fn collect_json(directory: &Path, found: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
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
    fn an_unknown_alias_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-partitions-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data")).expect("tree");
        fs::write(
            root.join("data/partitions.json"),
            r#"{"instruments":["BTCUSDT"],"aliases":{"X":"NOPE"},"partitions":{},"excluded_windows":[]}"#,
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.iter().any(|d| d.message.contains("alias")));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
