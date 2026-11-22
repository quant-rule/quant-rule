//! Resolve a frozen model artifact and a signal-mode pack, then publish a signal.
//!
//! The order is the contract: the pack is checked for kind, frozen state,
//! canonical bytes, id and signal mode before the model client is called, so a
//! fold pack can neither compute nor write. The model manifest is then checked
//! for schema, dataset, config and canonical artifact id, every listed object is
//! hash-checked, and only then does the pure computation run.
//!
//! No temporary directory is used. The model bundle is assembled from the
//! verified object bytes, and the signal bytes go directly to
//! `artifacts/<sha256 hex>` before the metadata-only catalog registration.

use std::collections::BTreeSet;

use quantrule_api::{ArtifactKind, Digest, PackKind, PackState, ProfileId};
use quantrule_app::pack::{StrategyMode, StrategyPack};
use quantrule_app::signal::{SignalComputeRequest, compute};
use quantrule_app::stages::strategy::ModelBundle;
use quantrule_runtime::adapters::put_if_absent;
use quantrule_runtime::client::ClientError;
use quantrule_service_api::{
    MODEL_BUNDLE_SCHEMA, ModelBundleManifest, SignalRequest, SignalResponse,
};

use crate::error::{SignalServiceError, client_failure};
use crate::state::StrategyState;

/// Resolve the inputs, compute the signal, write it and register the catalog row.
///
/// # Errors
///
/// Returns [`SignalServiceError`] when the pack is unknown or not a frozen
/// signal-mode pack, the model reference is unknown or disagrees with the
/// resolved manifest, a model object is absent or corrupt, or the object-store
/// or catalog write fails.
pub async fn compute_signal(
    state: &StrategyState,
    profile: ProfileId,
    request: &SignalRequest,
) -> Result<SignalResponse, SignalServiceError> {
    let pack = load_pack(state, profile.clone(), request).await?;
    let bundle = load_bundle(state, profile.clone(), request).await?;
    let model_artifact_id = request.model_artifact_id.clone();
    let compute_bundle = bundle.clone();
    let compute_pack = pack.clone();
    let artifact = tokio::task::spawn_blocking(move || {
        compute(&SignalComputeRequest {
            bundle: &compute_bundle,
            pack: &compute_pack,
            model_artifact_id: &model_artifact_id,
        })
    })
    .await
    .map_err(|error| SignalServiceError::Internal(error.to_string()))?
    .map_err(SignalServiceError::from)?;

    verify_artifact(
        &artifact,
        &pack,
        &request.model_artifact_id,
        &bundle.data_version,
    )?;

    let bytes = artifact.canonical_bytes().to_vec();
    let artifacts = state.artifacts.as_ref().ok_or_else(|| {
        SignalServiceError::Internal("no artifact registry is configured".to_owned())
    })?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| SignalServiceError::Internal("no object store is configured".to_owned()))?;
    let key = format!("artifacts/{}", artifact.id().as_hex());
    let wrote = put_if_absent(objects.as_ref(), &key, bytes.clone())
        .await
        .map_err(|error| SignalServiceError::Internal(error.to_string()))?;
    if !wrote {
        let stored = objects
            .get(&key)
            .await
            .map_err(|error| SignalServiceError::Internal(error.to_string()))?;
        if stored != bytes {
            return Err(SignalServiceError::Internal(
                "the stored signal object does not match its content address".to_owned(),
            ));
        }
    }
    let registration = artifacts
        .register(
            profile,
            ArtifactKind::Signal,
            artifact.id().clone(),
            bytes.len() as u64,
        )
        .await
        .map_err(client_failure)?;
    Ok(SignalResponse::new(
        registration.record,
        registration.created,
    ))
}

/// Re-check the computed artifact against the resolved frozen inputs.
///
/// This is the service-side companion to the pure computation: the pack, model
/// id, execution timeframe, data-version label and content id must all name the
/// inputs the caller resolved.
fn verify_artifact(
    artifact: &quantrule_app::signal::SignalArtifact,
    pack: &StrategyPack,
    model_artifact_id: &Digest,
    bundle_data_version: &str,
) -> Result<(), SignalServiceError> {
    artifact.verify_sources(pack, model_artifact_id)?;
    if artifact.data_version().is_empty() || artifact.data_version() != bundle_data_version {
        return Err(SignalServiceError::Invalid(
            "the signal artifact names another data version".to_owned(),
        ));
    }
    if Digest::sha256(artifact.canonical_bytes()) != *artifact.id() {
        return Err(SignalServiceError::Internal(
            "the signal artifact id does not cover its canonical bytes".to_owned(),
        ));
    }
    Ok(())
}

async fn load_pack(
    state: &StrategyState,
    profile: ProfileId,
    request: &SignalRequest,
) -> Result<StrategyPack, SignalServiceError> {
    let packs = state
        .packs
        .as_ref()
        .ok_or_else(|| SignalServiceError::Internal("no pack registry is configured".to_owned()))?;
    let record = packs
        .pack(profile.clone(), request.strategy_pack_id.clone())
        .await
        .map_err(client_failure)?
        .ok_or_else(|| SignalServiceError::PackNotFound(request.strategy_pack_id.clone()))?;
    if record.summary.kind != PackKind::Strategy {
        return Err(SignalServiceError::Invalid(format!(
            "the pack is {}, not a strategy pack",
            record.summary.kind
        )));
    }
    if record.summary.state != PackState::Frozen {
        return Err(SignalServiceError::Invalid(format!(
            "the strategy pack is {}, not frozen",
            record.summary.state
        )));
    }
    if record.summary.pack_id != record.sha256 {
        return Err(SignalServiceError::Invalid(
            "the pack row names two different content addresses".to_owned(),
        ));
    }
    let bytes = packs
        .pack_bytes(profile, request.strategy_pack_id.clone())
        .await
        .map_err(client_failure)?
        .ok_or_else(|| SignalServiceError::Invalid("the pack object is missing".to_owned()))?;
    if bytes.len() as u64 != record.summary.bytes
        || Digest::sha256(&bytes) != record.summary.pack_id
    {
        return Err(SignalServiceError::Invalid(
            "the pack object does not match its registry identity".to_owned(),
        ));
    }
    let pack = StrategyPack::from_stored_bytes(&bytes, "strategy pack")
        .map_err(|error| SignalServiceError::Invalid(error.to_string()))?;
    if pack.id() != &request.strategy_pack_id {
        return Err(SignalServiceError::Invalid(format!(
            "the pack resolves to {}, not {}",
            pack.id(),
            request.strategy_pack_id
        )));
    }
    if pack.payload().mode() != StrategyMode::Signal {
        return Err(SignalServiceError::Invalid(
            "a fold-mode strategy pack has no signal artifact; it runs inside the backtest fold"
                .to_owned(),
        ));
    }
    Ok(pack)
}

async fn load_bundle(
    state: &StrategyState,
    profile: ProfileId,
    request: &SignalRequest,
) -> Result<ModelBundle, SignalServiceError> {
    let model = state
        .model
        .as_ref()
        .ok_or_else(|| SignalServiceError::Internal("no model service is configured".to_owned()))?;
    let manifest = match model
        .bundle(
            profile,
            request.data_version.clone(),
            request.config_id.clone(),
        )
        .await
    {
        Ok(manifest) => manifest,
        Err(ClientError::Status { status: 404, .. }) => {
            return Err(SignalServiceError::ModelNotFound {
                data_version: request.data_version.clone(),
                config_id: request.config_id.clone(),
            });
        }
        Err(ClientError::Status {
            status, message, ..
        }) if (400..500).contains(&status) => {
            return Err(SignalServiceError::Invalid(format!(
                "the model service refused the request: {message}"
            )));
        }
        Err(error) => return Err(client_failure(error)),
    };
    if manifest.schema != MODEL_BUNDLE_SCHEMA {
        return Err(SignalServiceError::Invalid(format!(
            "the model manifest schema is {}, not {MODEL_BUNDLE_SCHEMA}",
            manifest.schema
        )));
    }
    if manifest.data_version != request.data_version {
        return Err(SignalServiceError::Invalid(
            "the model manifest belongs to another dataset".to_owned(),
        ));
    }
    if manifest.config_id != request.config_id {
        return Err(SignalServiceError::Invalid(
            "the model manifest belongs to another config".to_owned(),
        ));
    }
    if manifest.artifact_id() != request.model_artifact_id {
        return Err(SignalServiceError::Invalid(format!(
            "the model manifest is {}, not {}",
            manifest.artifact_id(),
            request.model_artifact_id
        )));
    }
    validate_manifest_layout(&manifest)?;
    let objects = state
        .objects
        .as_ref()
        .ok_or_else(|| SignalServiceError::Internal("no object store is configured".to_owned()))?;
    let mut provenance = None;
    let mut readings = None;
    let mut events = None;
    for artifact in &manifest.artifacts {
        let key = manifest.artifact_key(&artifact.name).ok_or_else(|| {
            SignalServiceError::Invalid(format!(
                "the model manifest key {} is not a bundle manifest",
                manifest.manifest_key
            ))
        })?;
        let bytes = objects
            .get(&key)
            .await
            .map_err(|error| SignalServiceError::Internal(error.to_string()))?;
        if bytes.len() as u64 != artifact.bytes || Digest::sha256(&bytes) != artifact.sha256 {
            return Err(SignalServiceError::Internal(format!(
                "the model bundle artifact {} does not match its manifest",
                artifact.name
            )));
        }
        match artifact.name.as_str() {
            "provenance.json" => provenance = Some(bytes),
            "readings.csv" => readings = Some(bytes),
            "events.csv" => events = Some(bytes),
            _ => {}
        }
    }
    let provenance = provenance.ok_or_else(|| {
        SignalServiceError::Invalid("the model bundle has no provenance.json".to_owned())
    })?;
    let readings = readings.ok_or_else(|| {
        SignalServiceError::Invalid("the model bundle has no readings.csv".to_owned())
    })?;
    let events = events.ok_or_else(|| {
        SignalServiceError::Invalid("the model bundle has no events.csv".to_owned())
    })?;
    ModelBundle::from_files(&provenance, &readings, &events)
        .map_err(|error| SignalServiceError::Invalid(error.to_string()))
}

/// Refuse a manifest key or artifact name that could escape the model namespace.
///
/// The object store is content-addressed, but the filesystem adapter still
/// treats a key as a path. A manifest is metadata from another service, so the
/// writer must reject `..`, separators and hidden names before any object read.
fn validate_manifest_layout(manifest: &ModelBundleManifest) -> Result<(), SignalServiceError> {
    if !manifest.manifest_key.starts_with("model/")
        || !manifest.manifest_key.ends_with("/manifest.json")
    {
        return Err(SignalServiceError::Invalid(format!(
            "the model manifest key {} is not a model bundle manifest",
            manifest.manifest_key
        )));
    }
    let mut names = BTreeSet::new();
    for artifact in &manifest.artifacts {
        if artifact.name.is_empty()
            || artifact.name.starts_with('.')
            || artifact.name.contains('/')
            || artifact.name.contains('\\')
            || artifact.name.contains("..")
        {
            return Err(SignalServiceError::Invalid(format!(
                "the model manifest names an unsafe artifact {:?}",
                artifact.name
            )));
        }
        if !names.insert(artifact.name.as_str()) {
            return Err(SignalServiceError::Invalid(format!(
                "the model manifest names {} twice",
                artifact.name
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn fixture() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../quantrule-app/tests/fixtures/signal/model")
    }

    fn pack() -> StrategyPack {
        let value = serde_json::json!({
            "schema": "quantrule.pack/v1",
            "kind": "strategy",
            "version": "1.0.0",
            "name": "signal-event-5m",
            "owner": "operator",
            "payload": {
                "mode": "signal",
                "engine": "event_composite",
                "execution_timeframe": "5m",
                "context_timeframes": [],
                "event_window_bars": 1,
                "time_stop_bars": 0,
                "giveback_atr": null,
                "giveback_arm_atr": 0.0,
                "require_contiguous": true,
                "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
                "entry": [
                    {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1,
                     "source": 4, "direction": 1}
                ],
                "exit": []
            }
        });
        StrategyPack::parse(&value, "pack.json").expect("the signal pack is valid")
    }

    fn manifest(manifest_key: &str, names: &[&str]) -> ModelBundleManifest {
        ModelBundleManifest {
            schema: MODEL_BUNDLE_SCHEMA.to_owned(),
            data_version: Digest::sha256(b"data"),
            config_id: "cfg_fixture".to_owned(),
            config_hash: Digest::sha256(b"config"),
            model_version: "M1.3".to_owned(),
            ladder: vec!["5m".to_owned()],
            manifest_key: manifest_key.to_owned(),
            artifacts: names
                .iter()
                .map(|name| quantrule_api::ArtifactRef {
                    name: (*name).to_owned(),
                    bytes: 1,
                    sha256: Digest::sha256(name.as_bytes()),
                })
                .collect(),
        }
    }

    #[test]
    fn a_manifest_key_or_artifact_name_cannot_escape_the_model_namespace() {
        validate_manifest_layout(&manifest("model/bundle/manifest.json", &["readings.csv"]))
            .expect("a normal model manifest");
        for key in ["manifest.json", "model/bundle", "../manifest.json"] {
            let error = validate_manifest_layout(&manifest(key, &["readings.csv"]))
                .expect_err("an invalid manifest key");
            assert!(matches!(error, SignalServiceError::Invalid(_)), "{error}");
        }
        for name in ["../readings.csv", "sub/readings.csv", ".hidden", ""] {
            let error = validate_manifest_layout(&manifest("model/bundle/manifest.json", &[name]))
                .expect_err("an unsafe artifact name");
            assert!(matches!(error, SignalServiceError::Invalid(_)), "{error}");
        }
        let error = validate_manifest_layout(&manifest("model/bundle/manifest.json", &["a", "a"]))
            .expect_err("a duplicate artifact name");
        assert!(matches!(error, SignalServiceError::Invalid(_)), "{error}");
    }

    #[test]
    fn a_verify_sources_mismatch_is_refused() {
        let bundle = ModelBundle::load(&fixture()).expect("the fixture bundle");
        let pack = pack();
        let model = Digest::sha256(b"model");
        let artifact = compute(&SignalComputeRequest {
            bundle: &bundle,
            pack: &pack,
            model_artifact_id: &model,
        })
        .expect("the fixture computes");
        verify_artifact(&artifact, &pack, &model, &bundle.data_version)
            .expect("the resolved inputs match");
        let error = verify_artifact(
            &artifact,
            &pack,
            &Digest::sha256(b"other"),
            &bundle.data_version,
        )
        .expect_err("another model id is refused");
        assert!(matches!(error, SignalServiceError::Invalid(_)));
        let error = verify_artifact(&artifact, &pack, &model, "another-data-version")
            .expect_err("another data version is refused");
        assert!(matches!(error, SignalServiceError::Invalid(_)));
    }
}
