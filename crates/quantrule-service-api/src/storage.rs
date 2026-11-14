//! The `quantrule.internal/v1` storage references.
//!
//! A service never sends a config body, a candle series, or a reading series to
//! another service. It sends a bundle, an object reference, and a manifest of
//! content-addressed artifacts. These types carry exactly that envelope.

use std::collections::BTreeMap;

use quantrule_api::{ArtifactRef, Digest};
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
    /// Whether the run config and its references validate.
    pub valid: bool,
    /// The validator message when invalid.
    pub message: Option<String>,
    /// SHA-256 over the bundle files, when valid.
    pub config_hash: Option<Digest>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn config_bundle() -> ConfigBundle {
        ConfigBundle {
            config_id: "cfg-1".to_owned(),
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
    fn a_config_bundle_round_trips() {
        let bundle = config_bundle();
        let text = serde_json::to_string(&bundle).expect("encode");
        assert_eq!(
            serde_json::from_str::<ConfigBundle>(&text).expect("decode"),
            bundle
        );
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
