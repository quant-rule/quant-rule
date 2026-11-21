//! Fail when a retired model, strategy, architecture label, or tool path
//! appears in a current artifact.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::report::Diagnostic;

const SCAN: [&str; 13] = [
    "AGENTS.md",
    "README.md",
    "docs/README.md",
    "docs/architecture",
    "docs/workflows",
    "docs/tutorials",
    "docs/governance",
    "docs/model/2026-11-05",
    "docs/config",
    "config/defaults",
    "config/examples",
    "config/research",
    "crates",
];
const ALLOWLIST: [&str; 9] = [
    "docs/registry/history.md",
    "crates/quantrule-shell/tests/architecture.rs",
    "crates/quantrule-app/src/config/tests.rs",
    "crates/quantrule-shell/tests/fixtures/config-run/model-v3.json",
    "crates/quantrule-evidence-service/migrations/0016_research_ledger.sql",
    "crates/quantrule-evidence-service/migrations/sqlite/0004_research_ledger.sql",
    "docs/workflows/deprecation-retirement.md",
    "docs/workflows/release-operations.md",
    "crates/quantrule-governance/src/checks/legacy.rs",
];
const PENDING: [&str; 1] = ["config/research/"];
const TOKENS: [&str; 33] = [
    "model v3",
    "model v4",
    "quantrule.model/v1",
    "quantrule.model/v2",
    "quantrule.model/v3",
    "strategy v2",
    "strategy v3",
    "strategy v4",
    "strategy v5",
    "strategy v6",
    "strategy v7",
    "strategy v8",
    "strategy v9",
    "strategy v10",
    "quantrule.strategy/v2",
    "quantrule.strategy/v3",
    "quantrule.strategy/v4",
    "quantrule.strategy/v5",
    "quantrule.strategy/v6",
    "quantrule.strategy/v7",
    "quantrule.strategy/v8",
    "v5 programme",
    "quantrule_analytics",
    "quantrule-analytics",
    "quantrule_model::axes",
    "crate::axes",
    "crate::dashboard",
    "crate::consensus",
    "MarketReport",
    "DashboardState",
    "TimeframeConsensus",
    "AxisContext",
    "AgreementCode",
];
const ARCH_TOKENS: [&str; 12] = [
    "download page",
    "static download",
    "Linux installer",
    "apps/desktop/src-tauri",
    "Tauri IPC",
    "Tauri desktop application",
    "five services",
    "eight containers",
    "local only",
    "single process",
    "shell-only fetch",
    "SQLite memory",
];
const RETIRED_PATHS: [&str; 4] = [
    "tools/simulation",
    "tools/charts",
    "tools/oracle",
    "tools/trades",
];
const RETIRED_ALLOWLIST: [&str; 7] = [
    "CHANGELOG.md",
    "docs/port-parity/registry.json",
    "docs/port-parity/PORT-001.golden.json",
    "docs/registry/history.md",
    "tools/check_legacy.py",
    "tools/test_check_legacy.py",
    "crates/quantrule-governance/src/checks/legacy.rs",
];
const PATH_HISTORY: [&str; 8] = [
    "docs/adr/",
    "docs/audits/",
    "docs/ai-transparency-log/",
    "docs/execution-plans/",
    "docs/reports/",
    "docs/releases/",
    "docs/improvement/",
    "docs/course/",
];
const PATH_SKIP_PARTS: [&str; 4] = ["node_modules", "__pycache__", ".venv", "venv"];

/// Scan labels and retired tool paths.
#[must_use]
pub fn check(root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut label_files = Vec::new();
    for entry in SCAN {
        let path = root.join(entry);
        if path.is_file() {
            label_files.push(path);
        } else if path.is_dir() {
            let mut found = Vec::new();
            collect(&path, &mut found);
            label_files.extend(found);
        }
    }
    label_files.sort();
    label_files.dedup();
    for path in label_files {
        let name = relative(&path, root);
        if ALLOWLIST.contains(&name.as_str())
            || PENDING.iter().any(|prefix| name.starts_with(prefix))
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if let Some(token) = TOKENS
                .iter()
                .find(|token| line.to_lowercase().contains(&token.to_lowercase()))
            {
                diagnostics.push(Diagnostic::line(
                    "legacy",
                    &path,
                    root,
                    index + 1,
                    format!("retired label {token:?}"),
                ));
                continue;
            }
            if let Some(token) = ARCH_TOKENS
                .iter()
                .find(|token| line.to_lowercase().contains(&token.to_lowercase()))
            {
                diagnostics.push(Diagnostic::line(
                    "legacy",
                    &path,
                    root,
                    index + 1,
                    format!("retired architecture term {token:?}"),
                ));
            }
        }
    }
    let current = current_files(root);
    for path in current {
        let name = relative(&path, root);
        if RETIRED_ALLOWLIST.contains(&name.as_str()) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            if let Some(token) = RETIRED_PATHS
                .iter()
                .find(|token| contains_path(line, token))
            {
                diagnostics.push(Diagnostic::line(
                    "legacy",
                    &path,
                    root,
                    index + 1,
                    format!("retired tool path {token:?}"),
                ));
            }
        }
    }
    diagnostics
}

fn collect(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else {
            found.push(path);
        }
    }
}

fn candidate_paths(root: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "--full-name",
            "-z",
        ])
        .output();
    let Ok(output) = output else {
        let mut found = Vec::new();
        collect(root, &mut found);
        return found;
    };
    if !output.status.success() {
        let mut found = Vec::new();
        collect(root, &mut found);
        return found;
    }
    String::from_utf8_lossy(&output.stdout)
        .split('\0')
        .filter(|name| !name.is_empty())
        .map(|name| root.join(name))
        .collect()
}

fn current_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for path in candidate_paths(root) {
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let name = relative.to_string_lossy().replace('\\', "/");
        if relative.components().next().is_some_and(|part| {
            part.as_os_str() == "data" || part.as_os_str() == ".git" || part.as_os_str() == "target"
        }) {
            continue;
        }
        if relative
            .components()
            .any(|part| PATH_SKIP_PARTS.contains(&part.as_os_str().to_string_lossy().as_ref()))
        {
            continue;
        }
        if PATH_HISTORY.iter().any(|prefix| name.starts_with(prefix)) {
            continue;
        }
        if !path.is_file() {
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
        found.push(path);
    }
    found.sort();
    found
}

fn contains_path(line: &str, path: &str) -> bool {
    let mut start = 0;
    while let Some(index) = line[start..].find(path) {
        let absolute = start + index;
        let before = line[..absolute].chars().next_back();
        let after = line[absolute + path.len()..].chars().next();
        let before_ok = before.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_' || ch == '-'));
        let after_ok = after.is_none_or(|ch| !(ch.is_alphanumeric() || ch == '_' || ch == '-'));
        if before_ok && after_ok {
            return true;
        }
        start = absolute + path.len();
    }
    false
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

    fn fixture(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("qr-legacy-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("crates")).expect("tree");
        root
    }

    #[test]
    fn a_retired_label_is_reported() {
        let root = fixture("label");
        fs::write(root.join("crates/example.rs"), "// model v3\n").expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("retired label"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn a_retired_tool_path_is_reported() {
        let root = fixture("path");
        fs::write(root.join("README.md"), "run tools/simulation\n").expect("fixture");
        let diagnostics = check(&root);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("retired tool path"))
        );
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn an_allowlisted_label_file_is_skipped() {
        let root = fixture("allow");
        let allowlisted = root.join("crates/quantrule-app/src/config/tests.rs");
        fs::create_dir_all(allowlisted.parent().expect("parent")).expect("tree");
        fs::write(allowlisted, "model v3\n").expect("fixture");
        let diagnostics = check(&root);
        assert!(diagnostics.is_empty());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
