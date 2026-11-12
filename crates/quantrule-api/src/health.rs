//! Health and version reporting.
//!
//! `/health` is the only health authority. It reports each dependency, not just
//! whether a container is running.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;
use crate::version::{ALL_CONTRACTS, API_VERSION};

/// The state of one dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    /// The check passed.
    Healthy,
    /// The check passed slowly or partially.
    Degraded,
    /// The check failed.
    Down,
}

impl ServiceState {
    /// Whether the stack can serve requests.
    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }
}

/// One dependency in the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// The service name.
    pub name: String,
    /// Its state.
    pub state: ServiceState,
    /// A short detail, present when not healthy.
    pub detail: Option<String>,
}

/// The aggregate report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    /// The contract version.
    pub v: String,
    /// The worst state across the dependencies.
    pub state: ServiceState,
    /// Every dependency.
    pub services: Vec<ServiceHealth>,
    /// Messages waiting to be consumed.
    pub queue_depth: u64,
    /// Worker instances that report a heartbeat.
    pub workers: u64,
    /// The image digest that serves the request.
    pub image_digest: Option<Digest>,
}

impl HealthReport {
    /// The worst state across the services, which is the report's state.
    #[must_use]
    pub fn aggregate(services: Vec<ServiceHealth>) -> ServiceState {
        if services
            .iter()
            .any(|entry| entry.state == ServiceState::Down)
        {
            return ServiceState::Down;
        }
        if services
            .iter()
            .any(|entry| entry.state == ServiceState::Degraded)
        {
            return ServiceState::Degraded;
        }
        ServiceState::Healthy
    }
}

/// What the server is and what it speaks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionInfo {
    /// The product version, for example `1.0.0`.
    pub product: String,
    /// The HTTP contract version.
    pub api: String,
    /// The image digest that serves the request.
    pub image_digest: Option<Digest>,
    /// Every contract and its version.
    pub contracts: Vec<(String, String)>,
}

impl VersionInfo {
    /// Build the report from the compile-time contract constants.
    #[must_use]
    pub fn current(product: impl Into<String>, image_digest: Option<Digest>) -> Self {
        Self {
            product: product.into(),
            api: API_VERSION.to_owned(),
            image_digest,
            contracts: ALL_CONTRACTS
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn service(name: &str, state: ServiceState) -> ServiceHealth {
        ServiceHealth {
            name: name.to_owned(),
            state,
            detail: None,
        }
    }

    #[test]
    fn aggregate_takes_the_worst_state() {
        assert_eq!(
            HealthReport::aggregate(vec![
                service("pg", ServiceState::Healthy),
                service("s3", ServiceState::Healthy),
            ]),
            ServiceState::Healthy
        );
        assert_eq!(
            HealthReport::aggregate(vec![
                service("pg", ServiceState::Healthy),
                service("s3", ServiceState::Degraded),
            ]),
            ServiceState::Degraded
        );
        assert_eq!(
            HealthReport::aggregate(vec![
                service("pg", ServiceState::Degraded),
                service("s3", ServiceState::Down),
            ]),
            ServiceState::Down
        );
    }

    #[test]
    fn availability_follows_the_aggregate() {
        assert!(ServiceState::Healthy.is_available());
        assert!(ServiceState::Degraded.is_available());
        assert!(!ServiceState::Down.is_available());
    }

    #[test]
    fn version_reports_every_contract() {
        let info = VersionInfo::current("1.0.0", None);
        assert_eq!(info.api, API_VERSION);
        assert_eq!(info.contracts.len(), ALL_CONTRACTS.len());
        assert!(
            info.contracts
                .iter()
                .all(|(_, value)| value.ends_with("/v1"))
        );
    }

    #[test]
    fn a_health_report_round_trips() {
        let report = HealthReport {
            v: API_VERSION.to_owned(),
            state: ServiceState::Healthy,
            services: vec![service("postgres", ServiceState::Healthy)],
            queue_depth: 0,
            workers: 2,
            image_digest: None,
        };
        let text = serde_json::to_string(&report).expect("encode");
        let back: HealthReport = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, report);
    }
}
