//! The model engine.
//!
//! The analytical model's contract is in `docs/model/`. This crate provides the
//! shared numeric kernel in [`kernel`], the indicator readings in [`readings`],
//! the flat confirmed-level registry in [`levels`], the progression series in
//! [`progression`], the shared event record and event detectors in [`events`],
//! the volatility and participation context in [`context`], the panel and
//! session artifacts, and the flat report in [`report`]. [`input`] owns the run
//! configuration and candle input, [`timeframe`] owns one timeframe's series,
//! and [`run`] orchestrates the ladder.
//!
//! The model publishes readings, discrete states, confirmed levels, events,
//! progression, and context per timeframe. It carries no axis score, no
//! consensus, no dashboard view, and no cross-timeframe aggregate: those were
//! the legacy four-axis architecture and are not part of M1.0.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod context;
pub mod events;
pub mod input;
pub mod kernel;
pub mod levels;
pub mod panel;
pub mod progression;
pub mod readings;
pub mod report;
pub mod retention;
pub mod run;
pub mod sessions;
pub mod spec;
pub mod timeframe;

/// The registry version that defines the published indicator and event codes.
///
/// It must match `docs/registry/indicators.json`; a registry change updates
/// both in the same pull request.
pub const REGISTRY_VERSION: &str = "2026-11-06.1";
