//! The research plane.
//!
//! Hindsight by design: a window of history is rendered, a human marks good and
//! bad entries, and a deterministic oracle marks the same window. Nothing here
//! reaches the signal path, and nothing here is a run.
//!
//! This crate holds the deterministic parts: window selection and the mark
//! schema. The oracle, review, and check follow.

#![forbid(unsafe_code)]

pub mod check;
pub mod marks;
pub mod oracle;
pub mod review;
pub mod window;

pub use check::{Check, CheckError, Criteria, Failure, check};
pub use marks::{Mark, MarkAction, MarkError, MarkOrigin, MarkQuality, Side};
pub use oracle::{Bar, OracleError, label};
pub use review::{Agreement, Counts, Disagreement, Review, review};
pub use window::{Window, WindowError, select};
