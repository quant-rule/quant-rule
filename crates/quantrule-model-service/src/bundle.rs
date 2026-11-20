//! The model-bundle writer and its internal manifest.
//!
//! The public model object is one JSON report. The internal bundle is the
//! artifact set the backtest worker materializes: the same flat report plus the
//! exhaustive readings, events, levels, legs, summary, and provenance. The
//! writer renders the shared application artifacts into a private directory,
//! copies each one into the object store by content-derived key, and publishes a
//! manifest that lists every artifact with its hash.
//!
//! The bundle never contains a strategy, execution, or portfolio value. Its
//! inputs are the dataset object, the config bundle, and the model engine
//! configuration.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::{ArtifactRef, Digest};
use quantrule_app::artifact::model_bundle::{ModelBundleError, model_bundle_from};
use quantrule_app::config::RunConfig;
use quantrule_app::pipeline;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_service_api::{MODEL_BUNDLE_SCHEMA, ModelBundleManifest};

use crate::model::ModelConfig;
use crate::model::series::{SeriesError, series};

/// The directory-segment model version, so an artifact key names the engine that
/// produced it.
pub(crate) const MODEL_VERSION: &str = quantrule_model::REGISTRY_VERSION;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// Why a model bundle could not be built or stored.
#[derive(Debug)]
pub(crate) enum BundleError {
    /// The configured model ladder is empty.
    EmptyLadder,
    /// The dataset bytes could not feed the model.
    Series(SeriesError),
    /// The requested period selected no candle.
    EmptyPeriod,
    /// An artifact could not be rendered or read.
    Output(String),
    /// The object store rejected a read or a write.
    Store(String),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyLadder => formatter.write_str("the model ladder is empty"),
            Self::Series(error) => error.fmt(formatter),
            Self::EmptyPeriod => {
                formatter.write_str("no candle falls inside the configured period")
            }
            Self::Output(message) => formatter.write_str(message),
            Self::Store(message) => formatter.write_str(message),
        }
    }
}

/// A built artifact set and the keys it will be stored under.
pub(crate) struct BuiltBundle {
    /// The prefix every artifact key shares.
    pub key_prefix: String,
    /// The key of the manifest itself.
    pub manifest_key: String,
    /// The computed timeframes, ascending.
    pub ladder: Vec<String>,
    /// Every artifact, ordered by name.
    pub artifacts: Vec<(String, Vec<u8>)>,
}

impl BuiltBundle {
    /// The object key of one artifact.
    #[must_use]
    pub fn artifact_key(&self, name: &str) -> String {
        format!("{}/{name}", self.key_prefix)
    }
}

/// Render the model bundle in a private directory and read every artifact.
///
/// The directory is removed before this returns. The report artifact carries the
/// same flat report the public route computes, rendered with the config-declared
/// data version exactly as the coupled `backtest run` writer does. The public
/// object names the content digest instead, so the two envelopes differ while
/// every model number is equal.
///
/// # Errors
///
/// Returns [`BundleError`] when the ladder is empty, the dataset cannot feed the
/// model, the period selects no candle, or an artifact cannot be rendered.
pub(crate) fn build(
    run_config: &RunConfig,
    model_config: &ModelConfig,
    content: &[u8],
    data_version: &Digest,
    config_hash: &Digest,
    image_digest: Option<&Digest>,
) -> Result<BuiltBundle, BundleError> {
    let mut ladder = model_config.ladder.clone();
    ladder.sort_by_key(|frame| frame.seconds());
    ladder.dedup();
    if ladder.is_empty() {
        return Err(BundleError::EmptyLadder);
    }
    let ladder_codes: Vec<String> = ladder.iter().map(|frame| frame.code().to_owned()).collect();

    let set = series(content, model_config.data.max_candles).map_err(BundleError::Series)?;
    let model = pipeline::model_run_from_set_with_ladder(&set, &ladder, &model_config.model);

    let staging = Staging::new();
    let output =
        model_bundle_from(run_config, staging.path(), &model).map_err(|error| match error {
            ModelBundleError::EmptyPeriod { .. } => BundleError::EmptyPeriod,
            other => BundleError::Output(other.to_string()),
        })?;
    let root = staging.path().join("model");
    let mut artifacts = Vec::with_capacity(output.names.len());
    for name in &output.names {
        let bytes = std::fs::read(root.join(name))
            .map_err(|error| BundleError::Output(format!("read {name}: {error}")))?;
        artifacts.push((name.clone(), bytes));
    }
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));

    let prefix = prefix(data_version, config_hash, image_digest, &ladder_codes);
    let manifest_key = format!("{prefix}/manifest.json");
    Ok(BuiltBundle {
        key_prefix: prefix,
        manifest_key,
        ladder: ladder_codes,
        artifacts,
    })
}

/// The content-addressed key prefix for one model bundle.
///
/// The image identity is part of the key because the bundle bytes include
/// presentation words (for example `summary.txt`); a rebuilt image must not
/// collide with a bundle written by an older one.
fn prefix(
    data_version: &Digest,
    config_hash: &Digest,
    image_digest: Option<&Digest>,
    ladder: &[String],
) -> String {
    let image = image_digest.map_or_else(
        || "no-image".to_owned(),
        |digest| digest.as_hex().to_owned(),
    );
    format!(
        "model/{}/{}/{}/{}/{}/bundle",
        data_version.as_hex(),
        config_hash.as_hex(),
        MODEL_VERSION,
        image,
        ladder.join("-")
    )
}

/// Store every artifact and publish the manifest, or return the equal manifest
/// that is already present.
///
/// A stored artifact is read back and hashed, so a silently corrupted object
/// fails the bundle instead of feeding the worker.
///
/// # Errors
///
/// Returns [`BundleError::Store`] when a read, write, or hash comparison fails.
pub(crate) async fn publish(
    objects: &dyn ObjectStore,
    built: &BuiltBundle,
    data_version: Digest,
    config_id: String,
    config_hash: Digest,
) -> Result<ModelBundleManifest, BundleError> {
    let mut refs = Vec::with_capacity(built.artifacts.len());
    for (name, bytes) in &built.artifacts {
        let digest = Digest::sha256(bytes);
        let key = built.artifact_key(name);
        let present = objects
            .exists(&key)
            .await
            .map_err(|error| BundleError::Store(error.to_string()))?;
        if present {
            let stored = objects
                .get(&key)
                .await
                .map_err(|error| BundleError::Store(error.to_string()))?;
            if Digest::sha256(&stored) != digest || stored.len() != bytes.len() {
                return Err(BundleError::Store(format!(
                    "the stored artifact {key} does not match the computed bundle"
                )));
            }
        } else {
            objects
                .put(&key, bytes.clone())
                .await
                .map_err(|error| BundleError::Store(error.to_string()))?;
        }
        if name.starts_with("simulation-") && name.ends_with(".json") {
            let simulation_digest = Digest::sha256(bytes);
            let simulation_key = quantrule_api::simulation::bundle_key(&simulation_digest);
            let present = objects
                .exists(&simulation_key)
                .await
                .map_err(|error| BundleError::Store(error.to_string()))?;
            if present {
                let stored = objects
                    .get(&simulation_key)
                    .await
                    .map_err(|error| BundleError::Store(error.to_string()))?;
                if Digest::sha256(&stored) != simulation_digest || stored.len() != bytes.len() {
                    return Err(BundleError::Store(format!(
                        "the stored simulation bundle {simulation_key} does not match"
                    )));
                }
            } else {
                objects
                    .put(&simulation_key, bytes.clone())
                    .await
                    .map_err(|error| BundleError::Store(error.to_string()))?;
            }
        }
        refs.push(ArtifactRef {
            name: name.clone(),
            bytes: bytes.len() as u64,
            sha256: digest,
        });
    }

    let manifest = ModelBundleManifest {
        schema: MODEL_BUNDLE_SCHEMA.to_owned(),
        data_version,
        config_id,
        config_hash,
        model_version: MODEL_VERSION.to_owned(),
        ladder: built.ladder.clone(),
        manifest_key: built.manifest_key.clone(),
        artifacts: refs,
    };
    let encoded =
        serde_json::to_vec(&manifest).map_err(|error| BundleError::Output(error.to_string()))?;

    let present = objects
        .exists(&built.manifest_key)
        .await
        .map_err(|error| BundleError::Store(error.to_string()))?;
    if present {
        let stored = objects
            .get(&built.manifest_key)
            .await
            .map_err(|error| BundleError::Store(error.to_string()))?;
        let decoded: ModelBundleManifest = serde_json::from_slice(&stored)
            .map_err(|error| BundleError::Store(format!("decode the manifest: {error}")))?;
        if decoded != manifest {
            return Err(BundleError::Store(
                "the stored manifest does not match the computed bundle".to_owned(),
            ));
        }
        return Ok(decoded);
    }
    objects
        .put(&built.manifest_key, encoded)
        .await
        .map_err(|error| BundleError::Store(error.to_string()))?;
    Ok(manifest)
}

/// A private directory that removes itself.
struct Staging(PathBuf);

impl Staging {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!(
            "quantrule-model-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_image_identity_is_part_of_the_bundle_key() {
        let data = Digest::sha256(b"data");
        let config = Digest::sha256(b"config");
        let ladder = ["1m".to_owned()];
        let first_image = Digest::sha256(b"image-a");
        let second_image = Digest::sha256(b"image-b");
        let first = prefix(&data, &config, Some(&first_image), &ladder);
        let second = prefix(&data, &config, Some(&second_image), &ladder);
        let none = prefix(&data, &config, None, &ladder);

        assert_ne!(first, second);
        assert!(first.contains(first_image.as_hex()));
        assert!(none.contains("/no-image/"));
    }
}
