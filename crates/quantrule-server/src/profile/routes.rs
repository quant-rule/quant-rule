//! Profile routes.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use quantrule_api::{CreateProfileRequest, ErrorCode, ProfileId, ProfileSummary};

use crate::profile::{ProfileError, ProfileRecord};
use crate::runtime::clock;
use crate::runtime::server::{ApiFailure, AppState, failure};

/// Build the wire summary for one profile.
async fn summary(state: &AppState, record: ProfileRecord) -> Result<ProfileSummary, ApiFailure> {
    let datasets = match state.catalog.as_ref() {
        Some(catalog) => catalog
            .count(record.profile_id.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
        None => 0,
    };
    // Runs and simulations gain their counts with their services; until then
    // their counts are zero rather than a number no service can support.
    Ok(ProfileSummary {
        profile_id: record.profile_id,
        name: record.name,
        datasets,
        runs: 0,
        simulations: 0,
    })
}

/// Create a profile, or return the existing one.
pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<CreateProfileRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ProfileSummary>), ApiFailure> {
    let Json(request) =
        payload.map_err(|_| failure(ErrorCode::ProfileInvalid, "the body is not a profile"))?;
    let profile_id = ProfileId::parse(request.profile_id.as_str())
        .map_err(|error| failure(ErrorCode::ProfileInvalid, error.to_string()))?;
    let name = request.name.trim();
    if name.is_empty() {
        return Err(failure(ErrorCode::ProfileInvalid, "a profile needs a name"));
    }
    let store = state
        .profiles
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no profile store is configured"))?;
    let record = ProfileRecord {
        profile_id: profile_id.clone(),
        name: name.to_owned(),
        created_at: clock::now_utc(),
    };
    match store.create(record.clone()).await {
        Ok(()) => {
            let summary = summary(&state, record).await?;
            Ok((StatusCode::CREATED, Json(summary)))
        }
        Err(ProfileError::AlreadyExists) => match store.get(profile_id).await {
            Ok(Some(existing)) => {
                let summary = summary(&state, existing).await?;
                Ok((StatusCode::OK, Json(summary)))
            }
            Ok(None) => Err(failure(
                ErrorCode::Internal,
                "the profile store refused a duplicate but holds no record",
            )),
            Err(error) => Err(failure(ErrorCode::Internal, error.to_string())),
        },
        Err(ProfileError::Store(error)) => Err(failure(ErrorCode::Internal, error)),
    }
}

/// List every profile with its counts.
pub async fn list_profiles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ProfileSummary>>, ApiFailure> {
    let store = state
        .profiles
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no profile store is configured"))?;
    let records = store
        .list()
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let mut summaries = Vec::with_capacity(records.len());
    for record in records {
        summaries.push(summary(&state, record).await?);
    }
    Ok(Json(summaries))
}
