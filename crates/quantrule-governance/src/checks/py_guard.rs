//! Fail when the product carries Python.

use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

const PRODUCT_ROOTS: [&str; 3] = ["crates", "apps", "docker"];
const SKIP: [&str; 10] = [
    "node_modules",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "test-results",
    "playwright-report",
    "blob-report",
    ".git",
];
const ALLOWED: [&str; 3] = ["tools/", "docs/", "data/"];
const REFERENCES: [&str; 3] = [
    "docker/README.md",
    "docker/check-no-python.sh",
    "apps/web/src/lib/labels.generated.ts",
];

/// Check the product tree for `.py` files and interpreter references.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut python_files = Vec::new();
    collect_python(root, root, &mut python_files);
    python_files.sort();
    for path in python_files {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let name = relative.to_string_lossy().replace('\\', "/");
        if !ALLOWED.iter().any(|prefix| name.starts_with(prefix)) {
            diagnostics.push(Diagnostic::path(
                "py-guard",
                &path,
                root,
                "Python file in the product tree",
            ));
        }
    }
    let mut product_files = Vec::new();
    for top in PRODUCT_ROOTS {
        collect_files(&root.join(top), &mut product_files);
    }
    let dockerfile = root.join("Dockerfile");
    if dockerfile.is_file() {
        product_files.push(dockerfile);
    }
    product_files.sort();
    for path in product_files {
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if REFERENCES.contains(&name.as_str()) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        if metadata.len() > 2 * 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        if bytes[..bytes.len().min(4096)].contains(&0) {
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if let Some(found) = interpreter_reference(line) {
                diagnostics.push(Diagnostic::line(
                    "py-guard",
                    &path,
                    root,
                    index + 1,
                    format!("Python interpreter reference {found:?}"),
                ));
            }
        }
    }
    diagnostics
}

fn collect_python(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !SKIP.contains(&name) && !(directory == root && name == "data") {
                collect_python(root, &path, found);
            }
        } else if name.ends_with(".py") {
            found.push(path);
        }
    }
}

fn collect_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if !SKIP.contains(&name) {
                collect_files(&path, found);
            }
        } else {
            found.push(path);
        }
    }
}

fn interpreter_reference(line: &str) -> Option<String> {
    let needle = ["py", "thon"].concat();
    let mut start = 0;
    while let Some(index) = line[start..].find(&needle) {
        let absolute = start + index;
        let before = line[..absolute].chars().next_back();
        if before
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '/' | '-'))
        {
            start = absolute + 6;
            continue;
        }
        let mut end = absolute + 6;
        if line[end..].starts_with('3') {
            end += 1;
        }
        if line[end..].starts_with('.') {
            let digits_end = line[end + 1..]
                .find(|ch: char| !ch.is_ascii_digit())
                .map_or(line.len(), |offset| end + 1 + offset);
            if digits_end > end + 1 {
                end = digits_end;
            }
        }
        let after = line[end..].chars().next();
        if after.is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_')) {
            return Some(line[absolute..end].to_owned());
        }
        start = absolute + 6;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_python_file_in_the_product_tree_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-py-guard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates/demo")).expect("tree");
        fs::write(root.join("crates/demo/bad.py"), "print('x')\n").expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Python file"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_python_file_under_tools_is_allowed() {
        let root = std::env::temp_dir().join(format!("qr-py-guard-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("tools")).expect("tree");
        fs::write(root.join("tools/check.py"), "print('x')\n").expect("fixture");
        assert!(check(&root).is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
