//! The model service.
//!
//! The service loads a dataset by reference through the data service, computes
//! the configured ladder, and stores content-addressed model objects. It owns
//! the public model route and the internal model-bundle reference the backtest
//! worker consumes. It holds no profile, run, strategy, execution, or portfolio
//! state; the gateway validates the profile and injects the header.

#![forbid(unsafe_code)]

mod bundle;
mod model;
mod routes;
mod state;

pub use routes::router;
pub use state::{ModelServices, ModelState};
