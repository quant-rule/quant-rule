//! Runtime-history rendering.
//!
//! Every kind prints one header and one deterministic row per record. Archived
//! records say `archived=<timestamp>`; active records say `archived=no`.

use std::io::{self, Write};

use quantrule_api::{
    DatasetSummary, ImprovementRunSummary, RunSummary, SimulationStatus, SimulationSummary,
    StageRunSummary,
};

/// Render one history kind.
pub(crate) fn history(
    writer: &mut dyn Write,
    kind: &str,
    runs: &[RunSummary],
    stages: &[StageRunSummary],
    research: &[ImprovementRunSummary],
    simulations: &[SimulationSummary],
    datasets: &[DatasetSummary],
) -> io::Result<()> {
    match kind {
        "run" => run_list(writer, runs),
        "stage" => stage_list(writer, stages),
        "research" => research_list(writer, research),
        "simulation" => simulation_list(writer, simulations),
        "dataset" => dataset_list(writer, datasets),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown history kind {kind}"),
        )),
    }
}

/// Render the run history table.
pub(crate) fn run_list(writer: &mut dyn Write, rows: &[RunSummary]) -> io::Result<()> {
    writeln!(writer, "runs: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} status={} data={} submitted={} archived={}",
            row.run_id,
            row.status.as_str(),
            row.data_version,
            row.submitted_at,
            archived_text(row.archived_at.as_deref())
        )?;
    }
    Ok(())
}

/// Render the stage history table.
pub(crate) fn stage_list(writer: &mut dyn Write, rows: &[StageRunSummary]) -> io::Result<()> {
    writeln!(writer, "stages: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} kind={} status={} submitted={}",
            row.stage_run_id,
            row.kind.as_str(),
            row.state.as_str(),
            row.submitted_at
        )?;
    }
    Ok(())
}

/// Render the research history table.
pub(crate) fn research_list(
    writer: &mut dyn Write,
    rows: &[ImprovementRunSummary],
) -> io::Result<()> {
    writeln!(writer, "research: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} tier={} state={} created={} archived={}",
            row.improvement_id,
            row.tier.as_str(),
            row.state.as_str(),
            row.created_at,
            archived_text(row.archived_at.as_deref())
        )?;
    }
    Ok(())
}

/// Render the simulation history table.
pub(crate) fn simulation_list(
    writer: &mut dyn Write,
    rows: &[SimulationSummary],
) -> io::Result<()> {
    writeln!(writer, "simulations: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} status={} windows={} archived={}",
            row.sim_id.0,
            simulation_status(row.status),
            row.windows,
            archived_text(row.archived_at.as_deref())
        )?;
    }
    Ok(())
}

/// Render the dataset history table.
pub(crate) fn dataset_list(writer: &mut dyn Write, rows: &[DatasetSummary]) -> io::Result<()> {
    writeln!(writer, "datasets: {}", rows.len())?;
    for row in rows {
        writeln!(
            writer,
            "{} name={} candles={} bytes={} archived={}",
            row.data_version,
            row.name,
            row.candles,
            row.bytes,
            archived_text(row.archived_at.as_deref())
        )?;
    }
    Ok(())
}

/// Render one archive receipt.
pub(crate) fn archived(writer: &mut dyn Write, kind: &str, id: &str) -> io::Result<()> {
    writeln!(writer, "archived {kind} {id}")
}

/// Render one erase receipt.
pub(crate) fn erased(writer: &mut dyn Write, kind: &str, id: &str, bytes: u64) -> io::Result<()> {
    writeln!(writer, "erased {kind} {id} bytes={bytes}")
}

/// Render a dry-run intent.
pub(crate) fn dry_run(
    writer: &mut dyn Write,
    action: &str,
    kind: &str,
    id: &str,
) -> io::Result<()> {
    writeln!(writer, "would {action} {kind} {id}")
}

fn archived_text(value: Option<&str>) -> &str {
    value.unwrap_or("no")
}

fn simulation_status(status: SimulationStatus) -> &'static str {
    match status {
        SimulationStatus::Draft => "draft",
        SimulationStatus::Marking => "marking",
        SimulationStatus::Reviewed => "reviewed",
        SimulationStatus::Checked => "checked",
        SimulationStatus::Decided => "decided",
    }
}
