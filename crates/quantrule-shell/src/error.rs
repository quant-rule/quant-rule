//! The shell's failure vocabulary.
//!
//! The shell is a client: its failures are a refused call, an unreachable
//! gateway, a response that does not match the contract, or a local file it
//! cannot read. The error envelope's stable code is kept as a string so a code
//! this build does not know still reaches the operator instead of becoming a
//! generic failure.

use std::fmt;
use std::io;

/// A command failed before it could print its result.
#[derive(Debug)]
pub(crate) enum RunError {
    /// The gateway answered with the shared error envelope.
    Api {
        /// The stable error code, such as `run.not_found`.
        code: String,
        /// The message a human can act on.
        message: String,
    },
    /// The gateway could not be reached or the HTTP exchange failed.
    Transport(String),
    /// The response body is not the contract type the command expects.
    Decode {
        /// The path that was called.
        path: String,
        /// The decoder's message.
        message: String,
    },
    /// A local file could not be read or written.
    File {
        /// The path involved.
        path: String,
        /// The operating-system message.
        message: String,
    },
    /// The stored profile or the environment is unusable.
    Profile(String),
    /// A command line is invalid.
    Usage(String),
    /// Writing the result failed.
    Output(io::Error),
}

impl RunError {
    /// Build an API failure from the envelope's strings.
    pub(crate) fn api(code: &str, message: &str) -> Self {
        Self::Api {
            code: code.to_owned(),
            message: message.to_owned(),
        }
    }

    /// Build a decode failure for one path.
    pub(crate) fn decode(path: &str, error: impl fmt::Display) -> Self {
        Self::Decode {
            path: path.to_owned(),
            message: error.to_string(),
        }
    }

    /// Build a local file failure for one path.
    pub(crate) fn file(path: impl fmt::Display, error: impl fmt::Display) -> Self {
        Self::File {
            path: path.to_string(),
            message: error.to_string(),
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Api { code, message } => write!(formatter, "{code}: {message}"),
            Self::Transport(message) => write!(formatter, "cannot reach the gateway: {message}"),
            Self::Decode { path, message } => {
                write!(
                    formatter,
                    "the response from {path} is not valid JSON: {message}"
                )
            }
            Self::File { path, message } => write!(formatter, "cannot use {path}: {message}"),
            Self::Profile(message) => formatter.write_str(message),
            Self::Usage(message) => formatter.write_str(message),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Output(error) => Some(error),
            _ => None,
        }
    }
}
