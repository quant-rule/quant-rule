//! Stage artifact contracts shared by the shell and the desktop.
//!
//! This module owns the provenance reader and the common artifact error. Stage
//! readers consume it instead of reimplementing the schema checks. The model
//! stays numeric; this module only reads and validates identity fields.

pub mod backtest;
pub mod labels;
pub mod model_bundle;
pub mod provenance;
pub mod render_report;
pub mod write;
