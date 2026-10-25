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
    /// The database file does not exist for a read-only command.
    Missing {
        /// The database path.
        path: PathBuf,
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
    /// The file is an uninitialized SQLite database, not a QuantRule store.
    Uninitialized {
        /// The database path.
        path: PathBuf,
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
    /// The dataset name does not exist.
    DatasetNotFound {
        /// The dataset name.
        name: String,
    },
    /// A query argument is valid syntactically but not for this table.
    InvalidQuery {
        /// What is wrong with the query.
        detail: String,
    },
    /// A stored dataset predates Layer 3 materialization.
    IncompleteDataset {
        /// The dataset name.
        name: String,
    },
    /// A stored dataset differs from a fresh build.
    VerificationMismatch {
        /// The first mismatch or structural problem.
        detail: String,
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
            Self::Missing { path } => {
                write!(formatter, "store not found: {}", path.display())
            }
            Self::Directory { path, source } => write!(
                formatter,
                "cannot create the directory for {}: {source}",
                path.display()
            ),
            Self::Migrate { source } => write!(formatter, "cannot migrate the store: {source}"),
            Self::Uninitialized { path } => write!(
                formatter,
                "{} is not a QuantRule store; run store init first",
                path.display()
            ),
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
            Self::DatasetNotFound { name } => write!(formatter, "dataset '{name}' was not found"),
            Self::InvalidQuery { detail } => write!(formatter, "invalid store query: {detail}"),
            Self::IncompleteDataset { name } => write!(
                formatter,
                "dataset '{name}' has no materialized Layer 3; re-ingest it into a fresh database"
            ),
            Self::VerificationMismatch { detail } => {
                write!(formatter, "verification failed: {detail}")
            }
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
            Self::UnsupportedVersion { .. }
            | Self::Uninitialized { .. }
            | Self::Corrupt { .. }
            | Self::Missing { .. } => None,
            Self::Conflict { .. }
            | Self::InvalidDataset { .. }
            | Self::DatasetNotFound { .. }
            | Self::InvalidQuery { .. }
            | Self::IncompleteDataset { .. }
            | Self::VerificationMismatch { .. } => None,
        }
    }
}
