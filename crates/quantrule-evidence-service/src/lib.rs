//! The evidence service.
//!
//! Evidence owns the `runs`, `research`, and diagnostic `stage` schemas, the run
//! state machine, the canonical digest, provenance, research memory, and the
//! stage lifecycle. It serves the public run and hypothesis surface the gateway
//! proxies and the internal lifecycle the backtest worker calls. It computes no
//! reading and no trade; the gateway validates the profile and injects the
//! header. A stage row is never evidence, never a baseline, and never part of a
//! canonical digest.

#![forbid(unsafe_code)]

pub mod evidence;
pub mod migrations;
pub mod operations;
pub mod postgres;
pub mod research;
pub mod routes;
pub mod runs;
pub mod sqlite;
pub mod stage;
pub mod state;

pub use routes::router;
pub use state::{EvidenceServices, EvidenceState};
