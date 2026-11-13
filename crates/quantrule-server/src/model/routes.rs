//! The model HTTP route.

use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use quantrule_api::{Digest, ErrorCode, ModelComputeRequest, ModelObject};
use quantrule_data::Timeframe;

use crate::data::series;
use crate::model::{ModelError, compute_sharded, load_config};
use crate::runtime::server::{ApiFailure, AppState, failure, request_profile, sync_configs};

/// Compute the model for a dataset and config, then store one JSON object.
pub async fn compute_model(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    payload: Result<Json<ModelComputeRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<ModelObject>), ApiFailure> {
    let profile = request_profile(&state, &headers).await?;
    let Json(request) = payload
        .map_err(|_| failure(ErrorCode::ConfigInvalid, "the body is not a model request"))?;
    let ModelComputeRequest {
        data_version,
        config_id,
        timeframes,
    } = request;

    let catalog = state
        .catalog
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no dataset catalog is configured"))?;
    let dataset = catalog
        .get(profile.clone(), data_version.to_string())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .ok_or_else(|| {
            failure(
                ErrorCode::DatasetNotFound,
                format!("dataset {data_version} is unknown"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&dataset.object_key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if Digest::sha256(&bytes) != dataset.summary.data_version {
        return Err(failure(
            ErrorCode::Internal,
            "the stored object does not match the catalog",
        ));
    }

    let configs = state
        .configs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no config catalog is configured"))?;
    let mut config = configs
        .get(profile.clone(), config_id.clone())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if config.is_none() {
        sync_configs(&state, &profile).await?;
        config = configs
            .get(profile.clone(), config_id.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }
    let config = config.ok_or_else(|| {
        failure(
            ErrorCode::ConfigInvalid,
            format!("config {config_id} is unknown"),
        )
    })?;
    if !config.valid {
        return Err(failure(
            ErrorCode::ConfigInvalid,
            config
                .message
                .unwrap_or_else(|| "the config is invalid".to_owned()),
        ));
    }
    let config_hash = config
        .config_hash
        .clone()
        .ok_or_else(|| failure(ErrorCode::ConfigInvalid, "a valid config carries no hash"))?;
    let files = config.files.clone();
    let model_config =
        tokio::task::spawn_blocking(move || load_config(&files, &std::env::temp_dir()))
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
            .map_err(|error| failure(ErrorCode::ConfigInvalid, error.to_string()))?;

    let requested = match timeframes {
        None => None,
        Some(values) => {
            if values.is_empty() {
                return Err(failure(
                    ErrorCode::ConfigInvalid,
                    "the model ladder is empty",
                ));
            }
            let mut parsed = Vec::with_capacity(values.len());
            for value in values {
                let timeframe = Timeframe::from_code(&value).ok_or_else(|| {
                    failure(
                        ErrorCode::ConfigInvalid,
                        format!("unknown timeframe {value}"),
                    )
                })?;
                parsed.push(timeframe);
            }
            Some(parsed)
        }
    };

    let limit = model_config.data.max_candles;
    let set = tokio::task::spawn_blocking(move || series(&bytes, limit))
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
        .map_err(|error| failure(ErrorCode::DatasetInvalid, error.to_string()))?;
    let label = data_version.to_string();
    let computed = tokio::task::spawn_blocking(move || {
        compute_sharded(&set, &model_config, requested.as_deref(), &label)
    })
    .await
    .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?
    .map_err(|error| match error {
        ModelError::UnsupportedTimeframe(_) | ModelError::EmptyLadder | ModelError::Config(_) => {
            failure(ErrorCode::ConfigInvalid, error.to_string())
        }
        ModelError::Output(_) => failure(ErrorCode::Internal, error.to_string()),
    })?;

    let model_version = quantrule_model::REGISTRY_VERSION;
    let codes: Vec<String> = computed
        .timeframes
        .iter()
        .map(|timeframe| timeframe.code().to_owned())
        .collect();
    let ladder = codes.join("-");
    let key = format!(
        "model/{}/{}/{}/{}.json",
        data_version.as_hex(),
        config_hash.as_hex(),
        model_version,
        ladder
    );
    let present = objects
        .exists(&key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if !present {
        objects
            .put(&key, computed.bytes.clone())
            .await
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    }

    Ok((
        StatusCode::OK,
        Json(ModelObject {
            data_version,
            config_id,
            config_hash,
            model_version: model_version.to_owned(),
            timeframes: codes,
            object_key: key,
            sha256: Digest::sha256(&computed.bytes),
            bytes: computed.bytes.len() as u64,
        }),
    ))
}
