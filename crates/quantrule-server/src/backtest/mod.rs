//! The backtest worker.
//!
//! The worker materializes a catalogued dataset and config, runs the existing
//! coupled backtest, writes the artifact set to CAS, computes the canonical
//! digest, and finalizes the run. It never renders a label or accepts a path.

pub mod retry;
pub mod worker;
pub mod workspace;

pub use retry::{RETRY_DELAYS, RetryDecision, decide};
pub use worker::{ProcessOutcome, WorkerError, WorkerServices, process};
pub use workspace::Workspace;
