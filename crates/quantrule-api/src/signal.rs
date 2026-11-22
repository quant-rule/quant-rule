//! The signal request and response shared by the clients and services.
//!
//! A request names a frozen model artifact by its canonical id, the dataset and
//! configuration it was computed from, and a frozen signal-mode strategy pack.
//! The response returns the stored artifact record and whether this call created
//! the catalog row.
//!
//! The request has no schema field: the public surface is versioned by
//! [`crate::API_VERSION`], and the closed field set is the contract. The response
//! carries its own schema token so a caller can refuse a shape it does not
//! understand. The shell and the browser read the same wire bytes the strategy
//! service produces.

use serde::{Deserialize, Serialize};

use crate::artifact::ArtifactRecord;
use crate::digest::Digest;

/// The documented signal request contract.
pub const SIGNAL_REQUEST_SCHEMA: &str = "quantrule.signal-request/v1";
/// The signal response schema token.
pub const SIGNAL_RESPONSE_SCHEMA: &str = "quantrule.signal-response/v1";

/// One signal computation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalRequest {
    /// The frozen model artifact the signal reads.
    pub model_artifact_id: Digest,
    /// The dataset the model was computed from.
    pub data_version: Digest,
    /// The configuration the model was computed from.
    pub config_id: String,
    /// The frozen signal-mode strategy pack.
    pub strategy_pack_id: Digest,
}

/// One computed signal artifact and its catalog outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalResponse {
    /// The response schema token.
    pub schema: String,
    /// The stored signal artifact record.
    pub artifact: ArtifactRecord,
    /// Whether this call created the catalog row.
    pub created: bool,
}

impl SignalResponse {
    /// Build a response for one stored artifact.
    #[must_use]
    pub fn new(artifact: ArtifactRecord, created: bool) -> Self {
        Self {
            schema: SIGNAL_RESPONSE_SCHEMA.to_owned(),
            artifact,
            created,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArtifactKind, ArtifactSummary};

    fn digest(seed: &[u8]) -> Digest {
        Digest::sha256(seed)
    }

    fn record() -> ArtifactRecord {
        let id = digest(b"signal");
        ArtifactRecord {
            summary: ArtifactSummary {
                artifact_id: id.clone(),
                kind: ArtifactKind::Signal,
                bytes: 128,
                created_at: "2026-11-22T00:00:00Z".to_owned(),
            },
            object_key: format!("artifacts/{}", id.as_hex()),
            sha256: id,
        }
    }

    #[test]
    fn the_documented_request_contract_is_pinned() {
        assert_eq!(SIGNAL_REQUEST_SCHEMA, "quantrule.signal-request/v1");
        assert_eq!(SIGNAL_RESPONSE_SCHEMA, "quantrule.signal-response/v1");
    }

    #[test]
    fn the_request_round_trips() {
        let request = SignalRequest {
            model_artifact_id: digest(b"model"),
            data_version: digest(b"data"),
            config_id: "cfg_1".to_owned(),
            strategy_pack_id: digest(b"pack"),
        };
        let text = serde_json::to_string(&request).expect("encode");
        assert_eq!(
            serde_json::from_str::<SignalRequest>(&text).expect("decode"),
            request
        );
    }

    #[test]
    fn the_response_round_trips_and_names_its_schema() {
        let response = SignalResponse::new(record(), true);
        let text = serde_json::to_string(&response).expect("encode");
        assert!(text.contains(SIGNAL_RESPONSE_SCHEMA));
        assert_eq!(
            serde_json::from_str::<SignalResponse>(&text).expect("decode"),
            response
        );
    }

    #[test]
    fn an_unknown_request_or_response_field_is_rejected() {
        let request = serde_json::json!({
            "model_artifact_id": digest(b"model"),
            "data_version": digest(b"data"),
            "config_id": "cfg_1",
            "strategy_pack_id": digest(b"pack"),
            "path": "/tmp/signal.json"
        });
        assert!(serde_json::from_value::<SignalRequest>(request).is_err());

        let response = serde_json::json!({
            "schema": SIGNAL_RESPONSE_SCHEMA,
            "artifact": serde_json::to_value(record()).expect("record"),
            "created": true,
            "bytes": 1
        });
        assert!(serde_json::from_value::<SignalResponse>(response).is_err());
    }
}
