//! The computed-artifact routes.
//!
//! These routes are service-to-service only. The signal writer arrives with its
//! own work item; this surface already owns the generic registration, reference
//! and retention operations it will call. No client path and no shell command
//! reaches them.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use quantrule_api::{
    ArtifactKind, ArtifactRecord, ArtifactReference, ArtifactReferenceCount,
    ArtifactReferenceRequest, CollectArtifactsResponse, Digest, ErrorCode,
    ReleaseArtifactReferenceRequest,
};
use quantrule_runtime::clock;
use serde::Deserialize;

use crate::artifacts::store::{ArtifactError, NewArtifact};
use crate::artifacts::{ARTIFACT_RETENTION_DAYS, ArtifactCatalog};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::DataState;

/// The largest computed artifact the data service accepts.
///
/// A signal artifact is one row per bar; a multi-year series fits well under
/// this ceiling. The cap is a request limit, not a model or strategy rule.
const MAX_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

/// The artifact routes before state is attached.
pub fn routes() -> Router<Arc<DataState>> {
    Router::new()
        .route("/internal/v1/artifacts", get(list_artifacts))
        .route(
            "/internal/v1/artifacts/{kind}",
            put(register_artifact).layer(DefaultBodyLimit::max(MAX_ARTIFACT_BYTES)),
        )
        .route("/internal/v1/artifacts/collect", post(collect_artifacts))
        .route("/internal/v1/artifacts/{kind}/{id}", get(get_artifact))
        .route(
            "/internal/v1/artifacts/{kind}/{id}/bytes",
            get(get_artifact_bytes),
        )
        .route(
            "/internal/v1/artifacts/{kind}/{id}/references",
            get(list_references).post(add_reference),
        )
        .route(
            "/internal/v1/artifacts/{kind}/{id}/references/release",
            post(release_reference),
        )
}

fn catalog(state: &DataState) -> Result<&Arc<dyn ArtifactCatalog>, ApiFailure> {
    state
        .artifacts
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no artifact catalog is configured"))
}

fn parse_kind(raw: &str) -> Result<ArtifactKind, ApiFailure> {
    ArtifactKind::from_code(raw).ok_or_else(|| {
        failure(
            ErrorCode::ArtifactInvalid,
            format!("{raw} is not a computed artifact kind"),
        )
    })
}

fn parse_id(raw: &str) -> Result<String, ApiFailure> {
    Digest::parse(raw)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::ArtifactInvalid, error.to_string()))
}

fn store_failure(error: ArtifactError) -> ApiFailure {
    match error {
        ArtifactError::NotFound => failure(ErrorCode::ArtifactNotFound, "the artifact is unknown"),
        ArtifactError::Referenced(count) => failure(
            ErrorCode::ArtifactConflict,
            format!("the artifact still has {count} reference(s)"),
        ),
        ArtifactError::Store(message) => failure(ErrorCode::Internal, message),
    }
}

/// One list filter.
#[derive(Debug, Deserialize)]
struct ArtifactListQuery {
    kind: Option<String>,
}

/// Register one computed artifact body by content id.
async fn register_artifact(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(kind): Path<String>,
    request: Request,
) -> Result<(StatusCode, Json<ArtifactRecord>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let catalog = catalog(&state)?;
    let body = axum::body::to_bytes(request.into_body(), MAX_ARTIFACT_BYTES)
        .await
        .map_err(|_| failure(ErrorCode::UploadTooLarge, "the artifact exceeds 64 MiB"))?;
    let artifact_id = Digest::sha256(&body);
    let object_key = format!("artifacts/{}", artifact_id.as_hex());
    let artifact = NewArtifact {
        artifact_id,
        kind,
        object_key,
        bytes: body.len() as u64,
        created_at: clock::now_utc()
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
    };

    let _gate = state.artifact_gate.lock().await;
    let exists = catalog
        .get(profile.clone(), kind, artifact.artifact_id.to_string())
        .await
        .map_err(store_failure)?
        .is_some();
    objects
        .put(&artifact.object_key, body.to_vec())
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let record = catalog
        .register(profile, artifact)
        .await
        .map_err(store_failure)?;
    let status = if exists {
        StatusCode::OK
    } else {
        StatusCode::CREATED
    };
    Ok((status, Json(record)))
}

async fn list_artifacts(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<ArtifactListQuery>, QueryRejection>,
) -> Result<Json<Vec<ArtifactRecord>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::ArtifactInvalid, "the artifact filter is invalid"))?;
    let kind = query
        .kind
        .as_deref()
        .map(parse_kind)
        .transpose()?
        .ok_or_else(|| {
            failure(
                ErrorCode::ArtifactInvalid,
                "an artifact list requires an explicit --kind",
            )
        })?;
    let rows = catalog(&state)?
        .list(profile, kind)
        .await
        .map_err(store_failure)?;
    Ok(Json(rows))
}

async fn get_artifact(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Json<ArtifactRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let id = parse_id(&id)?;
    let record = catalog(&state)?
        .get(profile, kind, id.clone())
        .await
        .map_err(store_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ArtifactNotFound,
                format!("artifact {id} is unknown"),
            )
        })?;
    Ok(Json(record))
}

async fn get_artifact_bytes(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Response, ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let id = parse_id(&id)?;
    let record = catalog(&state)?
        .get(profile, kind, id.clone())
        .await
        .map_err(store_failure)?
        .ok_or_else(|| {
            failure(
                ErrorCode::ArtifactNotFound,
                format!("artifact {id} is unknown"),
            )
        })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects
        .get(&record.object_key)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let expected =
        Digest::parse(&id).map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    if Digest::sha256(&bytes) != expected {
        return Err(failure(
            ErrorCode::Internal,
            "the stored artifact bytes do not match the catalog",
        ));
    }
    let etag = format!("\"{id}\"");
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::ETAG, etag),
        ],
        bytes,
    )
        .into_response())
}

async fn add_reference(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
    payload: Result<Json<ArtifactReferenceRequest>, JsonRejection>,
) -> Result<Json<ArtifactReferenceCount>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let id = parse_id(&id)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::ArtifactInvalid,
            "the body is not an artifact reference",
        )
    })?;
    let reference_id = request.reference_id.trim().to_owned();
    if reference_id.is_empty() || reference_id.len() > 256 {
        return Err(failure(
            ErrorCode::ArtifactInvalid,
            "a reference id must be between 1 and 256 characters",
        ));
    }
    let created_at =
        clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let _gate = state.artifact_gate.lock().await;
    let references = catalog(&state)?
        .add_reference(
            profile,
            kind,
            id,
            ArtifactReference {
                reference_id,
                kind: request.kind,
                created_at,
            },
        )
        .await
        .map_err(store_failure)?;
    Ok(Json(ArtifactReferenceCount { references }))
}

async fn release_reference(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
    payload: Result<Json<ReleaseArtifactReferenceRequest>, JsonRejection>,
) -> Result<Json<ArtifactReferenceCount>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let id = parse_id(&id)?;
    let Json(request) = payload.map_err(|_| {
        failure(
            ErrorCode::ArtifactInvalid,
            "the body is not a reference release",
        )
    })?;
    let _gate = state.artifact_gate.lock().await;
    let references = catalog(&state)?
        .release_reference(profile, kind, id, request.reference_id)
        .await
        .map_err(store_failure)?;
    Ok(Json(ArtifactReferenceCount { references }))
}

async fn list_references(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, String)>,
) -> Result<Json<Vec<ArtifactReference>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let kind = parse_kind(&kind)?;
    let id = parse_id(&id)?;
    catalog(&state)?
        .references(profile, kind, id)
        .await
        .map(Json)
        .map_err(store_failure)
}

/// Collect unreferenced artifacts older than the retention window.
async fn collect_artifacts(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
) -> Result<Json<CollectArtifactsResponse>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let now = SystemTime::now();
    let cutoff = now
        .checked_sub(Duration::from_secs(
            ARTIFACT_RETENTION_DAYS.saturating_mul(86_400),
        ))
        .ok_or_else(|| {
            failure(
                ErrorCode::Internal,
                "the system clock is before the retention window",
            )
        })?;
    let collected_at =
        clock::rfc3339_at(now).map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let cutoff = clock::rfc3339_at(cutoff)
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let _gate = state.artifact_gate.lock().await;
    let collected = catalog(&state)?
        .collect(profile, ArtifactKind::Signal, cutoff, collected_at)
        .await
        .map_err(store_failure)?;
    let mut response = CollectArtifactsResponse {
        collected: collected.len() as u64,
        ..CollectArtifactsResponse::default()
    };
    for artifact in collected {
        if artifact.object_shared {
            continue;
        }
        match objects.delete(&artifact.object_key).await {
            Ok(()) => {
                response.bytes = response.bytes.saturating_add(artifact.bytes);
                response.objects.push(artifact.object_key);
            }
            Err(error) => {
                // The row and tombstone are already committed. An orphan
                // object is a storage leak, never a missing shared artifact.
                tracing::warn!(
                    object_key = %artifact.object_key,
                    %error,
                    "collected artifact left an orphan object"
                );
            }
        }
    }
    Ok(Json(response))
}
