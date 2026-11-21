//! Check that the generated verification matrices cover the active registry.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::report::Diagnostic;

/// Check the indicator and event matrices.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let registry_path = root.join("docs/registry/indicators.json");
    let verification_path = root.join("docs/registry/verification.json");
    let indicators_path = root.join("docs/model/2026-11-06/indicator-verification.md");
    let events_path = root.join("docs/model/2026-11-06/event-verification.md");
    let (registry, verification) = match (load(&registry_path), load(&verification_path)) {
        (Ok(registry), Ok(verification)) => (registry, verification),
        _ => {
            return vec![Diagnostic::path(
                "verification-matrix",
                &registry_path,
                root,
                "cannot read the registry or verification manifest",
            )];
        }
    };
    let expected_indicators: Vec<String> = verification
        .get("indicators")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect();
    let expected_events: Vec<String> = verification
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect();
    let Registry {
        indicators: registry_indicators,
        events: registry_events,
    } = active_ids(&registry);
    let indicator_ids = matrix_ids(&indicators_path, true);
    let event_ids = matrix_ids(&events_path, false);
    let mut diagnostics = Vec::new();
    if expected_indicators.iter().cloned().collect::<BTreeSet<_>>() != registry_indicators {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            &verification_path,
            root,
            "the verification manifest and the active indicator registry differ",
        ));
    }
    if expected_events.iter().cloned().collect::<BTreeSet<_>>() != registry_events {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            &verification_path,
            root,
            "the verification manifest and the active event registry differ",
        ));
    }
    compare_matrix(
        root,
        &indicators_path,
        "indicator",
        &expected_indicators,
        &indicator_ids,
        &mut diagnostics,
    );
    compare_matrix(
        root,
        &events_path,
        "event",
        &expected_events,
        &event_ids,
        &mut diagnostics,
    );
    diagnostics
}

struct Registry {
    indicators: BTreeSet<String>,
    events: BTreeSet<String>,
}

fn active_ids(registry: &Value) -> Registry {
    let indicators = registry
        .get("indicators")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("active"))
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect();
    let events = registry
        .get("events")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("active"))
        .filter_map(|item| item.get("id")?.as_str().map(str::to_owned))
        .collect();
    Registry { indicators, events }
}

fn matrix_ids(path: &Path, indicator: bool) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let cells: Vec<_> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();
            if indicator {
                if cells.len() >= 2 && cells[0].parse::<usize>().is_ok() {
                    cells[1]
                        .strip_prefix('`')?
                        .strip_suffix('`')
                        .map(str::to_owned)
                } else {
                    None
                }
            } else if cells.len() >= 2 {
                cells[0]
                    .strip_prefix('`')?
                    .strip_suffix('`')
                    .map(str::to_owned)
            } else {
                None
            }
        })
        .collect()
}

fn compare_matrix(
    root: &Path,
    path: &Path,
    label: &str,
    expected: &[String],
    observed: &[String],
    diagnostics: &mut Vec<Diagnostic>,
) {
    if observed.len() != expected.len() {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            path,
            root,
            format!(
                "{label} matrix has {} rows, expected {}",
                observed.len(),
                expected.len()
            ),
        ));
    }
    let observed_set: BTreeSet<_> = observed.iter().cloned().collect();
    if observed_set.len() != observed.len() {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            path,
            root,
            format!("{label} matrix has duplicate rows"),
        ));
    }
    let expected_set: BTreeSet<_> = expected.iter().cloned().collect();
    let missing: Vec<_> = expected_set
        .difference(&observed_set)
        .take(10)
        .cloned()
        .collect();
    if !missing.is_empty() {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            path,
            root,
            format!("{label} matrix is missing: {}", missing.join(", ")),
        ));
    }
    let extra: Vec<_> = observed_set
        .difference(&expected_set)
        .take(10)
        .cloned()
        .collect();
    if !extra.is_empty() {
        diagnostics.push(Diagnostic::path(
            "verification-matrix",
            path,
            root,
            format!("{label} matrix has unknown rows: {}", extra.join(", ")),
        ));
    }
}

fn load(path: &Path) -> Result<Value, ()> {
    let text = fs::read_to_string(path).map_err(|_| ())?;
    serde_json::from_str(&text).map_err(|_| ())
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
            "qr-verification-matrix-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        root
    }

    #[test]
    fn a_missing_registry_is_reported() {
        let root = fixture("missing");
        assert!(
            check(&root)
                .iter()
                .any(|d| d.message.contains("cannot read the registry"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
