//! Metadata-only computed-artifact registration.
//!
//! The signal writer stores canonical bytes directly in the object store, then
//! asks the data service to record the catalog row. The request carries the byte
//! count only; the data service reads and hashes the object before it publishes
//! a row, so the catalog never points at unverified bytes.
//!
//! This is deliberately separate from the compatibility body PUT route. A
//! signal artifact can exceed the 64 MiB request cap, and sending it over HTTP
//! would make the transport, not the object store, the byte authority.

use quantrule_api::ArtifactRecord;
use serde::{Deserialize, Serialize};

/// One metadata-only artifact registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRegistrationRequest {
    /// The byte count of the object already stored at `artifacts/<hex>`.
    pub bytes: u64,
}

/// The stored artifact row and whether this call created it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRegistration {
    /// The stored catalog row.
    pub record: ArtifactRecord,
    /// Whether this call created the row.
    pub created: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{ArtifactKind, ArtifactSummary, Digest};

    #[test]
    fn the_registration_request_round_trips_and_rejects_unknown_fields() {
        let request = ArtifactRegistrationRequest { bytes: 128 };
        let text = serde_json::to_string(&request).expect("encode");
        assert_eq!(
            serde_json::from_str::<ArtifactRegistrationRequest>(&text).expect("decode"),
            request
        );
        assert!(
            serde_json::from_str::<ArtifactRegistrationRequest>(r#"{"bytes":1,"id":"x"}"#).is_err()
        );
    }

    #[test]
    fn the_registration_response_round_trips() {
        let id = Digest::sha256(b"signal");
        let response = ArtifactRegistration {
            record: ArtifactRecord {
                summary: ArtifactSummary {
                    artifact_id: id.clone(),
                    kind: ArtifactKind::Signal,
                    bytes: 128,
                    created_at: "2026-11-22T00:00:00Z".to_owned(),
                },
                object_key: format!("artifacts/{}", id.as_hex()),
                sha256: id,
            },
            created: false,
        };
        let text = serde_json::to_string(&response).expect("encode");
        assert_eq!(
            serde_json::from_str::<ArtifactRegistration>(&text).expect("decode"),
            response
        );
    }
}
