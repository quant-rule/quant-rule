//! Resolve a v5 run's references once for the worker.
//!
//! Pack and signal bytes come from the data service; the model-bundle manifest
//! is reconstructed from the materialized model files and their published
//! provenance ladder. The worker verifies every hash and fails closed before
//! the fold.

use quantrule_api::{ArtifactRef, ContentReference, Digest, ProfileId};
use quantrule_app::pack::{MemoryReferences, ReferenceKind};
use quantrule_runtime::client::DataClient;
use quantrule_service_api::{ConfigBundle, MODEL_BUNDLE_SCHEMA, ModelBundleManifest, RunRecord};

#[derive(Debug)]
pub(crate) enum ReferenceRead {
    /// The data service or object store is unavailable; retry.
    Store(String),
    /// The reference, bytes, or model bundle disagree; fail.
    Mismatch(String),
}

/// Resolve the v5 reference set once, before the fold.
pub(crate) async fn resolve_v5_references(
    profile: &ProfileId,
    record: &RunRecord,
    config: &ConfigBundle,
    model_files: &[(String, Vec<u8>)],
    data: &dyn DataClient,
) -> Result<Option<MemoryReferences>, ReferenceRead> {
    if config.references.is_empty() {
        return Ok(None);
    }
    let mut references = MemoryReferences::new();
    for reference in &config.references {
        match reference.kind {
            ReferenceKind::ModelArtifact => {
                if record.model_artifact_id.as_ref() != Some(&reference.id) {
                    return Err(ReferenceRead::Mismatch(
                        "the v5 model artifact is not the submitted model stage".to_owned(),
                    ));
                }
                let manifest = model_manifest(record, model_files)?;
                let bytes = serde_json::to_vec(&manifest).map_err(|error| {
                    ReferenceRead::Mismatch(format!("the model manifest is not JSON: {error}"))
                })?;
                references.insert_at(
                    reference.kind,
                    ContentReference {
                        id: reference.id.clone(),
                        hash: reference.hash.clone(),
                    },
                    bytes,
                );
            }
            ReferenceKind::SignalArtifact => {
                let bytes = data
                    .signal_bytes(profile.clone(), reference.id.clone())
                    .await
                    .map_err(|error| ReferenceRead::Store(error.to_string()))?
                    .ok_or_else(|| {
                        ReferenceRead::Mismatch(format!(
                            "signal artifact {} is unknown",
                            reference.id
                        ))
                    })?;
                if Digest::sha256(&bytes) != reference.hash {
                    return Err(ReferenceRead::Mismatch(format!(
                        "signal artifact {} does not match its hash",
                        reference.id
                    )));
                }
                references.insert_at(
                    reference.kind,
                    ContentReference {
                        id: reference.id.clone(),
                        hash: reference.hash.clone(),
                    },
                    bytes,
                );
            }
            _ => {
                let bytes = data
                    .pack_bytes(profile.clone(), reference.id.clone())
                    .await
                    .map_err(|error| ReferenceRead::Store(error.to_string()))?
                    .ok_or_else(|| {
                        ReferenceRead::Mismatch(format!("pack {} is unknown", reference.id))
                    })?;
                if Digest::sha256(&bytes) != reference.hash {
                    return Err(ReferenceRead::Mismatch(format!(
                        "pack {} does not match its hash",
                        reference.id
                    )));
                }
                references.insert_at(
                    reference.kind,
                    ContentReference {
                        id: reference.id.clone(),
                        hash: reference.hash.clone(),
                    },
                    bytes,
                );
            }
        }
    }
    Ok(Some(references))
}

fn model_manifest(
    record: &RunRecord,
    model_files: &[(String, Vec<u8>)],
) -> Result<ModelBundleManifest, ReferenceRead> {
    let (_, provenance) = model_files
        .iter()
        .find(|(name, _)| name == "provenance.json")
        .ok_or_else(|| ReferenceRead::Mismatch("the model bundle has no provenance".to_owned()))?;
    let value: serde_json::Value = serde_json::from_slice(provenance)
        .map_err(|error| ReferenceRead::Mismatch(format!("model provenance: {error}")))?;
    let ladder: Vec<String> = value
        .get("ladder")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| ReferenceRead::Mismatch("model provenance has no ladder".to_owned()))?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                ReferenceRead::Mismatch("a model ladder entry is not a string".to_owned())
            })
        })
        .collect::<Result<_, _>>()?;
    if ladder.is_empty() {
        return Err(ReferenceRead::Mismatch(
            "the model ladder is empty".to_owned(),
        ));
    }
    let artifacts = model_files
        .iter()
        .map(|(name, bytes)| ArtifactRef {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: Digest::sha256(bytes),
        })
        .collect();
    Ok(ModelBundleManifest {
        schema: MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version: record.data_version.clone(),
        config_id: record.config_id.clone(),
        config_hash: record.config_hash.clone(),
        model_version: record.model_version.clone(),
        ladder,
        manifest_key: "manifest.json".to_owned(),
        artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{PackKind, PackProvenance, PackState, RunArtifactReference};
    use quantrule_runtime::client::MemoryDataClient;
    use quantrule_service_api::NewRun;
    use serde_json::json;

    fn profile() -> ProfileId {
        ProfileId::parse("default").expect("profile")
    }

    fn strategy() -> RunArtifactReference {
        let id = Digest::sha256(b"strategy");
        RunArtifactReference {
            kind: ReferenceKind::StrategyPack,
            id: id.clone(),
            hash: id,
            pack: Some(PackProvenance {
                kind: PackKind::Strategy,
                name: "compression-fade-5m".to_owned(),
                version: "1.0.0".to_owned(),
                owner: "operator".to_owned(),
                state: PackState::Frozen,
            }),
        }
    }

    fn bundle(reference: RunArtifactReference) -> quantrule_service_api::ConfigBundle {
        quantrule_service_api::ConfigBundle {
            config_id: "cfg_v5".to_owned(),
            references: vec![reference],
            files: std::collections::BTreeMap::new(),
            valid: true,
            message: None,
            config_hash: None,
        }
    }

    fn record(model: Option<Digest>) -> RunRecord {
        RunRecord::queued(&NewRun {
            run_id: quantrule_api::RunId(Digest::sha256(b"run")),
            config_id: "cfg_v5".to_owned(),
            data_version: Digest::sha256(b"data"),
            config_hash: Digest::sha256(b"config"),
            model_version: "M1.0".to_owned(),
            image_digest: Digest::sha256(b"image"),
            model_artifact_id: model,
            lineage: Vec::new(),
            owner: "default".to_owned(),
            submitted_at: "2026-11-26T00:00:00Z".to_owned(),
        })
    }

    #[tokio::test]
    async fn a_missing_pack_fails_closed() {
        let data = MemoryDataClient::default();
        let error =
            resolve_v5_references(&profile(), &record(None), &bundle(strategy()), &[], &data)
                .await
                .expect_err("missing pack");
        assert!(matches!(error, ReferenceRead::Mismatch(_)));
    }

    #[tokio::test]
    async fn a_pack_is_inserted_once() {
        let data = MemoryDataClient::default();
        let reference = strategy();
        data.add_pack_bytes(&profile(), &reference.hash, b"strategy".to_vec());
        let references = resolve_v5_references(
            &profile(),
            &record(None),
            &bundle(reference.clone()),
            &[],
            &data,
        )
        .await
        .expect("resolved")
        .expect("v5");
        assert_eq!(
            references.get(
                ReferenceKind::StrategyPack,
                &ContentReference {
                    id: reference.id.clone(),
                    hash: reference.hash.clone(),
                }
            ),
            Some(b"strategy".as_slice())
        );
    }

    #[test]
    fn a_model_manifest_carries_the_artifact_set() {
        let reference = ContentReference {
            id: Digest::sha256(b"model"),
            hash: Digest::sha256(b"model"),
        };
        let model_files = vec![
            (
                "provenance.json".to_owned(),
                serde_json::to_vec(&json!({"ladder": ["5m"]})).expect("json"),
            ),
            ("readings.csv".to_owned(), b"rows".to_vec()),
        ];
        let record = record(Some(reference.hash.clone()));
        let manifest = model_manifest(&record, &model_files).expect("manifest");
        let expected = quantrule_api::StageArtifact::from_files(
            model_files
                .iter()
                .map(|(name, bytes)| ArtifactRef {
                    name: name.clone(),
                    bytes: bytes.len() as u64,
                    sha256: Digest::sha256(bytes),
                })
                .collect(),
        )
        .expect("artifact")
        .artifact_id;
        assert_eq!(manifest.artifact_id(), expected);
        assert_eq!(manifest.ladder, vec!["5m"]);
    }
}
