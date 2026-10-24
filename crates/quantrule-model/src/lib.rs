//! The model engine.
//!
//! The analytical model's contract is in `docs/model/`. Its implementation is
//! tracked as the sub-issues of issue #14. This crate currently provides the
//! shared numeric kernel in [`kernel`], the indicator readings in [`readings`],
//! the Direction, Momentum, Structure, and Progression axes in [`axes`], and
//! the shared event record and event detectors in [`events`], the volatility and
//! participation context in [`context`], and the cross-timeframe aggregates in
//! [`aggregate`]; [`state`] assembles the published market state and consensus.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod aggregate;
pub mod axes;
pub mod context;
pub mod events;
pub mod kernel;
pub mod readings;
pub mod state;
