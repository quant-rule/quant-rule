//! The service runtime.
//!
//! Configuration, logging, health, storage, queue, and shutdown, shared by every
//! binary. A service supplies its own probes and routes; nothing here decides a
//! measurement or a trade.

pub mod adapters;
pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod migrations;
pub mod server;
pub mod shutdown;

pub use config::Config;
pub use error::RuntimeError;
pub use health::HealthRegistry;
pub use server::AppState;
