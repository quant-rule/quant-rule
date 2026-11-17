//! The human decision and its atomic registry row.

use std::sync::Arc;

use crate::research::DecisionWrite;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, Path, State};
use axum::http::HeaderMap;
use quantrule_api::{
    BaselineRecord, DecideImprovementRequest, ErrorCode, ImprovementDecision, ImprovementRunDetail,
    ImprovementRunState, InvalidationRecord, RunState,
};

use super::{date_of, decision_failure, linked_run, load, now, refresh_digest, store};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::EvidenceState;

/// Record exactly one human decision, atomically with its registry row.
pub(crate) async fn decide(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    payload: Result<Json<DecideImprovementRequest>, JsonRejection>,
) -> Result<Json<ImprovementRunDetail>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::ResearchInvalid,
            "the body is not a decision request",
        )
    })?;
    if request.note.trim().is_empty() {
        return Err(failure(
            ErrorCode::ResearchInvalid,
            "a decision needs a non-empty note",
        ));
    }
    let store = store(&state)?;
    let detail = load(store.as_ref(), &profile, &id).await?;
    let detail = refresh_digest(&state, store.as_ref(), &profile, detail).await?;
    if detail.state == ImprovementRunState::Decided {
        return Err(failure(
            ErrorCode::ResearchConflict,
            "the decision is already recorded",
        ));
    }
    if detail.state == ImprovementRunState::Created {
        return Err(failure(
            ErrorCode::ResearchConflict,
            "a decision needs a submitted pipeline run",
        ));
    }
    let decided_at = now()?;
    let mut baseline = None;
    let mut invalidation = None;
    match request.decision {
        ImprovementDecision::Accepted => {
            let run = linked_run(&state, &profile, &detail).await?;
            if run.status != RunState::Succeeded {
                return Err(failure(
                    ErrorCode::ResearchConflict,
                    format!(
                        "an accepted decision needs a succeeded pipeline run; this run is {}",
                        run.status.as_str()
                    ),
                ));
            }
            if detail.canonical_digest.is_none() {
                return Err(failure(
                    ErrorCode::ResearchConflict,
                    "an accepted decision needs the canonical digest",
                ));
            }
            let Some(run_id) = detail.pipeline_run_id.clone() else {
                return Err(failure(
                    ErrorCode::ResearchConflict,
                    "an accepted decision needs a linked pipeline run",
                ));
            };
            baseline = Some(BaselineRecord {
                baseline_id: detail.improvement_id.clone(),
                tier: detail.manifest.tier,
                run_id: Some(run_id.to_string()),
                improvement_id: Some(detail.improvement_id.clone()),
                accepted_by: "Human".to_owned(),
                accepted_on: date_of(&decided_at),
                scope: format!(
                    "{}; partition {}; commit {}",
                    detail.manifest.hypothesis,
                    detail.manifest.partition.as_str(),
                    detail.manifest.code_commit
                ),
                supersedes: None,
                created_at: decided_at.clone(),
            });
        }
        ImprovementDecision::Invalidated => {
            let Some(run_id) = detail.pipeline_run_id.clone() else {
                return Err(failure(
                    ErrorCode::ResearchConflict,
                    "an invalidated decision needs a linked pipeline run",
                ));
            };
            invalidation = Some(InvalidationRecord {
                invalidation_id: detail.improvement_id.clone(),
                date: date_of(&decided_at),
                commit: detail.manifest.code_commit.clone(),
                removed: run_id.to_string(),
                reason: request.note.clone(),
                superseded_by: None,
                created_at: decided_at.clone(),
            });
        }
        ImprovementDecision::Rejected
        | ImprovementDecision::NoEdge
        | ImprovementDecision::NewHypothesis => {}
    }
    let updated = store
        .decide_with_registry(
            profile,
            detail.improvement_id,
            DecisionWrite {
                decision: request.decision,
                note: request.note,
                decided_at,
                baseline,
                invalidation,
            },
        )
        .await
        .map_err(decision_failure)?;
    Ok(Json(updated))
}
