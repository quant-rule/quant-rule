//! The one error type the config loader returns.
//!
//! Every rejection names the file it came from, the JSON path inside that file,
//! what the contract expected, and what was found. The display never includes a
//! stack trace or an absolute path, so a config error can be printed as-is.

use std::fmt;

/// The category of a config rejection.
///
/// The categories mirror the contract's error table. They are separate from the
/// message so tests can assert on the category without matching prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorKind {
    /// A referenced file does not exist or cannot be read.
    MissingFile,
    /// A file is not valid JSON.
    InvalidJson,
    /// The `schema` field is not the expected version.
    Schema,
    /// An object carries a field the schema does not define.
    UnknownField,
    /// A required field is absent.
    MissingField,
    /// A value has the wrong JSON type.
    Type,
    /// A value is outside its allowed range.
    Range,
    /// Two values disagree across files.
    Inconsistent,
    /// A value is valid in the schema but not supported in v1.
    Unsupported,
}

/// A configuration file was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    file: String,
    path: String,
    kind: ConfigErrorKind,
    expected: String,
    observed: String,
}

impl ConfigError {
    fn new(
        file: &str,
        path: &str,
        kind: ConfigErrorKind,
        expected: impl Into<String>,
        observed: impl Into<String>,
    ) -> Self {
        Self {
            file: file.to_owned(),
            path: path.to_owned(),
            kind,
            expected: expected.into(),
            observed: observed.into(),
        }
    }

    /// A referenced file is missing or unreadable.
    pub fn missing_file(file: &str) -> Self {
        Self::new(
            file,
            "$",
            ConfigErrorKind::MissingFile,
            "a readable file",
            "missing",
        )
    }

    /// A file could not be parsed as JSON.
    pub fn invalid_json(file: &str, message: &str) -> Self {
        Self::new(
            file,
            "$",
            ConfigErrorKind::InvalidJson,
            "valid JSON",
            message,
        )
    }

    /// A referenced artifact could not be parsed as JSON.
    pub fn invalid_json_at(file: &str, path: &str, message: &str) -> Self {
        Self::new(
            file,
            path,
            ConfigErrorKind::InvalidJson,
            "valid JSON",
            message,
        )
    }

    /// A content reference could not be resolved to bytes.
    pub fn missing_reference(file: &str, path: &str, expected: &str) -> Self {
        Self::new(
            file,
            path,
            ConfigErrorKind::MissingFile,
            expected,
            "missing",
        )
    }

    /// Moves a nested parser's error under the field that named it.
    ///
    /// A pack parser always reports at `$`, or under `$.payload`. The run
    /// reference that supplied the bytes is the real location a reader needs.
    pub(crate) fn prefix_path(mut self, prefix: &str) -> Self {
        if self.path == "$" {
            self.path = prefix.to_owned();
        } else if let Some(rest) = self.path.strip_prefix("$.") {
            self.path = format!("{prefix}.{rest}");
        }
        self
    }

    /// A referenced file exists but cannot be read.
    pub fn unreadable_file(file: &str) -> Self {
        Self::new(
            file,
            "$",
            ConfigErrorKind::MissingFile,
            "a readable file",
            "unreadable",
        )
    }

    /// The schemas do not match.
    pub fn schema(file: &str, path: &str, expected: &str, observed: &str) -> Self {
        Self::new(file, path, ConfigErrorKind::Schema, expected, observed)
    }

    /// An object carries an unknown field.
    pub fn unknown_field(file: &str, path: &str, field: &str) -> Self {
        Self::new(
            file,
            path,
            ConfigErrorKind::UnknownField,
            "a known field",
            field,
        )
    }

    /// A required field is absent.
    pub fn missing_field(file: &str, path: &str) -> Self {
        Self::new(
            file,
            path,
            ConfigErrorKind::MissingField,
            "a required field",
            "missing",
        )
    }

    /// A value has the wrong JSON type.
    pub fn wrong_type(file: &str, path: &str, expected: &str, observed: &str) -> Self {
        Self::new(file, path, ConfigErrorKind::Type, expected, observed)
    }

    /// A value is outside its allowed range.
    pub fn range(file: &str, path: &str, expected: &str, observed: &str) -> Self {
        Self::new(file, path, ConfigErrorKind::Range, expected, observed)
    }

    /// Two values disagree across files.
    pub fn inconsistent(file: &str, path: &str, expected: &str, observed: &str) -> Self {
        Self::new(
            file,
            path,
            ConfigErrorKind::Inconsistent,
            expected,
            observed,
        )
    }

    /// A value is valid in the schema but not supported in v1.
    pub fn unsupported(file: &str, path: &str, expected: &str, observed: &str) -> Self {
        Self::new(file, path, ConfigErrorKind::Unsupported, expected, observed)
    }

    /// The error category.
    #[cfg(test)]
    pub const fn kind(&self) -> ConfigErrorKind {
        self.kind
    }

    /// The JSON path inside the file.
    #[cfg(test)]
    pub fn path(&self) -> &str {
        &self.path
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "config error: {}: {}: expected {}, found {}",
            self.file, self.path, self.expected, self.observed
        )
    }
}

impl std::error::Error for ConfigError {}
