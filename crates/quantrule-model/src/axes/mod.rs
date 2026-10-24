//! The analytical model's four axes.
//!
//! Each axis module turns the indicator readings into one published primary
//! value and its own per-axis diagnostics. An axis reads only the readings its
//! own definition names; it never reads another axis's output, so a correlated
//! reading cannot become a second vote. The modules are pure: they perform no
//! I/O, read no clock, and use no randomness.
//!
//! Direction and Momentum are implemented here. Structure and Progression
//! follow in later work items and publish their own code families.

pub mod direction;
pub mod momentum;
