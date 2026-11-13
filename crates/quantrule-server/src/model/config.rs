//! Materialize a catalogued config bundle into the model inputs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_app::config::DataConfig;
use quantrule_app::config::RunConfig;
use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;

use crate::model::ModelError;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// The parts of a run config the model needs.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// The declared data ladder, session, and bounds.
    pub data: DataConfig,
    /// The model engine configuration.
    pub model: MarketStateConfig,
    /// The model ladder, or the full data ladder.
    pub ladder: Vec<Timeframe>,
}

/// Write a config bundle to a private directory and load it.
///
/// The directory is removed before this returns; only the parsed config values
/// survive.
///
/// # Errors
///
/// Returns [`ModelError::Config`] when a file cannot be written, the run config
/// does not validate, or the bundle is missing `run.json`.
pub fn load_config(
    files: &BTreeMap<String, String>,
    staging_root: &Path,
) -> Result<ModelConfig, ModelError> {
    if !files.contains_key("run.json") {
        return Err(ModelError::Config(
            "the config bundle has no run.json".to_owned(),
        ));
    }
    let root = staging_root.join(format!(
        "quantrule-model-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _staging = Staging(root.clone());
    std::fs::create_dir_all(&root)
        .map_err(|error| ModelError::Config(format!("create the staging directory: {error}")))?;
    for (relative, body) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ModelError::Config(format!("create {relative}: {error}")))?;
        }
        std::fs::write(&path, body)
            .map_err(|error| ModelError::Config(format!("write {relative}: {error}")))?;
    }
    let config = RunConfig::load(&root.join("run.json"))
        .map_err(|error| ModelError::Config(error.to_string()))?;
    let ladder = config
        .model_ladder
        .clone()
        .unwrap_or_else(|| config.data.available_timeframes.clone());
    Ok(ModelConfig {
        data: config.data,
        model: config.model,
        ladder,
    })
}

struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
