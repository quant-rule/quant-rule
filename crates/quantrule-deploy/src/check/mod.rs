//! Deployment checks by contract version.
//!
//! v2 keeps the original HTTP service, worker, and object-store probe and its
//! printed rows. v3 checks the descriptor, placement, pins, and secret names
//! locally, probes the single HTTPS ingress, and offers the read-only remote
//! Docker and overlay inspectors as explicit opt-ins. The default check never
//! starts Docker.

mod local;
mod remote;
mod report;
pub mod v2;
pub mod v3;

pub use v3::CheckOptions;
