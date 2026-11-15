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

/// One published event record.
///
/// Every field is a number or a code. The client maps a code to a word; the
/// service never renders a label. `occurred_bar` is the bar on which the
/// relationship happened and `available_bar` is the first closed bar on which
/// it became knowable, so a record never reaches an earlier bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRecord {
    /// The stable registry event code.
    pub event_code: u16,
    /// The event type: 1 level, 2 divergence, 3 signal.
    pub event_type: u8,
    /// The type's compact subtype code.
    pub event_subtype: u8,
    /// The stable source code of the reading that produced it.
    pub source: u16,
    /// The bar on which the underlying relationship occurred.
    pub occurred_bar: u64,
    /// The first closed bar on which it became knowable.
    pub available_bar: u64,
    /// The implication: -1 bearish, 0 neutral, +1 bullish.
    pub direction: i8,
    /// The identified EMA or confirmed swing level, when applicable.
    pub reference_code: Option<u32>,
    /// The main measured value at the occurrence.
    pub primary_value: Option<f64>,
    /// The compared level or value, when applicable.
    pub reference_value: Option<f64>,
}

/// The event batch for one dataset and timeframe.
///
/// A batch is the full event set the model publishes for the timeframe, in
/// availability order. It is evidence and is never truncated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventBatch {
    /// The dataset inspected.
    pub data_version: Digest,
    /// The timeframe code the batch belongs to.
    pub timeframe: String,
    /// Every record, in availability order.
    pub events: Vec<EventRecord>,
}
