//! The run list, create, detail, window selection, and series routes.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::{
    CreateSimulationRequest, Digest, ErrorCode, SimulationEraseResponse, SimulationId,
};
use quantrule_runtime::clock;
use serde::{Deserialize, Serialize};

use crate::bundle::{self, LoadedBundle};
use crate::criteria::Criteria;
use crate::records::{
    NewSimulation, RunRecord, SimulationDetail, SimulationSummary, WindowView, parse_status,
    status_text,
};
use crate::research::{self, SeriesView};
use crate::routes::{
    ApiFailure, bundle_failure, failure, load_run, objects, request_profile, research_failure,
    store, store_failure,
};
use crate::state::SimulationState;
use crate::store::{ArchiveOutcome, EraseOutcome};

/// The optional filters on a run list.
#[derive(Debug, Deserialize)]
pub(crate) struct RunListQuery {
    status: Option<String>,
    limit: Option<usize>,
    include_archived: Option<bool>,
}

/// The small ack a create returns.
#[derive(Debug, Serialize)]
pub(crate) struct Ack {
    sim_id: String,
}

pub(crate) async fn list_runs(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    query: Result<Query<RunListQuery>, QueryRejection>,
) -> Result<Json<Vec<SimulationSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the run query is malformed",
        )
    })?;
    let status = query
        .status
        .as_deref()
        .map(|value| {
            parse_status(value).ok_or_else(|| {
                failure(
                    ErrorCode::SimulationMarkInvalid,
                    format!("the status {value:?} is unknown"),
                )
            })
        })
        .transpose()?;
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let runs = if query.include_archived.unwrap_or(false) {
        store(&state)?
            .list_including_archived(profile, status, limit)
            .await
    } else {
        store(&state)?.list(profile, status, limit).await
    }
    .map_err(store_failure)?;
    Ok(Json(runs))
}

/// The typed confirmation required by a simulation erase.
#[derive(Debug, Deserialize)]
pub(crate) struct EraseQuery {
    confirm: Option<String>,
}

pub(crate) async fn archive_run(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
) -> Result<Json<SimulationSummary>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let archived_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    match store(&state)?
        .archive(profile, sim_id, archived_at)
        .await
        .map_err(store_failure)?
    {
        ArchiveOutcome::Archived(summary) => Ok(Json(summary)),
        ArchiveOutcome::NotFound => Err(failure(
            ErrorCode::SimulationNotFound,
            "the simulation run is unknown",
        )),
        ArchiveOutcome::AlreadyArchived => Err(failure(
            ErrorCode::HistoryState,
            "the simulation run is already archived",
        )),
    }
}

pub(crate) async fn erase_run(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
    query: Result<Query<EraseQuery>, QueryRejection>,
) -> Result<Json<SimulationEraseResponse>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the query is not a simulation erase confirmation",
        )
    })?;
    if query.confirm.as_deref() != Some(sim_id.as_str()) {
        return Err(failure(
            ErrorCode::HistoryState,
            "erase requires --confirm with the exact simulation id",
        ));
    }
    match store(&state)?
        .erase(profile, sim_id.clone())
        .await
        .map_err(store_failure)?
    {
        EraseOutcome::Erased(sim_id) => Ok(Json(SimulationEraseResponse {
            sim_id: SimulationId(sim_id),
        })),
        EraseOutcome::NotFound => Err(failure(
            ErrorCode::SimulationNotFound,
            "the simulation run is unknown",
        )),
        EraseOutcome::NotArchived => Err(failure(
            ErrorCode::HistoryState,
            "archive the simulation run before erasing it",
        )),
        EraseOutcome::Protected(status) => Err(failure(
            ErrorCode::HistoryProtected,
            format!(
                "a {} simulation run is evidence and cannot be erased",
                status_text(status)
            ),
        )),
    }
}

pub(crate) async fn create_run(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    payload: Result<Json<CreateSimulationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<Ack>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(mut request) = payload.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the body is not a simulation run",
        )
    })?;
    if request.premise.trim().is_empty() {
        return Err(failure(
            ErrorCode::SimulationMarkInvalid,
            "the premise must not be empty",
        ));
    }
    if request.criteria.trim().is_empty() {
        return Err(failure(
            ErrorCode::SimulationMarkInvalid,
            "the criteria must not be empty",
        ));
    }
    if request.seed > i64::MAX as u64 {
        return Err(failure(
            ErrorCode::SimulationMarkInvalid,
            "the seed does not fit a signed 64-bit integer",
        ));
    }
    request.premise = request.premise.trim().to_owned();
    request.criteria = request.criteria.trim().to_owned();
    // The shell's create form names the premise, bundle, seed, and criteria;
    // the browser adds the context inputs. An absent value is recorded as
    // unspecified rather than refused, so both clients reach one use case.
    request.inputs = if request.inputs.trim().is_empty() {
        "unspecified".to_owned()
    } else {
        request.inputs.trim().to_owned()
    };
    let criteria = Criteria::parse(&request.criteria)
        .map_err(|error| failure(ErrorCode::SimulationMarkInvalid, error.to_string()))?;
    let objects = objects(&state)?;
    let loaded = bundle::load(objects, &request.bundle)
        .await
        .map_err(bundle_failure)?;
    let window_spec = criteria
        .window_spec(&loaded, request.seed)
        .map_err(|error| failure(ErrorCode::SimulationMarkInvalid, error.to_string()))?;
    let oracle_spec = criteria
        .oracle_spec()
        .map_err(|error| failure(ErrorCode::SimulationMarkInvalid, error.to_string()))?;
    let (panel_code, _) = criteria
        .panel()
        .map_err(|error| failure(ErrorCode::SimulationMarkInvalid, error.to_string()))?;
    let sim_id = simulation_id(&profile, &request);
    let check_config = criteria
        .check_config(&sim_id, &loaded, &window_spec)
        .map_err(|error| failure(ErrorCode::SimulationMarkInvalid, error.to_string()))?;
    let inputs_digest = inputs_digest(
        &request,
        &window_spec,
        &oracle_spec,
        &check_config,
        panel_code,
    )?;

    let run = NewSimulation {
        sim_id: sim_id.clone(),
        premise: request.premise.clone(),
        bundle: request.bundle,
        inputs: request.inputs.clone(),
        inputs_digest,
        seed: request.seed,
        criteria: request.criteria,
        window_spec,
        oracle_spec,
        check_config,
        created_at: clock::now_utc()
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
    };
    store(&state)?
        .create(profile, run)
        .await
        .map_err(store_failure)?;
    Ok((StatusCode::CREATED, Json(Ack { sim_id })))
}

pub(crate) async fn get_run(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
) -> Result<Json<SimulationDetail>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    Ok(Json(run.detail()))
}

pub(crate) async fn select_windows(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
) -> Result<Json<Vec<WindowView>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    if let Some(selection) = &run.selection {
        return Ok(Json(window_views(selection)));
    }
    let objects = objects(&state)?;
    let loaded = bundle::load(objects, &run.bundle)
        .await
        .map_err(bundle_failure)?;
    let selection = research::selection_for(&run, &loaded).map_err(research_failure)?;
    let saved = store(&state)?
        .save_windows(profile, sim_id, selection)
        .await
        .map_err(store_failure)?;
    let selection = saved.selection.ok_or_else(|| {
        failure(
            ErrorCode::Internal,
            "the stored selection vanished after the write",
        )
    })?;
    Ok(Json(window_views(&selection)))
}

pub(crate) async fn get_series(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path((sim_id, chart)): Path<(String, String)>,
) -> Result<Json<SeriesView>, ApiFailure> {
    let chart = chart.parse::<u64>().map_err(|_| {
        failure(
            ErrorCode::SimulationNotFound,
            "the chart index is not a number",
        )
    })?;
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    let criteria = recorded_criteria(&run)?;
    let objects = objects(&state)?;
    let loaded = load_bundle(objects, &run).await?;
    let series = research::series(&run, &loaded, &criteria, chart).map_err(research_failure)?;
    Ok(Json(series))
}

/// The recorded criteria, which must decode because the create path wrote it.
pub(crate) fn recorded_criteria(run: &RunRecord) -> Result<Criteria, ApiFailure> {
    Criteria::parse(&run.criteria).map_err(|error| {
        failure(
            ErrorCode::Internal,
            format!("the stored criteria do not decode: {error}"),
        )
    })
}

/// Load the bundle a run recorded.
pub(crate) async fn load_bundle(
    objects: &Arc<dyn quantrule_runtime::adapters::ObjectStore>,
    run: &RunRecord,
) -> Result<LoadedBundle, ApiFailure> {
    bundle::load(objects, &run.bundle)
        .await
        .map_err(bundle_failure)
}

fn window_views(selection: &quantrule_simulation::Selection) -> Vec<WindowView> {
    selection
        .windows
        .iter()
        .enumerate()
        .map(|(index, window)| WindowView {
            index: u64::try_from(index).unwrap_or(u64::MAX),
            start: window.start_index,
            end: window.end_index,
        })
        .collect()
}

fn simulation_id(profile: &quantrule_api::ProfileId, request: &CreateSimulationRequest) -> String {
    let identity = serde_json::json!({
        "schema": "quantrule.simulation-run/v1",
        "profile": profile.as_str(),
        "premise": request.premise.as_str(),
        "bundle": request.bundle.to_string(),
        "inputs": request.inputs.as_str(),
        "seed": request.seed,
        "criteria": request.criteria.as_str(),
    });
    let digest = Digest::sha256(identity.to_string().as_bytes());
    format!("sim_{}", &digest.as_hex()[..24])
}

fn inputs_digest(
    request: &CreateSimulationRequest,
    window_spec: &quantrule_simulation::WindowSpec,
    oracle_spec: &quantrule_simulation::OracleSpec,
    check_config: &quantrule_simulation::CheckConfig,
    panel_code: u32,
) -> Result<Digest, ApiFailure> {
    let identity = serde_json::json!({
        "schema": "quantrule.simulation-inputs/v1",
        "premise": request.premise.as_str(),
        "bundle": request.bundle.to_string(),
        "inputs": request.inputs.as_str(),
        "seed": request.seed,
        "criteria": request.criteria.as_str(),
        "window": window_spec,
        "oracle": oracle_spec,
        "check": check_config,
        "panel": panel_code,
    });
    let bytes = serde_json::to_vec(&identity).map_err(|error| {
        failure(
            ErrorCode::Internal,
            format!("the inputs digest could not encode: {error}"),
        )
    })?;
    Ok(Digest::sha256(&bytes))
}
