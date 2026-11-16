//! The application-service error type.
//!
//! Every operation returns one explicit error. A configuration rejection stays
//! a [`ConfigError`], a source rejection stays a [`SourceError`], and a
//! workspace or export failure carries the path that failed. The display is the
//! same text a command prints; it never includes an absolute path for a config
//! error and never hides the failing file for an I/O error.

use std::fmt;
use std::io;
use std::path::PathBuf;

use crate::config::ConfigError;
use crate::csv::SourceError;
use crate::date::DateRangeError;
use crate::pipeline::PipelineError;

/// The application service could not complete the request.
#[derive(Debug)]
pub enum AppError {
    /// A run or engine configuration was rejected.
    Config(ConfigError),
    /// A candle source was rejected.
    Source(SourceError),
    /// A date bound was invalid.
    Date(DateRangeError),
    /// The configured model run failed.
    Pipeline(PipelineError),
    /// A dataset is missing or is not a file.
    Dataset {
        /// The supplied dataset path.
        path: PathBuf,
        /// Why the dataset was rejected.
        reason: String,
    },
    /// The workspace root is missing or not a directory.
    Workspace {
        /// The supplied root.
        path: PathBuf,
        /// Why the root was rejected.
        reason: String,
    },
    /// A filesystem operation failed.
    Io {
        /// The path being read or written.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(error) => write!(formatter, "{error}"),
            Self::Source(error) => write!(formatter, "{error}"),
            Self::Date(error) => write!(formatter, "{error}"),
            Self::Pipeline(error) => write!(formatter, "{error}"),
            Self::Dataset { path, reason } => {
                write!(formatter, "dataset error: {}: {reason}", path.display())
            }
            Self::Workspace { path, reason } => {
                write!(formatter, "workspace error: {}: {reason}", path.display())
            }
            Self::Io { path, source } => {
                write!(formatter, "io error: {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Source(error) => Some(error),
            Self::Date(error) => Some(error),
            Self::Pipeline(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::Dataset { .. } | Self::Workspace { .. } => None,
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(error: ConfigError) -> Self {
        Self::Config(error)
    }
}

impl From<SourceError> for AppError {
    fn from(error: SourceError) -> Self {
        Self::Source(error)
    }
}

impl From<DateRangeError> for AppError {
    fn from(error: DateRangeError) -> Self {
        Self::Date(error)
    }
}

impl From<PipelineError> for AppError {
    fn from(error: PipelineError) -> Self {
        Self::Pipeline(error)
    }
}
