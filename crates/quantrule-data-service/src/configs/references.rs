//! v5 reference resolution for the config bundle.
//!
//! The config catalog carries references, never pack or signal bodies. Packs
//! and the optional signal artifact are data-owned: the object bytes are read
//! and hashed before a reference is returned. A model artifact is
//! evidence-owned and is carried structurally here; evidence validates it
//! against the succeeded model stage.

use quantrule_api::{
    ArtifactKind, Digest, ErrorCode, PackProvenance, ProfileId, ReferenceKind, RunArtifactReference,
};
use quantrule_app::config::{RUN_SCHEMA_V5, parse_v5_references};
use serde_json::Value;

use crate::routes::{ApiFailure, failure};
use crate::state::DataState;

/// Resolve the v5 reference set a config bundle names.
pub(crate) async fn config_references(
    state: &DataState,
    profile: &ProfileId,
    body: &str,
) -> Result<Vec<RunArtifactReference>, ApiFailure> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| failure(ErrorCode::ConfigInvalid, error.to_string()))?;
    if value.get("schema").and_then(Value::as_str) != Some(RUN_SCHEMA_V5) {
        return Ok(Vec::new());
    }
    let references = parse_v5_references("run.json", &value)
        .map_err(|error| failure(ErrorCode::ConfigInvalid, error.to_string()))?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| failure(ErrorCode::Internal, "no object store is configured"))?;
    let mut resolved = Vec::new();
    for (kind, reference) in references.each() {
        let pack = if kind.pack_kind().is_some() {
            let registry = state
                .packs
                .as_ref()
                .ok_or_else(|| failure(ErrorCode::Internal, "no pack registry is configured"))?;
            let record = registry
                .get(profile.clone(), reference.id.to_string())
                .await
                .map_err(|error| failure(ErrorCode::PackNotFound, error.to_string()))?
                .ok_or_else(|| {
                    failure(
                        ErrorCode::PackNotFound,
                        format!("pack {} is unknown", reference.id),
                    )
                })?;
            let bytes = objects.get(&record.object_key).await.map_err(|error| {
                failure(
                    ErrorCode::Internal,
                    format!(
                        "the pack object {} is unreadable: {error}",
                        record.object_key
                    ),
                )
            })?;
            if Digest::sha256(&bytes) != record.sha256 || bytes.len() as u64 != record.summary.bytes
            {
                return Err(failure(
                    ErrorCode::Internal,
                    "the pack object does not match its registry identity",
                ));
            }
            Some(PackProvenance {
                kind: record.summary.kind,
                name: record.summary.name,
                version: record.summary.version,
                owner: record.summary.owner,
                state: record.summary.state,
            })
        } else {
            None
        };
        if kind == ReferenceKind::SignalArtifact
            && let Some(catalog) = state.artifacts.as_ref()
        {
            let record = catalog
                .get(
                    profile.clone(),
                    ArtifactKind::Signal,
                    reference.id.to_string(),
                )
                .await
                .map_err(|error| failure(ErrorCode::ArtifactNotFound, error.to_string()))?
                .ok_or_else(|| {
                    failure(
                        ErrorCode::ArtifactNotFound,
                        format!("signal artifact {} is unknown", reference.id),
                    )
                })?;
            let bytes = objects.get(&record.object_key).await.map_err(|error| {
                failure(
                    ErrorCode::Internal,
                    format!(
                        "the signal object {} is unreadable: {error}",
                        record.object_key
                    ),
                )
            })?;
            if Digest::sha256(&bytes) != record.sha256 || bytes.len() as u64 != record.summary.bytes
            {
                return Err(failure(
                    ErrorCode::Internal,
                    "the signal object does not match its catalog identity",
                ));
            }
        }
        resolved.push(RunArtifactReference {
            kind,
            id: reference.id.clone(),
            hash: reference.hash.clone(),
            pack,
        });
    }
    Ok(resolved)
}
