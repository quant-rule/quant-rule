//! The signal artifact and signal computation error.
//!
//! The error names the failing input and never a stack trace. A document error
//! carries the shared configuration error, which names the file and JSON path;
//! a source mismatch names the field whose content address disagrees.

use std::fmt;

use crate::config::ConfigError;
use crate::stages::strategy::StrategyContextError;

/// A signal artifact or signal computation failed.
#[derive(Debug)]
pub enum SignalError {
    /// The artifact document does not match the signal contract.
    Document(ConfigError),
    /// Stored bytes parse but are not the canonical signal artifact bytes.
    NonCanonical {
        /// The artifact name used in the message.
        file: String,
    },
    /// A fold-mode pack has no reusable signal artifact.
    FoldMode,
    /// Signal mode cannot compute a giveback exit.
    GivebackUnsupported,
    /// The model bundle does not publish the execution timeframe.
    MissingTimeframe {
        /// The missing timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The model bundle publishes the execution timeframe with no bars.
    EmptyExecution {
        /// The empty timeframe in seconds.
        timeframe_secs: u64,
    },
    /// The strategy's execution or context timeframes do not fit the model.
    Context(StrategyContextError),
    /// The model bundle has no snapshot at a published bar.
    MissingBar {
        /// The published bar index.
        bar_index: usize,
    },
    /// A stored artifact names another pack, model artifact, or timeframe.
    SourceMismatch {
        /// The field whose content address disagrees.
        field: &'static str,
        /// The expected wire value.
        expected: String,
        /// The observed wire value.
        found: String,
    },
}

impl fmt::Display for SignalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => error.fmt(formatter),
            Self::NonCanonical { file } => write!(
                formatter,
                "{file} is not a canonical signal artifact; the content id must cover the stored bytes"
            ),
            Self::FoldMode => formatter.write_str(
                "a fold-mode strategy pack has no signal artifact; it runs inside the backtest fold",
            ),
            Self::GivebackUnsupported => formatter.write_str(
                "signal mode cannot compute a giveback exit; giveback is fold-mode only until candles and MFE are an explicit signal input",
            ),
            Self::MissingTimeframe { timeframe_secs } => write!(
                formatter,
                "the model bundle does not publish the timeframe {timeframe_secs}s"
            ),
            Self::EmptyExecution { timeframe_secs } => write!(
                formatter,
                "the model bundle publishes no bar for the timeframe {timeframe_secs}s"
            ),
            Self::Context(error) => error.fmt(formatter),
            Self::MissingBar { bar_index } => {
                write!(formatter, "the model bundle has no snapshot at bar {bar_index}")
            }
            Self::SourceMismatch {
                field,
                expected,
                found,
            } => write!(
                formatter,
                "the signal artifact {field} is {found}; the frozen input is {expected}"
            ),
        }
    }
}

impl std::error::Error for SignalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Document(error) => Some(error),
            Self::Context(error) => Some(error),
            Self::NonCanonical { .. }
            | Self::FoldMode
            | Self::GivebackUnsupported
            | Self::MissingTimeframe { .. }
            | Self::EmptyExecution { .. }
            | Self::MissingBar { .. }
            | Self::SourceMismatch { .. } => None,
        }
    }
}

impl From<ConfigError> for SignalError {
    fn from(error: ConfigError) -> Self {
        Self::Document(error)
    }
}
