//! The backtest worker.
//!
//! The worker loads a catalogued dataset and config from the data client,
//! obtains the model bundle from the model client, runs strategy, execution,
//! and portfolio from that bundle, writes the artifact set to CAS, records the
//! computed digest through the evidence client, and publishes `run.computed`;
//! evidence finalizes the run. It never recomputes the model, renders a label,
//! or accepts a path.

mod resolve;
pub mod retry;
pub mod worker;
pub mod workspace;

pub use retry::{RETRY_DELAYS, RetryDecision, decide};
pub use worker::{ProcessOutcome, WorkerError, WorkerServices, process};
pub use workspace::Workspace;
