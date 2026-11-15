//! The evidence service.
//!
//! Evidence owns the `runs` and `research` schemas, the run state machine, the
//! canonical digest, provenance, and research memory. It serves the public run
//! and hypothesis surface the gateway proxies and the internal lifecycle the
//! backtest worker calls. It computes no reading and no trade; the gateway
//! validates the profile and injects the header.

#![forbid(unsafe_code)]

pub mod evidence;
pub mod migrations;
pub mod postgres;
pub mod research;
pub mod routes;
pub mod runs;
pub mod sqlite;
pub mod state;

pub use routes::router;
pub use state::{EvidenceServices, EvidenceState};
