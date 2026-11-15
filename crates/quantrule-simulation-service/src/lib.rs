//! The simulation service surface.
//!
//! The research plane is hindsight by design: a recorded model bundle is
//! windowed, a human marks good and bad entries, and a deterministic oracle
//! marks the same windows. Nothing here reaches the signal path, and nothing
//! here is a run.
//!
//! The service owns the `simulation` schema, serves the eleven documented
//! endpoints, and writes only simulation objects. It holds no queue, no run
//! store, no model client, and no strategy input.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod criteria;
pub mod migrations;
pub mod postgres;
pub mod records;
pub mod research;
pub mod routes;
pub mod sqlite;
pub mod state;
pub mod store;

pub use routes::router;
pub use state::{SimulationServices, SimulationState};
