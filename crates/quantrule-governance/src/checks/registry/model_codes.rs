//! Compare the registry codes with the Rust constants that publish them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::report::Diagnostic;

use super::finding;

pub(super) fn check_model_codes(
    root: &Path,
    registry: &Path,
    object: &Map<String, Value>,
    events: &[Value],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = root.join("crates/quantrule-model/src/events/codes.rs");
    let Ok(source) = fs::read_to_string(&path) else {
        diagnostics.push(finding(&path, root, "cannot read the model codes"));
        return;
    };
    let constants = u16_constants(&source, None);
    for event in events {
        let Some(ident) = event.get("id").and_then(Value::as_str) else {
            continue;
        };
        let expected = ident.to_uppercase().replace('.', "_");
        let Some(registry_code) = event.get("code").and_then(Value::as_i64) else {
            continue;
        };
        match constants.get(&expected) {
            None => diagnostics.push(finding(
                registry,
                root,
                format!("model codes are missing {expected} for {ident}"),
            )),
            Some(code) if i64::from(*code) != registry_code => diagnostics.push(finding(
                registry,
                root,
                format!("model code {expected} is {code}, registry is {registry_code}"),
            )),
            Some(_) => {}
        }
    }
    let count = usize_constants(&source, "COUNT").get("COUNT").copied();
    if count != Some(events.len()) {
        diagnostics.push(finding(
            registry,
            root,
            "model event COUNT does not match the registry event count",
        ));
    }
    let library = root.join("crates/quantrule-model/src/lib.rs");
    let Ok(library_text) = fs::read_to_string(&library) else {
        diagnostics.push(finding(&library, root, "cannot read the model lib"));
        return;
    };
    let expected = object.get("version").and_then(Value::as_str).unwrap_or("");
    let Some(found) = string_constant(&library_text, "REGISTRY_VERSION") else {
        diagnostics.push(finding(
            &library,
            root,
            "model REGISTRY_VERSION was not found",
        ));
        return;
    };
    if found != expected {
        diagnostics.push(finding(
            &library,
            root,
            format!("model REGISTRY_VERSION {found} != registry {expected}"),
        ));
    }
}

pub(super) fn check_source_codes(
    root: &Path,
    registry: &Path,
    object: &Map<String, Value>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let path = root.join("crates/quantrule-model/src/events/source_codes.rs");
    let Ok(source) = fs::read_to_string(&path) else {
        diagnostics.push(finding(&path, root, "cannot read the model source codes"));
        return;
    };
    let constants = u16_constants(&source, Some("_SOURCE_CODE"));
    if constants.is_empty() {
        diagnostics.push(finding(registry, root, "model source codes were not found"));
        return;
    }
    let registered: BTreeSet<i64> = object
        .get("codes")
        .and_then(|codes| codes.get("sources"))
        .and_then(Value::as_object)
        .map(|sources| {
            sources
                .keys()
                .filter_map(|code| code.parse::<i64>().ok())
                .collect()
        })
        .unwrap_or_default();
    let mut seen: BTreeMap<u16, String> = BTreeMap::new();
    for (name, code) in &constants {
        if !registered.contains(&i64::from(*code)) {
            diagnostics.push(finding(
                registry,
                root,
                format!("model source code {name} is not in the registry sources"),
            ));
        }
        if let Some(previous) = seen.get(code) {
            diagnostics.push(finding(
                registry,
                root,
                format!("model source code {code} is defined twice: {previous} and {name}"),
            ));
        } else {
            seen.insert(*code, name.clone());
        }
    }
}

fn u16_constants(source: &str, suffix: Option<&str>) -> BTreeMap<String, u16> {
    let mut constants = BTreeMap::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("pub const ") else {
            continue;
        };
        let Some((name, rest)) = rest.split_once(':') else {
            continue;
        };
        if suffix.is_some_and(|suffix| !name.ends_with(suffix)) {
            continue;
        }
        let Some((kind, rest)) = rest.split_once('=') else {
            continue;
        };
        if kind.trim() != "u16" {
            continue;
        }
        let value = rest.trim().trim_end_matches(';').trim();
        if let Ok(code) = value.parse::<u16>() {
            constants.insert(name.trim().to_owned(), code);
        }
    }
    constants
}

fn usize_constants(source: &str, name: &str) -> BTreeMap<String, usize> {
    let mut constants = BTreeMap::new();
    for line in source.lines() {
        let Some(rest) = line.trim().strip_prefix("pub const ") else {
            continue;
        };
        let Some((found, rest)) = rest.split_once(':') else {
            continue;
        };
        if found.trim() != name {
            continue;
        }
        let Some((kind, rest)) = rest.split_once('=') else {
            continue;
        };
        if kind.trim() != "usize" {
            continue;
        }
        if let Ok(value) = rest.trim().trim_end_matches(';').trim().parse::<usize>() {
            constants.insert(name.to_owned(), value);
        }
    }
    constants
}

fn string_constant(source: &str, name: &str) -> Option<String> {
    for line in source.lines() {
        let Some(line) = line.trim().strip_prefix("pub const ") else {
            continue;
        };
        if !line.starts_with(name) || !line.contains(": &str") {
            continue;
        }
        let (_, rest) = line.split_once('=')?;
        let rest = rest.trim();
        let value = rest.strip_prefix('"')?.split('"').next()?;
        return Some(value.to_owned());
    }
    None
}
