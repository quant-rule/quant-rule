//! The check and decision routes.
//!
//! The check is an event study over the recorded bundle. A set below thirty
//! events is refused with `simulation.check_incomplete`. The decision closes
//! marking; the predicate is enforced inside the repository transaction.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::ErrorCode;
use quantrule_runtime::clock;
use serde::Deserialize;

use crate::bundle;
use crate::records::{DecisionKind, DecisionRecord};
use crate::research::{self, CheckView};
use crate::routes::{
    ApiFailure, bundle_failure, failure, load_run, objects, request_profile, research_failure,
    store, store_failure,
};
use crate::state::SimulationState;

/// The body a decision carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DecisionRequest {
    decision: DecisionKind,
    #[serde(default)]
    note: String,
}

pub(crate) async fn run_check(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
) -> Result<Json<CheckView>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    let objects = objects(&state)?;
    let loaded = bundle::load(objects, &run.bundle)
        .await
        .map_err(bundle_failure)?;
    let record = research::check(&run, &loaded).map_err(research_failure)?;
    store(&state)?
        .record_check(profile, sim_id, record.clone())
        .await
        .map_err(store_failure)?;
    Ok(Json(research::check_view(&record, &run.inputs_digest)))
}

pub(crate) async fn record_decision(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
    payload: Result<Json<DecisionRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<DecisionRecord>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the body is not a simulation decision",
        )
    })?;
    let note = request.note.trim();
    if note.chars().count() > quantrule_simulation::marks::NOTE_LIMIT {
        return Err(failure(
            ErrorCode::SimulationMarkInvalid,
            format!(
                "the note is longer than {} characters",
                quantrule_simulation::marks::NOTE_LIMIT
            ),
        ));
    }
    let decision = DecisionRecord {
        decision: request.decision,
        note: note.to_owned(),
        decided_at_utc: clock::now_utc(),
    };
    let stored = store(&state)?
        .decide(profile, sim_id, decision)
        .await
        .map_err(store_failure)?;
    Ok((StatusCode::CREATED, Json(stored)))
}
