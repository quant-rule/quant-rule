//! The pack document error.
//!
//! A pack is a configuration document, so a schema rejection keeps the shared
//! [`ConfigError`] with its file and JSON path. The other variants are specific
//! to content addressing: a parser called for the wrong kind, or stored bytes
//! that are not the canonical form of the document they encode.

use std::fmt;

use crate::config::ConfigError;

use super::envelope::PackKind;

/// A pack document could not be parsed or addressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    /// The document disagrees with its schema.
    Document(ConfigError),
    /// A kind-specific parser received another pack kind.
    WrongKind {
        /// The kind the parser accepts.
        expected: PackKind,
        /// The kind the envelope declares.
        found: PackKind,
    },
    /// Stored bytes parse but are not the canonical bytes of their document.
    NonCanonical {
        /// The label used in the error message.
        file: String,
    },
}

impl fmt::Display for PackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => write!(formatter, "{error}"),
            Self::WrongKind { expected, found } => write!(
                formatter,
                "a {} pack parser cannot read a {} pack",
                expected.code(),
                found.code()
            ),
            Self::NonCanonical { file } => write!(
                formatter,
                "{file}: a stored pack must be canonical JSON bytes"
            ),
        }
    }
}

impl std::error::Error for PackError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            Self::WrongKind { .. } | Self::NonCanonical { .. } => None,
        }
    }
}

impl From<ConfigError> for PackError {
    fn from(error: ConfigError) -> Self {
        Self::Document(error)
    }
}
