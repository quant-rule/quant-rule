//! The analytical model's four axes.
//!
//! Each axis module turns the indicator readings into one published primary
//! value and its own per-axis diagnostics. An axis reads only the readings its
//! own definition names; it never reads another axis's output, so a correlated
//! reading cannot become a second vote. The modules are pure: they perform no
//! I/O, read no clock, and use no randomness.
//!
//! Direction, Momentum, Structure, and Progression are implemented here. The
//! remaining work items add the events, the cross-timeframe aggregates, and the
//! assembled market state.

pub mod direction;
pub mod momentum;
pub mod progression;
pub mod structure;
