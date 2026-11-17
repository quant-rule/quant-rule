//! The research, baseline, and invalidation command handlers.
//!
//! Every handler makes one gateway call, decodes the contract type, and hands
//! it to a renderer. The shell never generates an improvement id, computes a
//! comparison, or decides a run.

use std::io::Write;

use crate::client::{Client, encode};
use crate::command::types::GlobalOptions;
use crate::error::RunError;
use crate::output;
use quantrule_api::{
    BaselineRecord, CreateImprovementRequest, DecideImprovementRequest, ImprovementComparison,
    ImprovementDecision, ImprovementPartition, ImprovementRunDetail, ImprovementRunState,
    ImprovementRunSummary, ImprovementTier, InvalidationRecord,
};

macro_rules! decode {
    ($ty:ty, $response:expr, $path:expr) => {
        serde_json::from_slice::<$ty>(&$response.body)
            .map_err(|error| RunError::decode($path, error))?
    };
}

macro_rules! json_value {
    ($value:expr) => {
        serde_json::to_value($value)
            .map_err(|error| RunError::decode("the local JSON view", error))?
    };
}

/// Create one v3 pre-registration.
#[allow(clippy::too_many_arguments)]
pub(crate) fn research_create(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    tier: ImprovementTier,
    hypothesis: &str,
    partition: ImprovementPartition,
    data_version: &str,
    config_id: &str,
    criteria_id: Option<&str>,
    green_criteria: &serde_json::Value,
    baseline: Option<&str>,
    code_commit: &str,
    seed: u64,
    analytics: Option<bool>,
) -> Result<(), RunError> {
    let request = CreateImprovementRequest {
        tier,
        hypothesis: hypothesis.to_owned(),
        partition,
        data_version: quantrule_api::Digest::parse(data_version)
            .map_err(|error| RunError::Usage(format!("--data-version {error}")))?,
        config_id: config_id.to_owned(),
        criteria_id: criteria_id.map(str::to_owned),
        green_criteria: green_criteria
            .as_object()
            .cloned()
            .ok_or_else(|| RunError::Usage(String::from("--green-criteria expects an object")))?,
        baseline: baseline.map(str::to_owned),
        code_commit: code_commit.to_owned(),
        seed,
        analytics,
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the create request", error))?;
    let response = client.post_json("/research/runs", &body)?;
    let detail: ImprovementRunDetail = decode!(ImprovementRunDetail, response, "/research/runs");
    output::emit(globals.json, json_value!(&detail), stdout, |writer| {
        output::improvement_detail(writer, &detail)
    })
}

/// Submit the coupled pipeline run for one pre-registration.
pub(crate) fn research_submit(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    improvement_id: &str,
) -> Result<(), RunError> {
    let path = format!("/research/runs/{}/submit", encode(improvement_id));
    let response = client.post_empty(&path)?;
    let detail: ImprovementRunDetail = decode!(ImprovementRunDetail, response, path.as_str());
    output::emit(globals.json, json_value!(&detail), stdout, |writer| {
        output::improvement_detail(writer, &detail)
    })
}

/// List improvement runs.
pub(crate) fn research_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    tier: Option<ImprovementTier>,
    state: Option<ImprovementRunState>,
    limit: Option<usize>,
) -> Result<(), RunError> {
    let mut query = Vec::new();
    if let Some(tier) = tier {
        query.push(format!("tier={}", tier.as_str()));
    }
    if let Some(state) = state {
        query.push(format!("state={}", state.as_str()));
    }
    if let Some(limit) = limit {
        query.push(format!("limit={limit}"));
    }
    let path = if query.is_empty() {
        String::from("/research/runs")
    } else {
        format!("/research/runs?{}", query.join("&"))
    };
    let response = client.get(&path)?;
    let rows: Vec<ImprovementRunSummary> =
        decode!(Vec<ImprovementRunSummary>, response, path.as_str());
    output::emit(globals.json, json_value!(&rows), stdout, |writer| {
        output::improvement_list(writer, &rows)
    })
}

/// Read one improvement run.
pub(crate) fn research_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    improvement_id: &str,
) -> Result<(), RunError> {
    let path = format!("/research/runs/{}", encode(improvement_id));
    let response = client.get(&path)?;
    let detail: ImprovementRunDetail = decode!(ImprovementRunDetail, response, path.as_str());
    output::emit(globals.json, json_value!(&detail), stdout, |writer| {
        output::improvement_detail(writer, &detail)
    })
}

/// Compare one improvement run with a baseline.
pub(crate) fn research_compare(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    improvement_id: &str,
    baseline: Option<&str>,
) -> Result<(), RunError> {
    let path = match baseline {
        Some(baseline) => format!(
            "/research/runs/{}/compare?baseline={}",
            encode(improvement_id),
            encode(baseline)
        ),
        None => format!("/research/runs/{}/compare", encode(improvement_id)),
    };
    let response = client.get(&path)?;
    let comparison: ImprovementComparison = decode!(ImprovementComparison, response, path.as_str());
    output::emit(globals.json, json_value!(&comparison), stdout, |writer| {
        output::improvement_comparison(writer, &comparison)
    })
}

/// Record one human decision.
pub(crate) fn research_decide(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    improvement_id: &str,
    decision: ImprovementDecision,
    note: &str,
) -> Result<(), RunError> {
    let request = DecideImprovementRequest {
        decision,
        note: note.to_owned(),
    };
    let body = serde_json::to_string(&request)
        .map_err(|error| RunError::decode("the decision request", error))?;
    let path = format!("/research/runs/{}/decision", encode(improvement_id));
    let response = client.post_json(&path, &body)?;
    let detail: ImprovementRunDetail = decode!(ImprovementRunDetail, response, path.as_str());
    output::emit(globals.json, json_value!(&detail), stdout, |writer| {
        output::improvement_detail(writer, &detail)
    })
}

/// List accepted baselines.
pub(crate) fn baseline_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/research/baselines")?;
    let rows: Vec<BaselineRecord> = decode!(Vec<BaselineRecord>, response, "/research/baselines");
    output::emit(globals.json, json_value!(&rows), stdout, |writer| {
        output::baseline_list(writer, &rows)
    })
}

/// Show the current baseline for one tier.
pub(crate) fn baseline_show(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    tier: ImprovementTier,
) -> Result<(), RunError> {
    let path = format!("/research/baselines/{}", tier.as_str());
    let response = client.get(&path)?;
    let row: BaselineRecord = decode!(BaselineRecord, response, path.as_str());
    output::emit(globals.json, json_value!(&row), stdout, |writer| {
        output::baseline_detail(writer, &row)
    })
}

/// List invalidation tombstones.
pub(crate) fn invalidation_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
) -> Result<(), RunError> {
    let response = client.get("/research/invalidations")?;
    let rows: Vec<InvalidationRecord> =
        decode!(Vec<InvalidationRecord>, response, "/research/invalidations");
    output::emit(globals.json, json_value!(&rows), stdout, |writer| {
        output::invalidation_list(writer, &rows)
    })
}
