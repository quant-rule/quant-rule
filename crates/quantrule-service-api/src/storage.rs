//! The `quantrule.internal/v1` storage references.
//!
//! A service never sends a config body, a candle series, or a reading series to
//! another service. It sends a bundle, an object reference, and a manifest of
//! content-addressed artifacts. These types carry exactly that envelope.

use std::collections::BTreeMap;

use quantrule_api::{ArtifactRef, Digest, RunArtifactReference};
use serde::{Deserialize, Serialize};

/// The model bundle schema version.
pub const MODEL_BUNDLE_SCHEMA: &str = "quantrule.model-bundle/v1";

/// The six configuration files and their validation result.
///
/// The files are keyed by their path inside the bundle, including `run.json`.
/// `config_hash` is present only when the bundle validates; a missing hash is a
/// rejection, not an empty value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigBundle {
    /// Stable content-derived id.
    pub config_id: String,
    /// The bundle's files, keyed by relative path.
    pub files: BTreeMap<String, String>,
    /// The v5 reference set, empty for a v3 or v4 run.
    ///
    /// The set names content, not payloads. Evidence records it as lineage;
    /// the worker resolves the bytes once before the fold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<RunArtifactReference>,
    /// Whether the run config and its references validate.
    pub valid: bool,
    /// The validator message when invalid.
    pub message: Option<String>,
    /// SHA-256 over the bundle files, when valid.
    pub config_hash: Option<Digest>,
}

impl ConfigBundle {
    /// The model-input identity carried by the bundle.
    ///
    /// The strategy, execution, and portfolio files do not contribute; a
    /// strategy-only change therefore keeps the same model input.
    ///
    /// # Errors
    ///
    /// Returns a message when `run.json`, the data or model file, the data
    /// version, or the ladder is missing or mistyped.
    pub fn model_input_hash(&self) -> Result<Digest, String> {
        let run: serde_json::Value = serde_json::from_str(
            self.files
                .get("run.json")
                .ok_or_else(|| "the bundle has no run.json".to_owned())?,
        )
        .map_err(|error| format!("run.json is not JSON: {error}"))?;
        let data_key = run
            .get("data")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "run.json names no data file".to_owned())?;
        let model_key = run
            .get("model")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "run.json names no model file".to_owned())?;
        let model_config = self
            .files
            .get(model_key)
            .ok_or_else(|| format!("the bundle has no {model_key}"))?;
        let data: serde_json::Value = serde_json::from_str(
            self.files
                .get(data_key)
                .ok_or_else(|| format!("the bundle has no {data_key}"))?,
        )
        .map_err(|error| format!("{data_key} is not JSON: {error}"))?;
        let data_version = data
            .get("data_version")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{data_key} has no data_version"))?;
        let ladder = run
            .get("model_timeframes")
            .filter(|value| !value.is_null())
            .or_else(|| data.get("available_timeframes"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "the bundle names no model ladder".to_owned())?;
        let ladder: Vec<String> = ladder
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "a ladder entry is not a string".to_owned())
            })
            .collect::<Result<_, _>>()?;
        Ok(quantrule_api::model_input_hash(
            data_version,
            model_config.as_bytes(),
            &ladder,
        ))
    }
}

/// One immutable dataset as another service sees it.
///
/// The candle bytes stay in the object store; this envelope names them and
/// records the coverage a consumer can rely on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatasetObject {
    /// The content-derived data version.
    pub data_version: Digest,
    /// The CAS key of the canonical CSV.
    pub object_key: String,
    /// The SHA-256 of the stored object.
    pub sha256: Digest,
    /// The candle count.
    pub candles: u64,
    /// The first bar, as Unix seconds.
    pub first_bar: u64,
    /// The last bar, as Unix seconds.
    pub last_bar: u64,
    /// The stored byte count.
    pub bytes: u64,
}

/// One model bundle, as the worker consumes it.
///
/// The manifest lists every artifact with its hash; the worker materialises the
/// referenced objects and never recomputes the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBundleManifest {
    /// The schema token.
    pub schema: String,
    /// The dataset the model consumed.
    pub data_version: Digest,
    /// The configuration id.
    pub config_id: String,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model registry version.
    pub model_version: String,
    /// The computed timeframes, ascending.
    pub ladder: Vec<String>,
    /// The CAS key of the manifest itself.
    pub manifest_key: String,
    /// The artifact set, ordered by name.
    pub artifacts: Vec<ArtifactRef>,
}

impl ModelBundleManifest {
    /// The canonical model artifact id over the manifest entries.
    ///
    /// A model artifact is a file set, not one document. Its id is the
    /// canonical digest over `(name, sha256)`, so a caller that resolved a
    /// bundle can verify the id it was given without trusting the manifest
    /// field order.
    #[must_use]
    pub fn artifact_id(&self) -> Digest {
        let entries: Vec<(String, Digest)> = self
            .artifacts
            .iter()
            .map(|artifact| (artifact.name.clone(), artifact.sha256.clone()))
            .collect();
        Digest::canonical(&entries)
    }

    /// The object key of one manifest artifact.
    ///
    /// The manifest key names the bundle directory; each artifact name is
    /// relative to that directory.
    #[must_use]
    pub fn artifact_key(&self, name: &str) -> Option<String> {
        if self.manifest_key == "manifest.json" {
            return Some(name.to_owned());
        }
        let prefix = self.manifest_key.strip_suffix("/manifest.json")?;
        Some(format!("{prefix}/{name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn config_bundle() -> ConfigBundle {
        ConfigBundle {
            config_id: "cfg-1".to_owned(),
            references: Vec::new(),
            files: BTreeMap::from([
                ("run.json".to_owned(), "{\"model_id\":\"M1.3\"}".to_owned()),
                ("strategy.json".to_owned(), "{}".to_owned()),
            ]),
            valid: true,
            message: None,
            config_hash: Some(digest(b"bundle")),
        }
    }

    fn dataset_object() -> DatasetObject {
        DatasetObject {
            data_version: digest(b"data"),
            object_key: format!("datasets/{}", digest(b"data").as_hex()),
            sha256: digest(b"data"),
            candles: 10,
            first_bar: 1_700_000_000,
            last_bar: 1_700_086_400,
            bytes: 1_024,
        }
    }

    fn bundle_manifest() -> ModelBundleManifest {
        ModelBundleManifest {
            schema: MODEL_BUNDLE_SCHEMA.to_owned(),
            data_version: digest(b"data"),
            config_id: "cfg-1".to_owned(),
            config_hash: digest(b"bundle"),
            model_version: "M1.3".to_owned(),
            ladder: vec!["1m".to_owned(), "5m".to_owned()],
            manifest_key: "models/bundle/manifest.json".to_owned(),
            artifacts: vec![ArtifactRef {
                name: "model.json".to_owned(),
                bytes: 2,
                sha256: digest(b"{}"),
            }],
        }
    }

    #[test]
    fn a_model_input_hash_ignores_the_strategy() {
        let mut bundle = ConfigBundle {
            config_id: "cfg-model".to_owned(),
            references: Vec::new(),
            files: BTreeMap::from([
                (
                    "run.json".to_owned(),
                    r#"{"data":"data.json","model":"model.json"}"#.to_owned(),
                ),
                (
                    "data.json".to_owned(),
                    r#"{"data_version":"v1","available_timeframes":["1m","5m"]}"#.to_owned(),
                ),
                ("model.json".to_owned(), r#"{"schema":"v4"}"#.to_owned()),
                ("strategy.json".to_owned(), "{}".to_owned()),
            ]),
            valid: true,
            message: None,
            config_hash: Some(digest(b"bundle")),
        };
        let base = bundle.model_input_hash().expect("the model input");
        bundle
            .files
            .insert("strategy.json".to_owned(), r#"{"id":"changed"}"#.to_owned());
        assert_eq!(
            base,
            bundle.model_input_hash().expect("the strategy change")
        );
        bundle.files.insert(
            "model.json".to_owned(),
            r#"{"schema":"changed"}"#.to_owned(),
        );
        assert_ne!(base, bundle.model_input_hash().expect("the model change"));
    }

    #[test]
    fn a_config_bundle_round_trips() {
        let bundle = config_bundle();
        let text = serde_json::to_string(&bundle).expect("encode");
        assert_eq!(
            serde_json::from_str::<ConfigBundle>(&text).expect("decode"),
            bundle
        );
    }

    #[test]
    fn a_bundle_without_references_keeps_the_legacy_shape() {
        let bundle = config_bundle();
        let text = serde_json::to_string(&bundle).expect("encode");
        assert!(!text.contains("\"references\""), "{text}");
        let decoded: ConfigBundle = serde_json::from_str(&text).expect("decode");
        assert!(decoded.references.is_empty());
    }

    #[test]
    fn a_dataset_object_round_trips() {
        let object = dataset_object();
        let text = serde_json::to_string(&object).expect("encode");
        assert_eq!(
            serde_json::from_str::<DatasetObject>(&text).expect("decode"),
            object
        );
    }

    #[test]
    fn a_model_bundle_manifest_round_trips() {
        let manifest = bundle_manifest();
        let text = serde_json::to_string(&manifest).expect("encode");
        assert_eq!(
            serde_json::from_str::<ModelBundleManifest>(&text).expect("decode"),
            manifest
        );
        assert!(text.contains(MODEL_BUNDLE_SCHEMA));
    }

    #[test]
    fn the_model_artifact_id_is_the_canonical_entry_digest() {
        let manifest = bundle_manifest();
        let expected = Digest::canonical(&[(
            "model.json".to_owned(),
            manifest.artifacts[0].sha256.clone(),
        )]);
        assert_eq!(manifest.artifact_id(), expected);

        let mut reordered = manifest.clone();
        reordered.artifacts.push(ArtifactRef {
            name: "a.csv".to_owned(),
            bytes: 1,
            sha256: digest(b"a"),
        });
        let expected = reordered.artifact_id();
        reordered.artifacts.reverse();
        assert_eq!(reordered.artifact_id(), expected);
    }

    #[test]
    fn an_artifact_key_is_below_the_manifest_directory() {
        let manifest = bundle_manifest();
        assert_eq!(
            manifest.artifact_key("model.json").as_deref(),
            Some("models/bundle/model.json")
        );
        let mut flat = manifest;
        flat.manifest_key = "manifest.json".to_owned();
        assert_eq!(
            flat.artifact_key("model.json").as_deref(),
            Some("model.json")
        );
        flat.manifest_key = "models/bundle.json".to_owned();
        assert_eq!(flat.artifact_key("model.json"), None);
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        let text = format!(
            "{{\"data_version\":\"sha256:{}\",\"object_key\":\"k\",\"sha256\":\"sha256:{}\",\
             \"candles\":1,\"first_bar\":0,\"last_bar\":0,\"bytes\":1,\"path\":\"/tmp\"}}",
            "a".repeat(64),
            "b".repeat(64)
        );
        assert!(serde_json::from_str::<DatasetObject>(&text).is_err());
    }

    #[test]
    fn the_wire_forms_carry_no_payload_series() {
        let text = serde_json::to_string(&bundle_manifest()).expect("encode");
        assert!(!text.contains("candles\":["));
        assert!(!text.contains("readings"));
    }
}
