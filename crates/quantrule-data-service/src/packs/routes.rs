//! The pack registry routes.
//!
//! The public routes are the additive `/catalog/packs` surface a later client
//! reaches through the gateway. They take IDs and a profile header, never a
//! filesystem path. The internal routes expose the same pack to a service that
//! holds only its content id. No shell command, browser screen, run submission
//! or worker consumes these yet.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::QueryRejection;
use axum::extract::{DefaultBodyLimit, Path, Query, Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use quantrule_api::{Digest, ErrorCode, PackKind, PackRecord, PackState, PackSummary};
use quantrule_runtime::adapters::put_if_absent;
use quantrule_runtime::clock;
use serde::Deserialize;

use crate::packs::store::{PackFilter, PackRegistryError};
use crate::packs::{NewPack, parse_pack};
use crate::routes::{ApiFailure, failure, request_profile};
use crate::state::DataState;

/// The largest pack document the registry accepts.
///
/// A pack is a definition, not a data series; one MiB is far above any valid
/// pack and far below the dataset upload cap.
const MAX_PACK_BYTES: usize = 1024 * 1024;

/// The pack routes before state is attached.
pub fn routes() -> Router<Arc<DataState>> {
    Router::new()
        .route(
            "/catalog/packs",
            get(list_packs)
                .post(create_pack)
                .layer(DefaultBodyLimit::max(MAX_PACK_BYTES)),
        )
        .route("/catalog/packs/{id}", get(get_pack))
        .route("/catalog/packs/{id}/bytes", get(get_pack_bytes))
        .route("/catalog/packs/{id}/validate", post(validate_pack))
        .route("/catalog/packs/{id}/freeze", post(freeze_pack))
        .route("/catalog/packs/{id}/retire", post(retire_pack))
        .route("/internal/v1/packs/{id}", get(get_pack))
        .route("/internal/v1/packs/{id}/bytes", get(get_pack_bytes))
}

fn registry(state: &DataState) -> Result<&Arc<dyn crate::packs::PackRegistry>, ApiFailure> {
    state
        .packs
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no pack registry is configured"))
}

fn parse_id(raw: &str) -> Result<String, ApiFailure> {
    Digest::parse(raw)
        .map(|digest| digest.to_string())
        .map_err(|error| failure(ErrorCode::PackInvalid, error.to_string()))
}

fn store_failure(error: PackRegistryError) -> ApiFailure {
    match error {
        PackRegistryError::NotFound => failure(ErrorCode::PackNotFound, "the pack is unknown"),
        PackRegistryError::LabelConflict {
            existing,
            name,
            version,
        } => failure(
            ErrorCode::PackConflict,
            format!("pack {name} version {version} already names {existing}"),
        ),
        PackRegistryError::NotValidated(state) => failure(
            ErrorCode::PackConflict,
            format!("freeze requires a validated pack, not {state}"),
        ),
        PackRegistryError::NotFrozen(state) => failure(
            ErrorCode::PackConflict,
            format!("retire requires a frozen pack, not {state}"),
        ),
        PackRegistryError::Frozen => {
            failure(ErrorCode::PackConflict, "a frozen pack cannot be mutated")
        }
        PackRegistryError::Retired => {
            failure(ErrorCode::PackConflict, "a retired pack is terminal")
        }
        PackRegistryError::Store(message) => failure(ErrorCode::Internal, message),
    }
}

/// Resolve a pack row and read its canonical bytes from the object store.
///
/// The object store is the single byte authority. A missing object, an
/// unreadable object, a hash mismatch or a size mismatch fails closed; the
/// registry row is never returned without its verified bytes.
async fn verified_pack(
    state: &DataState,
    profile: quantrule_api::ProfileId,
    id: &str,
) -> Result<(PackRecord, Vec<u8>), ApiFailure> {
    let registry = registry(state)?;
    let record = registry
        .get(profile, id.to_owned())
        .await
        .map_err(store_failure)?
        .ok_or_else(|| failure(ErrorCode::PackNotFound, format!("pack {id} is unknown")))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let bytes = objects.get(&record.object_key).await.map_err(|error| {
        failure(
            ErrorCode::Internal,
            format!(
                "the pack object {} is missing or unreadable: {error}",
                record.object_key
            ),
        )
    })?;
    if Digest::sha256(&bytes) != record.sha256 || bytes.len() as u64 != record.summary.bytes {
        return Err(failure(
            ErrorCode::Internal,
            "the pack object does not match its registry identity",
        ));
    }
    Ok((record, bytes))
}

/// Register one uploaded pack document.
async fn create_pack(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    request: Request,
) -> Result<(StatusCode, Json<PackRecord>), ApiFailure> {
    let profile = request_profile(&headers)?;
    let body = axum::body::to_bytes(request.into_body(), MAX_PACK_BYTES)
        .await
        .map_err(|_| failure(ErrorCode::UploadTooLarge, "the pack exceeds one MiB"))?;
    let parsed =
        parse_pack(&body).map_err(|error| failure(ErrorCode::PackInvalid, error.to_string()))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let registry = registry(&state)?;
    let object_key = format!("packs/{}", parsed.pack_id.as_hex());
    let bytes = parsed.canonical;
    let pack = NewPack {
        pack_id: parsed.pack_id,
        kind: parsed.kind,
        name: parsed.name,
        version: parsed.version,
        owner: parsed.owner,
        object_key,
        bytes: bytes.len() as u64,
        created_at: clock::now_utc()
            .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
    };

    let _gate = state.pack_gate.lock().await;
    // The object is written first. A metadata failure leaves an orphan object,
    // which is safe and reconcilable; a row without bytes is not.
    put_if_absent(objects.as_ref(), &pack.object_key, bytes)
        .await
        .map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let outcome = registry
        .register(profile, pack)
        .await
        .map_err(store_failure)?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(outcome.record)))
}

/// The optional filters on a pack list.
#[derive(Debug, Deserialize)]
struct PackListQuery {
    kind: Option<String>,
    state: Option<String>,
}

async fn list_packs(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    query: Result<Query<PackListQuery>, QueryRejection>,
) -> Result<Json<Vec<PackSummary>>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let Query(query) =
        query.map_err(|_| failure(ErrorCode::PackInvalid, "the pack filter is invalid"))?;
    let kind = query
        .kind
        .as_deref()
        .map(|code| {
            PackKind::from_code(code).ok_or_else(|| {
                failure(ErrorCode::PackInvalid, format!("{code} is not a pack kind"))
            })
        })
        .transpose()?;
    let lifecycle = query
        .state
        .as_deref()
        .map(|code| {
            PackState::from_code(code).ok_or_else(|| {
                failure(
                    ErrorCode::PackInvalid,
                    format!("{code} is not a pack state"),
                )
            })
        })
        .transpose()?;
    let packs = registry(&state)?
        .list(
            profile,
            PackFilter {
                kind,
                state: lifecycle,
            },
        )
        .await
        .map_err(store_failure)?;
    Ok(Json(packs))
}

async fn get_pack(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PackRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_id(&id)?;
    let (record, _bytes) = verified_pack(&state, profile, &id).await?;
    Ok(Json(record))
}

async fn get_pack_bytes(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_id(&id)?;
    let (_record, bytes) = verified_pack(&state, profile, &id).await?;
    let etag = format!("\"{id}\"");
    if if_none_match(&headers, &etag) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_owned()),
            (header::ETAG, etag),
        ],
        Body::from(bytes),
    )
        .into_response())
}

fn if_none_match(headers: &HeaderMap, etag: &str) -> bool {
    headers
        .get(header::IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| {
            value.split(',').any(|candidate| {
                let candidate = candidate.trim();
                candidate == "*" || candidate == etag
            })
        })
}

/// Re-read the stored bytes and move a draft pack to validated.
async fn validate_pack(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PackRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_id(&id)?;
    let registry = registry(&state)?;
    let (record, bytes) = verified_pack(&state, profile.clone(), &id).await?;
    let parsed = parse_pack(&bytes).map_err(|error| {
        failure(
            ErrorCode::Internal,
            format!("the stored pack does not validate: {error}"),
        )
    })?;
    if parsed.canonical != bytes {
        return Err(failure(
            ErrorCode::Internal,
            "the stored pack bytes are not canonical",
        ));
    }
    if parsed.pack_id.to_string() != id
        || parsed.kind != record.summary.kind
        || parsed.name != record.summary.name
        || parsed.version != record.summary.version
        || parsed.owner != record.summary.owner
    {
        return Err(failure(
            ErrorCode::Internal,
            "the stored pack disagrees with its registry row",
        ));
    }
    let _gate = state.pack_gate.lock().await;
    let record = registry
        .validate(
            profile,
            id,
            clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?,
        )
        .await
        .map_err(store_failure)?;
    Ok(Json(record))
}

async fn freeze_pack(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PackRecord>, ApiFailure> {
    transition(state, headers, id, Transition::Freeze).await
}

async fn retire_pack(
    State(state): State<Arc<DataState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<PackRecord>, ApiFailure> {
    transition(state, headers, id, Transition::Retire).await
}

#[derive(Debug, Clone, Copy)]
enum Transition {
    Freeze,
    Retire,
}

async fn transition(
    state: Arc<DataState>,
    headers: HeaderMap,
    id: String,
    transition: Transition,
) -> Result<Json<PackRecord>, ApiFailure> {
    let profile = request_profile(&headers)?;
    let id = parse_id(&id)?;
    let (_verified, _bytes) = verified_pack(&state, profile.clone(), &id).await?;
    let at = clock::now_utc().map_err(|error| failure(ErrorCode::Internal, error.to_string()))?;
    let registry = registry(&state)?;
    let record = {
        let _gate = state.pack_gate.lock().await;
        match transition {
            Transition::Freeze => registry.freeze(profile, id, at).await,
            Transition::Retire => registry.retire(profile, id, at).await,
        }
        .map_err(store_failure)?
    };
    Ok(Json(record))
}
