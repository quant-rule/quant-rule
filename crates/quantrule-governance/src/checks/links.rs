//! Check that every relative Markdown link resolves inside the repository.

use std::fs;
use std::path::{Path, PathBuf};

use crate::report::Diagnostic;

const EXCLUDED: [&str; 4] = [".git", "node_modules", "target", "data"];

/// Check every Markdown file.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut files = Vec::new();
    collect_markdown(root, root, &mut files);
    files.sort();
    let mut diagnostics = Vec::new();
    for path in files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let mut fenced = false;
        for (index, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fenced = !fenced;
                continue;
            }
            if fenced {
                continue;
            }
            let line = strip_inline_code(line);
            for target in link_targets(&line) {
                let Some(resolved) = resolve(root, &path, target) else {
                    continue;
                };
                if !resolved.exists() {
                    diagnostics.push(Diagnostic::line(
                        "links",
                        &path,
                        root,
                        index + 1,
                        format!("broken link {target:?} -> {}", relative(&resolved, root)),
                    ));
                }
            }
        }
    }
    diagnostics
}

fn collect_markdown(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if !EXCLUDED.contains(&name) {
                collect_markdown(root, &path, found);
            }
        } else if path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
    let _ = root;
}

fn strip_inline_code(line: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    for ch in line.chars() {
        if ch == '`' {
            in_code = !in_code;
        } else if !in_code {
            out.push(ch);
        }
    }
    out
}

fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else {
            break;
        };
        let target = after[..end].split_whitespace().next().unwrap_or("").trim();
        if !target.is_empty() {
            targets.push(target);
        }
        rest = &after[end + 1..];
    }
    targets
}

fn resolve(root: &Path, source: &Path, target: &str) -> Option<PathBuf> {
    let target = target.split(['#', '?']).next().unwrap_or("");
    if target.is_empty()
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
        || target.starts_with("tel:")
    {
        return None;
    }
    let target = percent_decode(target);
    if target.starts_with('/') {
        Some(root.join(target.trim_start_matches('/')))
    } else {
        Some(source.parent().unwrap_or(root).join(target))
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            out.push(high * 16 + low);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_relative_link_is_reported() {
        let root = std::env::temp_dir().join(format!("qr-links-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("tree");
        fs::write(root.join("a.md"), "[missing](missing.md)\n").expect("fixture");
        let diagnostics = check(&root);
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("missing.md"));
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
