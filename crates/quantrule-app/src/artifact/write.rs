//! Deterministic atomic file output for stage artifacts.
//!
//! A write goes to a sibling temporary file and is renamed only after the bytes
//! are flushed, so an interrupted write never exposes a partial artifact under
//! the requested name.

use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// A file could not be written atomically.
#[derive(Debug)]
pub enum AtomicWriteError {
    /// The temporary file or its parent could not be created.
    Create {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// A write or flush failed.
    Write {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
    /// The temporary file could not be renamed.
    Rename {
        /// The requested output path.
        path: PathBuf,
        /// The I/O error.
        source: io::Error,
    },
}

impl fmt::Display for AtomicWriteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create { path, source } => {
                write!(formatter, "cannot create {}: {source}", path.display())
            }
            Self::Write { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
            Self::Rename { path, source } => {
                write!(formatter, "cannot replace {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for AtomicWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Create { source, .. }
            | Self::Write { source, .. }
            | Self::Rename { source, .. } => Some(source),
        }
    }
}

/// Writes bytes to `path` through a temporary sibling and an atomic rename.
///
/// # Errors
///
/// Returns [`AtomicWriteError`] when the parent cannot be created, the
/// temporary file cannot be written, or the rename fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), AtomicWriteError> {
    ensure_parent(path)?;
    let temp = temp_path(path);
    let mut file = match File::create(&temp) {
        Ok(file) => file,
        Err(source) => {
            let _ = fs::remove_file(&temp);
            return Err(AtomicWriteError::Create {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    if let Err(source) = file.write_all(bytes).and_then(|()| file.flush()) {
        let _ = fs::remove_file(&temp);
        return Err(AtomicWriteError::Write {
            path: path.to_path_buf(),
            source,
        });
    }

    if let Err(source) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(AtomicWriteError::Rename {
            path: path.to_path_buf(),
            source,
        });
    }

    Ok(())
}

fn ensure_parent(path: &Path) -> Result<(), AtomicWriteError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    fs::create_dir_all(parent).map_err(|source| AtomicWriteError::Create {
        path: path.to_path_buf(),
        source,
    })
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.as_os_str().to_os_string();
    temp.push(format!(".tmp.{}", std::process::id()));
    PathBuf::from(temp)
}

#[cfg(test)]
mod tests {
    use super::{temp_path, write_atomic};
    use std::fs;
    use std::path::PathBuf;

    fn artifact(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("quantrule-app-artifact-tests")
            .join(name)
    }

    #[test]
    fn atomic_write_replaces_the_target_and_leaves_no_temporary_file() {
        let path = artifact("atomic.bin");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("the artifact directory exists");
        }
        fs::write(&path, b"old").expect("the previous target is written");

        write_atomic(&path, b"new bytes").expect("the atomic write succeeds");

        assert_eq!(
            fs::read(&path).expect("the target is readable"),
            b"new bytes"
        );
        assert!(!temp_path(&path).exists());

        fs::remove_file(path).expect("the artifact is removed");
    }
}
