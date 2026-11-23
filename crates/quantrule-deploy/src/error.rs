//! Error types for the deployment tool.
//!
//! Every failure names the field or file that caused it, so an operator can
//! fix the descriptor rather than guess. The tool never panics on bad input.

use std::fmt;
use std::path::PathBuf;

/// One deployment-tool failure.
#[derive(Debug)]
pub enum Error {
    /// The descriptor or secrets file could not be read.
    Read {
        /// The path that failed.
        path: PathBuf,
        /// The operating-system error.
        source: std::io::Error,
    },
    /// The descriptor or secrets file is not valid JSON or dotenv syntax.
    Parse {
        /// The path that failed.
        path: PathBuf,
        /// The parser message.
        message: String,
    },
    /// A descriptor value is present but unsafe or inconsistent.
    Invalid {
        /// The descriptor field.
        field: String,
        /// Why the value was rejected.
        message: String,
    },
    /// A v3 rule was violated.
    ///
    /// The code is stable across releases so a client can act on it; the field
    /// is a dot path and the message tells the operator what to change.
    Rule {
        /// The stable `deploy.*` code.
        code: &'static str,
        /// The descriptor field or file, as a dot path.
        field: String,
        /// Why the value was rejected.
        message: String,
    },
    /// A placeholder in the descriptor has no value in the secrets file.
    MissingSecret {
        /// The missing environment-variable name.
        name: String,
    },
    /// An environment file could not be written.
    Write {
        /// The path that failed.
        path: PathBuf,
        /// The operating-system error.
        source: std::io::Error,
    },
    /// The command line is malformed.
    Usage(String),
}

impl Error {
    /// Build a v3 rule failure with a stable code.
    #[must_use]
    pub fn rule(code: &'static str, field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Rule {
            code,
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Parse { path, message } => {
                write!(formatter, "cannot parse {}: {message}", path.display())
            }
            Self::Invalid { field, message } => write!(formatter, "{field}: {message}"),
            Self::Rule {
                code,
                field,
                message,
            } => write!(formatter, "{code} at {field}: {message}"),
            Self::MissingSecret { name } => {
                write!(formatter, "the secrets file does not define {name}")
            }
            Self::Write { path, source } => {
                write!(formatter, "cannot write {}: {source}", path.display())
            }
            Self::Usage(message) => write!(formatter, "{message}"),
        }
    }
}

impl std::error::Error for Error {}
