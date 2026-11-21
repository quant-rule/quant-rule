//! Check the structural rules that the Refactor Workflow depends on.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::report::Diagnostic;

const ENGINE_CRATES: [&str; 3] = ["quantrule-data", "quantrule-model", "quantrule-backtest"];
const SHELL_STAGE_DIRS: [&str; 6] = [
    "analytics",
    "backtest",
    "execution",
    "portfolio",
    "qualification",
    "strategy",
];
const FORBIDDEN_ENGINE_MODULES: [&str; 9] = [
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "std::thread",
    "std::sync::Mutex",
    "std::time::SystemTime",
    "std::time::Instant",
    "rand",
];
const FORBIDDEN_RUST_FILES: [&str; 5] = [
    "utils.rs",
    "helpers.rs",
    "common.rs",
    "misc.rs",
    "manager.rs",
];

/// Check every structural rule and return the findings.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut violations = Vec::new();
    check_module_docs(root, &mut violations);
    check_unsafe_forbid(root, &mut violations);
    check_engine_purity(root, &mut violations);
    check_stage_command_coupling(root, &mut violations);
    check_artifact_command_coupling(root, &mut violations);
    check_date_module_coupling(root, &mut violations);
    check_naming(root, &mut violations);
    check_dated_docs(root, &mut violations);
    check_workflow_index(root, &mut violations);
    check_data_allowlist(root, &mut violations);
    violations
}

/// Render the structural scorecard.
#[must_use]
pub fn report(root: &Path) -> String {
    let sources = rust_sources(root);
    let mut metrics: Vec<(String, usize)> = sources
        .iter()
        .map(|path| (relative(root, path), file_lines(path)))
        .collect();
    metrics.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let crates: BTreeSet<String> = sources
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .filter_map(|path| path.components().nth(1))
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    let total: usize = metrics.iter().map(|(_, lines)| lines).sum();
    let mut lines = vec![
        "## Structural report".to_owned(),
        String::new(),
        "| Item | Observed |".to_owned(),
        "|---|---|".to_owned(),
        format!("| Rust source files | {} |", sources.len()),
        format!("| Rust source lines | {total} |"),
        format!("| Engine crates | {} |", ENGINE_CRATES.join(", ")),
        format!("| Workspace crates | {} |", crates.len()),
        String::new(),
        "### Largest Rust source files".to_owned(),
        String::new(),
        "| File | Lines |".to_owned(),
        "|---|---:|".to_owned(),
    ];
    for (path, count) in metrics.iter().take(15) {
        lines.push(format!("| `{path}` | {count} |"));
    }
    let over = |limit: usize| metrics.iter().filter(|(_, count)| *count > limit).count();
    lines.extend([
        String::new(),
        "### Size thresholds".to_owned(),
        String::new(),
        "| Threshold | Files |".to_owned(),
        "|---|---:|".to_owned(),
        format!("| over 600 lines | {} |", over(600)),
        format!("| over 800 lines | {} |", over(800)),
        format!("| over 1,000 lines | {} |", over(1000)),
        String::new(),
        "### Surface and documentation".to_owned(),
        String::new(),
        "| Item | Observed |".to_owned(),
        "|---|---|".to_owned(),
    ]);
    let public: usize = sources
        .iter()
        .map(|path| {
            read(path)
                .lines()
                .filter(|line| line.trim_start().starts_with("pub "))
                .count()
        })
        .sum();
    let crate_public: usize = sources
        .iter()
        .map(|path| {
            read(path)
                .lines()
                .filter(|line| line.trim_start().starts_with("pub(crate)"))
                .count()
        })
        .sum();
    let unwrap: usize = sources
        .iter()
        .filter(|path| !is_test_file(path))
        .map(|path| {
            read(path)
                .lines()
                .filter(|line| line.contains(".unwrap()") || line.contains(".expect("))
                .count()
        })
        .sum();
    let workflow_docs = count_direct_markdown(&root.join("docs/workflows"));
    let refactor_docs = count_files(&root.join("docs/refactors"), "md");
    lines.push(format!("| `pub` items | {public} |"));
    lines.push(format!("| `pub(crate)` items | {crate_public} |"));
    lines.push(format!(
        "| `unwrap`/`expect` lines in non-test source | {unwrap} |"
    ));
    lines.push(format!("| Workflow documents | {workflow_docs} |"));
    lines.push(format!("| Refactor documents | {refactor_docs} |"));
    lines.push(String::new());
    lines.join("\n")
}

fn check_module_docs(root: &Path, violations: &mut Vec<Diagnostic>) {
    for path in rust_sources(root) {
        if is_test_file(&path) {
            continue;
        }
        if !starts_with_module_doc(&path) {
            push(
                violations,
                root,
                &path,
                "module-doc",
                "source module must start with a //! module doc",
            );
        }
    }
}

fn check_unsafe_forbid(root: &Path, violations: &mut Vec<Diagnostic>) {
    let crates = root.join("crates");
    let Ok(entries) = fs::read_dir(&crates) else {
        return;
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("quantrule-"))
        })
        .collect();
    directories.sort();
    for directory in directories {
        let roots = [directory.join("src/lib.rs"), directory.join("src/main.rs")];
        let found: Vec<&PathBuf> = roots.iter().filter(|path| path.exists()).collect();
        if found.is_empty() {
            push(
                violations,
                root,
                &directory,
                "unsafe-forbid",
                "crate has no lib.rs or main.rs",
            );
            continue;
        }
        for path in found {
            let text = read(path);
            if !text.contains("#![forbid(unsafe_code)]") {
                push(
                    violations,
                    root,
                    path,
                    "unsafe-forbid",
                    "crate root must contain #![forbid(unsafe_code)]",
                );
            }
        }
    }
}

fn check_engine_purity(root: &Path, violations: &mut Vec<Diagnostic>) {
    for krate in ENGINE_CRATES {
        let directory = root.join("crates").join(krate).join("src");
        if !directory.exists() {
            push(
                violations,
                root,
                &directory,
                "engine-purity",
                "engine source directory is missing",
            );
            continue;
        }
        for path in rust_files(&directory) {
            if is_test_file(&path) {
                continue;
            }
            let text = read(&path);
            let mut modules = BTreeSet::new();
            for statement in use_statements(&text) {
                for imported in expand_use_tree(&statement, "") {
                    for module in FORBIDDEN_ENGINE_MODULES {
                        if imported == module || imported.starts_with(&format!("{module}::")) {
                            modules.insert(module);
                        }
                    }
                }
            }
            let compact: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
            for module in FORBIDDEN_ENGINE_MODULES {
                let needle = if module == "rand" { "rand::" } else { module };
                if contains_qualified(&compact, needle, module != "std::sync::Mutex") {
                    modules.insert(module);
                }
            }
            let mut findings: Vec<String> = modules
                .into_iter()
                .map(|module| format!("references {module}"))
                .collect();
            if text.contains("static mut ") {
                findings.push("contains 'static mut'".to_owned());
            }
            findings.sort();
            for finding in findings {
                push(
                    violations,
                    root,
                    &path,
                    "engine-purity",
                    &format!("engine source {finding}"),
                );
            }
        }
    }
}

fn check_stage_command_coupling(root: &Path, violations: &mut Vec<Diagnostic>) {
    let shell = root.join("crates/quantrule-shell/src");
    for stage in SHELL_STAGE_DIRS {
        let directory = shell.join(stage);
        if !directory.exists() {
            continue;
        }
        for path in rust_files(&directory) {
            if read(&path).contains("crate::command") {
                push(
                    violations,
                    root,
                    &path,
                    "stage-command-coupling",
                    "stage module must not reference the command layer",
                );
            }
        }
    }
}

fn check_artifact_command_coupling(root: &Path, violations: &mut Vec<Diagnostic>) {
    let shell = root.join("crates/quantrule-shell/src");
    let mut targets = vec![shell.join("request.rs")];
    let model_bundle = shell.join("model_bundle");
    if model_bundle.exists() {
        targets.extend(rust_files(&model_bundle));
    }
    for path in targets {
        if !path.exists() {
            continue;
        }
        if read(&path).contains("crate::command") {
            push(
                violations,
                root,
                &path,
                "artifact-command-coupling",
                "shared artifact module must not import the command layer",
            );
        }
    }
}

fn check_date_module_coupling(root: &Path, violations: &mut Vec<Diagnostic>) {
    let shell = root.join("crates/quantrule-shell/src");
    let source_date = shell.join("source/date.rs");
    if source_date.exists() {
        push(
            violations,
            root,
            &source_date,
            "date-module-coupling",
            "the date module must live at crate::date, not under source",
        );
    }
    let source_mod = shell.join("source/mod.rs");
    if source_mod.exists() {
        let text = read(&source_mod);
        if text.contains("mod date") || text.contains("date::{") {
            push(
                violations,
                root,
                &source_mod,
                "date-module-coupling",
                "source must not declare or re-export the date module",
            );
        }
    }
    for path in rust_files(&shell) {
        let text = read(&path);
        if text.contains("crate::source::date")
            || text.contains("crate::source::DateBounds")
            || text.contains("crate::source::DateRange")
        {
            push(
                violations,
                root,
                &path,
                "date-module-coupling",
                "date types must be imported from crate::date, not crate::source",
            );
        }
    }
}

fn check_naming(root: &Path, violations: &mut Vec<Diagnostic>) {
    for path in rust_sources(root) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !rust_file_name(&name) {
            push(
                violations,
                root,
                &path,
                "rust-file-name",
                "Rust source file name must be snake_case",
            );
        }
        if FORBIDDEN_RUST_FILES.contains(&name.as_str()) {
            push(
                violations,
                root,
                &path,
                "catch-all-module",
                "catch-all module names are not allowed",
            );
        }
    }
    let refactors = root.join("docs/refactors");
    if refactors.exists() {
        for path in markdown_files(&refactors) {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name != "README.md" && !refactor_doc_name(&name) {
                push(
                    violations,
                    root,
                    &path,
                    "refactor-doc-name",
                    "refactor document name must be kebab-case",
                );
            }
        }
    }
}

fn check_dated_docs(root: &Path, violations: &mut Vec<Diagnostic>) {
    let docs = root.join("docs");
    if !docs.exists() {
        return;
    }
    let templates = docs.join("workflows/templates");
    let course = docs.join("course");
    let Ok(entries) = fs::read_dir(&docs) else {
        return;
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    directories.sort();
    for directory in directories {
        let Ok(children) = fs::read_dir(&directory) else {
            continue;
        };
        let mut nested: Vec<PathBuf> = children
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        nested.sort();
        for path in nested {
            if path == templates || path.parent() == Some(course.as_path()) {
                continue;
            }
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !date_directory(&name) {
                push(
                    violations,
                    root,
                    &path,
                    "dated-docs-directory",
                    "second-level docs directory must be a UTC YYYY-MM-DD date",
                );
            }
        }
    }
}

fn check_workflow_index(root: &Path, violations: &mut Vec<Diagnostic>) {
    let workflows = root.join("docs/workflows");
    if !workflows.exists() {
        return;
    }
    let index = workflows.join("README.md");
    let text = read(&index);
    let Ok(entries) = fs::read_dir(&workflows) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    files.sort();
    for path in files {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if name == "README.md" {
            continue;
        }
        if !text.contains(&name) {
            push(
                violations,
                root,
                &path,
                "workflow-index",
                &format!(
                    "workflow document is not listed in {}",
                    relative(root, &index)
                ),
            );
        }
    }
}

fn check_data_allowlist(root: &Path, violations: &mut Vec<Diagnostic>) {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "data"])
        .output();
    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for path in text.lines().filter(|line| !line.is_empty()) {
        if !data_allowed(path) {
            push(
                violations,
                root,
                &root.join(path),
                "data-allowlist",
                "committed data/ file is not a README or partition index",
            );
        }
    }
}

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let crates = root.join("crates");
    let Ok(entries) = fs::read_dir(&crates) else {
        return Vec::new();
    };
    let mut sources = Vec::new();
    for entry in entries.flatten() {
        let directory = entry.path().join("src");
        if directory.is_dir() {
            sources.extend(rust_files(&directory));
        }
    }
    sources.sort();
    sources
}

fn rust_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(directory, "rs", &mut files);
    files.sort();
    files
}

fn markdown_files(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect(directory, "md", &mut files);
    files.sort();
    files
}

fn collect(directory: &Path, extension: &str, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, extension, found);
        } else if path.extension().is_some_and(|value| value == extension) {
            found.push(path);
        }
    }
}

fn count_files(directory: &Path, extension: &str) -> usize {
    let mut files = Vec::new();
    collect(directory, extension, &mut files);
    files.len()
}

fn count_direct_markdown(directory: &Path) -> usize {
    let Ok(entries) = fs::read_dir(directory) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "md")
                && entry.path().is_file()
        })
        .count()
}

fn file_lines(path: &Path) -> usize {
    let text = read(path);
    let mut lines = text.matches('\n').count();
    if !text.ends_with('\n') {
        lines += 1;
    }
    lines
}

fn starts_with_module_doc(path: &Path) -> bool {
    read(path)
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim_start().starts_with("//!"))
}

fn is_test_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    name == "tests.rs"
        || name.ends_with("_tests.rs")
        || path.components().any(|part| part.as_os_str() == "tests")
}

fn use_statements(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut statements = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim();
        let Some(offset) = use_offset(line) else {
            index += 1;
            continue;
        };
        let mut statement = line.to_owned();
        while !statement[offset..].contains(';') && index + 1 < lines.len() {
            index += 1;
            statement.push(' ');
            statement.push_str(lines[index].trim());
        }
        let body = statement[offset..].split(';').next().unwrap_or("");
        statements.push(body.trim().to_owned());
        index += 1;
    }
    statements
}

fn use_offset(line: &str) -> Option<usize> {
    let mut rest = line;
    if let Some(after) = rest.strip_prefix("pub") {
        if after.starts_with('(') {
            let close = after.find(')')?;
            rest = after[close + 1..].trim_start();
        } else {
            rest = after.trim_start();
        }
    }
    let after_use = rest.strip_prefix("use")?;
    if !after_use.starts_with(char::is_whitespace) {
        return None;
    }
    let prefix = line.len() - rest.len();
    Some(prefix + 3 + after_use.len() - after_use.trim_start().len())
}

fn expand_use_tree(item: &str, prefix: &str) -> Vec<String> {
    let item = item.trim();
    if item.is_empty() {
        return Vec::new();
    }
    if !item.contains('{') {
        let leaf = item.split(" as ").next().unwrap_or("").replace(' ', "");
        return vec![format!("{prefix}{leaf}").replace("::self", "")];
    }
    let Some((head, remainder)) = item.split_once('{') else {
        return Vec::new();
    };
    let mut depth = 1;
    let mut index = 0;
    let bytes: Vec<char> = remainder.chars().collect();
    while index < bytes.len() && depth > 0 {
        match bytes[index] {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        index += 1;
    }
    if depth != 0 || index == 0 {
        return Vec::new();
    }
    let inner: String = bytes[..index - 1].iter().collect();
    let mut paths = Vec::new();
    let prefix = format!("{prefix}{}", head.trim().replace(' ', ""));
    for part in split_use_items(&inner) {
        paths.extend(expand_use_tree(&part, &prefix));
    }
    paths
}

fn split_use_items(body: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut depth = 0;
    let mut current = String::new();
    for character in body.chars() {
        if character == ',' && depth == 0 {
            let item = current.trim().to_owned();
            if !item.is_empty() {
                items.push(item);
            }
            current.clear();
            continue;
        }
        match character {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        current.push(character);
    }
    let item = current.trim().to_owned();
    if !item.is_empty() {
        items.push(item);
    }
    items
}

fn contains_qualified(text: &str, needle: &str, trailing_boundary: bool) -> bool {
    let mut start = 0;
    while let Some(index) = text[start..].find(needle) {
        let absolute = start + index;
        let before = text[..absolute].chars().next_back();
        let after = text[absolute + needle.len()..].chars().next();
        let before_ok = before.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        let after_ok =
            !trailing_boundary || after.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_'));
        if before_ok && after_ok {
            return true;
        }
        start = absolute + needle.len();
    }
    false
}

fn rust_file_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".rs") else {
        return false;
    };
    !stem.is_empty()
        && stem
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

fn refactor_doc_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    !stem.is_empty()
        && stem
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
}

fn date_directory(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn data_allowed(path: &str) -> bool {
    if path == "data/README.md" || path == "data/partitions.json" {
        return true;
    }
    if path == "data/improvement/index.json" || path == "data/improvement/index.md" {
        return true;
    }
    let Some(rest) = path.strip_prefix("data/") else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    parts.len() == 2 && !parts[0].is_empty() && parts[1] == "README.md"
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn push(violations: &mut Vec<Diagnostic>, root: &Path, path: &Path, rule: &str, message: &str) {
    violations.push(Diagnostic::path(
        "structure",
        path,
        root,
        format!("{rule}: {message}"),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bad_rust_file_name_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-structure-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/demo/src")).expect("tree");
        fs::write(root.join("crates/demo/src/BadName.rs"), "//! demo\n").expect("fixture");
        fs::write(
            root.join("crates/demo/src/lib.rs"),
            "//! demo\n#![forbid(unsafe_code)]\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("rust-file-name"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_missing_module_doc_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-structure-doc-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/demo/src")).expect("tree");
        fs::write(
            root.join("crates/demo/src/lib.rs"),
            "#![forbid(unsafe_code)]\n",
        )
        .expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("module-doc"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
