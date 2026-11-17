//! The research-ledger HTTP surface.
//!
//! The routes expose the v3 pre-registration, its pipeline link and digest, the
//! human decision, the published-number comparison, and the append-only
//! baseline and invalidation lists. The service owns the ledger; the gateway
//! only proxies. A stage artifact is never accepted as a pipeline run.

mod compare;
mod create;
mod decision;
mod registry;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post};
use quantrule_api::{
    Digest, ErrorCode, ImprovementManifest, ImprovementPartition, ImprovementRunDetail,
    ImprovementRunState, ImprovementTier, ProfileId, RunId, RunState,
};
use quantrule_runtime::clock;
use quantrule_service_api::{ConfigBundle, RunRecord};
use serde_json::Value;

use crate::research::{ImprovementStore, ImprovementStoreError};
use crate::routes::{ApiFailure, failure};
use crate::state::EvidenceState;

/// The research routes before state is attached.
pub(super) fn routes() -> Router<Arc<EvidenceState>> {
    Router::new()
        .route("/research/runs", get(create::list).post(create::create))
        .route("/research/runs/{id}", get(create::show))
        .route("/research/runs/{id}/submit", post(create::submit))
        .route("/research/runs/{id}/compare", get(compare::compare))
        .route("/research/runs/{id}/decision", post(decision::decide))
        .route("/research/baselines", get(registry::baselines))
        .route("/research/baselines/{tier}", get(registry::baseline))
        .route("/research/invalidations", get(registry::invalidations))
}

/// The configured improvement store.
pub(super) fn store(state: &EvidenceState) -> Result<&Arc<dyn ImprovementStore>, ApiFailure> {
    state
        .improvements
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no improvement store is configured"))
}

/// Read one record or return the stable not-found error.
pub(super) async fn load(
    store: &dyn ImprovementStore,
    profile: &ProfileId,
    improvement_id: &str,
) -> Result<ImprovementRunDetail, ApiFailure> {
    store
        .get(profile.clone(), improvement_id.to_owned())
        .await
        .map_err(store_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ResearchNotFound,
                format!("improvement run {improvement_id} is unknown"),
            )
        })
}

/// Link the canonical digest lazily when the linked pipeline run has succeeded.
pub(super) async fn refresh_digest(
    state: &EvidenceState,
    store: &dyn ImprovementStore,
    profile: &ProfileId,
    detail: ImprovementRunDetail,
) -> Result<ImprovementRunDetail, ApiFailure> {
    if detail.canonical_digest.is_some() {
        return Ok(detail);
    }
    let Some(run_id) = detail.pipeline_run_id.clone() else {
        return Ok(detail);
    };
    let Some(runs) = state.runs.as_ref() else {
        return Ok(detail);
    };
    let Some(record) = runs
        .get(profile.clone(), run_id.to_string())
        .await
        .map_err(store_failure)?
    else {
        return Ok(detail);
    };
    if record.status != RunState::Succeeded {
        return Ok(detail);
    }
    let Some(digest) = record.digest else {
        return Ok(detail);
    };
    match store
        .link_digest(profile.clone(), detail.improvement_id.clone(), digest)
        .await
    {
        Ok(updated) => Ok(updated),
        Err(error) if error.to_string().contains("already") => {
            load(store, profile, &detail.improvement_id).await
        }
        Err(error) => Err(store_failure(error)),
    }
}

/// The linked pipeline run, or the conflict that says it is missing.
pub(super) async fn linked_run(
    state: &EvidenceState,
    profile: &ProfileId,
    detail: &ImprovementRunDetail,
) -> Result<RunRecord, ApiFailure> {
    let Some(run_id) = detail.pipeline_run_id.clone() else {
        return Err(failure(
            ErrorCode::ResearchConflict,
            "the improvement run has no linked pipeline run",
        ));
    };
    let runs = state
        .runs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no run store is configured"))?;
    runs.get(profile.clone(), run_id.to_string())
        .await
        .map_err(store_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ResearchConflict,
                format!("the linked pipeline run {run_id} is unknown"),
            )
        })
}

/// Read one JSON artifact published by a run.
///
/// The artifact is found by name in the run's verified set and read by the
/// content hash the run recorded. A missing object or invalid JSON is an error
/// for the caller to turn into a note or a conflict.
pub(super) async fn artifact_json(
    state: &EvidenceState,
    record: &RunRecord,
    name: &str,
) -> Result<Option<Value>, String> {
    let Some(artifact) = record
        .artifacts
        .iter()
        .find(|artifact| artifact.name == name)
    else {
        return Ok(None);
    };
    let Some(objects) = state.objects.as_ref() else {
        return Err("no object store is configured".to_owned());
    };
    let bytes = objects
        .get(&quantrule_service_api::RunManifest::blob_key(
            &artifact.sha256,
        ))
        .await
        .map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| format!("{name} is not JSON: {error}"))
}

/// Build the v3 manifest from a create request and the server-owned fields.
pub(super) fn manifest(
    request: &quantrule_api::CreateImprovementRequest,
    improvement_id: String,
    created_utc: String,
) -> ImprovementManifest {
    let frozen = match request.tier {
        ImprovementTier::Model => vec![ImprovementTier::Strategy, ImprovementTier::Portfolio],
        ImprovementTier::Strategy => vec![ImprovementTier::Model, ImprovementTier::Portfolio],
        ImprovementTier::Portfolio => vec![ImprovementTier::Model, ImprovementTier::Strategy],
    };
    let holdout_access = match request.partition {
        ImprovementPartition::PortfolioHoldout => quantrule_api::HoldoutAccess::Final,
        _ => quantrule_api::HoldoutAccess::None,
    };
    ImprovementManifest {
        schema: quantrule_api::IMPROVEMENT_RUN_SCHEMA.to_owned(),
        improvement_id,
        tier: request.tier,
        hypothesis: request.hypothesis.clone(),
        created_utc,
        changed: request.tier,
        frozen,
        partition: request.partition,
        data_version: request.data_version.clone(),
        config_id: request.config_id.clone(),
        criteria_id: request.criteria_id.clone(),
        green_criteria: request.green_criteria.clone(),
        holdout_access,
        baseline: request.baseline.clone(),
        code_commit: request.code_commit.clone(),
        seed: request.seed,
        command: Some("run submit".to_owned()),
        analytics: request.analytics,
    }
}

/// The next two-digit sequence for a tier and date.
pub(super) async fn next_id(
    store: &dyn ImprovementStore,
    profile: &ProfileId,
    tier: ImprovementTier,
    date: &str,
) -> Result<String, ApiFailure> {
    let prefix = format!("{}-{date}-", tier.id_prefix());
    let rows = store
        .list(profile.clone(), 1_000)
        .await
        .map_err(store_failure)?;
    let max = rows
        .iter()
        .filter_map(|row| row.improvement_id.strip_prefix(&prefix))
        .filter_map(|sequence| sequence.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    let next = max + 1;
    if next > 99 {
        return Err(failure(
            ErrorCode::ResearchConflict,
            format!("the {tier:?} sequence for {date} is exhausted"),
        ));
    }
    Ok(format!("{prefix}{next:02}"))
}

/// Refuse a config that is not a full coupled run.
pub(super) fn require_coupled(config: &ConfigBundle) -> Result<(), String> {
    let Some(body) = config.files.get("run.json") else {
        return Err("the config has no run.json".to_owned());
    };
    let value: Value =
        serde_json::from_str(body).map_err(|error| format!("run.json is not JSON: {error}"))?;
    match value.get("strategy").and_then(Value::as_str) {
        Some(strategy) if !strategy.trim().is_empty() => Ok(()),
        _ => Err(
            "a research run needs a full run config with a strategy; a model-only config is not accepted"
                .to_owned(),
        ),
    }
}

/// Map a lower-level validation failure onto the research error family.
pub(super) fn validation_failure(error: ApiFailure) -> ApiFailure {
    let (status, body) = error;
    let envelope = body.0;
    match envelope.error.code {
        ErrorCode::DatasetNotFound
        | ErrorCode::DatasetInvalid
        | ErrorCode::ConfigInvalid
        | ErrorCode::RunInvalid
        | ErrorCode::RunModelArtifactInvalid => {
            failure(ErrorCode::ResearchInvalid, envelope.error.message)
        }
        _ => (status, axum::Json(envelope)),
    }
}

/// Map a store failure onto the public envelope.
pub(super) fn store_failure(error: impl std::fmt::Display) -> ApiFailure {
    let message = error.to_string();
    if message.contains("already")
        || message.contains("needs")
        || message.contains("decision")
        || message.contains("submitted")
        || message.contains("registered")
    {
        failure(ErrorCode::ResearchConflict, message)
    } else {
        failure(ErrorCode::Internal, message)
    }
}

/// The decision error is a conflict when it names a state or duplicate rule.
pub(super) fn decision_failure(error: ImprovementStoreError) -> ApiFailure {
    let message = error.to_string();
    if message.contains("already")
        || message.contains("needs")
        || message.contains("decision")
        || message.contains("submitted")
        || message.contains("registered")
    {
        failure(ErrorCode::ResearchConflict, message)
    } else {
        failure(ErrorCode::Internal, message)
    }
}

/// The UTC timestamp for a server-generated field.
pub(super) fn now() -> Result<String, ApiFailure> {
    clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))
}

/// The calendar date of an RFC 3339 UTC timestamp.
pub(super) fn date_of(timestamp: &str) -> String {
    timestamp
        .split_once('T')
        .map_or(timestamp, |(date, _)| date)
        .to_owned()
}

/// Parse a tier path or filter.
pub(super) fn parse_tier(value: &str) -> Result<ImprovementTier, ApiFailure> {
    ImprovementTier::parse(value).ok_or_else(|| {
        failure(
            ErrorCode::ResearchInvalid,
            format!("unknown improvement tier {value}"),
        )
    })
}

/// Parse a state filter.
pub(super) fn parse_state(value: &str) -> Result<ImprovementRunState, ApiFailure> {
    ImprovementRunState::parse(value).ok_or_else(|| {
        failure(
            ErrorCode::ResearchInvalid,
            format!("unknown improvement state {value}"),
        )
    })
}

/// A pipeline run id parsed from a stored string.
pub(super) fn parse_run_id(value: &str) -> Option<RunId> {
    Digest::parse(value).ok().map(RunId)
}
