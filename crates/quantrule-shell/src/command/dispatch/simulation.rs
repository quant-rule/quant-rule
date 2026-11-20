//! The simulation command handlers.

use std::io::Write;

use quantrule_api::{CreateSimulationRequest, Digest, SimulationSummary};
use serde_json::{Value, json};

use super::path::with_query;
use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;

pub(super) fn simulation_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    status: Option<quantrule_api::SimulationStatus>,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(status) = status {
        query.push(format!("status={}", simulation_status(status)));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let path = with_query("/simulation/runs", &query);
    let response = client.get(&path)?;
    let rows: Vec<SimulationSummary> = decode_json!(Vec<SimulationSummary>, response, &path);
    output::emit(globals.json, json_view!(&rows), stdout, |writer| {
        output::simulation_list(writer, &rows)
    })
}

pub(super) struct SimulationInput<'a> {
    pub(super) premise: &'a str,
    pub(super) bundle: &'a Digest,
    pub(super) inputs: &'a str,
    pub(super) seed: u64,
    pub(super) criteria: &'a str,
}

pub(super) fn simulation_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    input: &SimulationInput<'_>,
) -> Result<(), RunError> {
    let request = CreateSimulationRequest {
        premise: input.premise.to_owned(),
        bundle: input.bundle.clone(),
        inputs: input.inputs.to_owned(),
        seed: input.seed,
        criteria: input.criteria.to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the simulation request", error))?;
    let response = client.post_json("/simulation/runs", &body)?;
    let value: Value = decode_json!(Value, response, "/simulation/runs");
    let sim_id = value
        .get("sim_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RunError::Decode {
            path: String::from("/simulation/runs"),
            message: String::from("the response carries no sim_id"),
        })?
        .to_owned();
    output::emit(globals.json, value, stdout, |writer| {
        writeln!(writer, "sim_id: {sim_id}")
    })
}

pub(super) fn simulation_windows(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/windows", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

pub(super) struct MarkInput<'a> {
    pub(super) sim_id: &'a str,
    pub(super) chart: &'a str,
    pub(super) bar: u64,
    pub(super) action: quantrule_api::MarkAction,
    pub(super) quality: quantrule_api::MarkQuality,
    pub(super) side: quantrule_api::Side,
    pub(super) note: &'a str,
}

pub(super) fn simulation_mark(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    mark: &MarkInput<'_>,
) -> Result<(), RunError> {
    let body = json!({
        "chart": mark.chart,
        "bar": mark.bar,
        "action": mark.action,
        "quality": mark.quality,
        "side": mark.side,
        "note": mark.note,
    })
    .to_string();
    let path = format!("/simulation/runs/{}/marks", encode(mark.sim_id));
    let response = client.post_json(&path, &body)?;
    let value: Value = decode_json!(Value, response, &path);
    let mark_id = value
        .get("mark_id")
        .and_then(Value::as_str)
        .ok_or_else(|| RunError::Decode {
            path: path.clone(),
            message: String::from("the response carries no mark_id"),
        })?
        .to_owned();
    output::emit(globals.json, value, stdout, |writer| {
        writeln!(writer, "mark_id: {mark_id}")
    })
}

pub(super) fn simulation_marks(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
    origin: Option<quantrule_api::MarkOrigin>,
) -> Result<(), RunError> {
    let path = match origin {
        Some(origin) => format!(
            "/simulation/runs/{}/marks?origin={}",
            encode(sim_id),
            mark_origin(origin)
        ),
        None => format!("/simulation/runs/{}/marks", encode(sim_id)),
    };
    let response = client.get(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

pub(super) fn simulation_oracle(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/oracle", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_records(writer, &value)
    })
}

pub(super) fn simulation_check(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
) -> Result<(), RunError> {
    let path = format!("/simulation/runs/{}/check", encode(sim_id));
    let response = client.post_empty(&path)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit(globals.json, value.clone(), stdout, |writer| {
        output::simulation_check(writer, &value)
    })
}

pub(super) fn simulation_decide(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    sim_id: &str,
    decision: &str,
    note: &str,
) -> Result<(), RunError> {
    let body = json!({ "decision": decision, "note": note }).to_string();
    let path = format!("/simulation/runs/{}/decision", encode(sim_id));
    let response = client.post_json(&path, &body)?;
    let value: Value = decode_json!(Value, response, &path);
    output::emit_raw(globals.json, value.clone(), stdout, |writer| {
        output::write_json_line(writer, &value)
    })
}

pub(super) fn simulation_status(status: quantrule_api::SimulationStatus) -> &'static str {
    match status {
        quantrule_api::SimulationStatus::Draft => "draft",
        quantrule_api::SimulationStatus::Marking => "marking",
        quantrule_api::SimulationStatus::Reviewed => "reviewed",
        quantrule_api::SimulationStatus::Checked => "checked",
        quantrule_api::SimulationStatus::Decided => "decided",
    }
}

pub(super) fn mark_origin(origin: quantrule_api::MarkOrigin) -> &'static str {
    match origin {
        quantrule_api::MarkOrigin::Human => "human",
        quantrule_api::MarkOrigin::Oracle => "oracle",
    }
}
