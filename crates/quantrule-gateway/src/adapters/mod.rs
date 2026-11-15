//! The gateway's repository adapters.
//!
//! The backend clients live in `quantrule-runtime`; these are the Postgres and
//! SQLite implementations of the gateway profile store. The data, run,
//! research, and evidence repositories moved to their service crates in P3 and
//! P5.

pub mod postgres;
pub mod sqlite;
