//! The local workspace boundary.
//!
//! A workspace is one directory that holds the researcher's configuration and
//! data. The service resolves every standard path against it and lists the
//! datasets and run configurations it contains. Listing is read-only and
//! sorted, so the same workspace always produces the same order.
//!
//! The layout is intentionally small and local:
//!
//! ```text
//! <root>/config/        run and engine configuration files
//! <root>/data/sources/  local OHLCV candle files
//! ```
//!
//! A missing optional directory is an empty list, not an error. A missing
//! workspace root is an error because every later operation depends on it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::dataset::Dataset;
use crate::error::AppError;

/// The standard subdirectory that holds run and engine configuration.
pub const CONFIG_DIR: &str = "config";
/// The standard subdirectory that holds local candle sources.
pub const SOURCES_DIR: &str = "data/sources";

/// One opened local workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Opens an existing directory as a workspace.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Workspace`] when the root does not exist or is not a
    /// directory.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, AppError> {
        let root = root.as_ref();
        let canonical = match fs::canonicalize(root) {
            Ok(path) => path,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Err(AppError::Workspace {
                    path: root.to_path_buf(),
                    reason: String::from("the workspace root is missing or is not a directory"),
                });
            }
            Err(source) => {
                return Err(AppError::Io {
                    path: root.to_path_buf(),
                    source,
                });
            }
        };
        if !canonical.is_dir() {
            return Err(AppError::Workspace {
                path: root.to_path_buf(),
                reason: String::from("the workspace root is missing or is not a directory"),
            });
        }
        Ok(Self { root: canonical })
    }

    /// Finds the nearest enclosing workspace from a file or directory.
    ///
    /// A workspace root is a directory that carries the standard configuration
    /// directory or the standard data directory. The search starts at the
    /// file's parent, or at the directory itself, and stops at the filesystem
    /// root.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Workspace`] when no enclosing workspace exists.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, AppError> {
        let start = start.as_ref();
        let mut current = if start.is_dir() {
            Some(start)
        } else {
            start.parent()
        };
        while let Some(candidate) = current {
            if candidate.join(CONFIG_DIR).is_dir() || candidate.join("data").is_dir() {
                return Self::open(candidate);
            }
            current = candidate.parent();
        }
        Err(AppError::Workspace {
            path: start.to_path_buf(),
            reason: String::from("no enclosing workspace with config/ or data/ was found"),
        })
    }

    /// The canonical workspace root.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The workspace name: its directory name.
    ///
    /// The name labels the workspace and is stable while the root is unchanged;
    /// a run name labels a run and never replaces this name.
    #[must_use]
    pub fn name(&self) -> String {
        self.root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.root.to_string_lossy().into_owned())
    }

    /// The standard configuration directory.
    #[must_use]
    pub fn config_dir(&self) -> PathBuf {
        self.root.join(CONFIG_DIR)
    }

    /// The standard candle-source directory.
    #[must_use]
    pub fn sources_dir(&self) -> PathBuf {
        self.root.join(SOURCES_DIR)
    }

    /// Every local candle dataset, sorted by path.
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] when a directory cannot be read.
    pub fn datasets(&self) -> Result<Vec<Dataset>, AppError> {
        let paths = collect_files(&self.sources_dir(), &|path| {
            path.extension().is_some_and(|extension| extension == "csv")
        })?;
        paths.into_iter().map(Dataset::open).collect()
    }

    /// Every run configuration under the standard configuration directory,
    /// sorted by path.
    ///
    /// The scan finds files named `run.json`; it does not validate them. A
    /// caller that wants a validated run uses [`crate::service::AppService`].
    ///
    /// # Errors
    ///
    /// Returns [`AppError::Io`] when a directory cannot be read.
    pub fn run_configs(&self) -> Result<Vec<PathBuf>, AppError> {
        collect_files(&self.config_dir(), &|path| {
            path.file_name().is_some_and(|name| name == "run.json")
        })
    }
}

/// Collects matching files below `root` in sorted order.
fn collect_files(root: &Path, accept: &impl Fn(&Path) -> bool) -> Result<Vec<PathBuf>, AppError> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    if !root.is_dir() {
        return Err(AppError::Workspace {
            path: root.to_path_buf(),
            reason: String::from("the expected workspace path is not a directory"),
        });
    }

    let mut files = Vec::new();
    walk(root, accept, &mut files)?;
    files.sort();
    Ok(files)
}

fn walk(
    directory: &Path,
    accept: &impl Fn(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(directory).map_err(|source| AppError::Io {
        path: directory.to_path_buf(),
        source,
    })?;
    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| AppError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let file_type = entry.file_type().map_err(|source| AppError::Io {
            path: entry.path(),
            source,
        })?;
        paths.push((entry.path(), file_type));
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));

    for (path, file_type) in paths {
        if file_type.is_dir() {
            walk(&path, accept, files)?;
        } else if file_type.is_file() && accept(&path) {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::Workspace;

    fn repository() -> Workspace {
        Workspace::open(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .expect("the repository is a workspace")
    }

    #[test]
    fn a_missing_root_is_rejected() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/app-service-tests/missing-workspace");
        assert!(Workspace::open(&path).is_err());
    }

    #[test]
    fn the_root_is_canonicalized() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/app-service-tests/canonical-workspace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the workspace is created");
        let dotted = root.join(".");
        let workspace = Workspace::open(&dotted).expect("the dotted root opens");
        assert_eq!(
            workspace.root().to_string_lossy(),
            fs::canonicalize(&root)
                .expect("the root resolves")
                .to_string_lossy()
        );
        assert_eq!(workspace.name(), "canonical-workspace");
    }

    #[test]
    fn the_repository_lists_the_example_run() {
        let configs = repository()
            .run_configs()
            .expect("the config tree is readable");
        assert!(
            configs
                .iter()
                .any(|path| path.ends_with("config/examples/btcusd-2024/run.json"))
        );
        let mut sorted = configs.clone();
        sorted.sort();
        assert_eq!(configs, sorted);
    }

    #[test]
    fn datasets_are_listed_in_sorted_order() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/app-service-tests/workspace");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("data/sources")).expect("the sources directory is created");
        fs::write(root.join("data/sources/b.csv"), "not parsed here\n")
            .expect("the fixture is written");
        fs::write(root.join("data/sources/a.csv"), "not parsed here\n")
            .expect("the fixture is written");
        fs::write(root.join("data/sources/readme.txt"), "ignored\n")
            .expect("the fixture is written");

        let workspace = Workspace::open(&root).expect("the workspace opens");
        let names: Vec<String> = workspace
            .datasets()
            .expect("the datasets are listed")
            .iter()
            .map(|dataset| dataset.name())
            .collect();
        assert_eq!(names, ["a", "b"]);
    }
}
