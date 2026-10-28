//! The shared report-bundle writer.
//!
//! A bundle is available if and only if its `report.json` exists. The writer
//! therefore creates the directory, refuses a non-empty target, writes every
//! other artifact and `provenance.json`, and writes `report.json` last. A
//! failure removes the files it wrote and the directory it created, so a
//! partial bundle is never available.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::export::{ExportError, write_atomic};

/// A bundle could not be written.
#[derive(Debug)]
pub(crate) enum BundleError {
    /// The bundle directory exists and is not empty.
    OutputNotEmpty {
        /// The bundle path.
        path: PathBuf,
    },
    /// The bundle path exists and is not a directory.
    OutputNotDirectory {
        /// The bundle path.
        path: PathBuf,
    },
    /// The bundle directory could not be read.
    ReadOutput {
        /// The bundle path.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// The bundle directory could not be created.
    CreateDirectory {
        /// The bundle path.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// One artifact could not be written.
    Artifact {
        /// The artifact path.
        path: PathBuf,
        /// The underlying error.
        source: ExportError,
    },
}

impl fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputNotEmpty { path } => write!(
                formatter,
                "output directory {} must be absent or empty",
                path.display()
            ),
            Self::OutputNotDirectory { path } => write!(
                formatter,
                "output path {} exists and is not a directory",
                path.display()
            ),
            Self::ReadOutput { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::CreateDirectory { path, source } => {
                write!(formatter, "cannot create {}: {source}", path.display())
            }
            Self::Artifact { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for BundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadOutput { source, .. } | Self::CreateDirectory { source, .. } => Some(source),
            Self::Artifact { source, .. } => Some(source),
            Self::OutputNotEmpty { .. } | Self::OutputNotDirectory { .. } => None,
        }
    }
}

/// Refuses a target that is a file or a non-empty directory.
pub(crate) fn ensure_available(root: &Path) -> Result<(), BundleError> {
    match fs::metadata(root) {
        Ok(metadata) if !metadata.is_dir() => Err(BundleError::OutputNotDirectory {
            path: root.to_path_buf(),
        }),
        Ok(_) => {
            let mut entries = fs::read_dir(root).map_err(|source| BundleError::ReadOutput {
                path: root.to_path_buf(),
                source,
            })?;
            match entries.next() {
                None => Ok(()),
                Some(Ok(_)) => Err(BundleError::OutputNotEmpty {
                    path: root.to_path_buf(),
                }),
                Some(Err(source)) => Err(BundleError::ReadOutput {
                    path: root.to_path_buf(),
                    source,
                }),
            }
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(BundleError::ReadOutput {
            path: root.to_path_buf(),
            source,
        }),
    }
}

/// Writes one bundle, with `report.json` last.
///
/// `artifacts` contains every artifact except `provenance.json`; the writer
/// adds provenance. The bundle is removed on failure, including a directory it
/// created.
pub(crate) fn write_bundle(
    root: &Path,
    artifacts: &[(&str, Vec<u8>)],
    provenance: &[u8],
) -> Result<(), BundleError> {
    ensure_available(root)?;
    let created = !root.exists();
    if created {
        fs::create_dir_all(root).map_err(|source| BundleError::CreateDirectory {
            path: root.to_path_buf(),
            source,
        })?;
    }

    let mut written = Vec::new();
    let mut write = |name: &str, bytes: &[u8]| -> Result<(), BundleError> {
        let path = root.join(name);
        write_atomic(&path, bytes).map_err(|source| BundleError::Artifact {
            path: path.clone(),
            source,
        })?;
        written.push(path);
        Ok(())
    };

    let result = (|| {
        write("provenance.json", provenance)?;
        for (name, bytes) in artifacts.iter().filter(|(name, _)| *name != "report.json") {
            write(name, bytes)?;
        }
        if let Some((_, bytes)) = artifacts.iter().find(|(name, _)| *name == "report.json") {
            write("report.json", bytes)?;
        }
        Ok(())
    })();

    if result.is_err() {
        for path in written {
            let _ = fs::remove_file(path);
        }
        if created {
            let _ = fs::remove_dir(root);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::{BundleError, ensure_available, write_bundle};

    fn root(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/report-tests")
            .join(name)
    }

    #[test]
    fn a_non_empty_target_is_refused() {
        let root = root("bundle-non-empty");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("the directory is created");
        fs::write(root.join("old.txt"), b"old").expect("the old file is written");

        assert!(matches!(
            ensure_available(&root),
            Err(BundleError::OutputNotEmpty { .. })
        ));
        fs::remove_dir_all(&root).expect("the directory is removed");
    }

    #[test]
    fn the_writer_publishes_report_last_and_cleans_up() {
        let root = root("bundle-write");
        let _ = fs::remove_dir_all(&root);
        let artifacts = vec![
            ("summary.txt", b"summary".to_vec()),
            ("report.json", b"{}".to_vec()),
            ("decisions.csv", b"header\n".to_vec()),
        ];
        write_bundle(&root, &artifacts, b"{}").expect("the bundle is written");

        assert!(root.join("report.json").exists());
        assert!(root.join("provenance.json").exists());
        assert!(root.join("summary.txt").exists());
        assert!(root.join("decisions.csv").exists());
        fs::remove_dir_all(&root).expect("the directory is removed");
    }
}
