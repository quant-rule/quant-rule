//! Simulation list and check rendering.

use std::io::{self, Write};

use quantrule_api::{SimulationStatus, SimulationSummary};
use serde_json::Value;

/// Render the simulation table.
pub(crate) fn simulation_list(
    writer: &mut dyn Write,
    simulations: &[SimulationSummary],
) -> io::Result<()> {
    let mut rows = simulations.to_vec();
    rows.sort_by(|left, right| left.sim_id.cmp(&right.sim_id));
    writeln!(writer, "simulations: {}", rows.len())?;
    for simulation in rows {
        writeln!(
            writer,
            "{} status={} windows={} marks={} premise={}",
            simulation.sim_id,
            simulation_status(simulation.status),
            simulation.windows,
            simulation.marks,
            simulation.premise
        )?;
    }
    Ok(())
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
/// Render the simulation check table and verdict.
///
/// The check payload is a research diagnostic; the shell prints its numbers
/// and the side verdicts without adding a label.
pub(crate) fn simulation_check(writer: &mut dyn Write, value: &Value) -> io::Result<()> {
    let field = |name: &str| {
        value
            .get(name)
            .map_or_else(|| String::from("-"), Value::to_string)
    };
    writeln!(
        writer,
        "check: {}",
        value
            .get("verdict")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    )?;
    writeln!(writer, "events: {}", field("events"))?;
    writeln!(writer, "mean_net: {}", field("mean_net"))?;
    writeln!(writer, "hit_rate: {}", field("hit_rate"))?;
    writeln!(writer, "control: {}", field("control"))?;
    writeln!(writer, "control_mean_net: {}", field("control_mean_net"))?;
    writeln!(writer, "control_hit_rate: {}", field("control_hit_rate"))?;
    writeln!(
        writer,
        "positive_half_years: {}",
        field("positive_half_years")
    )?;
    writeln!(writer, "passed: {}", field("passed"))?;
    for failure in value
        .get("failures")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writeln!(writer, "failure: {}", failure.as_str().unwrap_or("-"))?;
    }
    for side in value
        .get("sides")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        writeln!(
            writer,
            "side {}: {}",
            side.get("side").and_then(Value::as_str).unwrap_or("-"),
            side.get("verdict").and_then(Value::as_str).unwrap_or("-")
        )?;
    }
    Ok(())
}
