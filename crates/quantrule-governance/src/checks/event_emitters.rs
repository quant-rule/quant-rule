//! Check that every registered event code has a production detector reference.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::report::Diagnostic;

/// Check the registry against the production event sources.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let codes_path = root.join("crates/quantrule-model/src/events/codes.rs");
    let registry_path = root.join("docs/registry/indicators.json");
    let Ok(codes_text) = fs::read_to_string(&codes_path) else {
        return vec![Diagnostic::path(
            "event-emitters",
            &codes_path,
            root,
            "cannot read codes.rs",
        )];
    };
    let Ok(registry_text) = fs::read_to_string(&registry_path) else {
        return vec![Diagnostic::path(
            "event-emitters",
            &registry_path,
            root,
            "cannot read the registry",
        )];
    };
    let Ok(registry) = serde_json::from_str::<Value>(&registry_text) else {
        return vec![Diagnostic::path(
            "event-emitters",
            &registry_path,
            root,
            "the registry is not valid JSON",
        )];
    };
    let codes = constants(&codes_text);
    let mut sources = Vec::new();
    collect_production_sources(
        &root.join("crates/quantrule-model/src/events"),
        &mut sources,
    );
    let sources: Vec<String> = sources
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect();
    let referenced: Vec<&String> = codes
        .iter()
        .filter(|(name, _)| sources.iter().any(|text| contains_word(text, name)))
        .map(|(name, _)| name)
        .collect();
    let events = registry
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let by_code: BTreeMap<u64, &Value> = events
        .iter()
        .filter_map(|event| Some((event.get("code")?.as_u64()?, event)))
        .collect();
    let mut diagnostics = Vec::new();
    for (code, event) in &by_code {
        let names: Vec<&str> = codes
            .iter()
            .filter(|(_, value)| *value == code)
            .map(|(name, _)| name.as_str())
            .collect();
        if names
            .iter()
            .any(|name| referenced.iter().any(|r| *r == name))
        {
            continue;
        }
        let arithmetic = referenced.iter().any(|base_name| {
            let Some(base_code) = codes.get(*base_name) else {
                return false;
            };
            let Some(base) = by_code.get(base_code) else {
                return false;
            };
            let subtype = event.get("subtype").and_then(Value::as_u64).unwrap_or(0);
            let base_subtype = base.get("subtype").and_then(Value::as_u64).unwrap_or(0);
            event.get("event_type") == base.get("event_type")
                && event.get("source") == base.get("source")
                && *code >= *base_code
                && code - base_code == subtype.saturating_sub(base_subtype)
        });
        if !arithmetic {
            let id = event.get("id").and_then(Value::as_str).unwrap_or("?");
            let name = names.first().copied().unwrap_or("no constant");
            diagnostics.push(Diagnostic::path(
                "event-emitters",
                &registry_path,
                root,
                format!("no production emitter reference: {id} ({name}, code {code})"),
            ));
        }
    }
    diagnostics
}

fn constants(source: &str) -> BTreeMap<String, u64> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("pub const ")?;
            let (name, rest) = rest.split_once(':')?;
            let value = rest.split('=').nth(1)?.trim().trim_end_matches(';');
            Some((name.trim().to_owned(), value.parse().ok()?))
        })
        .collect()
}

fn collect_production_sources(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            collect_production_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name != "codes.rs" && name != "tests.rs" && !name.ends_with("_tests.rs") {
                found.push(path);
            }
        }
    }
}

fn contains_word(text: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(index) = text[start..].find(word) {
        let absolute = start + index;
        let before = text[..absolute].chars().next_back();
        let after = text[absolute + word.len()..].chars().next();
        let before_ok = before.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        let after_ok = after.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return true;
        }
        start = absolute + word.len();
    }
    false
}
