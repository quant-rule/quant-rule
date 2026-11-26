//! The backtest worker over in-memory services.
//!
//! The cases are split by concern so no test file outgrows the file-size rule.

mod cancel;
mod drain;
mod failure;
mod path;
mod portfolio_stage;
mod promotion;
mod scale;
mod source;
mod strategy_stage;
mod support;
mod v5;
