//! The backtest orchestration stage.
//!
//! The runner composes the model, strategy, execution, and portfolio stages
//! into one coupled pass. It writes the same stage bundles as the standalone
//! commands and a manifest that names every artifact.

mod run;

pub(crate) use run::{BacktestRunError, run};
