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
        "candle        One closed OHLCV bar. The model consumes candles only."
    )?;
    writeln!(
        writer,
        "timeframe     A first-class candle series. The default ladder is 1m, 5m, 15m, 1h."
    )?;
    writeln!(
        writer,
        "reading       A published indicator value, such as EMA, RSI, ATR, RVOL, MACD, or ADX/DI."
    )?;
    writeln!(
        writer,
        "state         A discrete code derived from a reading, independent of any axis score."
    )?;
    writeln!(
        writer,
        "level         A confirmed swing pivot, with occurrence and availability bars."
    )?;
    writeln!(
        writer,
        "event         A Level, Divergence, or Signal record; it is evidence, not a vote."
    )?;
    writeln!(
        writer,
        "progression   The structural leg at the minor and major scales, with its lifecycle and stage."
    )?;
    writeln!(
        writer,
        "context       Volatility and participation measurements, such as ATR ratio and RVOL ratio."
    )?;
    writeln!(
        writer,
        "availability  The first closed bar on which a measurement became knowable."
    )?;
    writeln!(
        writer,
        "M1.0          The current flat per-timeframe model: readings, states, levels, events, progression, and context."
    )?;
    writeln!(
        writer,
        "legacy axis   The retired Direction, Momentum, Structure, and Progression scores. No current command publishes them."
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
        "candles, candles-tolerant, timeframes",
    )?;
    workflow_row(
        writer,
        "Inspect events or the model report",
        "Model Research",
        "events, report model",
    )?;
    workflow_row(
        writer,
        "Experiment with model configs/events",
        "Model Research + Swarm",
        "report model; tools/research",
    )?;
    workflow_row(
        writer,
        "Add or change an indicator/event",
        "Model Change",
        "registry, validation, verification",
    )?;
    workflow_row(
        writer,
        "Test a strategy rule",
        "Strategy Research",
        "strategy explain/run, backtest run",
    )?;
    workflow_row(
        writer,
        "Test execution/portfolio mechanics",
        "Portfolio Research",
        "execution run, portfolio replay",
    )?;
    workflow_row(
        writer,
        "Validate the run composition",
        "Requirements and Design",
        "run check",
    )?;
    workflow_row(
        writer,
        "Change a data source/version/partition",
        "Data Lifecycle",
        "fetch, data/partitions.json",
    )?;
    workflow_row(
        writer,
        "Add a command, report, or capability",
        "Requirements & Design",
        "plan first",
    )?;
    workflow_row(
        writer,
        "Release, tag, rollback, or incident",
        "Release & Operations",
        "release note, CHANGELOG.md",
    )?;
    workflow_row(
        writer,
        "Retire a model/event/command/source",
        "Deprecation & Retirement",
        "history, invalidation",
    )?;
    workflow_row(
        writer,
        "Record a finding/run/baseline/status",
        "Recordkeeping",
        "docs/workflows/recordkeeping.md",
    )?;
    workflow_row(
        writer,
        "Run a pre-registered improvement",
        "Improvement Workflow",
        "Run Contract",
    )?;
    workflow_row(
        writer,
        "Make a run valid evidence",
        "Run Contract",
        "manifest, results, review, decision",
    )?;
    workflow_row(
        writer,
        "Run parallel pre-registered cells",
        "Research Swarm",
        "coordinator + cell agents",
    )?;
    writeln!(writer)?;
    writeln!(writer, "Model research quick path:")?;
    writeln!(
        writer,
        "  1. State the premise and screen it (data admissibility, sample size)."
    )?;
    writeln!(
        writer,
        "  2. Write a Program Charter; the human approves the cells and partitions."
    )?;
    writeln!(
        writer,
        "  3. Run the cells on model_dev; one cell per config and output."
    )?;
    writeln!(
        writer,
        "  4. Aggregate; the human promotes at most the declared primary cells."
    )?;
    writeln!(
        writer,
        "  5. Run model_validation once; grade A/B/C/D; accept or record no_edge."
    )?;
    writeln!(writer)?;
    writeln!(writer, "Rules:")?;
    writeln!(
        writer,
        "  - Strategy Research starts only after an accepted model baseline."
    )?;
    writeln!(
        writer,
        "  - Portfolio Research starts only after an accepted strategy baseline."
    )?;
    writeln!(
        writer,
        "  - Policy is human-owned; it is not a research tier."
    )?;
    writeln!(
        writer,
        "  - No automatic optimizer. Exploratory cells are not ranked and never open validation."
    )?;
    writeln!(
        writer,
        "  - There is no `swarm` shell command. The swarm is an operating procedure; use"
    )?;
    writeln!(
        writer,
        "    the stage commands and tools/research for the cells."
    )?;
    writeln!(
        writer,
        "  - Stage commands: report model, strategy run, execution run, portfolio replay,"
    )?;
    writeln!(writer, "    backtest run, qualify, analytics evaluate.")?;
    writeln!(writer)?;
    writeln!(writer, "Full index: docs/workflows/README.md")?;
    writeln!(
        writer,
        "Worked lab: docs/tutorials/2026-11-07/workflow-lab.md"
    )
}
