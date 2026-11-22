//! The strategy service.
//!
//! The service is a stateless adapter over `quantrule_app::signal`. It resolves
//! a frozen model bundle and a frozen signal-mode strategy pack, computes the
//! signal, writes the canonical bytes directly to the object store, and asks the
//! data service to record the catalog row.
//!
//! It owns no database, no queue and no migration. It makes one pack resolution
//! and one model resolution per request, never one per bar. Fold-mode packs stay
//! inside the backtest fold and are refused here.

#![forbid(unsafe_code)]

pub mod error;
pub mod resolve;
pub mod routes;
pub mod state;

pub use routes::router;
pub use state::{StrategyServices, StrategyState};
