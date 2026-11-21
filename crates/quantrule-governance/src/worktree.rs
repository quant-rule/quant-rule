//! Check, archive, and remove a merged task worktree safely.
//!
//! The worktree's `data/` directory is ignored by Git and lives only on the
//! operator's machine, so removal must archive it first and verify the copy.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const DISPOSABLE_DIRECTORIES: [&str; 5] = [
    "__pycache__",
    ".venv",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
];
const DISPOSABLE_SUFFIXES: [&str; 2] = [".pyc", ".pyo"];
const SCHEMA: &str = "quantrule.worktree-cleanup/v1";

/// Run `qr-governance worktree`.
///
/// Usage:
///
/// ```text
/// qr-governance worktree check <worktree>
/// qr-governance worktree archive <worktree> --dest <path> [--label <label>]
/// qr-governance worktree remove <worktree> --branch <branch> --archive-root <path> [--yes]
/// ```
#[must_use]
pub fn run(args: &[String]) -> ExitCode {
    match execute(args) {
        Ok((value, code)) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&value).unwrap_or_default()
            );
            ExitCode::from(code)
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(args: &[String]) -> Result<(Value, u8), String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Err("worktree needs check, archive, or remove".to_owned());
    };
    let mut index = 1;
    let worktree = args
        .get(index)
        .ok_or_else(|| "worktree needs a path".to_owned())?;
    let worktree = resolve(Path::new(worktree));
    index += 1;
    let mut dest = None;
    let mut label = None;
    let mut branch = None;
    let mut archive_root = None;
    let mut main_branch = "main".to_owned();
    let mut remote = None;
    let mut yes = false;
    while index < args.len() {
        match args[index].as_str() {
            "--dest" => dest = Some(argument(args, &mut index, "--dest")?),
            "--label" => label = Some(argument(args, &mut index, "--label")?),
            "--branch" => branch = Some(argument(args, &mut index, "--branch")?),
            "--archive-root" => archive_root = Some(argument(args, &mut index, "--archive-root")?),
            "--main-branch" => main_branch = argument(args, &mut index, "--main-branch")?,
            "--remote" => remote = Some(argument(args, &mut index, "--remote")?),
            "--yes" => yes = true,
            other => return Err(format!("unknown argument {other}")),
        }
        index += 1;
    }
    match command {
        "check" => {
            let value = inspect(&worktree)?;
            let code = if value.get("status").and_then(Value::as_str) == Some("clean") {
                0
            } else {
                2
            };
            Ok((value, code))
        }
        "archive" => {
            let dest = dest.ok_or_else(|| "archive needs --dest".to_owned())?;
            let value = archive(&worktree, Path::new(&dest), label.as_deref())?;
            Ok((value, 0))
        }
        "remove" => {
            let branch = branch.ok_or_else(|| "remove needs --branch".to_owned())?;
            let archive_root =
                archive_root.ok_or_else(|| "remove needs --archive-root".to_owned())?;
            let value = remove(
                &worktree,
                &branch,
                Path::new(&archive_root),
                &main_branch,
                remote.as_deref(),
                yes,
            )?;
            Ok((value, 0))
        }
        other => Err(format!("unknown worktree command {other}")),
    }
}

fn argument(args: &[String], index: &mut usize, name: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{name} needs a value"))
}

/// Classify a worktree without deleting anything.
///
/// # Errors
///
/// Returns an error when the path is not a Git worktree or Git fails.
pub fn inspect(worktree: &Path) -> Result<Value, String> {
    if !worktree.is_dir() {
        return Err(format!("{} is not a directory", worktree.display()));
    }
    if !worktree.join(".git").exists() {
        return Err(format!("{} is not a Git worktree", worktree.display()));
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(worktree)
        .args([
            "status",
            "--porcelain=v1",
            "--ignored=matching",
            "--untracked-files=all",
            "-z",
        ])
        .output()
        .map_err(|error| format!("git status failed: {error}"))?;
    if !output.status.success() {
        return Err(format!("git status failed in {}", worktree.display()));
    }
    let mut groups: [Vec<String>; 4] = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for chunk in output.stdout.split(|byte| *byte == 0) {
        if chunk.is_empty() {
            continue;
        }
        let status = String::from_utf8_lossy(&chunk[..2.min(chunk.len())]).into_owned();
        let path = if chunk.len() > 3 {
            String::from_utf8_lossy(&chunk[3..]).into_owned()
        } else {
            String::new()
        };
        if path.is_empty() {
            continue;
        }
        let group = classify(&path, &status);
        groups[group].push(path);
    }
    for group in &mut groups {
        group.sort();
    }
    let status = if !groups[2].is_empty() || !groups[3].is_empty() {
        "blocker"
    } else if !groups[0].is_empty() {
        "local_data"
    } else {
        "clean"
    };
    let mut blockers = groups[3].clone();
    blockers.extend(groups[2].clone());
    blockers.sort();
    let mut value = Map::new();
    value.insert("schema".to_owned(), json!(SCHEMA));
    value.insert(
        "worktree".to_owned(),
        json!(
            worktree
                .to_path_buf()
                .canonicalize()
                .unwrap_or_else(|_| worktree.to_path_buf())
                .to_string_lossy()
        ),
    );
    value.insert("status".to_owned(), json!(status));
    value.insert("local_data".to_owned(), json!(groups[0]));
    value.insert("disposable".to_owned(), json!(groups[1]));
    value.insert("tracked".to_owned(), json!(groups[2]));
    value.insert("blockers".to_owned(), json!(blockers));
    Ok(Value::Object(value))
}

fn classify(path: &str, status: &str) -> usize {
    if status != "??" && status != "!!" {
        return 2;
    }
    if path.starts_with("data/") {
        return 0;
    }
    if path.starts_with("target/")
        || DISPOSABLE_SUFFIXES
            .iter()
            .any(|suffix| path.ends_with(suffix))
        || Path::new(path).components().any(|part| {
            DISPOSABLE_DIRECTORIES.contains(&part.as_os_str().to_string_lossy().as_ref())
        })
    {
        return 1;
    }
    3
}

/// Copy local data to a dated archive and write a manifest. No deletion.
///
/// # Errors
///
/// Returns an error for blockers, unreadable files, or an existing target.
pub fn archive(worktree: &Path, dest: &Path, label: Option<&str>) -> Result<Value, String> {
    let check = inspect(worktree)?;
    let blockers = check
        .get("blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !blockers.is_empty() {
        return Err(format!(
            "{} has untracked or modified files that are not local data: {:?}",
            worktree.display(),
            blockers
        ));
    }
    let local_data: Vec<String> = check
        .get("local_data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if local_data.is_empty() {
        let mut value = Map::new();
        value.insert("schema".to_owned(), json!(SCHEMA));
        value.insert("worktree".to_owned(), check["worktree"].clone());
        value.insert("status".to_owned(), json!("nothing_to_archive"));
        value.insert("files".to_owned(), json!(0));
        value.insert("bytes".to_owned(), json!(0));
        return Ok(Value::Object(value));
    }
    let label = label
        .map(str::to_owned)
        .or_else(|| {
            worktree
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_default();
    let stamp = utc_stamp();
    let target = resolve(dest).join(&label).join(&stamp);
    if target.exists() {
        return Err(format!(
            "archive target {} already exists",
            target.display()
        ));
    }
    fs::create_dir_all(&target)
        .map_err(|error| format!("cannot create {}: {error}", target.display()))?;
    let mut files = Vec::new();
    let mut total = 0_u64;
    for relative in &local_data {
        let source = worktree.join(relative);
        let mut candidates = Vec::new();
        if source.is_dir() {
            collect_files(&source, &mut candidates);
            candidates.sort();
        } else {
            candidates.push(source.clone());
        }
        for candidate in candidates {
            let relative_path = candidate
                .strip_prefix(worktree)
                .unwrap_or(&candidate)
                .to_string_lossy()
                .replace('\\', "/");
            let destination = target.join(&relative_path);
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            fs::copy(&candidate, &destination)
                .map_err(|error| format!("cannot archive {}: {error}", candidate.display()))?;
            let size = fs::metadata(&destination)
                .map_err(|error| format!("cannot read {}: {error}", destination.display()))?
                .len();
            total += size;
            let mut entry = Map::new();
            entry.insert("path".to_owned(), json!(relative_path));
            entry.insert("bytes".to_owned(), json!(size));
            entry.insert("sha256".to_owned(), json!(sha256(&destination)?));
            files.push(Value::Object(entry));
        }
    }
    let source_commit = git(worktree, &["rev-parse", "HEAD"])?.trim().to_owned();
    let check_status = check
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut manifest = Map::new();
    manifest.insert("schema".to_owned(), json!(SCHEMA));
    manifest.insert("worktree".to_owned(), check["worktree"].clone());
    manifest.insert("destination".to_owned(), json!(target.to_string_lossy()));
    manifest.insert("label".to_owned(), json!(label));
    manifest.insert("archived_utc".to_owned(), json!(stamp));
    manifest.insert("source_commit".to_owned(), json!(source_commit));
    manifest.insert("files".to_owned(), Value::Array(files.clone()));
    manifest.insert("file_count".to_owned(), json!(files.len()));
    manifest.insert("bytes".to_owned(), json!(total));
    manifest.insert("check_status".to_owned(), json!(check_status));
    let value = Value::Object(manifest);
    fs::write(
        target.join("archive-manifest.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&value).unwrap_or_default()
        ),
    )
    .map_err(|error| format!("cannot write the archive manifest: {error}"))?;
    Ok(value)
}

/// Verify every file named by an archive manifest.
///
/// # Errors
///
/// Returns an error when a file is missing, short, or altered.
pub fn verify_archive(manifest: &Value) -> Result<(), String> {
    let destination = PathBuf::from(
        manifest
            .get("destination")
            .and_then(Value::as_str)
            .ok_or_else(|| "archive manifest has no destination".to_owned())?,
    );
    let files = manifest
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if manifest.get("file_count").and_then(Value::as_u64) != Some(files.len() as u64) {
        return Err(format!(
            "archive manifest {} counts {:?} files but lists {}",
            destination.display(),
            manifest.get("file_count"),
            files.len()
        ));
    }
    for entry in files {
        let relative = entry
            .get("path")
            .and_then(Value::as_str)
            .ok_or_else(|| "archive entry has no path".to_owned())?;
        let path = destination.join(relative);
        if !path.is_file() {
            return Err(format!("archived file {} is missing", path.display()));
        }
        let size = fs::metadata(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?
            .len();
        let expected = entry.get("bytes").and_then(Value::as_u64).unwrap_or(0);
        if size != expected {
            return Err(format!(
                "archived file {} is {size} bytes, manifest says {expected}",
                path.display()
            ));
        }
        let found = sha256(&path)?;
        let expected = entry.get("sha256").and_then(Value::as_str).unwrap_or("");
        if found != expected {
            return Err(format!(
                "archived file {} has hash {found}, manifest says {expected}",
                path.display()
            ));
        }
    }
    Ok(())
}

/// Archive and verify local data, check the branch is merged, then remove.
///
/// # Errors
///
/// Returns an error for missing `--yes`, blockers, an unmerged branch, or a
/// failed Git operation.
pub fn remove(
    worktree: &Path,
    branch: &str,
    archive_root: &Path,
    main_branch: &str,
    remote: Option<&str>,
    yes: bool,
) -> Result<Value, String> {
    if !yes {
        return Err("pass --yes to remove a worktree and branch".to_owned());
    }
    let main = main_worktree(worktree)?;
    if resolve(worktree) == main {
        return Err("refusing to remove the main worktree".to_owned());
    }
    let check = inspect(worktree)?;
    let blockers = check
        .get("blockers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !blockers.is_empty() {
        return Err(format!("worktree has blockers: {:?}", blockers));
    }
    if !branch_merged(&main, branch, main_branch) {
        return Err(format!("branch {branch} is not merged into {main_branch}"));
    }
    let local_data: Vec<String> = check
        .get("local_data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut archive_value = Value::Null;
    if !local_data.is_empty() {
        let label = worktree
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        archive_value = archive(worktree, archive_root, label.as_deref())?;
        verify_archive(&archive_value)?;
        remove_local_data(worktree, &local_data)?;
    }
    git(&main, &["worktree", "remove", &worktree.to_string_lossy()])?;
    git(&main, &["branch", "-d", branch])?;
    if let Some(remote) = remote {
        git(&main, &["push", remote, "--delete", branch])?;
    }
    let mut value = Map::new();
    value.insert("schema".to_owned(), json!(SCHEMA));
    value.insert("status".to_owned(), json!("removed"));
    value.insert("worktree".to_owned(), json!(worktree.to_string_lossy()));
    value.insert("branch".to_owned(), json!(branch));
    value.insert("archive".to_owned(), archive_value);
    Ok(Value::Object(value))
}

fn remove_local_data(worktree: &Path, local_data: &[String]) -> Result<(), String> {
    for relative in local_data {
        let source = worktree.join(relative);
        let result = if source.is_dir() {
            fs::remove_dir_all(&source)
        } else {
            fs::remove_file(&source)
        };
        result.map_err(|error| {
            format!(
                "cannot remove archived local data {}: {error}",
                source.display()
            )
        })?;
    }
    Ok(())
}

fn main_worktree(worktree: &Path) -> Result<PathBuf, String> {
    let output = git(worktree, &["worktree", "list", "--porcelain"])?;
    for line in output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            return Ok(resolve(Path::new(path)));
        }
    }
    Err("could not find the main worktree".to_owned())
}

fn branch_merged(main: &Path, branch: &str, main_branch: &str) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(main)
        .args(["merge-base", "--is-ancestor", branch, main_branch])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn git(directory: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .map_err(|error| {
            format!(
                "git {} failed in {}: {error}",
                arguments.join(" "),
                directory.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed in {}: {}",
            arguments.join(" "),
            directory.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn sha256(path: &Path) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn collect_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, found);
        } else {
            found.push(path);
        }
    }
}

fn resolve(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn utc_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        remainder / 3_600,
        (remainder % 3_600) / 60,
        remainder % 60
    )
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(name: &str) -> Self {
            let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "qr-worktree-{name}-{}-{unique}",
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).expect("tree");
            Self(path)
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git_ok(repo: &Path, arguments: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(arguments)
            .status()
            .expect("git");
        assert!(status.success(), "git {arguments:?}");
    }

    fn repo(root: &Path) -> PathBuf {
        let repo = root.join("repo");
        fs::create_dir(&repo).expect("repo");
        git_ok(&repo, &["init", "-b", "main"]);
        git_ok(&repo, &["config", "user.email", "test@example.com"]);
        git_ok(&repo, &["config", "user.name", "Test"]);
        fs::write(
            repo.join(".gitignore"),
            "/data/**\n!/data/**/\n!/data/**/README.md\n",
        )
        .expect("ignore");
        fs::create_dir(repo.join("data")).expect("data");
        fs::write(repo.join("data/README.md"), "local data\n").expect("readme");
        fs::write(repo.join("README.md"), "repo\n").expect("readme");
        git_ok(&repo, &["add", "."]);
        git_ok(&repo, &["commit", "-m", "init"]);
        repo
    }

    fn worktree(repo: &Path, root: &Path) -> PathBuf {
        let worktree = root.join("worktree");
        git_ok(
            repo,
            &[
                "worktree",
                "add",
                "-b",
                "feat/task",
                worktree.to_str().expect("path"),
                "main",
            ],
        );
        worktree
    }

    #[test]
    fn a_clean_worktree_is_clean() {
        let root = TempRoot::new("clean");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        assert_eq!(
            inspect(&worktree).expect("check")["status"].as_str(),
            Some("clean")
        );
    }

    #[test]
    fn local_data_is_archived() {
        let root = TempRoot::new("archive");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::write(worktree.join("data/local.txt"), "keep me\n").expect("local");
        let check = inspect(&worktree).expect("check");
        assert_eq!(check["status"].as_str(), Some("local_data"));
        let manifest = archive(&worktree, &root.0.join("archive"), Some("task")).expect("archive");
        assert_eq!(manifest["file_count"].as_u64(), Some(1));
        let archived =
            PathBuf::from(manifest["destination"].as_str().expect("path")).join("data/local.txt");
        assert_eq!(fs::read_to_string(archived).expect("read"), "keep me\n");
        assert!(
            PathBuf::from(manifest["destination"].as_str().expect("path"))
                .join("archive-manifest.json")
                .is_file()
        );
    }

    #[test]
    fn a_nested_cache_is_disposable() {
        let root = TempRoot::new("cache");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::create_dir_all(worktree.join("tools/__pycache__")).expect("cache");
        fs::write(worktree.join("tools/__pycache__/x.pyc"), "cache").expect("pyc");
        assert_eq!(
            inspect(&worktree).expect("check")["status"].as_str(),
            Some("clean")
        );
    }

    #[test]
    fn an_untracked_file_is_a_blocker() {
        let root = TempRoot::new("blocker");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::write(worktree.join("notes.txt"), "unclassified\n").expect("notes");
        let check = inspect(&worktree).expect("check");
        assert_eq!(check["status"].as_str(), Some("blocker"));
        assert!(archive(&worktree, &root.0.join("archive"), Some("task")).is_err());
    }

    #[test]
    fn an_unmerged_branch_is_refused() {
        let root = TempRoot::new("unmerged");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::write(worktree.join("work.txt"), "work\n").expect("work");
        git_ok(&worktree, &["add", "work.txt"]);
        git_ok(&worktree, &["commit", "-m", "work"]);
        assert!(
            remove(
                &worktree,
                "feat/task",
                &root.0.join("archive"),
                "main",
                None,
                true,
            )
            .is_err()
        );
        assert!(worktree.is_dir());
    }

    #[test]
    fn a_merged_branch_is_archived_and_removed() {
        let root = TempRoot::new("merged");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::write(worktree.join("data/local.txt"), "keep me\n").expect("local");
        let result = remove(
            &worktree,
            "feat/task",
            &root.0.join("archive"),
            "main",
            None,
            true,
        )
        .expect("remove");
        assert_eq!(result["status"].as_str(), Some("removed"));
        assert!(!worktree.exists());
        let archived = PathBuf::from(
            result["archive"]["destination"]
                .as_str()
                .expect("destination"),
        )
        .join("data/local.txt");
        assert_eq!(fs::read_to_string(archived).expect("read"), "keep me\n");
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_archive_leaves_the_worktree_in_place() {
        use std::os::unix::fs::symlink;
        let root = TempRoot::new("failed-archive");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        symlink(worktree.join("data/missing"), worktree.join("data/broken")).expect("symlink");
        assert!(
            remove(
                &worktree,
                "feat/task",
                &root.0.join("archive"),
                "main",
                None,
                true,
            )
            .is_err()
        );
        assert!(worktree.is_dir());
        let branches = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(["branch", "--list", "feat/task"])
            .output()
            .expect("git");
        assert!(!String::from_utf8_lossy(&branches.stdout).trim().is_empty());
    }

    #[test]
    fn a_tampered_archive_is_refused() {
        let root = TempRoot::new("tampered");
        let repo = repo(&root.0);
        let worktree = worktree(&repo, &root.0);
        fs::write(worktree.join("data/local.txt"), "keep me\n").expect("local");
        let manifest = archive(&worktree, &root.0.join("archive"), Some("task")).expect("archive");
        let archived =
            PathBuf::from(manifest["destination"].as_str().expect("path")).join("data/local.txt");
        fs::write(archived, "changed\n").expect("change");
        assert!(verify_archive(&manifest).is_err());
    }
}
