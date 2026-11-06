//! The model-bundle writer failure.
//!
//! The writer is shared by `report model` and `backtest run`; its error is
//! neutral so the command layer can wrap it without making the writer depend on
//! the command vocabulary. The messages are the ones the shell printed before
//! the move, and the tests lock them.

use std::fmt;
use std::io;

use crate::render_report::BundleError;

/// A model bundle could not be assembled or written.
#[derive(Debug)]
pub(crate) enum ModelBundleError {
    /// The requested period selected no candle.
    EmptyPeriod {
        /// The requested period as written.
        period: String,
    },
    /// The bundle directory is unavailable.
    Bundle(BundleError),
    /// An artifact could not be rendered.
    Output(io::Error),
}

impl fmt::Display for ModelBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPeriod { period } => {
                write!(
                    formatter,
                    "no candle falls inside the requested period {period}"
                )
            }
            Self::Bundle(error) => error.fmt(formatter),
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for ModelBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Bundle(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::EmptyPeriod { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModelBundleError;
    use crate::render_report::BundleError;
    use std::path::PathBuf;

    #[test]
    fn empty_period_message_is_unchanged() {
        let error = ModelBundleError::EmptyPeriod {
            period: "2025-01-01..2025-01-02".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "no candle falls inside the requested period 2025-01-01..2025-01-02"
        );
    }

    #[test]
    fn bundle_message_delegates() {
        let error = ModelBundleError::Bundle(BundleError::OutputNotEmpty {
            path: PathBuf::from("target/model"),
        });

        assert_eq!(
            error.to_string(),
            "output directory target/model must be absent or empty"
        );
    }

    #[test]
    fn output_message_is_unchanged() {
        let error = ModelBundleError::Output(std::io::Error::other("closed"));

        assert_eq!(error.to_string(), "cannot write output: closed");
    }
}
