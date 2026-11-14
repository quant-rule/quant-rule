//! The signal path uses only deterministic arithmetic.
//!
//! A transcendental or fused-multiply-add call may differ between CPUs, so the
//! source guard reads the engine modules and fails on a forbidden method. The
//! seeded tests prove the guard catches `powf` and `mul_add`.

use std::path::{Path, PathBuf};

/// The operations that are not part of the fixed numeric contract.
const FORBIDDEN: [&str; 18] = [
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "exp", "exp2", "ln", "log2", "log10",
    "powf", "powi", "cbrt", "hypot", "sin_cos", "mul_add",
];

fn signal_roots() -> Vec<PathBuf> {
    let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the server crate sits under crates")
        .to_path_buf();
    vec![
        crates.join("quantrule-data/src"),
        crates.join("quantrule-model/src"),
        crates.join("quantrule-backtest/src/strategy"),
        crates.join("quantrule-backtest/src/execution"),
        crates.join("quantrule-backtest/src/portfolio"),
        crates.join("quantrule-backtest/src/intent.rs"),
        crates.join("quantrule-backtest/src/side.rs"),
    ]
}

fn rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path.to_path_buf());
        }
        return;
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(path)
        .expect("read the engine source directory")
        .map(|entry| entry.expect("read one entry").path())
        .collect();
    entries.sort();
    for entry in entries {
        if entry.is_dir() {
            rust_files(&entry, files);
        } else if entry.extension().is_some_and(|extension| extension == "rs") {
            files.push(entry);
        }
    }
}

fn is_test_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name == "tests.rs" || name.ends_with("_tests.rs")
}

/// Test modules live at the bottom of a source file by convention, and their
/// generators may use transcendental math to make a fixture interesting. The
/// guard reads only the production prefix.
fn code_before_tests(text: &str) -> &str {
    text.split("#[cfg(test)]").next().unwrap_or(text)
}

fn scan_source(path: &Path) -> Vec<String> {
    let text = std::fs::read_to_string(path).expect("read one engine source file");
    let mut hits = Vec::new();
    for (index, line) in code_before_tests(&text).lines().enumerate() {
        let code = line.split("//").next().unwrap_or_default();
        for name in FORBIDDEN {
            if code.contains(&format!(".{name}(")) || code.contains(&format!("::{name}(")) {
                hits.push(format!("{}:{}: {name}", path.display(), index + 1));
            }
        }
    }
    hits
}

fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "quantrule-signal-math-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create the scratch directory");
    directory
}

#[test]
fn the_signal_path_uses_only_deterministic_arithmetic() {
    let roots = signal_roots();
    let mut files = Vec::new();
    for root in &roots {
        assert!(root.exists(), "missing signal-path root {}", root.display());
        rust_files(root, &mut files);
    }
    assert!(!files.is_empty(), "the signal path has no source files");

    let mut violations = Vec::new();
    for file in files {
        if !is_test_file(&file) {
            violations.extend(scan_source(&file));
        }
    }
    assert!(
        violations.is_empty(),
        "forbidden math in the signal path:\n{}",
        violations.join("\n")
    );
}

#[test]
fn a_seeded_powf_or_mul_add_fails_the_guard() {
    let directory = scratch("forbidden");
    let file = directory.join("seeded.rs");
    std::fs::write(
        &file,
        "fn seeded(x: f64) -> f64 {\n    let a = x.powf(2.0);\n    x.mul_add(2.0, a)\n}\n",
    )
    .expect("write the seeded source");
    let hits = scan_source(&file);
    assert!(
        hits.iter().any(|hit| hit.contains("powf")),
        "the guard missed powf: {hits:?}"
    );
    assert!(
        hits.iter().any(|hit| hit.contains("mul_add")),
        "the guard missed mul_add: {hits:?}"
    );
    std::fs::remove_dir_all(directory).expect("remove the scratch directory");
}

#[test]
fn the_allowlisted_arithmetic_passes() {
    let directory = scratch("allowed");
    let file = directory.join("clean.rs");
    std::fs::write(
        &file,
        "fn clean(x: f64) -> f64 {\n    (((x + 1.0) - 2.0) * 3.0 / 4.0)\n        .abs()\n        .sqrt()\n        .round()\n        .floor()\n        .ceil()\n        .trunc()\n}\n",
    )
    .expect("write the clean source");
    assert!(scan_source(&file).is_empty());
    std::fs::remove_dir_all(directory).expect("remove the scratch directory");
}
