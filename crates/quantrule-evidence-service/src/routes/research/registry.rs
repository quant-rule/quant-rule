//! Baseline and invalidation registry reads.

use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use quantrule_api::{BaselineRecord, ErrorCode, InvalidationRecord};

use super::{parse_tier, store, store_failure};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::EvidenceState;

/// List the accepted baselines for the active profile.
pub(crate) async fn baselines(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<BaselineRecord>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let rows = store.baselines(profile).await.map_err(store_failure)?;
    Ok(Json(rows))
}

/// Show the current baseline for one tier.
pub(crate) async fn baseline(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
    Path(tier): Path<String>,
) -> Result<Json<BaselineRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let tier = parse_tier(&tier)?;
    let store = store(&state)?;
    let rows = store.baselines(profile).await.map_err(store_failure)?;
    rows.into_iter()
        .find(|row| row.tier == tier)
        .map(Json)
        .ok_or_else(|| {
            failure(
                ErrorCode::ResearchNotFound,
                format!("no accepted {} baseline exists", tier.as_str()),
            )
        })
}

/// List the append-only invalidation tombstones.
pub(crate) async fn invalidations(
    State(state): State<Arc<EvidenceState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<InvalidationRecord>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let store = store(&state)?;
    let rows = store.invalidations(profile).await.map_err(store_failure)?;
    Ok(Json(rows))
}
