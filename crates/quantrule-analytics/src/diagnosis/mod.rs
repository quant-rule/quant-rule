//! The cross-engine diagnosis.
//!
//! The diagnosis is a read-only, post-run evaluation of one completed
//! backtest. It turns the run's plain facts into a scorecard, a strategy
//! funnel, data-quality and model-health tables, execution friction, and
//! portfolio attribution.
//!
//! The module is pure: the shell projects a completed run into
//! [`types::DiagnosisInput`], [`compute::diagnose`] computes the evidence, and
//! [`text`] renders deterministic text and CSV. No diagnosis value is read by
//! the strategy, execution, or portfolio engines, and no diagnosis is part of
//! the signal path.

pub mod compute;
pub mod text;
pub mod types;

pub use compute::diagnose;
