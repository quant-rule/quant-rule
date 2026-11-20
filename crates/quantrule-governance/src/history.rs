//! Repository evidence history.
//!
//! The command lists historical config and documentation records, archives a
//! selected set with a verified manifest, and erases only a manifest that still
//! matches the source bytes. It never touches runtime records or a service.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::manifest::{FileRecord, Manifest};

const USAGE: &str = "\
usage:
  qr-governance history list --kind <config|docs|records> [--before <YYYY-MM-DD>] [--include-undated] [--root <path>]
  qr-governance history archive --kind <kind> --out <dir> (--before <date> | --all) [--include-undated] [--dry-run] [--root <path>] [--created-unix <n>]
  qr-governance history erase --manifest <file> --confirm <archive_id> [--dry-run] [--root <path>]";

/// Run the `history` command family.
///
/// # Errors
///
/// Returns a usage, filesystem, or manifest error. The caller prints it and
/// exits non-zero.
pub fn run(args: &[String]) -> Result<(), String> {
    let (command, options) = parse(args)?;
    match command.as_str() {
        "list" => list(&options),
        "archive" => archive(&options),
        "erase" => erase(&options),
        other => Err(format!("unknown history command '{other}'\n{USAGE}")),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Config,
    Docs,
    Records,
}

impl Kind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "config" => Ok(Self::Config),
            "docs" => Ok(Self::Docs),
            "records" => Ok(Self::Records),
            other => Err(format!(
                "unknown kind '{other}'; use config, docs, or records"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Docs => "docs",
            Self::Records => "records",
        }
    }

    fn roots(self, root: &Path) -> Vec<PathBuf> {
        match self {
            Self::Config => {
                let mut roots = Vec::new();
                let research = root.join("config/research");
                if research.is_dir() {
                    roots.push(research);
                }
                if let Ok(entries) = fs::read_dir(root.join("config")) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if entry.path().is_dir()
                            && (name.starts_with("sr") || name == "sbe-01")
                            && name != "research"
                        {
                            roots.push(entry.path());
                        }
                    }
                }
                roots.sort();
                roots
            }
            Self::Docs => vec![
                root.join("docs/execution-plans"),
                root.join("docs/ai-transparency-log"),
                root.join("docs/audits"),
                root.join("docs/refactors"),
            ],
            Self::Records => vec![
                root.join("docs/improvement"),
                root.join("docs/reports"),
                root.join("docs/releases"),
            ],
        }
    }
}

#[derive(Debug)]
struct Options {
    kind: Option<Kind>,
    root: PathBuf,
    before: Option<String>,
    include_undated: bool,
    out: Option<PathBuf>,
    dry_run: bool,
    all: bool,
    manifest: Option<PathBuf>,
    confirm: Option<String>,
    created_unix: Option<u64>,
}

impl Options {
    fn root(&self) -> &Path {
        &self.root
    }

    fn kind(&self) -> Result<Kind, String> {
        self.kind
            .ok_or_else(|| format!("--kind is required\n{USAGE}"))
    }

    fn before(&self) -> Option<&str> {
        self.before.as_deref()
    }
}

fn parse(args: &[String]) -> Result<(String, Options), String> {
    if args.first().map(String::as_str) != Some("history") {
        return Err(USAGE.to_owned());
    }
    let command = args
        .get(1)
        .cloned()
        .ok_or_else(|| format!("history has no command\n{USAGE}"))?;
    let mut options = Options {
        kind: None,
        root: std::env::current_dir()
            .map_err(|error| format!("cannot read the current directory: {error}"))?,
        before: None,
        include_undated: false,
        out: None,
        dry_run: false,
        all: false,
        manifest: None,
        confirm: None,
        created_unix: None,
    };
    let mut index = 2;
    while index < args.len() {
        match args[index].as_str() {
            "--kind" => {
                let value = next(args, &mut index, "--kind")?;
                options.kind = Some(Kind::parse(&value)?);
            }
            "--root" => {
                let value = next(args, &mut index, "--root")?;
                options.root = PathBuf::from(value);
            }
            "--before" => {
                let value = next(args, &mut index, "--before")?;
                if !is_date(&value) {
                    return Err(format!("--before expects YYYY-MM-DD, got '{value}'"));
                }
                options.before = Some(value);
            }
            "--include-undated" => options.include_undated = true,
            "--out" => {
                let value = next(args, &mut index, "--out")?;
                options.out = Some(PathBuf::from(value));
            }
            "--dry-run" => options.dry_run = true,
            "--all" => options.all = true,
            "--manifest" => {
                let value = next(args, &mut index, "--manifest")?;
                options.manifest = Some(PathBuf::from(value));
            }
            "--confirm" => options.confirm = Some(next(args, &mut index, "--confirm")?),
            "--created-unix" => {
                let value = next(args, &mut index, "--created-unix")?;
                options.created_unix =
                    Some(value.parse().map_err(|_| {
                        format!("--created-unix expects an integer, got '{value}'")
                    })?);
            }
            other => return Err(format!("unknown option '{other}'\n{USAGE}")),
        }
        index += 1;
    }
    match command.as_str() {
        "list" | "archive" => {
            let _ = options.kind()?;
        }
        "erase" if options.manifest.is_none() || options.confirm.is_none() => {
            return Err(format!("erase requires --manifest and --confirm\n{USAGE}"));
        }
        "erase" => {}
        _ => {}
    }
    Ok((command, options))
}

fn next(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} requires a value\n{USAGE}"))
}

#[derive(Debug)]
struct Entry {
    path: String,
    bytes: u64,
    date: Option<String>,
}

fn list(options: &Options) -> Result<(), String> {
    let kind = options.kind()?;
    let entries = select(kind, options)?;
    let bytes: u64 = entries.iter().map(|entry| entry.bytes).sum();
    println!(
        "{}: {} files, {} bytes",
        kind.as_str(),
        entries.len(),
        bytes
    );
    for entry in entries {
        let date = entry.date.unwrap_or_else(|| String::from("undated"));
        println!("{}\t{}\t{}", entry.path, entry.bytes, date);
    }
    Ok(())
}

fn archive(options: &Options) -> Result<(), String> {
    let kind = options.kind()?;
    let out = options
        .out
        .as_ref()
        .ok_or_else(|| format!("archive requires --out\n{USAGE}"))?;
    if options.before().is_none() && !options.all {
        return Err(String::from("archive requires --before or --all"));
    }
    let entries = select(kind, options)?;
    if entries.is_empty() {
        return Err(String::from("no files selected"));
    }
    let archive_root = out.join(kind.as_str());
    let manifest_path = archive_root.join("manifest.json");
    if manifest_path.exists() {
        return Err(format!(
            "{} already exists; choose a new --out",
            manifest_path.display()
        ));
    }
    let bytes: u64 = entries.iter().map(|entry| entry.bytes).sum();
    if options.dry_run {
        println!(
            "would archive {} files, {} bytes from {} to {}",
            entries.len(),
            bytes,
            kind.as_str(),
            archive_root.display()
        );
        return Ok(());
    }
    let mut records = Vec::with_capacity(entries.len());
    for entry in &entries {
        let source = options.root().join(&entry.path);
        let sha256 = crate::manifest::digest_file(&source)
            .map_err(|error| format!("cannot hash {}: {error}", source.display()))?;
        let destination = archive_root.join(&entry.path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "cannot copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
        records.push(FileRecord {
            path: entry.path.clone(),
            bytes: entry.bytes,
            sha256,
        });
    }
    let manifest = Manifest::new(
        kind.as_str(),
        source_commit(options.root()),
        options.created_unix.unwrap_or_else(now_unix),
        records,
    );
    let text = serde_json::to_string_pretty(&manifest.to_json())
        .map_err(|error| format!("cannot encode the manifest: {error}"))?;
    fs::write(&manifest_path, format!("{text}\n"))
        .map_err(|error| format!("cannot write {}: {error}", manifest_path.display()))?;
    println!(
        "archived {} files, {} bytes to {}; archive_id: {}",
        entries.len(),
        bytes,
        archive_root.display(),
        manifest.archive_id
    );
    Ok(())
}

fn erase(options: &Options) -> Result<(), String> {
    let manifest_path = options.manifest.as_ref().expect("validated");
    let text = fs::read_to_string(manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|error| format!("manifest is not JSON: {error}"))?;
    let manifest = Manifest::from_json(&value)?;
    let confirm = options.confirm.as_deref().expect("validated");
    if confirm != manifest.archive_id
        && confirm.strip_prefix("sha256:") != manifest.archive_id.strip_prefix("sha256:")
    {
        return Err(format!(
            "--confirm does not match the manifest archive_id {}",
            manifest.archive_id
        ));
    }
    let mut bytes = 0u64;
    let mut paths = Vec::with_capacity(manifest.files.len());
    for file in &manifest.files {
        let path = safe_relative(&file.path)?;
        let source = options.root().join(path);
        let metadata = fs::metadata(&source)
            .map_err(|error| format!("cannot stat {}: {error}", source.display()))?;
        if !metadata.is_file() || metadata.len() != file.bytes {
            return Err(format!("manifest does not match {}", file.path));
        }
        let sha256 = crate::manifest::digest_file(&source)
            .map_err(|error| format!("cannot hash {}: {error}", source.display()))?;
        if sha256 != file.sha256 {
            return Err(format!("manifest does not match {}", file.path));
        }
        bytes += file.bytes;
        paths.push(source);
    }
    if options.dry_run {
        println!(
            "would erase {} files, {} bytes from {}",
            paths.len(),
            bytes,
            manifest.kind
        );
        return Ok(());
    }
    for path in &paths {
        fs::remove_file(path)
            .map_err(|error| format!("cannot erase {}: {error}", path.display()))?;
    }
    println!(
        "erased {} files, {} bytes from {}",
        paths.len(),
        bytes,
        manifest.kind
    );
    Ok(())
}

fn select(kind: Kind, options: &Options) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for root in kind.roots(options.root()) {
        if !root.is_dir() {
            continue;
        }
        walk(&root, options.root(), &mut entries)?;
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    if let Some(before) = options.before() {
        entries.retain(|entry| match &entry.date {
            Some(date) => date.as_str() < before,
            None => options.include_undated,
        });
    }
    Ok(entries)
}

fn walk(root: &Path, base: &Path, entries: &mut Vec<Entry>) -> Result<(), String> {
    let mut children: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|error| format!("cannot read {}: {error}", root.display()))?
        .flatten()
        .map(|entry| entry.path())
        .collect();
    children.sort();
    for path in children {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            walk(&path, base, entries)?;
        } else if metadata.is_file() {
            let relative = path
                .strip_prefix(base)
                .map_err(|_| format!("{} is outside {}", path.display(), base.display()))?;
            let relative = relative.to_string_lossy().replace('\\', "/");
            entries.push(Entry {
                date: date_of(&relative),
                path: relative,
                bytes: metadata.len(),
            });
        }
    }
    Ok(())
}

fn date_of(path: &str) -> Option<String> {
    for component in path.split('/') {
        if let Some(date) = find_date(component) {
            return Some(date);
        }
    }
    None
}

fn find_date(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index + 10 <= bytes.len() {
        if let Some(window) = value.get(index..index + 10)
            && is_date(window)
        {
            return Some(window.to_owned());
        }
        index += 1;
    }
    None
}

fn is_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..].iter().all(u8::is_ascii_digit)
}

fn safe_relative(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "manifest path '{}' is not a safe relative path",
            path.display()
        ));
    }
    Ok(path.to_path_buf())
}

fn source_commit(root: &Path) -> String {
    Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| String::from("unknown"))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_found_in_dated_components() {
        assert_eq!(
            date_of("docs/execution-plans/2026-11-20/a.md").as_deref(),
            Some("2026-11-20")
        );
        assert_eq!(
            date_of("config/research/PROGRAM-2026-11-05-03/program.json").as_deref(),
            Some("2026-11-05")
        );
        assert_eq!(date_of("config/sr1/program.json"), None);
    }

    #[test]
    fn unsafe_manifest_paths_are_refused() {
        assert!(safe_relative("../secret").is_err());
        assert!(safe_relative("/tmp/secret").is_err());
        assert!(safe_relative("config/research/a.json").is_ok());
    }
}
