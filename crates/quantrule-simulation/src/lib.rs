//! The research plane.
//!
//! Hindsight by design: a window of history is rendered, a human marks good and
//! bad entries, and a deterministic oracle marks the same window. Nothing here
//! reaches the signal path, and nothing here is a run.
//!
//! This crate holds the deterministic parts: window selection, the mark schema,
//! the hindsight oracle, the path label, the exit lab, the review, the check,
//! and the meta-label and context split.

#![forbid(unsafe_code)]

pub mod check;
pub mod exit;
pub mod marks;
pub mod meta;
pub mod oracle;
pub mod path;
pub mod review;
pub mod window;

pub use check::{Check, CheckError, Criteria, Failure, check};
pub use exit::{Bar as ExitBar, ExitError, ExitPolicy, ExitResult, evaluate as evaluate_exit, lab};
pub use marks::{Mark, MarkAction, MarkError, MarkOrigin, MarkQuality, Side};
pub use meta::{
    Bar as MetaBar, Cell, Check as MetaCheck, Config as MetaConfig, HalfYear, Mark as MetaMark,
    MetaError, MetaResult, Row as MetaRow, Stats, Verdict, build as build_meta,
    evaluate as evaluate_meta,
};
pub use oracle::{Bar, OracleError, label};
pub use path::{FirstTouch, Path, PathError, classify};
pub use review::{Agreement, Counts, Disagreement, Review, review};
pub use window::{Window, WindowError, select};
