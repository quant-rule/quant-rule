//! Typed records and the one memory error.
//!
//! The records mirror the v1 tables. Input records borrow their text so a
//! caller can insert without cloning; returned records own their values so a
//! caller can keep them after the query. A metric keeps its kind, so a null is
//! never confused with zero and an integer is never silently narrowed to a
//! float.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// The local research memory failed.
#[derive(Debug)]
pub enum MemoryError {
    /// SQLite rejected an operation.
    Database(rusqlite::Error),
    /// The database carries a schema this build does not support.
    UnsupportedSchema {
        /// The schema found in the database.
        found: String,
        /// The schema this build writes.
        expected: String,
    },
    /// The file is a SQLite database but not a QuantRule research memory.
    ForeignDatabase {
        /// The file that was inspected.
        path: PathBuf,
        /// What was found instead of a research memory.
        detail: String,
    },
    /// The database version is newer than this build.
    UnsupportedVersion {
        /// The version found in the database.
        found: i64,
        /// The version this build supports.
        expected: i64,
    },
    /// The import target already holds records.
    NonEmptyTarget,
    /// A backup or restore target already exists.
    TargetExists {
        /// The existing path.
        path: PathBuf,
    },
    /// An import document is not the expected shape.
    InvalidExport(String),
    /// A filesystem operation failed.
    Io {
        /// The path being read or written.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// JSON could not be read or written.
    Json(serde_json::Error),
    /// A record violates a cross-record rule.
    InvalidRecord(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(error) => write!(formatter, "memory database error: {error}"),
            Self::UnsupportedSchema { found, expected } => write!(
                formatter,
                "memory schema {found} is not supported; expected {expected}"
            ),
            Self::ForeignDatabase { path, detail } => write!(
                formatter,
                "memory database {} is not a QuantRule research memory: {detail}",
                path.display()
            ),
            Self::UnsupportedVersion { found, expected } => write!(
                formatter,
                "memory version {found} is newer than the supported version {expected}"
            ),
            Self::NonEmptyTarget => formatter.write_str("memory import target is not empty"),
            Self::TargetExists { path } => {
                write!(
                    formatter,
                    "memory target already exists: {}",
                    path.display()
                )
            }
            Self::InvalidExport(reason) => write!(formatter, "memory export is invalid: {reason}"),
            Self::Io { path, source } => {
                write!(formatter, "memory io error: {}: {source}", path.display())
            }
            Self::Json(error) => write!(formatter, "memory json error: {error}"),
            Self::InvalidRecord(reason) => write!(formatter, "memory record is invalid: {reason}"),
        }
    }
}

impl std::error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Database(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::UnsupportedSchema { .. }
            | Self::ForeignDatabase { .. }
            | Self::UnsupportedVersion { .. }
            | Self::NonEmptyTarget
            | Self::TargetExists { .. }
            | Self::InvalidExport(_)
            | Self::InvalidRecord(_) => None,
        }
    }
}

impl From<rusqlite::Error> for MemoryError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// One stored workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    /// The stable row ID.
    pub id: i64,
    /// The workspace root.
    pub root: String,
    /// The display name.
    pub name: String,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// A workspace to insert or update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewWorkspace<'a> {
    /// The workspace root; unique.
    pub root: &'a str,
    /// The display name.
    pub name: &'a str,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// One stored dataset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning workspace.
    pub workspace_id: i64,
    /// The dataset name.
    pub name: String,
    /// The local source path.
    pub path: String,
    /// The caller-supplied data version.
    pub data_version: String,
    /// The instrument.
    pub instrument: String,
    /// The accepted M1 candle count.
    pub candles: i64,
    /// The first open time as UTC Unix seconds.
    pub first_open_time: Option<i64>,
    /// The last open time as UTC Unix seconds.
    pub last_open_time: Option<i64>,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// A dataset to insert or update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewDataset<'a> {
    /// The owning workspace.
    pub workspace_id: i64,
    /// The dataset name.
    pub name: &'a str,
    /// The local source path.
    pub path: &'a str,
    /// The caller-supplied data version.
    pub data_version: &'a str,
    /// The instrument.
    pub instrument: &'a str,
    /// The accepted M1 candle count.
    pub candles: i64,
    /// The first open time as UTC Unix seconds.
    pub first_open_time: Option<i64>,
    /// The last open time as UTC Unix seconds.
    pub last_open_time: Option<i64>,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// One stored configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning workspace.
    pub workspace_id: i64,
    /// The run name.
    pub name: String,
    /// The run configuration path.
    pub path: String,
    /// The stable configuration fingerprint.
    pub config_hash: String,
    /// The schema version, such as `quantrule.run/v3`.
    pub schema_version: String,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// A configuration to insert or update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewConfig<'a> {
    /// The owning workspace.
    pub workspace_id: i64,
    /// The run name.
    pub name: &'a str,
    /// The run configuration path.
    pub path: &'a str,
    /// The stable configuration fingerprint.
    pub config_hash: &'a str,
    /// The schema version.
    pub schema_version: &'a str,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// One stored run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning workspace.
    pub workspace_id: i64,
    /// The optional configuration.
    pub config_id: Option<i64>,
    /// The caller-supplied run key; unique.
    pub run_key: String,
    /// The data version the run consumed.
    pub data_version: String,
    /// The configuration fingerprint.
    pub config_hash: String,
    /// The run status code.
    pub status: String,
    /// The UTC Unix second when the run started.
    pub started_at: i64,
    /// The UTC Unix second when the run finished.
    pub finished_at: Option<i64>,
    /// The adapter-facing summary JSON.
    pub summary_json: String,
}

/// A run to insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewRun<'a> {
    /// The owning workspace.
    pub workspace_id: i64,
    /// The optional configuration; it must belong to the same workspace.
    pub config_id: Option<i64>,
    /// The caller-supplied run key; unique.
    pub run_key: &'a str,
    /// The data version the run consumed.
    pub data_version: &'a str,
    /// The configuration fingerprint.
    pub config_hash: &'a str,
    /// The run status code.
    pub status: &'a str,
    /// The UTC Unix second when the run started.
    pub started_at: i64,
    /// The UTC Unix second when the run finished.
    pub finished_at: Option<i64>,
    /// The adapter-facing summary JSON.
    pub summary_json: &'a str,
}

/// One metric value.
#[derive(Debug, Clone, PartialEq)]
pub enum MetricValue {
    /// A real number.
    Real(f64),
    /// An integer.
    Integer(i64),
    /// Text that is not a number.
    Text(String),
    /// A measurement that is unavailable. It is never replaced by zero.
    Null,
}

impl MetricValue {
    /// The stable kind code stored in the database.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Real(_) => "real",
            Self::Integer(_) => "integer",
            Self::Text(_) => "text",
            Self::Null => "null",
        }
    }
}

/// One stored metric.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning run.
    pub run_id: i64,
    /// The metric name.
    pub name: String,
    /// The typed value.
    pub value: MetricValue,
}

/// One stored claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning run.
    pub run_id: i64,
    /// The claim type.
    pub claim_type: String,
    /// The claim statement.
    pub statement: String,
    /// The claim status.
    pub status: String,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// A claim to insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewClaim<'a> {
    /// The owning run.
    pub run_id: i64,
    /// The claim type.
    pub claim_type: &'a str,
    /// The claim statement.
    pub statement: &'a str,
    /// The claim status.
    pub status: &'a str,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
}

/// One stored note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRecord {
    /// The stable row ID.
    pub id: i64,
    /// The owning workspace.
    pub workspace_id: i64,
    /// The optional run the note belongs to.
    pub run_id: Option<i64>,
    /// The note body.
    pub body: String,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
    /// The UTC Unix second when the record was last updated.
    pub updated_at: i64,
}

/// A note to insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NewNote<'a> {
    /// The owning workspace.
    pub workspace_id: i64,
    /// The optional run; it must belong to the same workspace.
    pub run_id: Option<i64>,
    /// The note body.
    pub body: &'a str,
    /// The UTC Unix second when the record was created.
    pub created_at: i64,
    /// The UTC Unix second when the record was last updated.
    pub updated_at: i64,
}
