//! The mark and oracle routes.
//!
//! A mark is stored only after its window range, knowable-event bound, and note
//! limit are checked. The timestamp comes from the bar, never from the client.
//! Every write refreshes the two annotator-schema CSV mirrors; the database
//! stays authoritative.

use std::sync::Arc;

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::{ErrorCode, MarkAction, MarkOrigin, MarkQuality, Side};
use quantrule_runtime::clock;
use serde::Deserialize;

use crate::bundle;
use crate::records::{MarkAck, MarkView, parse_origin};
use crate::research::{self, MarkDraft};
use crate::routes::{
    ApiFailure, bundle_failure, failure, load_run, objects, request_profile, research_failure,
    store, store_failure,
};
use crate::state::SimulationState;

/// The body a client mark carries.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MarkRequest {
    #[serde(deserialize_with = "chart_number")]
    chart: u64,
    bar: u64,
    action: MarkAction,
    quality: MarkQuality,
    side: Side,
    #[serde(default)]
    note: String,
}

/// Accept the browser's number and the shell's string for one chart number.
fn chart_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        Number(u64),
        Text(String),
    }
    match Repr::deserialize(deserializer)? {
        Repr::Number(number) => Ok(number),
        Repr::Text(text) => text.trim().parse().map_err(serde::de::Error::custom),
    }
}

/// The optional origin filter.
#[derive(Debug, Deserialize)]
pub(crate) struct MarkListQuery {
    origin: Option<String>,
}

pub(crate) async fn create_mark(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
    payload: Result<Json<MarkRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<MarkAck>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the body is not a simulation mark",
        )
    })?;
    let run = load_run(&state, &profile, &sim_id).await?;
    if run.is_decided() {
        return Err(failure(
            ErrorCode::SimulationLocked,
            "marking is closed after a decision",
        ));
    }
    let objects = objects(&state)?;
    let loaded = bundle::load(objects, &run.bundle)
        .await
        .map_err(bundle_failure)?;
    let draft = MarkDraft {
        chart: request.chart,
        bar: request.bar,
        action: request.action,
        quality: request.quality,
        side: request.side,
        note: request.note,
    };
    let mut mark =
        research::build_mark(&run, &loaded, draft, MarkOrigin::Human).map_err(research_failure)?;
    mark.created_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let stored = store(&state)?
        .put_mark(profile.clone(), sim_id.clone(), mark)
        .await
        .map_err(store_failure)?;
    refresh_mirror(&state, &profile, &sim_id).await;
    Ok((
        StatusCode::CREATED,
        Json(MarkAck {
            mark_id: stored.mark_id,
        }),
    ))
}

pub(crate) async fn list_marks(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
    query: Result<Query<MarkListQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<Vec<MarkView>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::SimulationMarkInvalid,
            "the mark query is malformed",
        )
    })?;
    let origin = query
        .origin
        .as_deref()
        .map(|value| {
            parse_origin(value).ok_or_else(|| {
                failure(
                    ErrorCode::SimulationMarkInvalid,
                    format!("the origin {value:?} is unknown"),
                )
            })
        })
        .transpose()?;
    let marks = store(&state)?
        .list_marks(profile, run.sim_id, origin)
        .await
        .map_err(store_failure)?;
    Ok(Json(marks.into_iter().map(MarkView::from).collect()))
}

pub(crate) async fn delete_mark(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path((sim_id, mark_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiFailure> {
    let profile = request_profile(&headers)?;
    let _ = load_run(&state, &profile, &sim_id).await?;
    let deleted = store(&state)?
        .delete_mark(profile.clone(), sim_id.clone(), mark_id.clone())
        .await
        .map_err(store_failure)?;
    if !deleted {
        return Err(failure(
            ErrorCode::SimulationNotFound,
            format!("mark {mark_id} is unknown"),
        ));
    }
    refresh_mirror(&state, &profile, &sim_id).await;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn run_oracle(
    State(state): State<Arc<SimulationState>>,
    headers: HeaderMap,
    Path(sim_id): Path<String>,
) -> Result<Json<Vec<MarkView>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let run = load_run(&state, &profile, &sim_id).await?;
    if run.is_decided() {
        return Err(failure(
            ErrorCode::SimulationLocked,
            "marking is closed after a decision",
        ));
    }
    let objects = objects(&state)?;
    let loaded = bundle::load(objects, &run.bundle)
        .await
        .map_err(bundle_failure)?;
    let created_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let mut marks = research::oracle_marks(&run, &loaded).map_err(research_failure)?;
    for mark in &mut marks {
        mark.created_at.clone_from(&created_at);
    }
    let stored = store(&state)?
        .replace_oracle(profile.clone(), sim_id.clone(), marks)
        .await
        .map_err(store_failure)?;
    refresh_mirror(&state, &profile, &sim_id).await;
    Ok(Json(stored.into_iter().map(MarkView::from).collect()))
}

/// Rewrite both CSV mirrors after a mark mutation.
async fn refresh_mirror(state: &SimulationState, profile: &quantrule_api::ProfileId, sim_id: &str) {
    let (Some(store), Some(objects)) = (state.store.as_ref(), state.objects.as_ref()) else {
        return;
    };
    match store
        .list_marks(profile.clone(), sim_id.to_owned(), None)
        .await
    {
        Ok(marks) => {
            if let Err(error) = research::mirror_marks(objects, sim_id, &marks).await {
                tracing::error!(
                    sim_id = %sim_id,
                    error = %error,
                    "the CSV mark mirror could not be written; the database is authoritative"
                );
            }
        }
        Err(error) => tracing::error!(
            sim_id = %sim_id,
            error = %error,
            "the CSV mark mirror could not be rebuilt"
        ),
    }
}
