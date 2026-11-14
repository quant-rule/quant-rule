//! Static reference topics for the shell help.
//!
//! The glossary and workflow selection guide are static text. They define the
//! precise terms the command output uses and the workflow that owns a change.

use std::io::{self, Write};

/// Renders the precise-term glossary.
pub(super) fn render_glossary(writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "Glossary")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "data_version  Content identity of the normalized candles; the only name a dataset has."
    )?;
    writeln!(
        writer,
        "run           The full pipeline over one dataset and one config, with a content-derived id."
    )?;
    writeln!(
        writer,
        "stage         One data-plane step: data, model, backtest, or evidence."
    )?;
    writeln!(
        writer,
        "artifact      A file the run produced, named by its hash and size."
    )?;
    writeln!(
        writer,
        "digest        SHA-256 over the run's data artifacts; it excludes provenance and logs."
    )?;
    writeln!(
        writer,
        "profile       An organisational scope for datasets, runs, and simulations."
    )?;
    writeln!(
        writer,
        "simulation    A chart-first research run with a recorded bundle and human marks."
    )?;
    writeln!(
        writer,
        "mark          A human or oracle judgment on one bar of a simulation window."
    )?;
    writeln!(
        writer,
        "oracle        The deterministic hindsight labeler; idempotent for one inputs digest."
    )?;
    writeln!(
        writer,
        "check         The event study over an unmarked window; it reports, it does not decide."
    )?;
    writeln!(
        writer,
        "profile scope Not a security boundary. One principal; no isolation promise."
    )
}

/// Writes one row of the workflow selection table.
///
/// The column widths are fixed so the rendered text does not depend on the
/// terminal, which keeps the help output deterministic.
fn workflow_row(
    writer: &mut impl Write,
    intention: &str,
    workflow: &str,
    start: &str,
) -> io::Result<()> {
    writeln!(writer, "  {intention:<42} {workflow:<26} {start}")
}

/// Renders the workflow selection guide.
pub(super) fn render_workflows(writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "Workflow selection")?;
    writeln!(writer)?;
    writeln!(
        writer,
        "Choose the workflow by your intention, not by the command you happen to know."
    )?;
    writeln!(
        writer,
        "The full contracts are in docs/workflows/README.md."
    )?;
    writeln!(writer)?;
    workflow_row(writer, "If you want to...", "Workflow", "Start with")?;
    workflow_row(
        writer,
        "Find or fix a defect",
        "Audit",
        "audit document, then remediation plan",
    )?;
    workflow_row(
        writer,
        "Restructure without behavior change",
        "Refactor",
        "refactor assessment, then plan",
    )?;
    workflow_row(
        writer,
        "Inspect candles or timeframes",
        "Data Lifecycle",
        "inspect candles, inspect timeframes",
    )?;
    workflow_row(
        writer,
        "Inspect events or the model report",
        "Model Research",
        "inspect events, run submit",
    )?;
    workflow_row(
        writer,
        "Test a strategy hypothesis",
        "Strategy Research",
        "run submit",
    )?;
    workflow_row(
        writer,
        "Test execution or portfolio mechanics",
        "Portfolio Research",
        "run submit",
    )?;
    workflow_row(
        writer,
        "Explore historical charts",
        "Simulation",
        "simulation create",
    )?;
    workflow_row(
        writer,
        "Change a data source or partition",
        "Data Lifecycle",
        "dataset upload, data/partitions.json",
    )?;
    workflow_row(
        writer,
        "Add a command or report",
        "Requirements & Design",
        "requirements record, then plan",
    )?;
    workflow_row(
        writer,
        "Release or tag",
        "Release & Operations",
        "release note, then gate",
    )?;
    workflow_row(
        writer,
        "Retire a model or command",
        "Deprecation & Retirement",
        "retirement record",
    )
}
