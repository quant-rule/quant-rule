//! Published-number comparison for one improvement run.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use quantrule_api::{ComparisonRow, ErrorCode, ImprovementComparison, ImprovementTier, RunState};
use quantrule_service_api::RunRecord;
use serde::Deserialize;
use serde_json::Value;

use super::{artifact_json, linked_run, load, parse_run_id, refresh_digest, store, store_failure};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::EvidenceState;

/// The optional baseline override.
#[derive(Debug, Deserialize)]
pub(crate) struct CompareQuery {
    baseline: Option<String>,
}

const MODEL: &str = "model/report.json";
const PORTFOLIO: &str = "portfolio/metrics.json";

/// Compare the candidate's published numbers with the selected baseline.
pub(crate) async fn compare(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    query: Result<Query<CompareQuery>, QueryRejection>,
) -> Result<axum::Json<ImprovementComparison>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) = query.map_err(|_| {
        failure(
            ErrorCode::ResearchInvalid,
            "the query is not a comparison filter",
        )
    })?;
    let store = store(&state)?;
    let detail = load(store.as_ref(), &profile, &id).await?;
    let detail = refresh_digest(&state, store.as_ref(), &profile, detail).await?;
    let candidate = linked_run(&state, &profile, &detail).await?;
    if candidate.status != RunState::Succeeded {
        return Err(failure(
            ErrorCode::ResearchConflict,
            format!(
                "the candidate pipeline run is {}; a comparison needs a succeeded run",
                candidate.status.as_str()
            ),
        ));
    }
    let candidate_digest = candidate.digest.clone().ok_or_else(|| {
        failure(
            ErrorCode::ResearchConflict,
            "the candidate pipeline run has no canonical digest",
        )
    })?;
    let candidate_name = match detail.manifest.tier {
        ImprovementTier::Model => MODEL,
        _ => PORTFOLIO,
    };
    let candidate_json = artifact_json(&state, &candidate, candidate_name)
        .await
        .map_err(|message| failure(ErrorCode::ResearchConflict, message))?
        .ok_or_else(|| {
            failure(
                ErrorCode::ResearchConflict,
                format!("the candidate run does not publish {candidate_name}"),
            )
        })?;

    let baselines = store
        .baselines(profile.clone())
        .await
        .map_err(store_failure)?;
    let requested = query.baseline.or(detail.manifest.baseline.clone());
    let baseline = match requested {
        Some(id) => baselines
            .into_iter()
            .find(|row| row.baseline_id == id)
            .ok_or_else(|| {
                failure(
                    ErrorCode::ResearchNotFound,
                    format!("baseline {id} is unknown"),
                )
            })?,
        None => baselines
            .into_iter()
            .find(|row| row.tier == detail.manifest.tier)
            .ok_or_else(|| {
                failure(
                    ErrorCode::ResearchNotFound,
                    format!(
                        "no accepted {} baseline exists",
                        detail.manifest.tier.as_str()
                    ),
                )
            })?,
    };

    let (baseline_run, baseline_digest, mut note) = baseline_run(&state, &profile, &baseline).await;
    if baseline_run.is_none() && note.is_none() {
        note = Some(
            "the baseline has no succeeded pipeline run; baseline metrics are unavailable"
                .to_owned(),
        );
    }
    let baseline_json = match baseline_run.as_ref() {
        Some(run) => match artifact_json(&state, run, candidate_name).await {
            Ok(Some(value)) => Some(value),
            Ok(None) => {
                note = Some(format!(
                    "the baseline run does not publish {candidate_name}; baseline metrics are unavailable"
                ));
                None
            }
            Err(message) => {
                note = Some(format!("the baseline metrics could not be read: {message}"));
                None
            }
        },
        None => None,
    };
    let rows = match detail.manifest.tier {
        ImprovementTier::Model => model_rows(&candidate_json, baseline_json.as_ref())?,
        _ => portfolio_rows(&candidate_json, baseline_json.as_ref()),
    };

    Ok(axum::Json(ImprovementComparison {
        improvement_id: detail.improvement_id,
        tier: detail.manifest.tier,
        candidate_run_id: candidate.run_id,
        baseline_id: Some(baseline.baseline_id),
        candidate_digest: Some(candidate_digest),
        baseline_digest,
        rows,
        note,
    }))
}

async fn baseline_run(
    state: &EvidenceState,
    profile: &quantrule_api::ProfileId,
    baseline: &quantrule_api::BaselineRecord,
) -> (
    Option<RunRecord>,
    Option<quantrule_api::Digest>,
    Option<String>,
) {
    let store = match super::store(state) {
        Ok(store) => store,
        Err(error) => return (None, None, Some(message(error))),
    };
    let run_id = if let Some(improvement_id) = &baseline.improvement_id {
        match store.get(profile.clone(), improvement_id.clone()).await {
            Ok(Some(record)) => record.pipeline_run_id.map(|id| id.to_string()),
            Ok(None) => None,
            Err(error) => return (None, None, Some(error.to_string())),
        }
    } else {
        baseline.run_id.clone()
    };
    let Some(run_id) = run_id else {
        return (
            None,
            None,
            Some(
                "the baseline is a historical registry row without a pipeline run; baseline metrics are unavailable"
                    .to_owned(),
            ),
        );
    };
    let Some(run_id) = parse_run_id(&run_id) else {
        return (
            None,
            None,
            Some(
                "the baseline names a historical run range, not a pipeline run; baseline metrics are unavailable"
                    .to_owned(),
            ),
        );
    };
    let Some(runs) = state.runs.as_ref() else {
        return (None, None, Some("no run store is configured".to_owned()));
    };
    let record = match runs.get(profile.clone(), run_id.to_string()).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return (
                None,
                None,
                Some("the baseline pipeline run is unknown".to_owned()),
            );
        }
        Err(error) => return (None, None, Some(error.to_string())),
    };
    if record.status != RunState::Succeeded {
        return (
            None,
            record.digest.clone(),
            Some(format!(
                "the baseline pipeline run is {}; baseline metrics are unavailable",
                record.status.as_str()
            )),
        );
    }
    let digest = record.digest.clone();
    (Some(record), digest, None)
}

fn message(error: ApiFailure) -> String {
    let (_, body) = error;
    body.0.error.message
}

fn model_rows(
    candidate: &Value,
    baseline: Option<&Value>,
) -> Result<Vec<ComparisonRow>, ApiFailure> {
    let frames = candidate
        .pointer("/flat_report/timeframes")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            failure(
                ErrorCode::ResearchConflict,
                "the candidate report has no flat_report.timeframes",
            )
        })?;
    let baseline_frames = baseline
        .and_then(|value| value.pointer("/flat_report/timeframes"))
        .and_then(Value::as_array)
        .map(|frames| {
            frames
                .iter()
                .filter_map(|frame| {
                    let label = frame
                        .get("timeframe_secs")
                        .and_then(Value::as_u64)
                        .map(timeframe_label)?;
                    Some((label, frame))
                })
                .collect::<BTreeMap<String, &Value>>()
        })
        .unwrap_or_default();
    let fields = [
        "bars",
        "expected_bars",
        "missing_bars",
        "coverage_share",
        "session_expected_bars",
        "session_missing_bars",
        "session_coverage_share",
    ];
    let mut rows = Vec::new();
    for frame in frames {
        let Some(label) = frame
            .get("timeframe_secs")
            .and_then(Value::as_u64)
            .map(timeframe_label)
        else {
            continue;
        };
        let baseline_frame = baseline_frames.get(&label).copied();
        for field in fields {
            let candidate_value = number(frame, field);
            let baseline_value = baseline_frame.and_then(|frame| number(frame, field));
            rows.push(row(
                format!("{label}.{field}"),
                candidate_value,
                baseline_value,
            ));
        }
    }
    Ok(rows)
}

fn portfolio_rows(candidate: &Value, baseline: Option<&Value>) -> Vec<ComparisonRow> {
    let fields = [
        "total_return",
        "max_drawdown",
        "trades",
        "profit_factor",
        "win_rate",
        "exposure",
        "net_pnl",
        "final_equity",
    ];
    fields
        .into_iter()
        .map(|field| {
            row(
                field.to_owned(),
                number(candidate, field),
                baseline.and_then(|value| number(value, field)),
            )
        })
        .collect()
}

fn row(metric: String, candidate: Option<f64>, baseline: Option<f64>) -> ComparisonRow {
    let delta = match (candidate, baseline) {
        (Some(candidate), Some(baseline)) => Some(candidate - baseline),
        _ => None,
    };
    ComparisonRow {
        metric,
        candidate,
        baseline,
        delta,
    }
}

fn number(value: &Value, field: &str) -> Option<f64> {
    let value = value.get(field)?;
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|number| number as f64))
        .or_else(|| value.as_u64().map(|number| number as f64))
}

fn timeframe_label(seconds: u64) -> String {
    match seconds {
        60 => "1m".to_owned(),
        300 => "5m".to_owned(),
        900 => "15m".to_owned(),
        3_600 => "1h".to_owned(),
        14_400 => "4h".to_owned(),
        86_400 => "1d".to_owned(),
        other => format!("{other}s"),
    }
}
