//! Shared pipeline use cases.
//!
//! Each module owns one stage's orchestration behind the application service.
//! The shell and the desktop pass explicit paths and options; the module owns
//! file access, configuration loading, and artifact writing, and returns a typed
//! outcome. No module computes an indicator, a rule, a fill, or an accounting
//! value.

pub mod analytics;
pub mod backtest;
pub mod data;
pub mod execution;
pub mod model;
pub mod portfolio;
pub mod qualification;
pub mod strategy;
