//! The model bundle shared by the report and backtest commands.
//!
//! The module assembles the flat model view, renders every artifact, and writes
//! the bundle. It contains no command parsing, no indicator, and no strategy
//! logic. It lives in the application artifact layer so every adapter shares one
//! model-bundle contract.

mod artifacts;
mod bundle;
mod error;
mod provenance;

pub use bundle::{BundleOutput, SpecRunArtifacts, model_bundle_from, model_bundle_with_spec};
pub use error::ModelBundleError;
pub use provenance::{panel_provenance, resolve_spec_path};

use crate::config::RunConfig;

/// The requested period as written, for the summary and the empty-period error.
pub fn period_label(config: &RunConfig) -> String {
    match (&config.data.from, &config.data.to) {
        (Some(from), Some(to)) => format!("{from}..{to}"),
        (Some(from), None) => format!("{from}..open"),
        (None, Some(to)) => format!("open..{to}"),
        (None, None) => String::from("full file"),
    }
}
