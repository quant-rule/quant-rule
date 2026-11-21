//! Check the Rust artifact label table against the registry.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::report::Diagnostic;

/// Check every scalar and pair label function.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let registry_path = root.join("docs/registry/indicators.json");
    let rust_path = root.join("crates/quantrule-app/src/artifact/labels.rs");
    let Ok(registry_text) = fs::read_to_string(&registry_path) else {
        return vec![Diagnostic::path(
            "labels",
            &registry_path,
            root,
            "cannot read the registry",
        )];
    };
    let Ok(registry) = serde_json::from_str::<Value>(&registry_text) else {
        return vec![Diagnostic::path(
            "labels",
            &registry_path,
            root,
            "the registry is not valid JSON",
        )];
    };
    let Ok(source) = fs::read_to_string(&rust_path) else {
        return vec![Diagnostic::path(
            "labels",
            &rust_path,
            root,
            "cannot read the Rust label table",
        )];
    };
    let mut diagnostics = Vec::new();
    let Some(codes) = registry.get("codes") else {
        return vec![Diagnostic::path(
            "labels",
            &registry_path,
            root,
            "the registry has no codes object",
        )];
    };
    for (function, key) in [
        ("event_type_label", "event_types"),
        ("source_label", "sources"),
        ("stage_label", "stages"),
        ("lifecycle_label", "lifecycles"),
        ("scale_label", "scales"),
        ("target_basis_label", "target_bases"),
    ] {
        let expected = code_map(codes, key);
        let actual = scalar_arms(&source, function);
        compare(
            root,
            &rust_path,
            function,
            &expected,
            &actual,
            &mut diagnostics,
        );
    }
    let expected_events: BTreeMap<u64, String> = registry
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|event| {
            Some((
                event.get("code")?.as_u64()?,
                event.get("id")?.as_str()?.to_owned(),
            ))
        })
        .collect();
    let actual_events = scalar_arms(&source, "event_name");
    compare(
        root,
        &rust_path,
        "event_name",
        &expected_events,
        &actual_events,
        &mut diagnostics,
    );
    let mut expected_subtypes = BTreeMap::new();
    for (type_code, key) in [
        (1u64, "level_subtypes"),
        (2, "divergence_subtypes"),
        (3, "signal_subtypes"),
    ] {
        for (subtype, label) in code_map(codes, key) {
            expected_subtypes.insert((type_code, subtype), label);
        }
    }
    let actual_subtypes = pair_arms(&source, "event_subtype_label");
    compare_pairs(
        root,
        &rust_path,
        "event_subtype_label",
        &expected_subtypes,
        &actual_subtypes,
        &mut diagnostics,
    );
    diagnostics
}

fn code_map(codes: &Value, key: &str) -> BTreeMap<u64, String> {
    codes
        .get(key)
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter_map(|(code, label)| Some((code.parse().ok()?, label.as_str()?.to_owned())))
        .collect()
}

fn scalar_arms(source: &str, name: &str) -> BTreeMap<u64, String> {
    let mut arms = BTreeMap::new();
    let Some(block) = function_block(source, name) else {
        return arms;
    };
    for line in block.lines() {
        let Some((left, right)) = line.split_once("=>") else {
            continue;
        };
        if left.contains('(') {
            continue;
        }
        let Some(code) = left.trim().parse::<u64>().ok() else {
            continue;
        };
        if let Some(label) = quoted(right) {
            arms.insert(code, label);
        }
    }
    arms
}

fn pair_arms(source: &str, name: &str) -> BTreeMap<(u64, u64), String> {
    let mut arms = BTreeMap::new();
    let Some(block) = function_block(source, name) else {
        return arms;
    };
    for line in block.lines() {
        let Some((left, right)) = line.split_once("=>") else {
            continue;
        };
        let left = left.trim().trim_start_matches('(').trim_end_matches(')');
        let Some((first, second)) = left.split_once(',') else {
            continue;
        };
        let (Ok(first), Ok(second)) = (first.trim().parse::<u64>(), second.trim().parse::<u64>())
        else {
            continue;
        };
        if let Some(label) = quoted(right) {
            arms.insert((first, second), label);
        }
    }
    arms
}

fn function_block<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    let start = source.find(&format!("pub fn {name}("))?;
    let end = source[start..].find("\n}")? + start;
    Some(&source[start..end])
}

fn quoted(value: &str) -> Option<String> {
    let start = value.find('"')? + 1;
    let end = value[start..].find('"')? + start;
    Some(value[start..end].to_owned())
}

fn compare(
    root: &Path,
    path: &Path,
    name: &str,
    expected: &BTreeMap<u64, String>,
    actual: &BTreeMap<u64, String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (code, label) in expected {
        match actual.get(code) {
            None => diagnostics.push(Diagnostic::path(
                "labels",
                path,
                root,
                format!("{name}{code} is missing"),
            )),
            Some(actual) if actual != label => diagnostics.push(Diagnostic::path(
                "labels",
                path,
                root,
                format!("{name}{code} is {actual:?}; the registry says {label:?}"),
            )),
            Some(_) => {}
        }
    }
    for code in actual.keys().filter(|code| !expected.contains_key(code)) {
        diagnostics.push(Diagnostic::path(
            "labels",
            path,
            root,
            format!("{name}{code} is not in the registry"),
        ));
    }
}

fn compare_pairs(
    root: &Path,
    path: &Path,
    name: &str,
    expected: &BTreeMap<(u64, u64), String>,
    actual: &BTreeMap<(u64, u64), String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (code, label) in expected {
        match actual.get(code) {
            None => diagnostics.push(Diagnostic::path(
                "labels",
                path,
                root,
                format!("{name}{code:?} is missing"),
            )),
            Some(actual) if actual != label => diagnostics.push(Diagnostic::path(
                "labels",
                path,
                root,
                format!("{name}{code:?} is {actual:?}; the registry says {label:?}"),
            )),
            Some(_) => {}
        }
    }
    for code in actual.keys().filter(|code| !expected.contains_key(code)) {
        diagnostics.push(Diagnostic::path(
            "labels",
            path,
            root,
            format!("{name}{code:?} is not in the registry"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn empty_root(name: &str) -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "qr-labels-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        root
    }

    #[test]
    fn the_repository_table_matches_the_registry() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(check(&root).is_empty(), "{:?}", check(&root));
    }

    #[test]
    fn a_missing_registry_is_reported() {
        let root = empty_root("registry");
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("cannot read the registry"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
