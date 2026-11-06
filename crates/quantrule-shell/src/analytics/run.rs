//! The `analytics evaluate` command: descriptive evaluation of result artifacts.
//!
//! The command reads a backtest bundle, builds the analytics metrics and tables,
//! and writes the analytics bundle. It writes no verdict, no acceptance, and no
//! strategy decision.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

use serde_json::json;

use crate::command::types::AnalyticsEvaluateArgs;
use crate::render_report::{self, BundleError};

use super::evidence::{AnalyticsEvidence, AnalyticsEvidenceError};
use super::metrics;
use super::render;

/// The analytics command failed.
#[derive(Debug)]
pub(crate) enum AnalyticsRunError {
    /// The evidence bundle is missing or invalid.
    Evidence(AnalyticsEvidenceError),
    /// The analytics bundle could not be written.
    Bundle(BundleError),
    /// An analytics artifact could not be encoded.
    Json(serde_json::Error),
    /// The writer failed.
    Output(io::Error),
}

impl fmt::Display for AnalyticsRunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evidence(error) => error.fmt(formatter),
            Self::Bundle(error) => error.fmt(formatter),
            Self::Json(error) => {
                write!(formatter, "cannot encode the analytics artifact: {error}")
            }
            Self::Output(error) => write!(formatter, "cannot write output: {error}"),
        }
    }
}

impl std::error::Error for AnalyticsRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Evidence(error) => Some(error),
            Self::Bundle(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

/// Runs `analytics evaluate`.
///
/// # Errors
///
/// Returns [`AnalyticsRunError`] when the evidence bundle, the output target, or
/// the writer fails.
pub(crate) fn run(
    args: &AnalyticsEvaluateArgs,
    writer: &mut impl Write,
) -> Result<(), AnalyticsRunError> {
    let evidence = AnalyticsEvidence::load(&args.evidence).map_err(AnalyticsRunError::Evidence)?;
    let metrics = metrics::build(&evidence);

    let metrics_json =
        render::metrics_json_bytes(&evidence, &metrics).map_err(AnalyticsRunError::Json)?;
    let summary = render::summary_bytes(&evidence, &metrics).into_bytes();
    let provenance =
        render::provenance_json_bytes(&evidence, &metrics).map_err(AnalyticsRunError::Json)?;

    let out = args
        .out
        .clone()
        .or_else(|| args.evidence.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    let root = out.join("analytics");

    let mut artifacts: Vec<(&str, Vec<u8>)> = Vec::with_capacity(metrics.tables.len() + 2);
    for table in &metrics.tables {
        artifacts.push((table.file.as_str(), table.text.as_bytes().to_vec()));
    }
    artifacts.push(("metrics.json", metrics_json));
    artifacts.push(("summary.txt", summary));
    render_report::write_stage_bundle(&root, &artifacts, &provenance, "metrics.json")
        .map_err(AnalyticsRunError::Bundle)?;

    if args.json {
        let value = json!({
            "schema": "quantrule.analytics-metrics/v1",
            "bundle": &root,
            "metrics": root.join("metrics.json"),
            "tables": root.join("tables"),
            "summary": root.join("summary.txt"),
            "trade_count": evidence.portfolio.trades,
            "table_count": metrics.tables.len(),
        });
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&value).map_err(AnalyticsRunError::Json)?
        )
        .map_err(AnalyticsRunError::Output)?;
    } else {
        writeln!(writer, "metrics: {}", root.join("metrics.json").display())
            .map_err(AnalyticsRunError::Output)?;
        writeln!(writer, "tables: {}", root.join("tables").display())
            .map_err(AnalyticsRunError::Output)?;
        writeln!(writer, "summary: {}", root.join("summary.txt").display())
            .map_err(AnalyticsRunError::Output)?;
    }
    writer.flush().map_err(AnalyticsRunError::Output)
}

#[cfg(test)]
mod tests {
    use super::AnalyticsRunError;

    #[test]
    fn the_error_display_names_the_stage() {
        let error = AnalyticsRunError::Output(std::io::Error::other("closed"));
        assert!(error.to_string().contains("cannot write output"));
    }
}
