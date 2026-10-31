//! Errors raised by the causal fill simulator.

use super::{FillError, OrderError, PositionError};

/// A fill or decision could not be simulated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    /// A decision was submitted before any bar had been advanced.
    SubmitBeforeAdvance,
    /// A second order was submitted while one was already pending.
    PendingOrderExists,
    /// The decision carries a different bar index than the bar it was given
    /// with.
    DecisionBarMismatch {
        /// The engine's next bar index.
        expected: usize,
        /// The decision's bar index.
        found: usize,
    },
    /// The decision carries a different open time than the bar it was given
    /// with.
    DecisionTimeMismatch {
        /// The bar's open time.
        expected: i64,
        /// The decision's open time.
        found: i64,
    },
    /// The configured slippage is at least 100% of the reference price.
    SlippageAtOrAboveOne,
    /// A bracket fill was labelled as an entry or exit, which cannot happen.
    UnexpectedFillReason,
    /// An order could not be constructed.
    Order(OrderError),
    /// A fill could not be constructed.
    Fill(FillError),
    /// A position could not be constructed.
    Position(PositionError),
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubmitBeforeAdvance => {
                write!(
                    formatter,
                    "a decision cannot be submitted before a bar is advanced"
                )
            }
            Self::PendingOrderExists => {
                write!(formatter, "a pending order already exists for the next bar")
            }
            Self::DecisionBarMismatch { expected, found } => write!(
                formatter,
                "the decision is for bar {found}, but the engine expects bar {expected}"
            ),
            Self::DecisionTimeMismatch { expected, found } => write!(
                formatter,
                "the decision is for time {found}, but the bar opens at {expected}"
            ),
            Self::SlippageAtOrAboveOne => {
                write!(formatter, "the execution slippage must be below 100%")
            }
            Self::UnexpectedFillReason => {
                write!(formatter, "a bracket fill must be a stop or a target")
            }
            Self::Order(source) => source.fmt(formatter),
            Self::Fill(source) => source.fmt(formatter),
            Self::Position(source) => source.fmt(formatter),
        }
    }
}

impl std::error::Error for ExecutionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Order(source) => Some(source),
            Self::Fill(source) => Some(source),
            Self::Position(source) => Some(source),
            _ => None,
        }
    }
}
