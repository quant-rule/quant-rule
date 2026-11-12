//! The portfolio stage: turn fills into trades, equity, funding, and metrics.
//!
//! The stage consumes an execution fills bundle and the run's portfolio
//! configuration, and it reads the execution timeframe candles for marks. It
//! does not decide a signal, a fill, or a strategy rule.

mod fills;
mod run;

pub use run::{PortfolioOutcome, PortfolioRequest, PortfolioRunError, replay};
