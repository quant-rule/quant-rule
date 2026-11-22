//! The evidence service.
//!
//! Evidence owns the `runs`, `research`, and diagnostic `stage` schemas, the run
//! state machine, the canonical digest, provenance, research memory, and the
//! stage lifecycle. It serves the public run and hypothesis surface the gateway
//! proxies and the internal lifecycle the backtest worker calls. It computes no
//! reading and no trade; the gateway validates the profile and injects the
//! header. A stage row is never evidence, never a baseline, and never part of a
//! canonical digest.
//!
//! The service also owns the two records that make a cross-database step safe:
//! the run-notification outbox and the dataset-reference release tombstone.
//! Both commit with the evidence row they describe and are retried by a
//! reconciler, so no message is lost after a commit and no reference is left
//! behind after an erase.
//!
//! Two replicas may serve reads. `runs.leader_lease` names the one finalizer;
//! only its term can finish a run, and a standby takes over after the lease
//! expires.

#![forbid(unsafe_code)]

pub mod evidence;
pub mod leader;
pub mod lineage;
pub mod migrations;
pub mod operations;
pub mod outbox;
pub mod postgres;
pub mod postgres_finalize;
pub mod postgres_leader;
pub mod postgres_outbox;
pub mod postgres_runs;
pub mod reconcile;
pub mod releases;
pub mod research;
pub mod routes;
pub mod runs;
pub mod stage;
pub mod state;

pub use routes::router;
pub use state::{EvidenceServices, EvidenceState};
