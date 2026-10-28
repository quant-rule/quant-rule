//! The strategy rule contract.
//!
//! A strategy reads one closed model bar at a time and returns a causal
//! [`Decision`]. It may hold position state between calls, but it may read only
//! the bar it is given and the bars it has already been given. It never creates
//! an order, a fill, a position, or a P&L record.
//!
//! The first strategy is the M15 axis-agreement rule: long when Direction,
//! Momentum force, and Structure score are all strictly above the long
//! threshold, short when all are strictly below the short threshold, and
//! neutral otherwise. Equality at a threshold is neutral. A missing reading is
//! unavailable, not neutral.
//!
//! The rule is defined in
//! `docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`.

mod config;
mod decision;
mod input;
#[cfg(test)]
mod tests;

pub use config::{
    DEFAULT_LONG_THRESHOLD, DEFAULT_SHORT_THRESHOLD, DEFAULT_TIMEFRAME_SECS, StrategyConfig,
    StrategyConfigError,
};
pub use decision::{Decision, Entry, Exit, ExitReason, Side, Strategy};
pub use input::{Agreement, StrategyInput};
