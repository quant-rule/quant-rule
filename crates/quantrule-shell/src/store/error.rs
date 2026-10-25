//! Store errors.
//!
//! Every failure names the file or the schema operation that failed. The
//! connection never panics on a bad file; it returns one of these variants.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A store operation failed.
#[derive(Debug)]
pub(crate) enum StoreError {
    /// The database file could not be opened.
    Open {
        /// The database path.
        path: PathBuf,
        /// The underlying SQLite error.
        source: rusqlite::Error,
    },
    /// The database directory could not be created.
    Directory {
        /// The database path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A migration or schema statement failed and was rolled back.
    Migrate {
        /// The underlying SQLite error.
        source: rusqlite::Error,
    },
    /// The database's schema is newer than this binary supports.
    UnsupportedVersion {
        /// The version found in the file.
        found: i64,
        /// The version this binary supports.
        supported: i64,
    },
    /// The file is not a readable SQLite database.
    Corrupt {
        /// The database path.
        path: PathBuf,
        /// The SQLite or integrity message.
        detail: String,
    },
    /// A query failed after the database was opened.
    Query {
        /// The underlying SQLite error.
        source: rusqlite::Error,
    },
    /// The dataset name exists with a different identity.
    Conflict {
        /// The dataset name.
        name: String,
    },
    /// The dataset cannot be written because its identity or bar count is invalid.
    InvalidDataset {
        /// What is wrong with the dataset.
        detail: String,
    },
    /// A write failed and was rolled back.
    Write {
        /// The underlying SQLite error.
        source: rusqlite::Error,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open { path, source } => {
                write!(formatter, "cannot open {}: {source}", path.display())
            }
            Self::Directory { path, source } => write!(
                formatter,
                "cannot create the directory for {}: {source}",
                path.display()
            ),
            Self::Migrate { source } => write!(formatter, "cannot migrate the store: {source}"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "the store schema version {found} is newer than the supported version {supported}"
            ),
            Self::Corrupt { path, detail } => {
                write!(
                    formatter,
                    "{} is not a valid store: {detail}",
                    path.display()
                )
            }
            Self::Query { source } => write!(formatter, "store query failed: {source}"),
            Self::Conflict { name } => write!(
                formatter,
                "dataset '{name}' already exists with a different identity"
            ),
            Self::InvalidDataset { detail } => {
                write!(formatter, "cannot ingest the dataset: {detail}")
            }
            Self::Write { source } => write!(formatter, "cannot write the store: {source}"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Open { source, .. } | Self::Migrate { source } | Self::Query { source } => {
                Some(source)
            }
            Self::Write { source } => Some(source),
            Self::Directory { source, .. } => Some(source),
            Self::UnsupportedVersion { .. } | Self::Corrupt { .. } => None,
            Self::Conflict { .. } | Self::InvalidDataset { .. } => None,
        }
    }
}
