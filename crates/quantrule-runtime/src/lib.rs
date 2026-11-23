//! The QuantRule service runtime.
//!
//! Configuration, logging, health, storage, queue, migrations, shutdown, and the
//! internal client contract, shared by every binary. A service supplies its own
//! probes, migrations, and routes; nothing here decides a measurement or a
//! trade.
//!
//! The crate depends on no engine and no `quantrule-app`. That boundary is what
//! lets the gateway link the runtime without linking the pipeline.

#![forbid(unsafe_code)]

pub mod adapters;
pub mod backends;
pub mod client;
pub mod clock;
pub mod config;
pub mod error;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod migrations;
pub mod observability;
pub mod queue;
pub mod reconnect;
pub mod request_id;
pub mod retry;
pub mod serve;
pub mod shutdown;

pub use config::Config;
pub use error::RuntimeError;
pub use health::HealthRegistry;
pub use serve::{serve, serve_config, serve_on};
pub use shutdown::{Drain, Shutdown, ShutdownTrigger, watch_signal};
