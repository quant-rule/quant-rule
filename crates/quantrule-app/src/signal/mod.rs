//! The signal artifact and the signal-mode reference position.
//!
//! A signal artifact is the reusable alpha output of a frozen model artifact
//! and a signal-mode strategy pack. It is content-addressed, causal, and
//! independent of execution and portfolio policy: the same model artifact plus
//! pack plus schema always produces the same bytes.
//!
//! The module is pure. It reads a published [`crate::stages::strategy::ModelBundle`]
//! and a validated [`crate::pack::StrategyPack`], calls the existing strategy
//! engine, and returns the artifact. It performs no I/O, reads no clock, uses no
//! randomness, and never reaches a service, a registry, or a client. The
//! strategy service is a separate adapter over this API.
//!
//! Fold-mode packs are refused. Giveback is a fold-only exit until candles and
//! MFE become an explicit signal input; a signal-mode pack already refuses a
//! non-null giveback, and the reference returns an explicit error rather than
//! silently dropping a giveback exit it cannot compute.

pub mod artifact;
mod compute;
mod error;
mod reference;

#[cfg(test)]
mod tests;

pub use artifact::{
    SIGNAL_ARTIFACT_SCHEMA, SignalAction, SignalArtifact, SignalIntent, SignalReason, SignalRow,
    SignalSide,
};
pub use compute::{SignalComputeRequest, compute};
pub use error::SignalError;
