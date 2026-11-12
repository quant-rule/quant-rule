//! Contract version constants.
//!
//! A version is a string, not a negotiation. A service that cannot speak the
//! current version refuses the request rather than guessing.

/// The HTTP contract spoken by the gateway and every client.
pub const API_VERSION: &str = "quantrule.api/v1";

/// The run manifest recorded by the evidence service.
pub const RUN_CONTRACT: &str = "quantrule.run/v1";

/// The dataset record returned by the data service.
pub const DATASET_CONTRACT: &str = "quantrule.dataset/v1";

/// The evidence bundle written by the evidence service.
pub const BUNDLE_CONTRACT: &str = "quantrule.bundle/v1";

/// The control-plane message envelope.
pub const QUEUE_CONTRACT: &str = "quantrule.queue/v1";

/// The numeric-code-to-word table generated for the clients.
pub const LABELS_CONTRACT: &str = "quantrule.labels/v1";

/// The deployment topology contract.
pub const DEPLOY_CONTRACT: &str = "quantrule.deploy/v1";

/// Every contract version, for `/version` and for provenance records.
pub const ALL_CONTRACTS: [(&str, &str); 7] = [
    ("api", API_VERSION),
    ("run", RUN_CONTRACT),
    ("dataset", DATASET_CONTRACT),
    ("bundle", BUNDLE_CONTRACT),
    ("queue", QUEUE_CONTRACT),
    ("labels", LABELS_CONTRACT),
    ("deploy", DEPLOY_CONTRACT),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_contract_carries_a_version_suffix() {
        for (name, value) in ALL_CONTRACTS {
            assert!(value.starts_with("quantrule."), "{name} is not namespaced");
            assert!(value.ends_with("/v1"), "{name} is not versioned");
        }
    }
}
