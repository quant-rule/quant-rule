//! The server's repository adapters.
//!
//! The backend clients live in `quantrule-runtime`; these are the Postgres
//! implementations of the server's domain stores. They move with their domain
//! modules to the service crates in P3 and P5.

pub mod postgres;
