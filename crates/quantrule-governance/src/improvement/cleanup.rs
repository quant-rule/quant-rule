//! Report retired workflow terms and legacy run paths in repository text.

use std::fs;
use std::path::{Path, PathBuf};

const BANNED_TERMS: [&str; 3] = [
    "Continuous Improvement",
    "continuous improvement",
    "Governing / Experiment Control Swarm",
];
const LEGACY_PATHS: [&str; 3] = ["data/labs", "data/reports", "data/research"];
const SKIP_DIRECTORIES: [&str; 6] = [
    ".git",
    "target",
    "node_modules",
    "__pycache__",
    ".venv",
    "venv",
];
const MAX_BYTES: u64 = 5 * 1024 * 1024;
const EXCERPT_LIMIT: usize = 160;

/// Options for the read-only cleanup scan.
pub struct Options {
    /// Repository root.
    pub root: PathBuf,
    /// Files or directories to scan.
    pub paths: Vec<String>,
    /// Files or directories to skip.
    pub excludes: Vec<String>,
    /// Check retired terms only.
    pub terms_only: bool,
}

/// Run the cleanup scan and return the process exit code.
#[must_use]
pub fn run(options: &Options) -> u8 {
    let roots: Vec<PathBuf> = options
        .paths
        .iter()
        .map(|path| resolve(&options.root, path))
        .collect();
    let excludes: Vec<PathBuf> = options
        .excludes
        .iter()
        .map(|path| resolve(&options.root, path))
        .collect();
    let mut files = Vec::new();
    for root in roots {
        if !root.exists() {
            eprintln!("error: scan path not found: {}", root.display());
            return 1;
        }
        if root.is_file() {
            if !inside(&root, &excludes) {
                files.push(root);
            }
        } else {
            collect(&root, &excludes, &mut files);
        }
    }
    files.sort_by_key(|path| display(&options.root, path));
    let terms: Vec<&str> = if options.terms_only {
        BANNED_TERMS.to_vec()
    } else {
        BANNED_TERMS
            .iter()
            .chain(LEGACY_PATHS.iter())
            .copied()
            .collect()
    };
    let mut findings = Vec::new();
    let mut skipped = 0;
    for path in &files {
        match scan(path, &terms) {
            Ok(matches) => {
                let name = display(&options.root, path);
                for (number, term, excerpt) in matches {
                    findings.push((name.clone(), number, term, excerpt));
                }
            }
            Err(()) => skipped += 1,
        }
    }
    if findings.is_empty() {
        let suffix = if skipped > 0 {
            format!("; {skipped} binary, large, or unreadable file(s) skipped")
        } else {
            String::new()
        };
        println!("checked {} file(s); no banned terms{suffix}", files.len());
        return 0;
    }
    for (name, number, term, excerpt) in &findings {
        println!("{name}:{number}: {term}: {excerpt}");
    }
    println!("{} finding(s) in {} file(s)", findings.len(), files.len());
    1
}

fn scan(path: &Path, terms: &[&str]) -> Result<Vec<(usize, String, String)>, ()> {
    let metadata = fs::metadata(path).map_err(|_| ())?;
    if metadata.len() > MAX_BYTES {
        return Err(());
    }
    let data = fs::read(path).map_err(|_| ())?;
    if data[..data.len().min(4096)].contains(&0) {
        return Err(());
    }
    let text = String::from_utf8_lossy(&data);
    let mut findings = Vec::new();
    for (number, line) in text.lines().enumerate() {
        if !terms.iter().any(|term| line.contains(term)) {
            continue;
        }
        let mut excerpt = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if excerpt.chars().count() > EXCERPT_LIMIT {
            excerpt = excerpt.chars().take(EXCERPT_LIMIT - 3).collect::<String>() + "...";
        }
        for term in terms {
            if line.contains(term) {
                findings.push((number + 1, (*term).to_owned(), excerpt.clone()));
            }
        }
    }
    Ok(findings)
}

fn collect(directory: &Path, excludes: &[PathBuf], found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    for path in paths {
        if inside(&path, excludes) {
            continue;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            if !SKIP_DIRECTORIES.contains(&name.as_str()) {
                collect(&path, excludes, found);
            }
        } else {
            found.push(path);
        }
    }
}

fn inside(path: &Path, excludes: &[PathBuf]) -> bool {
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    excludes
        .iter()
        .any(|exclude| path == *exclude || path.starts_with(exclude))
}

fn resolve(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    path.canonicalize().unwrap_or(path)
}

fn display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn root(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("qr-cleanup-{name}-{}-{unique}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("tree");
        path
    }

    #[test]
    fn a_banned_term_is_found() {
        let root = root("term");
        fs::write(root.join("README.md"), "Continuous Improvement\n").expect("file");
        let options = Options {
            root: root.clone(),
            paths: vec!["README.md".to_owned()],
            excludes: Vec::new(),
            terms_only: false,
        };
        assert_eq!(run(&options), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_legacy_path_is_found() {
        let root = root("path");
        fs::create_dir_all(root.join("data")).expect("data");
        fs::write(root.join("data/notes.md"), "see data/labs\n").expect("file");
        let options = Options {
            root: root.clone(),
            paths: vec!["data".to_owned()],
            excludes: Vec::new(),
            terms_only: false,
        };
        assert_eq!(run(&options), 1);
        let _ = fs::remove_dir_all(root);
    }
}
