//! The model service contract.
//!
//! A model object is content-addressed JSON over one dataset and one model
//! configuration. The service returns a reference, never a reading series.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// A request to compute the model for one dataset and configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelComputeRequest {
    /// The dataset to consume.
    pub data_version: Digest,
    /// The run config that supplies the model configuration and ladder.
    pub config_id: String,
    /// A subset of the configured ladder; absent means the whole ladder.
    #[serde(default)]
    pub timeframes: Option<Vec<String>>,
}

/// The stored model object returned by the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelObject {
    /// The dataset consumed.
    pub data_version: Digest,
    /// The configuration applied.
    pub config_id: String,
    /// The canonical configuration hash.
    pub config_hash: Digest,
    /// The model registry version.
    pub model_version: String,
    /// The computed timeframes, ascending.
    pub timeframes: Vec<String>,
    /// The CAS key.
    pub object_key: String,
    /// The hash of the stored bytes.
    pub sha256: Digest,
    /// The stored byte count.
    pub bytes: u64,
}
