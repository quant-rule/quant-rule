//! The shared client-parity routes for the in-process shell fixture.

use super::{FixtureState, Request, Response};

/// Answer a parity operation from the shared fixture, when one is installed.
///
/// Only the read-only operations the two clients share are routed here. Every
/// other command keeps the mutable default fixture, so the existing
/// printed-output tests are unchanged.
pub(super) fn parity_route(
    state: &FixtureState,
    request: &Request,
    segments: &[&str],
) -> Option<Response> {
    let fixture = state.parity.as_ref()?;
    let responses = &fixture["responses"];
    let stage_id = fixture["stage_id"].as_str().unwrap_or_default();
    let strategy_stage_id = fixture["strategy_stage_id"].as_str().unwrap_or_default();
    let portfolio_stage_id = fixture["portfolio_stage_id"].as_str().unwrap_or_default();
    let research_id = fixture["research_id"].as_str().unwrap_or_default();
    match (request.method.as_str(), segments) {
        ("POST", ["research", "runs"]) => {
            return responses
                .get("research_create")
                .map(|value| Response::json(201, value));
        }
        ("POST", ["research", "runs", id, "submit"]) if *id == research_id => {
            return responses
                .get("research_submit")
                .map(|value| Response::json(202, value));
        }
        ("POST", ["research", "runs", id, "decision"]) if *id == research_id => {
            return responses
                .get("research_decision")
                .map(|value| Response::json(200, value));
        }
        ("POST", ["stages", "model", "runs"]) => {
            return responses
                .get("stage_submit")
                .map(|value| Response::json(202, value));
        }
        ("POST", ["stages", "strategy", "runs"]) => {
            return responses
                .get("stage_run_strategy")
                .map(|value| Response::json(202, value));
        }
        ("POST", ["stages", "portfolio", "runs"]) => {
            return responses
                .get("stage_run_portfolio")
                .map(|value| Response::json(202, value));
        }
        ("GET", ["stages", "runs", id]) if *id == strategy_stage_id => {
            return responses
                .get("strategy_stage")
                .map(|value| Response::json(200, value));
        }
        ("GET", ["stages", "runs", id]) if *id == portfolio_stage_id => {
            return responses
                .get("portfolio_stage")
                .map(|value| Response::json(200, value));
        }
        ("GET", ["stages", "runs"]) => {
            return responses
                .get("stage_list")
                .map(|value| Response::json(200, value));
        }
        ("GET", ["stages", "runs", id]) if *id == stage_id => {
            return responses
                .get("stage")
                .map(|value| Response::json(200, value));
        }
        ("POST", ["stages", "runs", id, "cancel"]) if *id == stage_id => {
            return responses
                .get("stage_cancelled")
                .map(|value| Response::json(202, value));
        }
        ("DELETE", ["stages", "runs", id]) if *id == stage_id => {
            return Some(Response::bytes(204, "application/json", Vec::new()));
        }
        ("GET", ["stages", "runs", id, "artifacts", name]) if *id == strategy_stage_id => {
            let artifact = fixture["strategy_artifacts"]
                .as_array()?
                .iter()
                .find(|artifact| artifact["name"].as_str() == Some(*name))?;
            return Some(Response::bytes(
                200,
                "application/octet-stream",
                artifact["body"].as_str()?.as_bytes().to_vec(),
            ));
        }
        ("GET", ["stages", "runs", id, "artifacts", prefix, name]) if *id == portfolio_stage_id => {
            let artifact_name = format!("{prefix}/{name}");
            let artifact = fixture["portfolio_artifacts"]
                .as_array()?
                .iter()
                .find(|artifact| artifact["name"].as_str() == Some(artifact_name.as_str()))?;
            return Some(Response::bytes(
                200,
                "application/octet-stream",
                artifact["body"].as_str()?.as_bytes().to_vec(),
            ));
        }
        ("GET", ["stages", "runs", id, "artifacts", name]) if *id == stage_id => {
            let artifact = fixture["stage_artifacts"]
                .as_array()?
                .iter()
                .find(|artifact| artifact["name"].as_str() == Some(*name))?;
            return Some(Response::bytes(
                200,
                if (*name).ends_with(".json") {
                    "application/json"
                } else {
                    "text/csv"
                },
                artifact["body"].as_str()?.as_bytes().to_vec(),
            ));
        }
        _ => {}
    }
    if request.method != "GET" {
        return None;
    }
    let key = match segments {
        ["health"] => "health",
        ["catalog", "datasets"] => "datasets",
        ["catalog", "datasets", id] if Some(*id) == fixture["dataset_id"].as_str() => "dataset",
        ["catalog", "configs"] => "configs",
        ["runs"] => "runs",
        ["runs", id] if Some(*id) == fixture["run_id"].as_str() => "run",
        ["runs", id, "digest"] if Some(*id) == fixture["run_id"].as_str() => "digest",
        ["research", "runs"] => "research_list",
        ["research", "runs", id] if Some(*id) == Some(research_id) => "research",
        ["research", "runs", id, "compare"] if Some(*id) == Some(research_id) => "research_compare",
        ["research", "baselines"] => "baselines",
        ["research", "baselines", "model"] => "baseline",
        ["research", "invalidations"] => "invalidations",
        ["inspect", "candles"] => "candles",
        ["inspect", "timeframes"] => "timeframes",
        ["inspect", "events"] => "events",
        ["profiles"] => "profiles",
        ["runs", id, "artifacts", ..] if Some(*id) == fixture["run_id"].as_str() => {
            return Some(Response::bytes(
                200,
                "application/octet-stream",
                state.artifact_bytes.clone(),
            ));
        }
        _ => return None,
    };
    responses.get(key).map(|value| Response::json(200, value))
}
