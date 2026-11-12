//! The Tauri command boundary.
//!
//! Every command delegates to `quantrule-app`. The views are serializable
//! snapshots of application-service values; the desktop layer never parses a
//! config, computes a reading, or writes SQL.

mod memory;
mod pipeline;
mod scope;
mod views;
mod workspace;

#[cfg(test)]
mod tests;

pub use memory::*;
pub use pipeline::*;
pub use workspace::*;
